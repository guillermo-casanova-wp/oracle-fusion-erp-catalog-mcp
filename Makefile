SHELL := /bin/sh

BUMP ?= patch
DRY_RUN ?=
RELEASE ?=
DATABASE ?= catalog.sqlite

.PHONY: help verify compile check test release catalog-release

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make verify' \
		'  make compile' \
		'  make release [BUMP=patch|minor|major] [DRY_RUN=1]' \
		'  make catalog-release RELEASE=26B DATABASE=path/to/catalog.sqlite'

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

catalog-release:
	@test -n "$(RELEASE)" || \
		(printf '%s\n' 'RELEASE is required, for example RELEASE=26B' >&2; exit 2)
	scripts/catalog-release.sh --release "$(RELEASE)" --database "$(DATABASE)"
