.PHONY: import

build:
	cargo build --all

release:
	cargo build --all --release

import:
	cargo run --release -p import

query:
	cargo run -p query --features no-embed
