default:
	@just --list

format:
	cargo fmt

format-check:
	cargo fmt --check

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

check: format-check lint test

ci: check

macos-build:
    ./scripts/build-macos.sh

macos-test:
    ./scripts/test-macos.sh

macos-format-check:
    swift format lint --strict --recursive macos/Sources macos/Tests macos/Package.swift
