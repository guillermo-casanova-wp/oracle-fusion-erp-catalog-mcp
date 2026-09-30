SHELL := /bin/sh

VERSION ?=
DRY_RUN ?=

.PHONY: help verify check test release

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make verify' \
		'  make release VERSION=0.2.0 [DRY_RUN=1]'

verify:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo test --workspace

check:
	cargo check

test:
	cargo test --workspace

release:
	@test -n "$(VERSION)" || \
		(printf '%s\n' 'VERSION is required, for example VERSION=0.2.0' >&2; exit 2)
	@if [ -n "$(DRY_RUN)" ]; then \
		scripts/release.sh "$(VERSION)" --dry-run; \
	else \
		scripts/release.sh "$(VERSION)"; \
	fi
