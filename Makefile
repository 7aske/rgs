BIN=./target/release/rgs

default_recipe: run

run:
	cargo run

release: src/main.rs
	cargo build --release
	command -v cargo-strip && cargo-strip

install: release
	sudo cp $(BIN) /usr/bin/cgs

clean:
	cargo clean