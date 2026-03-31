# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: MIT

.PHONY: help install build run test test-lib test-cov test-cov-view test-cov-ci \
        lint format docs docs-serve docs-clean \
        cargo-deny gitleaks gitleaks-install \
        semgrep semgrep-install semgrep-sarif \
        license-check license-report \
        security-scan-local security-scan-quick \
        clean

BINARY_NAME     := forage
CONF_PATH       ?= /etc/bind/named.conf
DOCS_PORT       ?= 8000

# Security tool versions
GITLEAKS_VERSION ?= 8.21.2
SEMGREP_VERSION  ?= 1.154.0

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-22s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Check that the Rust toolchain is installed
	@echo "Checking Rust toolchain..."
	@rustup --version || echo "Install Rust from https://rustup.rs"
	@cargo --version
	@echo "✓ Rust toolchain ready"

build: ## Build the binary (debug)
	cargo build

build-release: ## Build the binary (release)
	cargo build --release

run: ## Run forage against CONF_PATH (default: /etc/bind/named.conf)
	RUST_LOG=info cargo run -- --conf $(CONF_PATH)

run-debug: ## Run forage with debug logging
	RUST_LOG=debug cargo run -- --conf $(CONF_PATH) --debug

test: ## Run all tests
	cargo test --all

test-lib: ## Run library (unit) tests only
	cargo test --lib

test-cov: ## Run tests with coverage (HTML report)
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; cargo install cargo-tarpaulin; }
	cargo tarpaulin --out Html --output-dir coverage --exclude-files '*_tests.rs' --timeout 300
	@echo "✓ Coverage report: coverage/tarpaulin-report.html"

test-cov-view: test-cov ## Run coverage and open the HTML report
	open coverage/tarpaulin-report.html 2>/dev/null || echo "Open coverage/tarpaulin-report.html manually"

test-cov-ci: ## Run coverage for CI (text output, no browser)
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; cargo install cargo-tarpaulin; }
	cargo tarpaulin --out Stdout --exclude-files '*_tests.rs' --timeout 300

lint: ## Run clippy and fmt check
	cargo fmt -- --check
	cargo clippy -- -D warnings

format: ## Format source code with rustfmt
	cargo fmt

docs: ## Build rustdoc documentation
	cargo doc --no-deps
	@echo "✓ Docs built in target/doc/$(BINARY_NAME)/"

docs-serve: docs ## Build and serve rustdoc locally
	@echo "Serving docs at http://localhost:$(DOCS_PORT)/$(BINARY_NAME)/"
	python3 -m http.server $(DOCS_PORT) --directory target/doc

docs-clean: ## Remove generated documentation
	rm -rf target/doc
	@echo "✓ Docs cleaned"

# ── Security scanning ─────────────────────────────────────────────────────────

cargo-deny: ## Check dependencies for security, licenses, and supply chain issues
	@command -v cargo-deny >/dev/null 2>&1 || { echo "Installing cargo-deny..."; cargo install cargo-deny; }
	@echo "Running cargo-deny checks..."
	@cargo deny check
	@echo "✓ cargo-deny passed"

gitleaks-install: ## Install gitleaks from GitHub with checksum verification
	@if ! command -v gitleaks >/dev/null 2>&1; then \
		echo "Installing gitleaks v$(GITLEAKS_VERSION)..."; \
		OS=$$(uname -s | tr '[:upper:]' '[:lower:]'); \
		ARCH=$$(uname -m); \
		case "$$ARCH" in \
			x86_64) ARCH="x64" ;; \
			aarch64|arm64) ARCH="arm64" ;; \
		esac; \
		PLATFORM="$${OS}_$${ARCH}"; \
		TARBALL="gitleaks_$(GITLEAKS_VERSION)_$${PLATFORM}.tar.gz"; \
		BASE_URL="https://github.com/gitleaks/gitleaks/releases/download/v$(GITLEAKS_VERSION)"; \
		echo "Downloading gitleaks for $${PLATFORM}..."; \
		curl -sSL -o /tmp/$${TARBALL} $${BASE_URL}/$${TARBALL}; \
		echo "Downloading checksums..."; \
		curl -sSL -o /tmp/gitleaks_checksums.txt $${BASE_URL}/gitleaks_$(GITLEAKS_VERSION)_checksums.txt; \
		echo "Verifying checksum..."; \
		cd /tmp && grep "$${TARBALL}" gitleaks_checksums.txt > checksum_file.txt; \
		if [ "$$OS" = "darwin" ]; then \
			shasum -a 256 -c checksum_file.txt || { echo "Checksum verification failed!"; rm -f $${TARBALL} gitleaks_checksums.txt checksum_file.txt; exit 1; }; \
		else \
			sha256sum -c checksum_file.txt || { echo "Checksum verification failed!"; rm -f $${TARBALL} gitleaks_checksums.txt checksum_file.txt; exit 1; }; \
		fi; \
		tar -xzf /tmp/$${TARBALL} -C /tmp; \
		if [ -w /usr/local/bin ]; then \
			mv /tmp/gitleaks /usr/local/bin/; \
		else \
			echo "Installing to ~/.local/bin (need sudo for /usr/local/bin)..."; \
			mkdir -p ~/.local/bin; \
			mv /tmp/gitleaks ~/.local/bin/; \
			export PATH="$$HOME/.local/bin:$$PATH"; \
		fi; \
		rm -f /tmp/$${TARBALL} /tmp/gitleaks_checksums.txt /tmp/checksum_file.txt; \
		echo "✓ Gitleaks v$(GITLEAKS_VERSION) installed successfully"; \
	else \
		echo "✓ Gitleaks already installed"; \
	fi

gitleaks: gitleaks-install ## Scan for hardcoded secrets and credentials
	@echo "Scanning for secrets with gitleaks..."
	@gitleaks detect --source . --verbose --redact
	@echo "✓ Gitleaks scan completed"

semgrep-install: ## Install Semgrep via pipx
	@if ! command -v semgrep >/dev/null 2>&1; then \
		echo "Installing Semgrep v$(SEMGREP_VERSION) via pipx..."; \
		if ! command -v python3 >/dev/null 2>&1; then \
			echo "Error: Python 3 is required but not found. Please install Python 3.10 or later."; \
			exit 1; \
		fi; \
		if ! command -v pipx >/dev/null 2>&1; then \
			echo "Error: pipx is required but not found."; \
			echo "Install pipx with: python3 -m pip install --user pipx && python3 -m pipx ensurepath"; \
			exit 1; \
		fi; \
		pipx install "semgrep==$(SEMGREP_VERSION)"; \
		if ! command -v semgrep >/dev/null 2>&1; then \
			echo "⚠  Semgrep installed but not in PATH. Run: pipx ensurepath"; \
			exit 1; \
		else \
			echo "✓ Semgrep v$(SEMGREP_VERSION) installed successfully"; \
		fi; \
	else \
		echo "✓ Semgrep already installed: $$(semgrep --version | head -n1)"; \
	fi

semgrep: semgrep-install ## Run Semgrep SAST analysis (Rust ruleset)
	@echo "Running Semgrep security analysis..."
	@semgrep scan \
		--config=p/rust \
		--severity=ERROR \
		--severity=WARNING \
		--exclude='target/' \
		--exclude='*.lock' \
		--metrics=off \
		. || echo "⚠  Semgrep found potential issues (see output above)"
	@echo "✓ Semgrep scan completed"

semgrep-sarif: semgrep-install ## Run Semgrep and output SARIF (for GitHub Security)
	@echo "Running Semgrep security analysis (SARIF output)..."
	@semgrep scan \
		--config=p/rust \
		--severity=ERROR \
		--severity=WARNING \
		--exclude='target/' \
		--exclude='*.lock' \
		--sarif \
		--output=semgrep-results.sarif \
		--metrics=off \
		. || true
	@echo "✓ SARIF results written to semgrep-results.sarif"

license-check: ## Check dependency licenses against policy (fails on GPL/AGPL/SSPL)
	@command -v cargo-license >/dev/null 2>&1 || { echo "Installing cargo-license..."; cargo install cargo-license; }
	@echo "Checking dependency licenses..."
	@VIOLATIONS=$$(cargo license --json 2>/dev/null | \
		jq -r '.[] | select(.license | test("(^| )GPL|AGPL|SSPL|EUPL|CDDL"; "i")) | "\(.name) \(.version): \(.license)"'); \
	if [ -n "$$VIOLATIONS" ]; then \
		echo "❌ Prohibited license(s) found:"; \
		echo "$$VIOLATIONS"; \
		exit 1; \
	else \
		echo "✓ All dependency licenses are compliant"; \
	fi

license-report: ## Generate full license report (licenses.json + summary)
	@command -v cargo-license >/dev/null 2>&1 || { echo "Installing cargo-license..."; cargo install cargo-license; }
	@echo "Generating license report..."
	@cargo license --json > licenses.json
	@echo "✓ License report written to licenses.json"
	@echo ""
	@echo "License Summary:"
	@cargo license 2>/dev/null | sort | uniq -c | sort -rn | head -20

security-scan-local: cargo-deny gitleaks ## Run local security scans (pre-commit)
	@echo "✓ Local security scans completed"

security-scan-quick: cargo-deny gitleaks license-check ## Run quick security scans (for CI)
	@echo "✓ Quick security scans completed"

# ── Housekeeping ──────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cargo clean
	rm -rf target/ coverage/ licenses.json semgrep-results.sarif
	@echo "✓ Clean complete"
