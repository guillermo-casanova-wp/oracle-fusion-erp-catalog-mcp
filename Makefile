SHELL := /bin/sh

BINARY := oracle-fusion-erp-catalog-mcp
RELEASE ?= 26B
MODULE ?= all
AGENT ?= all
DATABASE ?=
VERSION ?=
DRY_RUN ?=

.PHONY: help verify check test sync install release

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make verify RELEASE=26B' \
		'  make sync RELEASE=26B MODULE=scm' \
		'  make install AGENT=cursor DATABASE=/path/catalog.sqlite' \
		'  make release VERSION=0.2.0 [DRY_RUN=1]'

verify:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo test --workspace

check:
	cargo check

test:
	cargo test --workspace

sync:
	cargo run -- sync --release "$(RELEASE)" --module "$(MODULE)"

install:
	@test -n "$(DATABASE)" || \
		(printf '%s\n' 'DATABASE is required, for example DATABASE=/path/catalog.sqlite' >&2; exit 2)
	cargo run -- install "$(AGENT)" --database "$(DATABASE)"

release:
	@test -n "$(VERSION)" || \
		(printf '%s\n' 'VERSION is required, for example VERSION=0.2.0' >&2; exit 2)
	@if [ -n "$(DRY_RUN)" ]; then \
		scripts/release.sh "$(VERSION)" --dry-run; \
	else \
		scripts/release.sh "$(VERSION)"; \
	fi
