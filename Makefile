.PHONY: format format-check test-core check-bridge check-gpui preflight setup-hooks

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

preflight: format-check test-core check-bridge
ifeq ($(shell uname -s),Darwin)
	$(MAKE) check-gpui
endif

setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks enabled from .githooks"
