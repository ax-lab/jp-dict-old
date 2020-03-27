extern crate neon;
#[macro_use]
extern crate neon_serde;
#[macro_use]
extern crate serde_derive;

extern crate jp_dict;

use neon::prelude::*;

#[derive(Serialize, Deserialize)]
struct Term {
	expression: String,
	reading: String,
	score: i32,
	frequency: Option<u32>,
	source: String,
	glossary: Vec<String>,
	rules: Vec<Tag>,
	definition_tags: Vec<Tag>,
	term_tags: Vec<Tag>,
}

#[derive(Serialize, Deserialize)]
struct Tag {
	name: String,
	category: String,
	notes: String,
}

export! {
	fn search_terms(input: String) -> Vec<Term> {
		let db = jp_dict::get_db();
		let mut set = jp_dict::ResultSet::default();
		db.search_prefix(input, &mut set);

		let mut results = Vec::new();
		for index in set.iter() {
			let src = db.term(index).unwrap();
			let term = Term{
				expression: src.expression().to_string(),
				reading: src.reading().to_string(),
				score: src.score(),
				frequency: src.frequency(),
				source: src.source().to_string(),
				glossary: src.glossary().map(|x| x.to_string()).collect(),
				rules: src.rules().map(to_tag).collect(),
				definition_tags: src.definition_tags().map(to_tag).collect(),
				term_tags: src.term_tags().map(to_tag).collect(),
			};
			results.push(term);
		}

		return results;

		fn to_tag<'db, 'a>(item: jp_dict::Tag<'db, 'a>) -> Tag {
			Tag {
				name: item.name().to_string(),
				category: item.category().to_string(),
				notes: item.notes().to_string(),
			}
		}
	}
}

// fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
// 	Ok(cx.string("hello node"))
// }

// register_module!(mut cx, { cx.export_function("hello", hello) });
