//! Data structures for the organized dictionary data.

use std::collections::HashMap;
use std::fs;
use std::io::BufWriter;
use std::io::Result;

use crate::dict::{Dict, Kanji, Tag, Term};

#[derive(Default)]
pub struct Wrapper {
	/// Frequency map of terms to number of appearances.
	freq_terms: HashMap<String, u32>,

	/// Frequency map of kanji to number of appearances.
	freq_kanji: HashMap<String, u32>,

	/// List of terms from all dictionaries.
	terms: Vec<Term>,

	/// List of kanji from all dictionaries.
	kanji: Vec<Kanji>,

	/// Set of tags from all dictionaries by name.
	tag_map: HashMap<String, Tag>,
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
			self.map_tags(it.term_tags.clone());
			self.map_tags(it.definition_tags.clone());
			self.map_tags(it.rules.clone());
			self.terms.push(it);
		}

		for it in dict.kanji {
			self.map_tags(it.tags.clone());
			self.map_tags(it.stats.keys().cloned().collect());
			self.kanji.push(it);
		}
	}

	/// Outputs all data to code files.
	pub fn output(self) -> Result<()> {
		let mut w = db::Writer::new();

		let mut tag_map = HashMap::new();
		for (index, (key, tag)) in self.tag_map.into_iter().enumerate() {
			let tag = db::TagData {
				name: w.intern(tag.name),
				category: w.intern(tag.category),
				order: tag.order,
				notes: w.intern(tag.notes),
			};
			w.push_tag(tag);
			tag_map.insert(key, index as u32);
		}

		for kanji in self.kanji {
			let meanings: Vec<_> = kanji.meanings.into_iter().map(|x| w.intern(x)).collect();
			let kunyomi: Vec<_> = kanji.kunyomi.into_iter().map(|x| w.intern(x)).collect();
			let onyomi: Vec<_> = kanji.onyomi.into_iter().map(|x| w.intern(x)).collect();

			let tags: Vec<_> = kanji
				.tags
				.into_iter()
				.map(|x| tag_map.get(&x).cloned().unwrap())
				.collect();

			let mut stats: Vec<_> = kanji.stats.into_iter().collect();
			stats.sort_by(|a, b| a.0.cmp(&b.0));
			let stats: Vec<_> = stats
				.into_iter()
				.map(|(k, v)| (tag_map.get(&k).cloned().unwrap(), w.intern(v)))
				.collect();

			w.push_kanji(db::KanjiData {
				character: kanji.character,
				frequency: self
					.freq_kanji
					.get(&kanji.character.to_string())
					.map(|x| *x as u32)
					.unwrap_or(0),
				meanings: meanings,
				kunyomi: kunyomi,
				onyomi: onyomi,
				tags: tags,
				stats: stats,
			});
		}

		for term in self.terms {
			let frequency = self
				.freq_terms
				.get(&term.expression)
				.map(|x| *x as u32)
				.unwrap_or(0);
			let term = db::TermData {
				expression: w.intern(term.expression),
				reading: w.intern(term.reading),
				search_key: w.intern(term.search_key),
				score: term.score,
				sequence: term.sequence,
				frequency: frequency,
				glossary: term.glossary.into_iter().map(|x| w.intern(x)).collect(),
				rules: term
					.rules
					.into_iter()
					.map(|x| tag_map.get(&x).cloned().unwrap())
					.collect(),
				term_tags: term
					.term_tags
					.into_iter()
					.map(|x| tag_map.get(&x).cloned().unwrap())
					.collect(),
				definition_tags: term
					.definition_tags
					.into_iter()
					.map(|x| tag_map.get(&x).cloned().unwrap())
					.collect(),
			};
			w.push_term(term);
		}

		println!("... writing data/dictionary.in...");
		let mut output = BufWriter::new(fs::File::create("data/dictionary.in")?);
		w.write(&mut output)
	}

	fn import_tag(&mut self, tag: Tag) {
		if let Some(mut old_tag) = self.tag_map.get_mut(&tag.name) {
			if tag.notes.len() > 0 && tag.notes != old_tag.notes {
				if old_tag.notes.len() > 0 {
					old_tag.notes = format!("{} / {}", old_tag.notes, tag.notes);
				} else {
					old_tag.notes = tag.notes;
				}
			}
			if tag.category != "" && tag.category != old_tag.category {
				if old_tag.category != "" {
					eprintln!(
						"WARNING: overridden category of tag `{}` (was `{}`, with `{}`)",
						tag.name, old_tag.category, tag.category,
					)
				}
				old_tag.category = tag.category;
			}
		} else {
			self.tag_map.insert(tag.name.clone(), tag);
		}
	}

	fn map_tags(&mut self, tags: Vec<String>) {
		for name in tags {
			self.import_tag(Tag {
				name: name,
				category: String::new(),
				order: 0,
				notes: String::new(),
			})
		}
	}
}
