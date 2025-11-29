default:
	@just --list

run:
	@clear
	cargo run

release:
	@clear
	cargo build --release

release-run:
	@clear
	cargo run --release

release-all:
	@clear
	cargo build --release
	cargo zigbuild --release --target x86_64-unknown-linux-musl
	@# Doesn't compile, but also I don't care
	@#cargo zigbuild --release --target x86_64-pc-windows-gnu
	cargo zigbuild --release --target x86_64-apple-darwin
