default:
	@just --list

run:
	@clear
	cargo run

release:
	@clear
	cargo build --release
	@clear
	cargo zigbuild --release --target x86_64-unknown-linux-musl
	@clear
	cargo zigbuild --release --target x86_64-apple-darwin
