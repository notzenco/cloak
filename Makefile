.PHONY: build test lint fmt clean

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

clean:
	cargo clean

check: fmt-check lint test
