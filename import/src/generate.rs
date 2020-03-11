//! Data structures for the organized dictionary data.

use std::collections::HashMap;
use std::fs;

use std::io::BufWriter;

use crate::dict::{Dict, Kanji, Tag, Term};
use db::{KanjiRow, TagRow, TermRow, DB};

pub use db::Result;

#[derive(Default)]
pub struct Wrapper {
	db: DB,

	/// Frequency map of terms to number of appearances.
	freq_terms: HashMap<String, u32>,

	/// Frequency map of kanji to number of appearances.
	freq_kanji: HashMap<String, u32>,

	/// Map of string to interned string IDs. Used only during build.
	intern_indexes: HashMap<String, usize>,

	/// Map interned string IDs to the respective `db.tags` index
	tag_map: HashMap<usize, usize>,
}

impl Wrapper {
	/// Imports dictionary data into the dictionary.
	pub fn import_dict(&mut self, dict: Dict) {
		for it in dict.tags {
			self.import_tag(it);
		}

		for it in dict.meta_terms {
			self.freq_terms.insert(it.expression, it.data);
		}

		for it in dict.meta_kanji {
			self.freq_kanji.insert(it.expression, it.data);
		}

		for it in dict.terms {
			self.import_term(it);
		}

		for it in dict.kanji {
			self.import_kanji(it);
		}
	}

	/// Post-processing after finishing all `import_dict` calls.
	pub fn finish_import(&mut self) {
		let start = std::time::Instant::now();
		println!("\n>>> Building indexes...");

		for it in self.db.terms.iter_mut() {
			it.frequency = self
				.freq_terms
				.get(&self.db.strings[it.expression])
				.cloned();
		}

		for it in self.db.kanji.iter_mut() {
			it.frequency = self.freq_kanji.get(&it.character.to_string()).cloned();
		}

		self.db.build_indexes(&mut self.intern_indexes);
		println!("... Finished building in {:?}", start.elapsed());
	}

	/// Dumps information about the database to the console.
	pub fn dump_info(&self) {
		let str_bytes = self.db.strings.iter().map(|x| x.len()).sum();
		println!(
			"- Strings: {}\t~ {:>9}",
			self.db.strings.len(),
			bytes(str_bytes)
		);
	}

	/// Outputs all data to code files.
	pub fn output(&self) -> Result<()> {
		println!("... writing data/dictionary.in...");
		let mut output = BufWriter::new(fs::File::create("data/dictionary.in")?);
		self.db.serialize(&mut output)
	}

	fn import_term(&mut self, term: Term) {
		let row = TermRow {
			expression: self.intern(term.expression),
			reading: self.intern(term.reading),
			score: term.score,
			sequence: term.sequence,
			glossary: self.intern_all(term.glossary),
			frequency: None,
			rules: self.get_tags(term.rules),
			term_tags: self.get_tags(term.term_tags),
			definition_tags: self.get_tags(term.definition_tags),
		};
		self.db.terms.push(row);
	}

	fn import_kanji(&mut self, kanji: Kanji) {
		let row = KanjiRow {
			character: kanji.character,
			meanings: self.intern_all(kanji.meanings),
			onyomi: self.intern_all(kanji.onyomi),
			kunyomi: self.intern_all(kanji.kunyomi),
			frequency: None,
			tags: self.get_tags(kanji.tags),
			stats: kanji
				.stats
				.into_iter()
				.map(|(k, v)| (self.intern(k), self.intern(v)))
				.collect(),
		};
		self.db.kanji.push(row);
	}

	fn import_tag(&mut self, tag: Tag) {
		let name_id = self.intern(tag.name);
		let category_id = self.intern(tag.category);
		if let Some(&old_tag_id) = self.tag_map.get(&name_id) {
			let old_tag = &mut self.db.tags[old_tag_id];
			if tag.notes.len() > 0 && tag.notes != old_tag.notes {
				if old_tag.notes.len() > 0 {
					old_tag.notes = format!("{} / {}", old_tag.notes, tag.notes);
				} else {
					old_tag.notes = tag.notes;
				}
			}
			if category_id != 0 && category_id != old_tag.category {
				if old_tag.category != 0 {
					eprintln!(
						"WARNING: overridden category of tag `{}` (was `{}`, with `{}`)",
						&self.db.strings[name_id],
						&self.db.strings[old_tag.category],
						&self.db.strings[category_id]
					)
				}
				old_tag.category = category_id;
			}
		} else {
			let row = TagRow {
				name: name_id,
				category: category_id,
				order: tag.order,
				notes: tag.notes,
			};
			let row_id = self.db.tags.len();
			self.db.tags.push(row);
			self.tag_map.insert(name_id, row_id);
		}
	}

	fn get_tags(&mut self, tags: Vec<String>) -> Vec<usize> {
		tags.into_iter().map(|x| self.get_tag(x)).collect()
	}

	fn get_tag(&mut self, name: String) -> usize {
		let name_id = self.intern(name);
		if let Some(&tag_id) = self.tag_map.get(&name_id) {
			tag_id
		} else {
			let row = TagRow {
				name: name_id,
				category: 0,
				order: 0,
				notes: String::new(),
			};
			let row_id = self.db.tags.len();
			self.db.tags.push(row);
			self.tag_map.insert(name_id, row_id);
			row_id
		}
	}

	fn intern_all(&mut self, values: Vec<String>) -> Vec<usize> {
		values.into_iter().map(|x| self.intern(x)).collect()
	}

	fn intern(&mut self, value: String) -> usize {
		self.db.intern(value, &mut self.intern_indexes)
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
