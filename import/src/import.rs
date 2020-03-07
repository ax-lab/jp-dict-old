//! Import of Yomichan compatible data.

use std::collections::HashMap;
use std::fs;
use std::io;

use regex::Regex;
use serde::Deserialize;
use serde_json;

use dict::*;

/// Imports a `.zip` file containing Yomichan compatible dictionary data.
pub fn import_file<P: AsRef<std::path::Path>>(path: P) -> Result<Dict, std::io::Error> {
	/// The index file contains the basic information about the dictionary data.
	const INDEX_FILE_NAME: &'static str = "index.json";

	let start = std::time::Instant::now();

	let path = path.as_ref();
	let path_str = path.to_string_lossy();
	println!("\n>>> Importing from {:}", path_str);

	let file = fs::File::open(path)?;
	let mut archive = zip::ZipArchive::new(file)?;

	let index_file = archive.by_name(INDEX_FILE_NAME)?;
	let mut dict: Dict = serde_json::from_reader(index_file)?;

	println!("... {:} -- {:}", dict.title, dict.revision);
	if dict.format != 3 {
		eprintln!(
			"WARNING: format for `{:}` ({:}) is `{:}` (expected `3`)",
			dict.title, path_str, dict.format
		);
	}

	for i in 0..archive.len() {
		let file = archive.by_index(i)?;
		if !file.is_file() {
			continue;
		}

		let path = file.sanitized_name();
		let name = path.to_string_lossy();
		if name == INDEX_FILE_NAME {
			continue;
		}

		if let Some(ext) = path.extension() {
			if ext != "json" {
				continue;
			}
		} else {
			continue;
		}

		import_entry(&mut dict, &name, || Ok(file))?;
	}

	println!("... Elapsed {:?}", start.elapsed());

	use std::cmp::max;
	println!(
		"... Loaded {} terms / {} kanji / {} tags",
		max(dict.terms.len(), dict.meta_terms.len()),
		max(dict.kanji.len(), dict.meta_kanji.len()),
		dict.tags.len()
	);

	Ok(dict)
}

fn import_entry<F, R>(dict: &mut Dict, filename: &str, open: F) -> io::Result<()>
where
	F: FnOnce() -> io::Result<R>,
	R: io::Read,
{
	let kind = get_kind(filename);
	if let Some(kind) = kind {
		let entry_file = open()?;
		match kind {
			DataKind::Term => {
				#[derive(Deserialize)]
				struct TermRow(
					String,      // expression
					String,      // reading
					String,      // definition tags (CSV)
					String,      // rules (CSV)
					i32,         // score
					Vec<String>, // glossary
					i32,         // sequence
					String,      // term tags (CSV)
				);
				let rows: Vec<TermRow> = serde_json::from_reader(entry_file)?;
				for it in rows {
					dict.terms.push(Term {
						expression: it.0,
						reading: it.1,
						definition_tags: csv(&it.2),
						rules: csv(&it.3),
						score: it.4,
						glossary: it.5,
						sequence: it.6,
						term_tags: csv(&it.7),
					});
				}
			}
			DataKind::Kanji => {
				#[derive(Deserialize)]
				struct KanjiRow(
					char,                    // character
					String,                  // onyomi (CSV)
					String,                  // kunyomi (CSV)
					String,                  // tags (CSV)
					Vec<String>,             // meanings
					HashMap<String, String>, // stats
				);
				let rows: Vec<KanjiRow> = serde_json::from_reader(entry_file)?;
				for it in rows {
					dict.kanji.push(Kanji {
						character: it.0,
						onyomi: csv(&it.1),
						kunyomi: csv(&it.2),
						tags: csv(&it.3),
						meanings: it.4,
						stats: it.5,
					});
				}
			}
			DataKind::Tag => {
				#[derive(Deserialize)]
				struct TagRow(
					String, // name
					String, // category
					i32,    // order
					String, // notes
					i32,    // score
				);
				let rows: Vec<TagRow> = serde_json::from_reader(entry_file)?;
				for it in rows {
					dict.tags.push(Tag {
						name: it.0,
						category: it.1,
						order: it.2,
						notes: it.3,
						score: it.4,
					});
				}
			}
			DataKind::KanjiMeta => {
				for it in read_meta(entry_file)? {
					dict.meta_kanji.push(it);
				}
			}
			DataKind::TermMeta => {
				for it in read_meta(entry_file)? {
					dict.meta_terms.push(it);
				}
			}
		}
	}

	Ok(())
}

fn read_meta<R: io::Read>(input: R) -> io::Result<Vec<Meta>> {
	#[derive(Deserialize)]
	struct MetaRow(
		String, // expression
		String, // mode
		u64,    // data
	);
	let rows: Vec<MetaRow> = serde_json::from_reader(input)?;
	let mut result: Vec<Meta> = Vec::new();
	for it in rows {
		result.push(Meta {
			expression: it.0,
			mode: it.1,
			data: it.2,
		});
	}
	Ok(result)
}

fn csv(ls: &str) -> Vec<String> {
	if ls.len() == 0 {
		Vec::new()
	} else {
		ls.split(' ').map(|s| String::from(s)).collect()
	}
}

fn get_kind(file_name: &str) -> Option<DataKind> {
	lazy_static! {
		static ref RE: Regex = Regex::new(r"(_bank(_\d+)?)?\.json$").unwrap();
	}
	match RE.replace_all(file_name, "").to_lowercase().as_str() {
		"term" => Some(DataKind::Term),
		"kanji" => Some(DataKind::Kanji),
		"tag" => Some(DataKind::Tag),
		"kanji_meta" => Some(DataKind::KanjiMeta),
		"term_meta" => Some(DataKind::TermMeta),
		_ => None,
	}
}
