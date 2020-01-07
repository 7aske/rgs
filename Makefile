BIN=./target/release/rgs

default_recipe: run

run:
	cargo run

release: src/main.rs
	cargo build --release

install: release
	sudo cp $(BIN) /usr/bin/cgs
