//! Raw database structure.

/// Unsigned 32 bit integer in LE (little endian) byte order.
///
/// Both Raw integer types are used for platform independent persistence of the
/// database:
///
/// - During database write the integer is converted to LE byte order (a no-op
///   on most common platforms).
///
/// - For loading the database is memory mapped, so conversion happens only when
///   values are used (again a no-op on most platforms).
///
///   - For the rare BE platform case we keep the integer in LE format and pay
///     the conversion price for every use, instead of trying to map to the
///     native integer format on load (which would be more efficient but would
///     also mean a long delay when loading the database).
#[derive(Copy, Clone)]
pub struct RawUint32(u32);

impl std::convert::From<u32> for RawUint32 {
	#[inline]
	fn from(item: u32) -> Self {
		Self(item.to_le())
	}
}

impl std::convert::Into<u32> for RawUint32 {
	#[inline]
	fn into(self) -> u32 {
		u32::from_le(self.0)
	}
}

impl std::convert::Into<usize> for RawUint32 {
	#[inline]
	fn into(self) -> usize {
		let index: u32 = self.into();
		index as usize
	}
}

/// Signed 32 bit integer in LE (little endian) byte order.
///
/// See also `RawUint32`
#[derive(Copy, Clone)]
pub struct RawInt32(i32);

impl std::convert::From<i32> for RawInt32 {
	#[inline]
	fn from(item: i32) -> Self {
		Self(item.to_le())
	}
}

impl std::convert::Into<i32> for RawInt32 {
	#[inline]
	fn into(self) -> i32 {
		i32::from_le(self.0)
	}
}

/// Handle for a serialized string in the persisted database.
///
/// Strings in the database are interned for de-duplication and all the string
/// data is stored in a single binary blob. Strings are stored by their offset
/// and byte length.
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct StrHandle {
	pub offset: RawUint32,
	pub length: RawUint32,
}

impl StrHandle {
	/// Converts the raw offset and length into a `(start, end)` range that can
	/// be used to index the string data.
	pub fn range(&self) -> (usize, usize) {
		let offset: u32 = self.offset.into();
		let length: u32 = self.length.into();
		(offset as usize, (offset + length) as usize)
	}
}

/// Handle for a serialized vector.
///
/// Similar to strings, any list data in the database (e.g. term definitions,
/// kanji readings, tag indexes) are serialized as a vector of integers.
///
/// All vectors are stored as a single binary blob and referenced by their
/// offset and length (in items).
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VecHandle {
	pub offset: RawUint32,
	pub length: RawUint32,
}

impl VecHandle {
	/// Converts the raw offset and length into a `(start, end)` range that can
	/// be used to index the vector data.
	pub fn range(&self) -> (usize, usize) {
		let offset: u32 = self.offset.into();
		let length: u32 = self.length.into();
		(offset as usize, (offset + length) as usize)
	}
}

/// Raw structure for a serialized Tag.
#[repr(C, packed)]
pub struct TagRaw {
	pub name: RawUint32,
	pub category: RawUint32,
	pub order: RawInt32,
	pub notes: RawUint32,
}

/// Raw structure for a serialized Kanji.
#[repr(C, packed)]
pub struct KanjiRaw {
	pub character: RawUint32,
	pub frequency: RawUint32,
	pub source: RawUint32,
	pub meanings: VecHandle,
	pub onyomi: VecHandle,
	pub kunyomi: VecHandle,
	pub tags: VecHandle,
	pub stats: VecHandle,
}

/// Raw structure for a serialized Term.
#[repr(C, packed)]
pub struct TermRaw {
	pub expression: RawUint32,
	pub reading: RawUint32,
	pub search_key: RawUint32,
	pub score: RawInt32,
	pub sequence: RawUint32,
	pub frequency: RawUint32,
	pub source: RawUint32,
	pub glossary: VecHandle,
	pub rules: VecHandle,
	pub term_tags: VecHandle,
	pub definition_tags: VecHandle,
}

/// Serialized row in the term index.
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct TermIndex {
	pub key: RawUint32,
	pub term: RawUint32,
}

/// Serialized row in the character index.
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct CharIndex {
	pub character: RawUint32,
	pub indexes: VecHandle,
}
