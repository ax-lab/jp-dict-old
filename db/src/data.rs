use std::fmt;

use super::TagRaw;
use super::TermRaw;
use super::DB;

/// A tag from the database.
pub struct Tag<'db, 'a: 'db> {
	pub(super) data: &'a DB<'db>,
	pub(super) item: &'a TagRaw,
}

impl<'db, 'a: 'db> Tag<'db, 'a> {
	/// Tag name.
	pub fn name(&self) -> &'db str {
		self.data.get_str(self.item.name)
	}

	/// Tag category.
	pub fn category(&self) -> &'db str {
		self.data.get_str(self.item.category)
	}

	/// Tag order. Can be used to sort the list of tags in a search result.
	pub fn order(&self) -> i32 {
		self.item.order.into()
	}

	/// Tag notes.
	pub fn notes(&self) -> &'db str {
		self.data.get_str(self.item.notes)
	}
}

impl<'db, 'a: 'db> fmt::Display for Tag<'db, 'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.name())?;

		let category = self.category();
		if category.len() > 0 {
			write!(f, " [{}]", category)?;
		}

		let notes = self.notes();
		if notes.len() > 0 {
			write!(f, " -- {}", notes)?;
		}

		Ok(())
	}
}

/// Term from the database.
pub struct Term<'db, 'a: 'db> {
	pub(super) pos: usize,
	pub(super) data: &'a DB<'db>,
	pub(super) item: &'a TermRaw,
}

impl<'db, 'a: 'db> Term<'db, 'a> {
	/// Main Japanese expression for the term.
	pub fn expression(&self) -> &'db str {
		self.data.get_str(self.item.expression)
	}

	/// Reading for the term, if available.
	pub fn reading(&self) -> &'db str {
		self.data.get_str(self.item.reading)
	}

	/// Search key provides an additional search key for the term. This is
	/// a filtered version of the expression or reading.
	pub fn search_key(&self) -> &'db str {
		self.data.get_str(self.item.search_key)
	}

	/// Score provides an additional attribute in which to order the terms in
	/// a search result.
	pub fn score(&self) -> i32 {
		self.item.score.into()
	}

	/// Sequence number for the entry in the source dictionary.
	pub fn sequence(&self) -> u32 {
		self.item.sequence.into()
	}

	/// Number of occurrences for the term in the frequency database (based only
	/// on the expression).
	pub fn frequency(&self) -> Option<u32> {
		let frequency: u32 = self.item.frequency.into();
		if frequency > 0 {
			Some(frequency)
		} else {
			None
		}
	}

	/// Source dictionary name.
	pub fn source(&self) -> &'db str {
		self.data.get_str(self.item.source)
	}

	/// English definitions for the term.
	pub fn glossary(&'a self) -> impl 'a + Iterator<Item = &'db str> {
		let (sta, end) = self.item.glossary.range();
		self.data.vector_data[sta..end]
			.iter()
			.map(move |&index| self.data.get_str(index))
	}

	/// Semantic rules for the term (tag indexes).
	pub fn rules(&'a self) -> impl 'a + Iterator<Item = Tag<'db, 'a>> {
		self.data.get_tags(self.item.rules)
	}

	/// Tag indexes for the japanese term.
	pub fn term_tags(&'a self) -> impl 'a + Iterator<Item = Tag<'db, 'a>> {
		self.data.get_tags(self.item.term_tags)
	}

	/// Tag indexes for the english definition.
	pub fn definition_tags(&'a self) -> impl 'a + Iterator<Item = Tag<'db, 'a>> {
		self.data.get_tags(self.item.definition_tags)
	}
}

impl<'db, 'a: 'db> fmt::Display for Term<'db, 'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "#{} - {}", self.pos + 1, self.expression())?;

		let reading = self.reading();
		if reading.len() > 0 {
			write!(f, " [{}", reading)?;
			let search_key = self.search_key();
			if search_key.len() > 0 {
				write!(f, " / {}", search_key)?;
			}
			write!(f, "]")?;
		} else if self.search_key().len() > 0 {
			write!(f, "[{}]", self.search_key())?;
		}

		if let Some(frequency) = self.frequency() {
			write!(f, " #{}", frequency)?;
		}

		if self.sequence() != 0 || self.score() != 0 {
			write!(f, " (")?;

			if self.sequence() != 0 {
				write!(f, "sequence: {}", self.sequence())?;
			}

			if self.score() != 0 {
				if self.sequence() != 0 {
					write!(f, " / ")?;
				}
				write!(f, "score: {}", self.score())?;
			}

			write!(f, ")")?;
		}

		write!(f, " -- source: {}", self.source())?;
		write!(f, "\n")?;

		for (i, it) in self.glossary().enumerate() {
			if i > 0 {
				write!(f, ", ")?;
			} else {
				write!(f, "\n    ")?;
			}
			write!(f, "{}", it)?;
		}

		let rules: Vec<_> = self.rules().collect();
		if rules.len() > 0 {
			write!(f, "\n\n    Rules:")?;
			for tag in rules {
				write!(f, "\n    -> {}", tag)?;
			}
		}

		let term_tags: Vec<_> = self.term_tags().collect();
		if term_tags.len() > 0 {
			write!(f, "\n\n    Term tags:")?;
			for tag in term_tags {
				write!(f, "\n    -> {}", tag)?;
			}
		}

		let definition_tags: Vec<_> = self.definition_tags().collect();
		if definition_tags.len() > 0 {
			write!(f, "\n\n    Definition tags:")?;
			for tag in definition_tags {
				write!(f, "\n    -> {}", tag)?;
			}
		}

		Ok(())
	}
}
