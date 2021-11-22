PREFIX ?= /usr
BIN=./target/release/rgs

default_recipe: run

run:
	cargo run

release: src/main.rs
	cargo build --release
	command -v cargo-strip && cargo-strip

install: release
	sudo install -m 0755 $(BIN) $(DESTDIR)$(PREFIX)/bin/cgs

clean:
	cargo clean
