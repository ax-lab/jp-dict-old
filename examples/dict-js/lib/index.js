let dict = require("../native");
let terms = dict.search_terms("ともだち");

console.log(`\nFound ${terms.length} terms\n`);

const all_tags = {};
const map_tags = tag => {
	all_tags[tag.name] = tag;
	return tag.name;
};

let counter = 0;
for (const term of terms) {
	counter++;

	const item = counter.toString().padStart(2);
	const expr = term.expression.padEnd(15);
	const read = (term.reading ? ` \t` + term.reading : ``).padEnd(20);
	const frequency = term.frequency ? `f:${term.frequency}` : ``;
	const score = term.score ? (frequency ? ` / s:${term.score}` : `s:${term.score}`) : ``;
	const suffix = frequency || score ? `\t${frequency}${score}` : ``;
	console.log(`\n${item}) ${expr}${read}${suffix}`);
	console.log();
	for (const it of term.glossary) {
		console.log(`    > ${it}`);
	}

	if (term.term_tags.length || term.rules.length || term.definition_tags.length) {
		console.log();
		if (term.term_tags.length) {
			const tags = term.term_tags.map(map_tags).join(", ");
			console.log(`    [term] ${tags}`);
		}
		if (term.definition_tags.length) {
			const tags = term.definition_tags.map(map_tags).join(", ");
			console.log(`    [tags] ${tags}`);
		}
		if (term.rules.length) {
			const tags = term.rules.map(map_tags).join(", ");
			console.log(`    [rule] ${tags}`);
		}
	}
}

console.log(`\n## Tags\n`);
const all_tag_keys = [];
for (const key in all_tags) {
	all_tag_keys.push(key);
}
all_tag_keys.sort();
for (const key of all_tag_keys) {
	const tag = all_tags[key];
	const category = tag.category ? `[${tag.category}]` : ``;
	console.log(`- ${key.padEnd(10)} ${tag.notes.padEnd(50)} ${category}`);
}
