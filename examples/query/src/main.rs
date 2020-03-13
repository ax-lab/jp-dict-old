extern crate x_jp_data;

fn main() {
	let start = std::time::Instant::now();
	let db = x_jp_data::get_db();
	println!(
		"\nLoaded {} terms / {} kanji in {:?}",
		db.terms.len(),
		db.kanji.len(),
		start.elapsed()
	);
}
