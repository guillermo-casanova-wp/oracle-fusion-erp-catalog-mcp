SHELL := /bin/sh

BUMP ?= patch
DRY_RUN ?=

.PHONY: help verify compile check test release

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make verify' \
		'  make compile' \
		'  make release [BUMP=patch|minor|major] [DRY_RUN=1]'

verify:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo test --workspace

compile:
	cargo build

check:
	cargo check

test:
	cargo test --workspace

release:
	@test -n "$(BUMP)" || \
		(printf '%s\n' 'BUMP is required, for example BUMP=patch' >&2; exit 2)
	@if [ -n "$(DRY_RUN)" ]; then \
		scripts/release.sh --bump "$(BUMP)" --dry-run; \
	else \
		scripts/release.sh --bump "$(BUMP)" --yes; \
	fi
