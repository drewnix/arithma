# Arithma Makefile
# Provides commands for development, testing, and pre-commit verification

# Default paths and settings
FRONTEND_DIR := frontend
PKG_DIR := pkg
FRONTEND_PUBLIC_PKG_DIR := $(FRONTEND_DIR)/public/pkg
DOCKER_IMAGE := arithma-frontend
DOCKER_TAG := latest

.PHONY: help
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

################################################################################
# Basic Rust Commands
################################################################################

.PHONY: rust-build
rust-build: ## Build the Rust backend
	cargo build

.PHONY: rust-build-release
rust-build-release: ## Build the Rust backend in release mode
	cargo build --release

.PHONY: rust-test
rust-test: ## Run Rust tests
	cargo test

.PHONY: rust-test-full
rust-test-full: ## Run Rust tests with all features
	cargo test --all-features --verbose

.PHONY: rust-fmt
rust-fmt: ## Format Rust code
	cargo fmt

.PHONY: rust-fmt-check
rust-fmt-check: ## Check Rust formatting without making changes
	cargo fmt -- --check

.PHONY: rust-clippy
rust-clippy: ## Run Clippy with warnings as errors
	RUSTFLAGS="--allow=unexpected_cfgs" cargo clippy -- -D warnings

.PHONY: rust-clippy-all
rust-clippy-all: ## Run Clippy with all warnings enabled
	RUSTFLAGS="--allow=unexpected_cfgs" cargo clippy -- -W clippy::all

################################################################################
# WASM Commands
################################################################################

.PHONY: wasm-build
wasm-build: ## Build the WebAssembly module
	cargo update -p wasm-bindgen-macro
	RUSTFLAGS="--allow=unexpected_cfgs" wasm-pack build --target web

.PHONY: wasm-build-release
wasm-build-release: ## Build the WebAssembly module in release mode
	cargo update -p wasm-bindgen-macro
	RUSTFLAGS="--allow=unexpected_cfgs" wasm-pack build --target web --release

.PHONY: wasm-copy
wasm-copy: ## Copy WASM files to the frontend public directory
	mkdir -p $(FRONTEND_PUBLIC_PKG_DIR)
	cp -r $(PKG_DIR)/* $(FRONTEND_PUBLIC_PKG_DIR)/

################################################################################
# Frontend Commands
################################################################################

.PHONY: frontend-install
frontend-install: ## Install frontend dependencies
	cd $(FRONTEND_DIR) && npm install

.PHONY: frontend-dev
frontend-dev: ## Start frontend development server
	cd $(FRONTEND_DIR) && npm run dev

.PHONY: frontend-build
frontend-build: ## Build the frontend
	cd $(FRONTEND_DIR) && npm run build

.PHONY: frontend-build-ci
frontend-build-ci: ## Build the frontend with CI-specific steps
	cd $(FRONTEND_DIR) && npm run build:ci

.PHONY: frontend-lint
frontend-lint: ## Lint the frontend code
	cd $(FRONTEND_DIR) && npm run lint

.PHONY: frontend-typecheck
frontend-typecheck: ## Type check the frontend code
	cd $(FRONTEND_DIR) && npm run typecheck

.PHONY: frontend-storybook
frontend-storybook: ## Start Storybook
	cd $(FRONTEND_DIR) && npm run storybook

################################################################################
# Security and Dependency Commands
################################################################################

.PHONY: security-audit
security-audit: ## Run security audit on dependencies (requires cargo-audit)
	@command -v cargo-audit >/dev/null 2>&1 || { echo "cargo-audit is not installed. Run 'cargo install cargo-audit' first."; exit 1; }
	cargo audit

.PHONY: check-outdated
check-outdated: ## Check for outdated dependencies (requires cargo-outdated)
	@command -v cargo-outdated >/dev/null 2>&1 || { echo "cargo-outdated is not installed. Run 'cargo install cargo-outdated' first."; exit 1; }
	cargo outdated

################################################################################
# Combined Workflows
################################################################################

.PHONY: ci
ci: rust-fmt-check rust-clippy rust-build rust-test wasm-build wasm-copy frontend-lint frontend-typecheck frontend-build-ci ## Run all CI checks (similar to the CI workflow)

.PHONY: ci-full
ci-full: rust-fmt-check rust-clippy-all rust-build-release rust-test-full wasm-build-release wasm-copy frontend-lint frontend-typecheck frontend-build-ci ## Run comprehensive CI checks (similar to nightly build)

.PHONY: build-all
build-all: rust-build wasm-build wasm-copy frontend-build ## Build everything (backend, WASM, and frontend)

.PHONY: build-all-release
build-all-release: rust-build-release wasm-build-release wasm-copy frontend-build ## Build everything in release mode

.PHONY: precommit
precommit: ci ## Run pre-commit checks to verify code is ready for committing

.PHONY: arithma-setup
arithma-setup: wasm-build wasm-copy ## Setup Arithma WASM for frontend development
	mkdir -p $(FRONTEND_DIR)/node_modules/arithma
	cp -r $(PKG_DIR)/* $(FRONTEND_DIR)/node_modules/arithma/ 2>/dev/null || true

################################################################################
# Project Setup
################################################################################

.PHONY: setup
setup: ## Set up the project for development
	@command -v wasm-pack >/dev/null 2>&1 || { echo "Installing wasm-pack..."; cargo install wasm-pack; }
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; cargo install cargo-audit; }
	@command -v cargo-outdated >/dev/null 2>&1 || { echo "Installing cargo-outdated..."; cargo install cargo-outdated; }
	$(MAKE) rust-build
	$(MAKE) wasm-build
	$(MAKE) frontend-install
	$(MAKE) arithma-setup
	@echo "Project setup complete!"
	@echo "Run 'make help' to see available commands"

################################################################################
# Docker Commands
################################################################################

.PHONY: docker-build
docker-build: ## Build the Docker container
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .

.PHONY: docker-run
docker-run: ## Run the Docker container locally
	docker run -p 3000:80 $(DOCKER_IMAGE):$(DOCKER_TAG)

.PHONY: docker-up
docker-up: ## Start the container using docker-compose
	docker-compose up -d

.PHONY: docker-down
docker-down: ## Stop the container using docker-compose
	docker-compose down

.PHONY: docker-logs
docker-logs: ## View logs from the running container
	docker-compose logs -f

# Default Target
################################################################################

.DEFAULT_GOAL := help