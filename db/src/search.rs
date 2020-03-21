use std::collections::BTreeSet;

use super::TermIndex;
use super::DB;

/// Store the search results for a DB.
#[derive(Default)]
pub struct ResultSet {
	indexes: BTreeSet<usize>,
}

impl ResultSet {
	pub fn len(&self) -> usize {
		self.indexes.len()
	}

	pub fn iter<'a>(&'a self) -> ResultSetIter<'a> {
		ResultSetIter {
			iter: self.indexes.iter(),
		}
	}
}

pub struct ResultSetIter<'a> {
	iter: std::collections::btree_set::Iter<'a, usize>,
}

impl<'a> std::iter::Iterator for ResultSetIter<'a> {
	type Item = usize;

	fn next(&mut self) -> Option<usize> {
		if let Some(index) = self.iter.next() {
			Some(*index)
		} else {
			None
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		self.iter.size_hint()
	}
}

impl<'a> DB<'a> {
	/// Search for an exact term in the database inserting the found term
	/// indexes into the `out` result set.
	///
	/// Returns the number of matches.
	pub fn search_term<S: AsRef<str>>(&self, term: S, out: &mut ResultSet) -> usize {
		self.do_search_index(term, true, self.index_prefix_jp, out)
	}

	/// Search for term in the database by the given prefix inserting the found
	/// term indexes into the `out` result set.
	///
	/// Returns the number of matches.
	pub fn search_prefix<S: AsRef<str>>(&self, prefix: S, out: &mut ResultSet) -> usize {
		self.do_search_index(prefix, false, self.index_prefix_jp, out)
	}

	fn do_search_index<S: AsRef<str>>(
		&self,
		keyword: S,
		full_match: bool,
		index: &[TermIndex],
		out: &mut ResultSet,
	) -> usize {
		if let Some((sta, end)) = self.do_search_index_range(keyword, full_match, index) {
			let start_count = out.len();
			for index in sta..=end {
				let index: usize = self.index_prefix_jp[index].term.into();
				out.indexes.insert(index);
			}
			out.len() - start_count
		} else {
			0
		}
	}

	/// Searches the given keyword in the provided index. If `full_match` is
	/// true, only matches the full term, otherwise does a prefix search.
	fn do_search_index_range<S: AsRef<str>>(
		&self,
		keyword: S,
		full_match: bool,
		index: &[TermIndex],
	) -> Option<(usize, usize)> {
		use std::cmp::Ordering;

		let keyword = keyword.as_ref();

		if keyword.len() > 0 {
			let cmp: Box<dyn (FnMut(&TermIndex) -> Ordering)> = if full_match {
				// For `full_match` use a straightforward comparison
				Box::from(|it: &TermIndex| {
					let other = self.get_str(it.key);
					other.cmp(keyword)
				})
			} else {
				// In prefix mode, first compare the prefix
				Box::from(|it: &TermIndex| {
					let other = self.get_str(it.key);
					if other.starts_with(keyword) {
						std::cmp::Ordering::Equal
					} else {
						other.cmp(keyword)
					}
				})
			};

			if let Ok(pos) = index.binary_search_by(cmp) {
				let last = index.len() - 1;
				let mut sta = pos;
				let mut end = pos;

				// In prefix mode, expand the result range to include all
				// prefixed results
				if !full_match {
					while sta > 0 && self.get_str(index[sta - 1].key).starts_with(keyword) {
						sta -= 1;
					}
					while end < last && self.get_str(index[end + 1].key).starts_with(keyword) {
						end += 1;
					}
				}

				Some((sta, end))
			} else {
				None
			}
		} else {
			None
		}
	}
}
