extern crate x_jp_data;

fn main() {
	let start = std::time::Instant::now();
	let db = x_jp_data::get_db();
	println!("\nLoaded in {:?}", start.elapsed());
}
