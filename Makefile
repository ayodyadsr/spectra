.PHONY: build test demo lint fmt clean

build:
	cargo build --release

test:
	cargo test --release

demo: build
	./target/release/spectra check \
		--baseline examples/vault_baseline \
		--candidate examples/vault_candidate \
		--format markdown

demo-json: build
	./target/release/spectra check \
		--baseline examples/vault_baseline \
		--candidate examples/vault_candidate \
		--format json

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean
