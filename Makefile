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

################################################################################
# Helm Chart Commands
################################################################################

.PHONY: helm-lint
helm-lint: ## Lint the Helm chart
	helm lint charts/arithma

.PHONY: helm-template
helm-template: ## Generate Kubernetes manifests from the Helm chart
	helm template arithma charts/arithma

.PHONY: helm-install
helm-install: ## Install the Helm chart to the current Kubernetes context
	helm install arithma charts/arithma

.PHONY: helm-upgrade
helm-upgrade: ## Upgrade the installed Helm chart
	helm upgrade arithma charts/arithma

.PHONY: helm-uninstall
helm-uninstall: ## Uninstall the Helm chart
	helm uninstall arithma

.PHONY: k8s-deploy-local
k8s-deploy-local: docker-build ## Build and deploy to Kubernetes using LoadBalancer and locally built image 
	@echo "Creating a Kubernetes ConfigMap for image pull policy..."
	kubectl delete configmap local-registry-config 2>/dev/null || true
	kubectl create configmap local-registry-config --from-literal=pullPolicy=Never
	@echo "Updating values.yaml for local deployment..."
	sed -i '' 's|pullPolicy: IfNotPresent|pullPolicy: Never|g' charts/arithma/values.yaml
	sed -i '' 's|type: ClusterIP|type: LoadBalancer|g' charts/arithma/values.yaml
	@echo "Saving Docker image to tar file..."
	docker save $(DOCKER_IMAGE):$(DOCKER_TAG) -o /tmp/arithma-image.tar
	@echo "Loading image into Kubernetes nodes..."
	kubectl get nodes -o wide | tail -n +2 | awk '{print $$6}' | xargs -I {} scp /tmp/arithma-image.tar {}:/tmp/
	kubectl get nodes -o wide | tail -n +2 | awk '{print $$6}' | xargs -I {} ssh {} "docker load -i /tmp/arithma-image.tar"
	@echo "Installing Helm chart..."
	helm install arithma charts/arithma
	@echo "Application deployed to Kubernetes with locally loaded images"

.PHONY: k8s-deploy-registry
k8s-deploy-registry: docker-build ## Build and deploy to Kubernetes using a registry
	@echo "Tagging Docker image for remote registry..."
	docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) namazu.local:5000/$(DOCKER_IMAGE):$(DOCKER_TAG)
	@echo "Pushing Docker image to remote registry..."
	docker push namazu.local:5000/$(DOCKER_IMAGE):$(DOCKER_TAG)
	@echo "Updating values.yaml with registry information..."
	sed -i '' 's|repository: $(DOCKER_IMAGE)|repository: namazu.local:5000/$(DOCKER_IMAGE)|g' charts/arithma/values.yaml
	sed -i '' 's|pullPolicy: Never|pullPolicy: IfNotPresent|g' charts/arithma/values.yaml
	@echo "Installing Helm chart..."
	helm install arithma charts/arithma
	@echo "Application deployed to Kubernetes using registry"

.PHONY: k8s-deploy
k8s-deploy: k8s-deploy-local ## Build and deploy to Kubernetes (default method)
	@echo "Used default deployment method (local)"

# Default Target
################################################################################

.DEFAULT_GOAL := help