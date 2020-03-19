//! Internal data structures for the dictionary database.

extern crate unicode_segmentation;

use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::time::Instant;

use unicode_segmentation::UnicodeSegmentation;

use std::io::Result;

/// Writer helper for the database.
///
/// Writing the database happens in the following phases:
/// - All tags are added to the writer using `push_tag`.
/// - Terms and kanji are added using `push_term` and `push_kanji` methods.
///   - Term and kanji tags must be converted to their respective indexes
///     using `get_tag` or `get_tags`.
/// - The database is written using `write`. During the write method indexes
///   are built and the database is output using a binary format designed to
///   be memory mapped on loading.
///
/// All strings used in tags, terms and kanji must be interned using the
/// `intern` method.
pub struct Writer {
	terms: Vec<TermData>,
	kanji: Vec<KanjiData>,

	tags: Vec<TagData>,
	tag_index: HashMap<String, u32>,

	string_list: Vec<(u32, u32)>,
	string_data: String,
	string_hash: HashMap<String, u32>,
}

impl Writer {
	/// Returns a new empty instance of a Writer.
	pub fn new() -> Writer {
		let mut out = Writer {
			terms: Default::default(),
			kanji: Default::default(),

			tags: Default::default(),
			tag_index: Default::default(),

			string_list: Default::default(),
			string_data: Default::default(),
			string_hash: Default::default(),
		};

		// Make sure the empty string is always interned as zero.
		out.intern(String::new());

		out
	}

	/// Add a new tag to write to the database.
	///
	/// All tags for the database should be added before trying to add terms and
	/// kanji that use those tags.
	pub fn push_tag(&mut self, tag: TagData) {
		let name = self.string(tag.name).to_string();
		self.tag_index.insert(name, self.tags.len() as u32);
		self.tags.push(tag);
	}

	/// Builds a `Vec<u32>` of tag indexes from a list of tag names.
	pub fn get_tags<T: IntoIterator<Item = S>, S: AsRef<str>>(&self, names: T) -> Vec<u32> {
		let mut out = Vec::new();
		for name in names.into_iter() {
			out.push(self.get_tag(name));
		}
		out
	}

	/// Returns a tag index from its name.
	pub fn get_tag<S: AsRef<str>>(&self, name: S) -> u32 {
		self.tag_index[name.as_ref()]
	}

	/// Add a new term to write to the database.
	pub fn push_term(&mut self, term: TermData) {
		self.terms.push(term);
	}

	/// Add a new kanji to write to the database.
	pub fn push_kanji(&mut self, kanji: KanjiData) {
		self.kanji.push(kanji);
	}

	/// Intern a string to the database and returns its serialized index.
	pub fn intern(&mut self, value: String) -> u32 {
		if let Some(&index) = self.string_hash.get(&value) {
			index
		} else {
			let offset = self.string_data.len() as u32;
			let length = value.len() as u32;
			let index = self.string_list.len() as u32;
			self.string_list.push((offset, length));
			self.string_data.push_str(value.as_str());
			self.string_hash.insert(value, index);
			index
		}
	}

	/// Return an interned string from its index.
	pub fn string(&self, index: u32) -> &str {
		let (offset, length) = self.string_list[index as usize];
		let sta = offset as usize;
		let end = sta + (length as usize);
		&self.string_data[sta..end]
	}

	/// Writes the database data to an `std::io::Write`.
	///
	/// The binary representation of the database is designed to be memory
	/// mapped on load. Note that `u32` are written in LE format.
	pub fn write<W: std::io::Write>(mut self, writer: &mut W) -> std::io::Result<()> {
		let start = Instant::now();

		//
		// Sort terms and kanji by relevance
		//

		self.terms.sort_by(|a, b| {
			if a.frequency != b.frequency {
				b.frequency.cmp(&a.frequency)
			} else {
				b.score.cmp(&a.score)
			}
		});

		self.kanji.sort_by(|a, b| b.frequency.cmp(&a.frequency));

		//
		// Build indexes
		//

		// The prefix index stores a one-to-one mapping of the japanese key
		// (expression, reading or key) to the term index. The keys are sorted
		// to enable a simple binary search for a prefix.

		let mut index_prefix_jp = Vec::new();
		for (i, it) in self.terms.iter().enumerate() {
			let index = i as u32;
			index_prefix_jp.push((it.expression, index));
			if it.reading > 0 {
				index_prefix_jp.push((it.reading, index));
			}
			if it.search_key > 0 {
				index_prefix_jp.push((it.search_key, index));
			}
		}

		index_prefix_jp.sort_by(|a, b| self.string(a.0).cmp(self.string(b.0)));

		// The suffix index is exactly like the prefix but keys are sorted by
		// the reverse string. When searching for a suffix, the search string
		// must be likewise reversed before performing the binary search.

		// We cache the reverse string to avoid having to recompute each
		// comparison
		let mut rev_strings: HashMap<u32, String> = HashMap::new();
		let mut rev = |index: u32| -> String {
			let entry = rev_strings
				.entry(index)
				.or_insert_with(|| self.string(index).graphemes(true).rev().collect());
			entry.clone()
		};

		// Clone the prefix index and sort by the reversed key
		let mut index_suffix_jp = index_prefix_jp.clone();
		index_suffix_jp.sort_by(|a, b| {
			let rev_a = rev(a.0);
			let rev_b = rev(b.0);
			rev_a.cmp(&rev_b)
		});

		// Per-character index used for "contains" style queries and fuzzy
		// searching.
		let mut index_chars_jp = HashMap::new();
		let mut total_indexes = 0;
		let mut max_indexes = 0;
		for (i, it) in self.terms.iter().enumerate() {
			let index = i as u32;
			let mut key = String::new();
			key.push_str(self.string(it.expression));
			key.push_str(self.string(it.reading));
			for chr in key.chars() {
				let entry = index_chars_jp.entry(chr).or_insert_with(|| HashSet::new());
				entry.insert(index);
			}
		}

		for (_key, entries) in index_chars_jp.iter() {
			total_indexes += entries.len();
			max_indexes = std::cmp::max(max_indexes, entries.len());
		}

		let num_char_keys = index_chars_jp.len();
		println!(
			"... built index in {:?} (terms = {}, chars = {} / avg {} / max {})",
			start.elapsed(),
			index_prefix_jp.len(),
			num_char_keys,
			total_indexes / num_char_keys,
			max_indexes,
		);

		//
		// Serialization
		//

		let start = Instant::now();

		let mut raw = DB::default();
		let mut vector_data: Vec<u32> = Vec::new();

		let mut push_vec = |vec: Vec<u32>| -> VecHandle {
			if vec.len() == 0 {
				VecHandle {
					offset: 0u32.into(),
					length: 0u32.into(),
				}
			} else {
				let offset = vector_data.len() as u32;
				let length = vec.len() as u32;
				vector_data.append(&mut vec.into_iter().map(|x| x.into()).collect::<Vec<u32>>());
				VecHandle {
					offset: offset.into(),
					length: length.into(),
				}
			}
		};

		for tag in self.tags {
			raw.tags.push(TagRaw {
				name: tag.name.into(),
				category: tag.category.into(),
				order: tag.order.into(),
				notes: tag.notes.into(),
			});
		}

		for kanji in self.kanji {
			raw.kanji.push(KanjiRaw {
				character: (kanji.character as u32).into(),
				frequency: kanji.frequency.into(),
				meanings: push_vec(kanji.meanings),
				onyomi: push_vec(kanji.onyomi),
				kunyomi: push_vec(kanji.kunyomi),
				tags: push_vec(kanji.tags),
				stats: push_vec(
					kanji
						.stats
						.into_iter()
						.flat_map(|x| vec![x.0, x.1])
						.collect(),
				),
			});
		}

		for term in self.terms {
			raw.terms.push(TermRaw {
				expression: term.expression.into(),
				reading: term.reading.into(),
				search_key: term.search_key.into(),
				score: term.score.into(),
				sequence: term.sequence.into(),
				frequency: term.frequency.into(),
				glossary: push_vec(term.glossary),
				rules: push_vec(term.rules),
				term_tags: push_vec(term.term_tags),
				definition_tags: push_vec(term.definition_tags),
			});
		}

		raw.index_prefix_jp = index_prefix_jp
			.into_iter()
			.map(|(key, term)| TermIndex {
				key: key.into(),
				term: term.into(),
			})
			.collect();

		raw.index_suffix_jp = index_suffix_jp
			.into_iter()
			.map(|(key, term)| TermIndex {
				key: key.into(),
				term: term.into(),
			})
			.collect();

		// Convert the chars index into a mappable format
		raw.index_chars_jp = index_chars_jp
			.into_iter()
			.map(|(key, val)| {
				let mut indexes = val.into_iter().collect::<Vec<_>>();
				indexes.sort();
				let indexes = push_vec(indexes);
				CharIndex {
					character: (key as u32).into(),
					indexes: indexes,
				}
			})
			.collect();

		raw.string_list = self
			.string_list
			.into_iter()
			.map(|(offset, length)| StrHandle {
				offset: offset.into(),
				length: length.into(),
			})
			.collect();
		raw.string_data = self.string_data;
		raw.vector_data = vector_data;

		println!("... prepared raw data in {:?}", start.elapsed());

		raw.write(writer)
	}
}

/// Tag data for writing.
pub struct TagData {
	/// Tag name (interned string).
	pub name: u32,
	/// Tag category (interned string).
	pub category: u32,
	/// Tag order. Can be used to sort the list of tags in a search result.
	pub order: i32,
	/// Tag notes (interned string).
	pub notes: u32,
}

/// Kanji data for writing.
pub struct KanjiData {
	/// Kanji character.
	pub character: char,
	/// Number of occurrences for the kanji in the frequency database. Zero if
	/// not available.
	pub frequency: u32,
	/// List of meanings for the kanji (interned strings).
	pub meanings: Vec<u32>,
	/// Onyomi readings for the kanji (interned strings).
	pub onyomi: Vec<u32>,
	/// Kunyomi readings for the kanji (interned strings).
	pub kunyomi: Vec<u32>,
	/// List of tags for the kanji.
	pub tags: Vec<u32>,
	/// Additional information for the kanji as a list of `(stat, info)` where
	/// the `stat` is a tag index and `info` is an interned string.
	pub stats: Vec<(u32, u32)>,
}

/// Term data for writing.
pub struct TermData {
	/// Main expression for the term.
	pub expression: u32,
	/// Reading for the term, if available.
	pub reading: u32,
	/// Search key provides an additional search key for the term. This is
	/// a filtered version of the expression or reading.
	pub search_key: u32,
	/// Score provides an additional attribute in which to order the terms in
	/// a search result.
	pub score: i32,
	/// Sequence number for the entry in the source dictionary.
	pub sequence: u32,
	/// Number of occurrences for the term in the frequency database (based only
	/// on the expression). Zero if not available.
	pub frequency: u32,
	/// English definitions for the term (interned strings).
	pub glossary: Vec<u32>,
	/// Semantic rules for the term (tag indexes).
	pub rules: Vec<u32>,
	/// Tag indexes for the japanese term.
	pub term_tags: Vec<u32>,
	/// Tag indexes for the english definition.
	pub definition_tags: Vec<u32>,
}

/// Raw database data.
#[derive(Default)]
pub struct DB {
	tags: Vec<TagRaw>,
	terms: Vec<TermRaw>,
	kanji: Vec<KanjiRaw>,
	index_prefix_jp: Vec<TermIndex>,
	index_suffix_jp: Vec<TermIndex>,
	index_chars_jp: Vec<CharIndex>,
	string_list: Vec<StrHandle>,
	string_data: String,
	vector_data: Vec<u32>,
}

impl DB {
	pub fn load(data: &[u8]) -> DB {
		// TODO: implement loading
		DB::default()
	}

	pub fn write<W: std::io::Write>(self, writer: &mut W) -> std::io::Result<()> {
		write_all(writer, self.tags)?;
		write_all(writer, self.terms)?;
		write_all(writer, self.kanji)?;
		write_all(writer, self.index_prefix_jp)?;
		write_all(writer, self.index_suffix_jp)?;
		write_all(writer, self.index_chars_jp)?;
		write_vec(writer, self.vector_data)?;
		write_all(writer, self.string_list)?;
		write_len(writer, self.string_data.len())?;
		writer.write(self.string_data.as_bytes())?;
		Ok(())
	}
}

struct RawUint32(u32);

impl std::convert::From<u32> for RawUint32 {
	#[inline]
	fn from(item: u32) -> Self {
		Self(item.to_le())
	}
}

struct RawInt32(i32);

impl std::convert::From<i32> for RawInt32 {
	#[inline]
	fn from(item: i32) -> Self {
		Self(item.to_le())
	}
}

/// Raw structure for a serialized Tag.
#[repr(C, packed)]
struct TagRaw {
	name: RawUint32,
	category: RawUint32,
	order: RawInt32,
	notes: RawUint32,
}

/// Raw structure for a serialized Kanji.
#[repr(C, packed)]
struct KanjiRaw {
	character: RawUint32,
	frequency: RawUint32,
	meanings: VecHandle,
	onyomi: VecHandle,
	kunyomi: VecHandle,
	tags: VecHandle,
	stats: VecHandle,
}

/// Raw structure for a serialized Term.
#[repr(C, packed)]
struct TermRaw {
	expression: RawUint32,
	reading: RawUint32,
	search_key: RawUint32,
	score: RawInt32,
	sequence: RawUint32,
	frequency: RawUint32,
	glossary: VecHandle,
	rules: VecHandle,
	term_tags: VecHandle,
	definition_tags: VecHandle,
}

/// Serialized row for a term index.
#[repr(C, packed)]
struct TermIndex {
	key: RawUint32,
	term: RawUint32,
}

/// Serialized row for a character index.
#[repr(C, packed)]
struct CharIndex {
	character: RawUint32,
	indexes: VecHandle,
}

/// Handle for a serialized string.
#[repr(C, packed)]
struct StrHandle {
	offset: RawUint32,
	length: RawUint32,
}

/// Handle for a serialized vector.
#[repr(C, packed)]
struct VecHandle {
	offset: RawUint32,
	length: RawUint32,
}

#[inline]
fn write_vec<W: io::Write>(writer: &mut W, vec: Vec<u32>) -> Result<()> {
	write_len(writer, vec.len())?;
	for val in vec {
		write_u32(writer, val)?;
	}
	Ok(())
}

#[inline]
fn write_len<W: io::Write>(writer: &mut W, value: usize) -> Result<()> {
	write_u32(writer, value as u32)
}

#[inline]
fn write_u32<W: io::Write>(writer: &mut W, value: u32) -> Result<()> {
	writer.write(&value.to_le_bytes())?;
	Ok(())
}

#[inline]
fn write_all<W: io::Write, L: IntoIterator<Item = T>, T: Sized>(
	writer: &mut W,
	values: L,
) -> Result<()> {
	let items = values.into_iter().collect::<Vec<T>>();
	write_len(writer, items.len())?;
	for it in items {
		write_raw(writer, &it)?;
	}
	Ok(())
}

#[inline]
fn write_raw<W: io::Write, T: Sized>(writer: &mut W, value: &T) -> Result<()> {
	let bytes = unsafe { to_bytes(value) };
	writer.write(bytes)?;
	Ok(())
}

#[inline]
unsafe fn to_bytes<T: Sized>(value: &T) -> &[u8] {
	std::slice::from_raw_parts((value as *const T) as *const u8, std::mem::size_of::<T>())
}
