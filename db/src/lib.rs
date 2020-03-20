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

		let mut raw = Raw::default();
		let mut vector_data: Vec<u32> = Vec::new();

		let mut push_vec = |mut vec: Vec<u32>| -> VecHandle {
			if vec.len() == 0 {
				VecHandle {
					offset: 0u32.into(),
					length: 0u32.into(),
				}
			} else {
				let offset = vector_data.len() as u32;
				let length = vec.len() as u32;
				vector_data.append(&mut vec);
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

/// Raw database structure.
pub struct DB<'a> {
	tags: &'a [TagRaw],
	terms: &'a [TermRaw],
	kanji: &'a [KanjiRaw],
	index_prefix_jp: &'a [TermIndex],
	index_suffix_jp: &'a [TermIndex],
	index_chars_jp: &'a [CharIndex],
	vector_data: &'a [RawUint32],
	string_list: &'a [StrHandle],
	string_data: &'a str,
}

impl<'a> DB<'a> {
	pub fn load(data: &'a [u8]) -> DB<'a> {
		unsafe {
			let (tags, data) = read_slice::<TagRaw>(data);
			let (terms, data) = read_slice::<TermRaw>(data);
			let (kanji, data) = read_slice::<KanjiRaw>(data);
			let (index_prefix_jp, data) = read_slice::<TermIndex>(data);
			let (index_suffix_jp, data) = read_slice::<TermIndex>(data);
			let (index_chars_jp, data) = read_slice::<CharIndex>(data);
			let (vector_data, data) = read_slice::<RawUint32>(data);
			let (string_list, data) = read_slice::<StrHandle>(data);
			let (string_data, _) = read_slice::<u8>(data);
			let string_data = std::str::from_utf8_unchecked(string_data);
			DB {
				tags: tags,
				terms: terms,
				kanji: kanji,
				index_prefix_jp: index_prefix_jp,
				index_suffix_jp: index_suffix_jp,
				index_chars_jp: index_chars_jp,
				vector_data: vector_data,
				string_list: string_list,
				string_data: string_data,
			}
		}
	}
}

impl<'a> DB<'a> {
	pub fn check(&self) {
		let start = Instant::now();

		for tag in self.tags.iter() {
			self.check_string(tag.name, "tag name");
			self.check_string(tag.category, "tag category");
			self.check_string(tag.notes, "tag notes");
		}

		for term in self.terms.iter() {
			self.check_string(term.expression, "term expression");
			self.check_string(term.reading, "term reading");
			self.check_string(term.search_key, "term search key");
			self.check_vector_strings(term.glossary, "term glossary");
			self.check_vector_tags(term.rules, "term rules");
			self.check_vector_tags(term.term_tags, "term tags");
			self.check_vector_tags(term.definition_tags, "term definition tags");
		}

		for kanji in self.kanji.iter() {
			self.check_vector_strings(kanji.meanings, "kanji meanings");
			self.check_vector_strings(kanji.onyomi, "kanji onyomi");
			self.check_vector_strings(kanji.kunyomi, "kanji kunyomi");
			self.check_vector_tags(kanji.tags, "kanji tags");

			self.check_vector(kanji.stats, "kanji stats");
			let (sta, end) = kanji.stats.range();
			let mut iter = self.vector_data[sta..end].iter();
			while let Some(&stat_tag) = iter.next() {
				let stat_tag: u32 = stat_tag.into();
				let stat_tag = stat_tag as usize;
				let stat_val = iter.next().expect("kanji stat tag missing value");
				assert!(stat_tag <= self.tags.len(), "kanji stat tag out of bounds");
				self.check_string(*stat_val, "kanji stat value");
			}
		}

		for row in self.index_prefix_jp.iter() {
			self.check_term_index(*row, "prefix index");
		}

		for row in self.index_suffix_jp.iter() {
			self.check_term_index(*row, "suffix index");
		}

		let mut chars_cnt = 0;
		let mut chars_max = 0;
		for row in self.index_chars_jp.iter() {
			let count: u32 = row.indexes.length.into();
			let count = count as usize;
			chars_cnt += count;
			chars_max = std::cmp::max(chars_max, count);
			self.check_vector_terms(row.indexes, "index chars row");
		}
		let chars_len = self.index_chars_jp.len();
		let chars_avg = chars_cnt / chars_len;

		for (index, s) in self.string_list.iter().enumerate() {
			let (sta, end) = s.range();
			assert!(
				sta <= self.string_data.len(),
				"string #{}: string start out of bounds",
				index + 1
			);
			assert!(
				end <= self.string_data.len(),
				"string #{}: string end out of bounds",
				index + 1
			);
		}

		println!("Database check finished (elapsed {:?})", start.elapsed());
		println!(
			"-> {} terms / {} kanji / {} tags",
			self.terms.len(),
			self.kanji.len(),
			self.tags.len()
		);
		println!(
			"-> {} indexed terms / {} chars ({} avg / {} max / {} total)",
			self.index_chars_jp.len(),
			chars_len,
			chars_avg,
			chars_max,
			chars_cnt,
		);
		println!(
			"-> {} vector data",
			bytes(self.vector_data.len() * std::mem::size_of::<u32>())
		);
		println!(
			"-> {} string data ({} strings)",
			bytes(self.string_data.len()),
			self.string_list.len()
		);
	}

	fn check_term_index(&self, row: TermIndex, name: &str) {
		self.check_string(row.key, name);
		let index: u32 = row.term.into();
		let index = index as usize;
		assert!(index <= self.terms.len(), "{}: term out of bounds", name);
	}

	fn check_string(&self, index: RawUint32, name: &str) {
		let index: u32 = index.into();
		let index = index as usize;
		assert!(
			index < self.string_list.len(),
			"{}: string index out of bounds",
			name
		);
	}

	fn check_vector_strings(&self, vec: VecHandle, name: &str) {
		self.check_vector(vec, name);
		let (sta, end) = vec.range();
		let name = format!("{} string index:", name);
		let name = name.as_str();
		for &index in self.vector_data[sta..end].iter() {
			self.check_string(index, name);
		}
	}

	fn check_vector_tags(&self, vec: VecHandle, name: &str) {
		self.check_vector(vec, name);
		let (sta, end) = vec.range();
		for &index in self.vector_data[sta..end].iter() {
			let index: u32 = index.into();
			let index = index as usize;
			assert!(index < self.tags.len(), "{}: tag index out of bounds", name);
		}
	}

	fn check_vector_terms(&self, vec: VecHandle, name: &str) {
		self.check_vector(vec, name);
		let (sta, end) = vec.range();
		for &index in self.vector_data[sta..end].iter() {
			let index: u32 = index.into();
			let index = index as usize;
			assert!(
				index < self.terms.len(),
				"{}: term index out of bounds",
				name
			);
		}
	}

	fn check_vector(&self, vec: VecHandle, name: &str) {
		let (sta, end) = vec.range();
		assert!(
			sta <= self.vector_data.len(),
			"{}: vector start out of bounds",
			name
		);
		assert!(
			end <= self.vector_data.len(),
			"{}: vector end out of bounds",
			name
		);
	}
}

/// Raw database structure used for building and writing the database.
#[derive(Default)]
struct Raw {
	tags: Vec<TagRaw>,
	terms: Vec<TermRaw>,
	kanji: Vec<KanjiRaw>,
	index_prefix_jp: Vec<TermIndex>,
	index_suffix_jp: Vec<TermIndex>,
	index_chars_jp: Vec<CharIndex>,
	vector_data: Vec<u32>,
	string_list: Vec<StrHandle>,
	string_data: String,
}

impl Raw {
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

#[derive(Copy, Clone)]
struct RawUint32(u32);

impl std::convert::From<u32> for RawUint32 {
	#[inline]
	fn from(item: u32) -> Self {
		Self(item.to_le())
	}
}

impl std::convert::Into<u32> for RawUint32 {
	#[inline]
	fn into(self) -> u32 {
		u32::from_le(self.0)
	}
}

#[derive(Copy, Clone)]
struct RawInt32(i32);

impl std::convert::From<i32> for RawInt32 {
	#[inline]
	fn from(item: i32) -> Self {
		Self(item.to_le())
	}
}

impl std::convert::Into<i32> for RawInt32 {
	#[inline]
	fn into(self) -> i32 {
		i32::from_le(self.0)
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
#[derive(Copy, Clone)]
struct TermIndex {
	key: RawUint32,
	term: RawUint32,
}

/// Serialized row for a character index.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct CharIndex {
	character: RawUint32,
	indexes: VecHandle,
}

/// Handle for a serialized string.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct StrHandle {
	offset: RawUint32,
	length: RawUint32,
}

impl StrHandle {
	fn range(&self) -> (usize, usize) {
		let offset: u32 = self.offset.into();
		let length: u32 = self.length.into();
		(offset as usize, (offset + length) as usize)
	}
}

/// Handle for a serialized vector.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct VecHandle {
	offset: RawUint32,
	length: RawUint32,
}

impl VecHandle {
	fn range(&self) -> (usize, usize) {
		let offset: u32 = self.offset.into();
		let length: u32 = self.length.into();
		(offset as usize, (offset + length) as usize)
	}
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

#[inline]
unsafe fn read_slice<U>(src: &[u8]) -> (&[U], &[u8]) {
	const U32_LEN: usize = std::mem::size_of::<u32>();

	assert!(src.len() >= U32_LEN);
	let count: &[u32] = cast_slice(&src[0..U32_LEN]);
	let count = u32::from_le(count[0]) as usize;
	let src = &src[U32_LEN..];

	let item_size = std::mem::size_of::<U>();
	let data_size = item_size * count;
	let data = &src[..data_size];
	let next = &src[data_size..];
	(cast_slice(data), next)
}

#[inline]
unsafe fn cast_slice<T, U>(src: &[T]) -> &[U] {
	let data_size = std::mem::size_of_val(src);
	let item_size = std::mem::size_of::<U>();
	assert_eq!(data_size % item_size, 0);
	std::slice::from_raw_parts(src.as_ptr() as *const U, data_size / item_size)
}

fn bytes(value: usize) -> String {
	if value == 1 {
		String::from("1 byte")
	} else if value < 1024 {
		format!("{} bytes", value)
	} else if value < 1024 * 1024 {
		let kb = (value as f64) / 1024.0;
		format!("{:.2} KB", kb)
	} else {
		let mb = (value as f64) / (1024.0 * 1024.0);
		format!("{:.2} MB", mb)
	}
}
