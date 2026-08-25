.PHONY: format format-check test-core check-bridge check-gpui test-gpui preflight setup-hooks

format:
	cargo fmt --all

format-check:
	cargo fmt --all -- --check

test-core:
	cargo test -p bifur-core

check-bridge:
	cargo check -p bifur-bridge

check-gpui:
	cargo check -p bifur

test-gpui:
	cargo test -p bifur --lib

preflight: format-check test-core check-bridge
ifeq ($(shell uname -s),Darwin)
	$(MAKE) check-gpui
	$(MAKE) test-gpui
endif

setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks enabled from .githooks"
