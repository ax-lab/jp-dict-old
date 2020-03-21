use db::DB;

#[cfg(any(debug_assertions, feature = "no-embed"))]
pub fn get_db() -> &'static DB<'static> {
	lazy_static! {
		static ref DATA: Vec<u8> = {
			let mut dict_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
			dict_path.push("data/dictionary.in");
			std::fs::read(dict_path).unwrap()
		};
		static ref DATABASE: DB<'static> = DB::load(&DATA[..]);
	}
	&DATABASE
}

#[cfg(not(any(debug_assertions, feature = "no-embed")))]
static DATA: &[u8] = include_bytes!("../data/dictionary.in");

#[cfg(not(any(debug_assertions, feature = "no-embed")))]
#[inline]
pub fn get_db() -> &'static DB<'static> {
	lazy_static! {
		static ref DATABASE: DB<'static> = DB::load(DATA);
	}
	&DATABASE
}
