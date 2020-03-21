//! Internal data structures for the dictionary database.

extern crate unicode_segmentation;

use std::time::Instant;

mod raw;
use raw::*;

mod data;
pub use data::*;

mod writer;
pub use writer::*;

mod search;
pub use search::*;

/// Root structure for the Japanese database.
///
/// The structure can be loaded from a binary blob using the [load](DB::load)
/// method.
///
/// [Writer] can be used to generate a binary blob for the database.
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

impl<'db> DB<'db> {
	pub fn term<'a: 'db>(&'a self, index: usize) -> Option<Term<'db, 'a>> {
		if index < self.terms.len() {
			Some(Term {
				pos: index,
				data: self,
				item: &self.terms[index],
			})
		} else {
			None
		}
	}

	fn get_tag<'a: 'db>(&'a self, index: RawUint32) -> Tag<'db, 'a> {
		let index: usize = index.into();
		Tag {
			data: self,
			item: &self.tags[index],
		}
	}

	fn get_tags<'a: 'db>(&'a self, tags: VecHandle) -> impl 'a + Iterator<Item = Tag<'db, 'a>> {
		let (sta, end) = tags.range();
		self.vector_data[sta..end]
			.iter()
			.map(move |&index| self.get_tag(index))
	}

	fn get_str(&self, index: RawUint32) -> &'db str {
		let index: usize = index.into();
		let string = &self.string_list[index];
		let (sta, end) = string.range();
		&self.string_data[sta..end]
	}
}

impl<'a> DB<'a> {
	/// Does a sanity check on the database structure and outputs some database
	/// statistics. This method is used only for debugging purposes.
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
			self.check_string(term.source, "term source");
			self.check_vector_strings(term.glossary, "term glossary");
			self.check_vector_tags(term.rules, "term rules");
			self.check_vector_tags(term.term_tags, "term tags");
			self.check_vector_tags(term.definition_tags, "term definition tags");
		}

		for kanji in self.kanji.iter() {
			self.check_vector_strings(kanji.meanings, "kanji meanings");
			self.check_vector_strings(kanji.onyomi, "kanji onyomi");
			self.check_vector_strings(kanji.kunyomi, "kanji kunyomi");
			self.check_string(kanji.source, "kanji source");
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
