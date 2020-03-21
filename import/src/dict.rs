//! Data structures for the source Yomichan data.

use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;

/// Dictionary data imported from a Yomichan internal format.
#[derive(Deserialize)]
pub struct Dict {
	/// Dictionary name.
	pub title: String,

	/// Dictionary format (expected `3`).
	pub format: u32,

	/// Dictionary revision tag.
	pub revision: String,

	/// List of imported terms.
	#[serde(skip)]
	pub terms: Vec<Term>,

	/// List of imported kanji.
	#[serde(skip)]
	pub kanji: Vec<Kanji>,

	/// Definition of tags used by the dictionary terms/kanji.
	#[serde(skip)]
	pub tags: Vec<Tag>,

	/// Frequency metadata for terms.
	#[serde(skip)]
	pub meta_terms: Vec<Meta>,

	/// Frequency metadata for kanji.
	#[serde(skip)]
	pub meta_kanji: Vec<Meta>,
}

/// Dictionary entry for a term.
///
/// Each entry contains a single definition for the term given by `expression`.
/// The definition itself consists of one or more `glossary` items.
pub struct Term {
	/// Term expression.
	pub expression: String,

	/// Kana reading for this term.
	pub reading: String,

	/// Processed search key for this term. Derived from the reading.
	pub search_key: String,

	/// Tags for the term definitions.
	pub definition_tags: Vec<String>,

	/// Rules that affect the entry inflections. Those are also tags.
	///
	/// One of `adj-i`, `v1`, `v5`, `vk`, `vs`.
	///
	/// - `adj-i` adjective (keiyoushi)
	/// - `v1`    Ichidan verb
	/// - `v5`    Godan verb
	/// - `vk`    Kuru verb - special class (e.g. `いって来る`, `來る`)
	/// - `vs`    noun or participle which takes the aux. verb suru
	pub rules: Vec<String>,

	/// Score for this entry. Higher values have precedence.
	pub score: i32,

	/// Definition for this entry.
	pub glossary: Vec<String>,

	/// Sequence number for this entry in the dictionary.
	pub sequence: u32,

	/// Tags for the main term.
	pub term_tags: Vec<String>,

	/// Source database name.
	pub source: String,
}

impl fmt::Display for Term {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "-> {}", self.expression)?;
		if self.reading.len() > 0 && self.reading != self.expression {
			write!(f, " 「{}」", self.reading)?;
		}
		write!(f, " -- {}/{}", self.sequence, self.score)?;
		if self.term_tags.len() > 0 {
			write!(f, "  {}", self.term_tags.join(", "))?;
		}
		writeln!(f)?;

		let tags = self.definition_tags.len();
		let rules = self.rules.len();
		if tags > 0 || rules > 0 {
			write!(f, "   [")?;
			if tags > 0 {
				write!(f, "tags: {}", self.definition_tags.join(", "))?;
			}
			if rules > 0 {
				if tags > 0 {
					write!(f, " / ")?;
				}
				write!(f, "rules: {}", self.rules.join(", "))?;
			}
			write!(f, "]\n")?;
		}

		write!(f, "   {}", self.glossary.join("; "))?;
		Ok(())
	}
}

/// Dictionary entry for a kanji.
pub struct Kanji {
	/// Kanji character.
	pub character: char,

	/// Onyomi (chinese) readings for the Kanji.
	pub onyomi: Vec<String>,

	/// Kunyomi (japanese) readings for the Kanji.
	pub kunyomi: Vec<String>,

	/// Tags for the Kanji.
	pub tags: Vec<String>,

	/// Meanings for the kanji.
	pub meanings: Vec<String>,

	/// Additional kanji information. The keys in `stats` are further detailed
	/// by the dictionary tags.
	pub stats: HashMap<String, String>,

	/// Source database name.
	pub source: String,
}

impl fmt::Display for Kanji {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "-> {}", self.character)?;
		let on = self.onyomi.len();
		let kun = self.kunyomi.len();
		if on > 0 || kun > 0 {
			write!(f, " 「")?;
			if on > 0 {
				write!(f, "ON: {}", self.onyomi.join("  "))?;
			}
			if kun > 0 {
				if on > 0 {
					write!(f, " / ")?;
				}
				write!(f, "KUN: {}", self.kunyomi.join("  "))?;
			}
			write!(f, " 」")?;
		}
		writeln!(f)?;
		if self.tags.len() > 0 {
			write!(f, "   [{}]\n", self.tags.join(", "))?;
		}
		write!(f, "   {}", self.meanings.join("; "))?;
		if self.stats.len() > 0 {
			let mut pairs: Vec<_> = self.stats.iter().collect();
			pairs.sort();
			let pairs: Vec<_> = pairs
				.into_iter()
				.map(|(key, val)| format!("{}: {}", key, val))
				.collect();
			let mut counter = 0;
			for it in pairs {
				if counter % 8 == 0 {
					write!(f, "\n   : ")?;
				} else {
					write!(f, ", ")?;
				}
				write!(f, "{}", it)?;
				counter += 1;
			}
		}
		Ok(())
	}
}

/// Tag for an `Kanji` or `Term`. For kanji dictionary.
///
/// For a `Kanji`, this is also used to describe the `stats` keys.
pub struct Tag {
	/// Name to reference this tag.
	pub name: String,

	/// Category for this tag. This can be used to group related tags.
	pub category: String,

	/// Sort order for this tag (less is higher). This has higher priority than
	/// the name.
	pub order: i32,

	/// Description for this tag.
	pub notes: String,
}

impl fmt::Display for Tag {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{} ({}): {} -- {}",
			self.name, self.category, self.notes, self.order,
		)
	}
}

/// Frequency metadata for kanji or terms.
pub struct Meta {
	/// Kanji or term.
	pub expression: String,

	/// Always `"freq"`.
	pub mode: String,

	/// Metadata value.
	pub data: u32,
}

impl fmt::Display for Meta {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{} = {} ({})", self.expression, self.data, self.mode)
	}
}

pub enum DataKind {
	Term,
	Kanji,
	Tag,
	KanjiMeta,
	TermMeta,
}
