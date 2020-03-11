extern crate x_jisho_data;

fn main() {
	let start = std::time::Instant::now();
	let db = x_jisho_data::get_db();
	println!(
		"\nLoaded {} terms / {} kanji in {:?}",
		db.terms.len(),
		db.kanji.len(),
		start.elapsed()
	);
}
