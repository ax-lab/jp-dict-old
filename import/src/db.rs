//! Data structures for the organized dictionary data.

use std::collections::HashMap;

use crate::dict::{Dict, Tag, Term};

/// Root structure for dictionary data.
#[derive(Default)]
pub struct DB {
	strings_en: Vec<String>,
	strings_jp: Vec<String>,
	index_en: HashMap<String, usize>,
	index_jp: HashMap<String, usize>,
	freq_terms: HashMap<String, u64>,
	freq_kanji: HashMap<String, u64>,

	tags: HashMap<String, Tag>,
}

impl DB {
	/// Imports dictionary data into the dictionary.
	pub fn import_dict(&mut self, dict: Dict) {
		for it in dict.tags {
			self.import_tag(it);
		}

		for it in dict.terms {
			self.import_term(it);
		}
		for it in dict.meta_terms {
			self.freq_terms.insert(it.expression, it.data);
		}
		for it in dict.meta_kanji {
			self.freq_kanji.insert(it.expression, it.data);
		}
	}

	/// Dumps information about the database to the console.
	pub fn dump_info(&self) {
		let bytes_jp = self.strings_jp.iter().map(|x| x.len()).sum();
		let bytes_en = self.strings_en.iter().map(|x| x.len()).sum();
		println!(
			"- Strings JP: {}\t~ {:>9}",
			self.strings_jp.len(),
			bytes(bytes_jp)
		);
		println!(
			"- Strings EN: {}\t~ {:>9}",
			self.strings_en.len(),
			bytes(bytes_en)
		);
	}

	fn import_tag(&mut self, tag: Tag) {
		if let Some(old_tag) = self.tags.get_mut(&tag.name) {
			if tag.notes.len() > 0 && tag.notes != old_tag.notes {
				if old_tag.notes.len() > 0 {
					old_tag.notes = format!("{} / {}", old_tag.notes, tag.notes);
				} else {
					old_tag.notes = tag.notes;
				}
			}
			if tag.category.len() > 0 && tag.category != old_tag.category {
				if old_tag.category.len() > 0 {
					eprintln!(
						"WARNING: overridden category of tag `{}` (was `{}`, with `{}`)",
						tag.name, old_tag.category, tag.category
					);
				}
				old_tag.category = tag.category;
			}
		} else {
			self.tags.insert(tag.name.clone(), tag);
		}
	}

	fn import_term(&mut self, term: Term) {
		intern_string(&mut self.strings_jp, &mut self.index_jp, term.expression);
		intern_string(&mut self.strings_jp, &mut self.index_jp, term.reading);
		for it in term.glossary {
			intern_string(&mut self.strings_en, &mut self.index_en, it);
		}
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

fn intern_string(
	strings: &mut Vec<String>,
	index: &mut HashMap<String, usize>,
	value: String,
) -> usize {
	let entry = index.entry(value.clone()).or_insert(strings.len());
	if *entry == strings.len() {
		strings.push(value);
	}
	return *entry;
}
