.PHONY: build test lint fmt clean bench

build:
	cargo build --workspace

test:
	cargo test --workspace

lint:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

bench:
	cargo bench --package cloak-core

clean:
	cargo clean

doc:
	cargo doc --workspace --no-deps --open

check: fmt-check lint test
