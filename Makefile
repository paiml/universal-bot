# Universal Bot Makefile - PMAT Compliant Build System
# Version: 1.0.0
# Last Updated: 2025-01-17

# Colors for output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
BLUE := \033[0;34m
NC := \033[0m # No Color

# Build configuration
CARGO := cargo
# RUSTFLAGS := -D warnings  # Temporarily disabled for initial development
RUST_BACKTRACE := 1
RUST_LOG := debug

# Coverage configuration
COVERAGE_THRESHOLD := 85
LLVM_PROFILE_FILE := target/coverage/%p-%m.profraw

# Docker configuration
DOCKER_IMAGE := universal-bot
DOCKER_TAG := latest
DOCKER_REGISTRY := registry.universal-bot.io

# AWS configuration
AWS_REGION ?= us-east-1
AWS_PROFILE ?= default

.PHONY: all
all: quality build ## Run quality checks and build

.PHONY: help
help: ## Show this help message
	@echo "Universal Bot - PMAT Compliant Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'

##@ Development

.PHONY: setup
setup: ## Initial project setup
	@echo "$(BLUE)Setting up Universal Bot development environment...$(NC)"
	@rustup update stable
	@rustup component add rustfmt clippy llvm-tools-preview
	@cargo install cargo-audit cargo-tarpaulin cargo-criterion cargo-flamegraph
	@cargo install cargo-watch cargo-expand cargo-deps cargo-outdated
	@echo "$(GREEN)✓ Development environment ready$(NC)"

.PHONY: dev
dev: ## Run in development mode with auto-reload
	@echo "$(BLUE)Starting development server...$(NC)"
	@cargo watch -x 'run --bin universal-bot' -w src/

.PHONY: build
build: ## Build the project in release mode
	@echo "$(BLUE)Building Universal Bot...$(NC)"
	@RUSTFLAGS="$(RUSTFLAGS)" cargo build --release
	@echo "$(GREEN)✓ Build complete$(NC)"

.PHONY: run
run: ## Run the application
	@cargo run --bin universal-bot

##@ Quality Assurance

.PHONY: quality
quality: lint test test-doc test-property coverage security quality-proxy ## Run all quality checks with PMAT enforcement
	@echo "$(GREEN)✓ All quality checks passed$(NC)"

.PHONY: quality-proxy
quality-proxy: ## Run PMAT quality proxy with real-time enforcement
	@echo "$(BLUE)Running PMAT Quality Proxy...$(NC)"
	@cargo run --bin quality-proxy -- \
		--mode strict \
		--coverage-threshold $(COVERAGE_THRESHOLD) \
		--enable-satd-detection \
		--enable-property-generation \
		--enable-complexity-analysis \
		--output-format json > target/quality-report.json
	@echo "$(GREEN)✓ Quality proxy analysis complete: target/quality-report.json$(NC)"

.PHONY: lint
lint: fmt-check clippy ## Run all linting checks
	@echo "$(GREEN)✓ Linting complete$(NC)"

.PHONY: fmt
fmt: ## Format code with rustfmt
	@echo "$(BLUE)Formatting code...$(NC)"
	@cargo fmt --all
	@echo "$(GREEN)✓ Code formatted$(NC)"

.PHONY: fmt-check
fmt-check: ## Check code formatting
	@echo "$(BLUE)Checking code format...$(NC)"
	@cargo fmt --all -- --check || (echo "$(RED)✗ Format check failed. Run 'make fmt'$(NC)" && exit 1)
	@echo "$(GREEN)✓ Format check passed$(NC)"

.PHONY: clippy
clippy: ## Run clippy linter
	@echo "$(BLUE)Running clippy...$(NC)"
	@RUSTFLAGS="$(RUSTFLAGS)" cargo clippy --all-targets --all-features
	@echo "$(GREEN)✓ Clippy passed$(NC)"

##@ Testing

.PHONY: test
test: ## Run all tests
	@echo "$(BLUE)Running tests...$(NC)"
	@RUST_BACKTRACE=$(RUST_BACKTRACE) cargo test --all-features
	@echo "$(GREEN)✓ Tests passed$(NC)"

.PHONY: test-verbose
test-verbose: ## Run tests with verbose output
	@RUST_BACKTRACE=full cargo test --all-features -- --nocapture --test-threads=1

.PHONY: test-doc
test-doc: ## Run documentation tests
	@echo "$(BLUE)Running doc tests...$(NC)"
	@cargo test --doc --all-features
	@echo "$(GREEN)✓ Doc tests passed$(NC)"

.PHONY: test-property
test-property: ## Run property-based tests
	@echo "$(BLUE)Running property tests...$(NC)"
	@cargo test --features property-testing property_
	@echo "$(GREEN)✓ Property tests passed$(NC)"

.PHONY: test-property-generate
test-property-generate: ## Auto-generate property tests using MCP tools
	@echo "$(BLUE)Auto-generating property tests...$(NC)"
	@cargo run --bin mcp-property-generator -- \
		--source-dir src/ \
		--output-dir tests/generated/ \
		--test-types roundtrip,idempotency,invariant
	@echo "$(GREEN)✓ Property tests generated$(NC)"

.PHONY: test-integration
test-integration: ## Run integration tests
	@echo "$(BLUE)Running integration tests...$(NC)"
	@cargo test --test '*' --features integration-tests
	@echo "$(GREEN)✓ Integration tests passed$(NC)"

.PHONY: test-single
test-single: ## Run a single test (use TEST=test_name)
	@cargo test $(TEST) -- --nocapture

.PHONY: coverage
coverage: ## Generate test coverage report
	@echo "$(BLUE)Generating coverage report...$(NC)"
	@cargo tarpaulin --out Html --output-dir target/coverage \
		--exclude-files "*/tests/*" \
		--exclude-files "*/benches/*" \
		--exclude-files "*/examples/*" \
		--fail-under $(COVERAGE_THRESHOLD) || \
		(echo "$(RED)✗ Coverage below $(COVERAGE_THRESHOLD)%$(NC)" && exit 1)
	@echo "$(GREEN)✓ Coverage report generated: target/coverage/index.html$(NC)"

.PHONY: coverage-open
coverage-open: coverage ## Generate and open coverage report
	@open target/coverage/index.html || xdg-open target/coverage/index.html

##@ Benchmarking

.PHONY: bench
bench: ## Run benchmarks
	@echo "$(BLUE)Running benchmarks...$(NC)"
	@cargo criterion --message-format=json | tee target/criterion/report.json
	@echo "$(GREEN)✓ Benchmarks complete$(NC)"

.PHONY: bench-compare
bench-compare: ## Compare benchmark results with baseline
	@cargo criterion --baseline base --message-format=json

.PHONY: flamegraph
flamegraph: ## Generate flamegraph for performance analysis
	@echo "$(BLUE)Generating flamegraph...$(NC)"
	@cargo flamegraph --bin universal-bot -o target/flamegraph.svg
	@echo "$(GREEN)✓ Flamegraph saved to target/flamegraph.svg$(NC)"

##@ Security

.PHONY: security
security: audit check-deps satd-scan ## Run security checks with SATD detection
	@echo "$(GREEN)✓ Security checks passed$(NC)"

.PHONY: satd-scan
satd-scan: ## Scan for Self-Admitted Technical Debt (SATD)
	@echo "$(BLUE)Scanning for SATD markers...$(NC)"
	@cargo run --bin satd-detector -- \
		--source-dir src/ \
		--include-comments \
		--severity-threshold medium \
		--output-format json > target/satd-report.json
	@if [ -s target/satd-report.json ]; then \
		echo "$(YELLOW)⚠ SATD issues found: target/satd-report.json$(NC)"; \
	else \
		echo "$(GREEN)✓ No SATD issues detected$(NC)"; \
	fi

.PHONY: audit
audit: ## Run security audit on dependencies
	@echo "$(BLUE)Running security audit...$(NC)"
	@cargo audit || (echo "$(RED)✗ Security vulnerabilities found$(NC)" && exit 1)
	@echo "$(GREEN)✓ No vulnerabilities found$(NC)"

.PHONY: check-deps
check-deps: ## Check for outdated dependencies
	@echo "$(BLUE)Checking dependencies...$(NC)"
	@cargo outdated --exit-code 1 || echo "$(YELLOW)⚠ Some dependencies are outdated$(NC)"

##@ Documentation

.PHONY: docs
docs: ## Build documentation
	@echo "$(BLUE)Building documentation...$(NC)"
	@cargo doc --no-deps --all-features
	@echo "$(GREEN)✓ Documentation built$(NC)"

.PHONY: docs-open
docs-open: docs ## Build and open documentation
	@cargo doc --no-deps --all-features --open

.PHONY: docs-deps
docs-deps: ## Build documentation with dependencies
	@cargo doc --all-features

##@ Examples

.PHONY: example-basic
example-basic: ## Run basic bot example
	@cargo run --example basic_bot

.PHONY: example-plugins
example-plugins: ## Run plugin system demo
	@cargo run --example plugin_demo

.PHONY: example-mcp-quality
example-mcp-quality: ## Run MCP quality proxy example
	@cargo run --example mcp_quality_demo

.PHONY: examples
examples: ## Run all examples
	@for example in basic_bot plugin_demo; do \
		echo "$(BLUE)Running example: $$example$(NC)"; \
		cargo run --example $$example || exit 1; \
	done
	@echo "$(GREEN)✓ All examples completed$(NC)"

##@ MCP Integration

.PHONY: mcp-server
mcp-server: ## Start MCP server with quality proxy
	@echo "$(BLUE)Starting Universal Bot MCP Server...$(NC)"
	@cargo run --bin universal-bot-mcp -- \
		--enable-quality-proxy \
		--enable-satd-detection \
		--enable-property-generation \
		--coverage-threshold $(COVERAGE_THRESHOLD) \
		--port 8080

.PHONY: mcp-client-test
mcp-client-test: ## Test MCP server functionality
	@echo "$(BLUE)Testing MCP server...$(NC)"
	@cargo run --bin mcp-client-test -- \
		--server-url http://localhost:8080 \
		--test-quality-tools \
		--test-property-generation

.PHONY: mcp-quality-watch
mcp-quality-watch: ## Watch for code changes and run quality checks
	@echo "$(BLUE)Starting quality watch mode...$(NC)"
	@cargo watch -x 'run --bin quality-proxy -- --mode watch --auto-fix'

##@ Docker

.PHONY: docker-build
docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(NC)"
	@docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .
	@echo "$(GREEN)✓ Docker image built: $(DOCKER_IMAGE):$(DOCKER_TAG)$(NC)"

.PHONY: docker-run
docker-run: docker-build ## Run Docker container
	@docker run -it --rm \
		-e AWS_REGION=$(AWS_REGION) \
		-e AWS_PROFILE=$(AWS_PROFILE) \
		-v ~/.aws:/root/.aws:ro \
		$(DOCKER_IMAGE):$(DOCKER_TAG)

.PHONY: docker-push
docker-push: docker-build ## Push Docker image to registry
	@docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	@docker push $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	@echo "$(GREEN)✓ Image pushed to $(DOCKER_REGISTRY)$(NC)"

##@ AWS Integration

.PHONY: aws-deploy
aws-deploy: build ## Deploy to AWS Lambda
	@echo "$(BLUE)Deploying to AWS Lambda...$(NC)"
	@cargo lambda build --release
	@cargo lambda deploy --iam-role arn:aws:iam::ACCOUNT:role/lambda-role
	@echo "$(GREEN)✓ Deployed to AWS Lambda$(NC)"

.PHONY: aws-test
aws-test: ## Test AWS integration
	@echo "$(BLUE)Testing AWS integration...$(NC)"
	@AWS_REGION=$(AWS_REGION) cargo test --features aws-integration
	@echo "$(GREEN)✓ AWS integration tests passed$(NC)"

##@ Release

.PHONY: release-check
release-check: quality docs ## Pre-release checks
	@echo "$(BLUE)Running release checks...$(NC)"
	@cargo publish --dry-run
	@echo "$(GREEN)✓ Release checks passed$(NC)"

.PHONY: release
release: release-check ## Create a new release
	@echo "$(BLUE)Creating release...$(NC)"
	@read -p "Version (current: $$(cargo pkgid | cut -d# -f2)): " version; \
	cargo release $$version
	@echo "$(GREEN)✓ Release complete$(NC)"

##@ Maintenance

.PHONY: clean
clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	@cargo clean
	@rm -rf target/coverage target/criterion target/flamegraph.svg
	@echo "$(GREEN)✓ Clean complete$(NC)"

.PHONY: update
update: ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(NC)"
	@cargo update
	@echo "$(GREEN)✓ Dependencies updated$(NC)"

.PHONY: fix
fix: fmt ## Auto-fix code issues
	@echo "$(BLUE)Auto-fixing issues...$(NC)"
	@cargo fix --allow-dirty --allow-staged
	@cargo clippy --fix --allow-dirty --allow-staged
	@echo "$(GREEN)✓ Auto-fix complete$(NC)"

##@ CI/CD

.PHONY: ci
ci: ## Run CI pipeline locally
	@echo "$(BLUE)Running CI pipeline...$(NC)"
	@$(MAKE) clean
	@$(MAKE) setup
	@$(MAKE) quality
	@$(MAKE) build
	@$(MAKE) examples
	@echo "$(GREEN)✓ CI pipeline passed$(NC)"

.PHONY: pre-commit
pre-commit: fmt lint test ## Run pre-commit hooks
	@echo "$(GREEN)✓ Pre-commit checks passed$(NC)"

##@ Metrics

.PHONY: loc
loc: ## Count lines of code
	@echo "$(BLUE)Lines of code:$(NC)"
	@tokei src/ --exclude "*/tests/*" --exclude "*/benches/*"

.PHONY: complexity
complexity: ## Analyze code complexity
	@echo "$(BLUE)Analyzing complexity...$(NC)"
	@cargo complexity --threshold 10

.PHONY: deps-tree
deps-tree: ## Show dependency tree
	@cargo tree --no-dedupe

.PHONY: bloat
bloat: ## Analyze binary size
	@cargo bloat --release --crates

##@ Utilities

.PHONY: watch
watch: ## Watch for changes and run tests
	@cargo watch -x test -x clippy

.PHONY: expand
expand: ## Expand macros for debugging
	@cargo expand

.PHONY: install
install: build ## Install binary locally
	@echo "$(BLUE)Installing universal-bot...$(NC)"
	@cargo install --path .
	@echo "$(GREEN)✓ Installed to ~/.cargo/bin/universal-bot$(NC)"

.PHONY: uninstall
uninstall: ## Uninstall binary
	@cargo uninstall universal-bot

# Default target
.DEFAULT_GOAL := help