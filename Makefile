.PHONY: build test demo lint fmt clean

build:
	cargo build --release

test:
	cargo test --release

demo: build
	./target/release/spectra check \
		--old examples/lending_v1.json \
		--new examples/lending_v2.json \
		--format markdown

demo-json: build
	./target/release/spectra check \
		--old examples/lending_v1.json \
		--new examples/lending_v2.json \
		--format json

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean
