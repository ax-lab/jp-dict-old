use db::DB;

static DATA: &[u8] = include_bytes!("../data/dictionary.in");

pub fn get_db() -> &'static DB {
	lazy_static! {
		static ref DATABASE: DB = DB::load(DATA);
	}
	&DATABASE
}
