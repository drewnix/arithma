.PHONY: build test check fmt clippy wasm mcp clean help

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build the library and MCP server
	cargo build --release

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
	cargo build --release --bin arithma-mcp
	@echo "Binary: target/release/arithma-mcp"
	@ls -lh target/release/arithma-mcp | awk '{print "Size:", $$5}'

clean: ## Remove build artifacts
	cargo clean
