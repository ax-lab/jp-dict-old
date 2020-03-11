//! Internal data structures for the dictionary database.

extern crate bincode;
extern crate serde;
extern crate unicode_segmentation;

use std::collections::HashMap;
use std::io;

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

pub use bincode::Result;

#[derive(Default, Serialize, Deserialize)]
pub struct DB {
	pub tags: Vec<TagRow>,
	pub terms: Vec<TermRow>,
	pub kanji: Vec<KanjiRow>,

	pub strings: Vec<String>,

	// Index of sorted term/readings to japanese terms.
	index_prefix_jp: Vec<(usize, usize)>,

	// Index of sorted term/readings suffixes to japanese terms. Suffixes are
	// reversed strings.
	index_suffix_jp: Vec<(usize, usize)>,
}

impl DB {
	/// Load a DB instance from a slice of bytes.
	pub fn load(bytes: &[u8]) -> Result<DB> {
		bincode::deserialize(bytes)
	}

	/// Serialize the DB instance to a writer.
	pub fn serialize<W: io::Write>(&self, writer: W) -> Result<()> {
		bincode::serialize_into(writer, self)
	}

	/// Returns an interned string by index.
	pub fn string(&self, index: usize) -> &str {
		self.strings[index].as_str()
	}

	/// Interns a string into the database.
	pub fn intern(&mut self, value: String, indexes: &mut HashMap<String, usize>) -> usize {
		if self.strings.len() == 0 {
			// Make sure that the empty string is mapped to the zero index
			self.strings.push(String::new());
			indexes.insert(String::new(), 0);
		}
		if let Some(index) = indexes.get(&value) {
			*index
		} else {
			let next = self.strings.len();
			indexes.insert(value.clone(), next);
			self.strings.push(value);
			next
		}
	}

	/// Build all the indexes in the DB. Should be called after loading all data
	/// tables.
	pub fn build_indexes(&mut self, intern_indexes: &mut HashMap<String, usize>) {
		// Sort terms and kanji by their frequency so that we have a them
		// already ordered by relevancy:

		self.terms
			.sort_by(|a: &TermRow, b: &TermRow| -> std::cmp::Ordering {
				b.frequency.cmp(&a.frequency)
			});

		self.kanji
			.sort_by(|a: &KanjiRow, b: &KanjiRow| -> std::cmp::Ordering {
				b.frequency.cmp(&a.frequency)
			});

		// Build the index for prefix Japanese search. This is basically all
		// terms and readings sorted to that we can do a binary search.
		let mut index_prefix_jp = Vec::new();
		for (index, it) in self.terms.iter().enumerate() {
			if it.expression != 0 {
				index_prefix_jp.push((it.expression, index));
			}
			if it.reading != 0 {
				index_prefix_jp.push((it.reading, index));
			}
		}
		index_prefix_jp.sort_by(|a, b| self.strings[a.0].cmp(&self.strings[b.0]));
		self.index_prefix_jp = index_prefix_jp;

		// Build the index for suffix Japanese search. This is the same as the
		// prefix but with all keys reversed.
		let mut index_suffix_jp = self.index_prefix_jp.clone();
		for it in index_suffix_jp.iter_mut() {
			let s: String = self.strings[it.0].graphemes(true).rev().collect();
			let s = self.intern(s, intern_indexes);
			it.0 = s;
		}
		index_suffix_jp.sort_by(|a, b| self.strings[a.0].cmp(&self.strings[b.0]));
		self.index_suffix_jp = index_suffix_jp;
	}
}

#[derive(Default, Serialize, Deserialize)]
pub struct TagRow {
	pub name: usize,
	pub category: usize,
	pub order: i32,
	pub notes: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct KanjiRow {
	pub character: char,
	pub frequency: Option<u32>,
	pub meanings: Vec<usize>,
	pub onyomi: Vec<usize>,
	pub kunyomi: Vec<usize>,
	pub tags: Vec<usize>,
	pub stats: HashMap<usize, usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TermRow {
	pub expression: usize,
	pub reading: usize,
	pub score: i32,
	pub sequence: i32,
	pub frequency: Option<u32>,
	pub glossary: Vec<usize>,
	pub rules: Vec<usize>,
	pub term_tags: Vec<usize>,
	pub definition_tags: Vec<usize>,
}
