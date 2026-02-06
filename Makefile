BIN := $(shell grep -m1 "^name" Cargo.toml | sed "s/.*\"\(.*\)\"/\1/")
DBG_BIN := target/debug/$(BIN)
REL_BIN := target/release/$(BIN)
SOURCES := $(shell find src -name '*.rs') Cargo.toml Cargo.lock

# Why a Makefile? Because `cargo run --release` redoes LTO every time

.PHONY: default
default: build release

.PHONY: check
check: $(REL_BIN)
	$(REL_BIN)

.PHONY: run
run: $(DBG_BIN)
	$(DBG_BIN)

.PHONY: build
build: $(DBG_BIN)

.PHONY: release
release: $(REL_BIN)

$(DBG_BIN): $(SOURCES)
	cargo build

$(REL_BIN): $(SOURCES)
	cargo build --release
