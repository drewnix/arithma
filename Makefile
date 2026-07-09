.PHONY: build install release test check fmt clippy wasm mcp clean help

PREFIX ?= $(HOME)/.local

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build the library and MCP server
	cargo build --release --workspace

install: build ## Install arithma and arithma-mcp to PREFIX (default: ~/.local)
	@mkdir -p $(PREFIX)/bin
	cp target/release/arithma $(PREFIX)/bin/
	cp target/release/arithma-mcp $(PREFIX)/bin/
	@echo "Installed to $(PREFIX)/bin/arithma and $(PREFIX)/bin/arithma-mcp"

release: ## Tag a release (usage: make release V=0.2.0)
ifndef V
	$(error Usage: make release V=x.y.z)
endif
	@if [ -n "$$(git status --porcelain)" ]; then echo "Error: working tree is dirty" >&2; exit 1; fi
	sed -i '' 's/^version = ".*"/version = "$(V)"/' Cargo.toml
	cargo check --workspace 2>/dev/null
	git add Cargo.toml Cargo.lock
	git commit -m "release: v$(V)"
	git tag "v$(V)"
	@echo "Tagged v$(V). Push with: git push origin main --tags"

test: ## Run all tests
	cargo test --all

check: fmt clippy test ## Run all checks (format, lint, test)

fmt: ## Check formatting
	cargo fmt -- --check

clippy: ## Run linter
	RUSTFLAGS="--allow=unexpected_cfgs" cargo clippy -- -D warnings

wasm: ## Build WebAssembly module
	RUSTFLAGS="--allow=unexpected_cfgs" wasm-pack build --target web --release
	@rm -f frontend/public/pkg/*
	@cp pkg/* frontend/public/pkg/
	@echo "Copied WASM to frontend/public/pkg/"

mcp: ## Build the MCP server (release)
	cargo build --release -p arithma-mcp-server --bin arithma-mcp
	@echo "Binary: target/release/arithma-mcp"
	@ls -lh target/release/arithma-mcp | awk '{print "Size:", $$5}'

clean: ## Remove build artifacts
	cargo clean
