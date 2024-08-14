PREFIX ?= /usr
OUTDIR = ./target/release
BIN=./target/release/rgs
VERSION=$(shell sed -n 's/^version *= *"\([0-9.]*\)"/\1/p' Cargo.toml)
ARCH=$(shell uname -m)

default_recipe: run

run:
	cargo run

release: src/main.rs
	cargo build --release
	command -v cargo-strip && cargo-strip
	cp $(BIN) $(OUTDIR)/rgs-$(VERSION)-linux-$(ARCH)
	sha256sum $(OUTDIR)/rgs-$(VERSION)-linux-$(ARCH) > $(OUTDIR)/rgs-$(VERSION)-linux-$(ARCH).sha256
	cat $(OUTDIR)/rgs-$(VERSION)-linux-$(ARCH).sha256

install: release
	sudo install -m 0755 $(BIN) $(DESTDIR)$(PREFIX)/bin/cgs

clean:
	cargo clean
