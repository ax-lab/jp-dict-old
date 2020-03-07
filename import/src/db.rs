//! Data structures for the organized dictionary data.

use std::collections::HashMap;

use crate::dict::{Dict, Term};

/// Root structure for dictionary data.
#[derive(Default)]
pub struct DB {
	strings_en: Vec<String>,
	strings_jp: Vec<String>,
	index_en: HashMap<String, usize>,
	index_jp: HashMap<String, usize>,
	freq_terms: HashMap<String, u64>,
	freq_kanji: HashMap<String, u64>,
}

impl DB {
	/// Imports dictionary data into the dictionary.
	pub fn import_dict(&mut self, dict: Dict) {
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
