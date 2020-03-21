#![feature(or_patterns)]

use std::time::Instant;

extern crate rustyline;

extern crate x_jp_data;

use rustyline::error::ReadlineError;
use rustyline::Editor;

fn main() {
	let start = std::time::Instant::now();
	let db = x_jp_data::get_db();
	println!("\nLoaded in {:?}\n", start.elapsed());
	db.check();
	println!();

	let mut rl = Editor::<()>::new();
	loop {
		let input = rl.readline(">> ");
		match input {
			Ok(line) => {
				let line = line.as_str();
				rl.add_history_entry(line);
				println!();

				let mut first = true;
				for it in line.split(' ') {
					let it = it.trim();
					if it.len() > 0 {
						match it.parse::<usize>() {
							Ok(index) => {
								let mut exists = false;
								if index > 0 {
									if let Some(term) = db.term(index - 1) {
										exists = true;
										if !first {
											println!();
										} else {
											first = false;
										}
										println!("{}", term);
									}
								}
								if !exists {
									println!("Term {} does not exist", index)
								}
							}
							Err(_) => {
								if !first {
									println!();
								} else {
									first = false;
								}

								println!("Searching for `{}`...", it);
								let mut results = x_jp_data::ResultSet::default();

								let start = Instant::now();
								let count = db.search_term(it, &mut results);
								println!(
									"- Exact search found {} term(s) in {:?}",
									count,
									start.elapsed()
								);

								let start = Instant::now();
								let count = db.search_prefix(it, &mut results);
								println!(
									"- Prefix search found {} term(s) in {:?}",
									count,
									start.elapsed()
								);

								for index in results.iter().take(5) {
									println!("\n{}", db.term(index).unwrap());
								}
							}
						}
					}
				}
				println!();
			}
			Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
				println!();
				break;
			}
			Err(err) => println!("\n   Error: {}\n", err),
		}
	}
}
