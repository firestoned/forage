# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: Apache-2.0

.PHONY: help install build build-release run run-debug test test-lib test-ci \
        coverage coverage-install test-cov test-cov-view e2e-coverage \
        lint format format-check clippy clippy-pedantic set-version \
        docs docs-serve docs-clean \
        calm-validate calm-docs calm-docs-check \
        e2e-schema e2e-apply e2e-update e2e-all e2e-clean e2e-diagnostics \
        sbom-generate sbom-stage sbom-annotate sbom-check \
        release-tarball provenance-subjects release-assets \
        cargo-deny cargo-machete gitleaks gitleaks-install \
        semgrep semgrep-install semgrep-sarif \
        license-check license-report \
        security-scan-local security-scan-quick \
        clean

BINARY_NAME     := forage
CONF_PATH       ?= /etc/bind/named.conf
DOCS_PORT       ?= 8000
TARGET          ?= x86_64-unknown-linux-gnu

# bindy release whose CRDs forage's output is verified against (ADR-0002,
# ADR-0003). Bump deliberately, with src/crd.rs and src/crd_tests.rs.
BINDY_VERSION   ?= v0.7.1

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

test-lib: ## Run unit tests only (forage is a binary crate: no lib target)
	cargo test --bins

test-ci: ## Run tests in the release profile for TARGET, reusing the release build (CI)
	cargo test --locked --release --target $(TARGET)

# ── Coverage (ADR-0005) ──────────────────────────────────────────────────────
# cargo-llvm-cov follows the forage binary into the subprocesses the CLI
# tests and e2e suites spawn. Reports land in target/coverage/<tier>/:
# html/, lcov.info, summary.json, uncovered.txt.
COVERAGE_DIR      ?= target/coverage
COVERAGE_GATE     := --fail-uncovered-lines 0 --fail-under-functions 100

coverage-install: ## Install cargo-llvm-cov and llvm-tools if missing
	@command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov --locked
	@rustup component add llvm-tools-preview >/dev/null 2>&1 || true

coverage-reports: ## Write html, lcov, summary.json, uncovered.txt for TIER from the current profile data, and publish the summary (usage: make coverage-reports TIER=e2e TITLE="...")
	@if [ -z "$(TIER)" ]; then echo "Error: TIER required"; exit 1; fi
	@mkdir -p $(COVERAGE_DIR)/$(TIER)
	cargo llvm-cov report --html --output-dir $(COVERAGE_DIR)/$(TIER)
	cargo llvm-cov report --lcov --output-path $(COVERAGE_DIR)/$(TIER)/lcov.info
	cargo llvm-cov report --json --summary-only --output-path $(COVERAGE_DIR)/$(TIER)/summary.json
	cargo llvm-cov report --summary-only --show-missing-lines > $(COVERAGE_DIR)/$(TIER)/uncovered.txt
	@./scripts/coverage-summary.sh $(COVERAGE_DIR)/$(TIER)/summary.json \
		"$(or $(TITLE),Coverage: $(TIER))" $(COVERAGE_DIR)/$(TIER)/uncovered.txt

coverage: coverage-install ## Unit + integration coverage, reports, and the 100% gate (CI)
	cargo llvm-cov clean --workspace
	cargo llvm-cov --locked --no-report
	@$(MAKE) --no-print-directory coverage-reports TIER=unit-integration TITLE="Coverage: unit + integration"
	cargo llvm-cov report --summary-only $(COVERAGE_GATE)

test-cov: coverage ## Alias for coverage

test-cov-view: coverage ## Run coverage and open the HTML report
	open $(COVERAGE_DIR)/unit-integration/html/index.html 2>/dev/null \
		|| echo "Open $(COVERAGE_DIR)/unit-integration/html/index.html manually"

# One shell: the instrumentation env from show-env must cover the build, the
# suites AND the reports, or `report` reads cargo test's profiles instead.
e2e-coverage: coverage-install ## e2e suites against an instrumented binary; reports only, no gate (CI)
	@set -e; eval "$$(cargo llvm-cov show-env --sh 2>/dev/null)"; \
	cargo llvm-cov clean --workspace; \
	cargo build --locked; \
	FORAGE_BIN=$(abspath target/debug/forage) BINDY_VERSION=$(BINDY_VERSION) \
		KIND_CLUSTER=forage-e2e-coverage ./tests/e2e/forage-e2e.sh all; \
	$(MAKE) --no-print-directory coverage-reports TIER=e2e \
		TITLE="Coverage: e2e (bindy $(BINDY_VERSION) CRDs on kind)"

lint: format-check clippy ## Run fmt check and clippy

format: ## Format source code with rustfmt
	cargo fmt

format-check: ## Fail if any file is not rustfmt-formatted (CI)
	cargo fmt -- --check

clippy: ## Run clippy on all targets, warnings are errors (CI)
	cargo clippy --all-targets --all-features -- -D warnings

clippy-pedantic: ## Run the cargo-quality skill's pedantic clippy (roadmap 02 tracks the backlog)
	cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -A clippy::module_name_repetitions

set-version: ## Set the package version in Cargo.toml (usage: make set-version VERSION=1.2.3)
	@if [ -z "$(VERSION)" ]; then echo "Error: VERSION required"; exit 1; fi
	@perl -i -pe 's/^version\s*=\s*".*"/version     = "$(VERSION)"/' Cargo.toml
	@grep '^version' Cargo.toml

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
	@version=$$(cargo deny --version); \
	case "$$version" in \
		*" 0.19."*) cargo deny check --config .cargo/deny.toml ;; \
		*) cargo deny --config .cargo/deny.toml check ;; \
	esac
	@echo "✓ cargo-deny passed"

cargo-machete: ## Check for unused dependencies
	@command -v cargo-machete >/dev/null 2>&1 || { echo "Installing cargo-machete..."; cargo install cargo-machete; }
	@cargo machete

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

# ── CALM (Architecture as Code) ───────────────────────────────────────────────
# FINOS CALM models live in ./calm; docs/src/architecture/calm-*.md is
# GENERATED from them. Node.js (>=20) is required; the CLI is fetched on demand
# via npx, pinned for reproducibility.
CALM_CLI_VERSION ?= 1.47.1
CALM ?= npx --yes @finos/calm-cli@$(CALM_CLI_VERSION)

calm-validate: ## Schema-validate every calm/*.architecture.json against CALM 1.2
	@command -v npx >/dev/null 2>&1 || { echo "Error: npx (Node.js >=20) not found."; exit 1; }
	@for f in calm/*.architecture.json; do \
		echo "==> validating $$f"; \
		$(CALM) validate -a "$$f" -f pretty || exit 1; \
	done
	@echo "✓ All CALM models valid."

calm-docs: ## Regenerate the Mermaid architecture pages from the CALM models
	@command -v npx >/dev/null 2>&1 || { echo "Error: npx (Node.js >=20) not found."; exit 1; }
	@CALM_CLI_VERSION=$(CALM_CLI_VERSION) ./scripts/calm-docs.sh

calm-docs-check: ## Verify the committed CALM Mermaid pages match the models (CI drift gate)
	@$(MAKE) --no-print-directory calm-docs
	@git diff --exit-code -- docs/src/architecture/calm-*.md \
		|| { echo "ERROR: CALM docs are stale. Run 'make calm-docs' and commit the result."; exit 1; }
	@echo "✓ CALM docs are up to date."

# ── e2e against bindy's CRDs (ADR-0003) ──────────────────────────────────────
# Needs kind, kubectl, curl and a container runtime. FORAGE_BIN defaults to
# target/release/forage (built here if missing). Each suite owns its cluster.
FORAGE_BIN ?= target/release/forage
E2E_ENV    := FORAGE_BIN=$(abspath $(FORAGE_BIN)) BINDY_VERSION=$(BINDY_VERSION)

$(FORAGE_BIN):
	cargo build --release --locked

e2e-schema: $(FORAGE_BIN) ## e2e: server-side dry run of forage output against bindy CRDs (strict)
	$(E2E_ENV) ./tests/e2e/forage-e2e.sh schema

e2e-apply: $(FORAGE_BIN) ## e2e: apply, spec round-trip, zone selectors, idempotency, determinism
	$(E2E_ENV) ./tests/e2e/forage-e2e.sh apply

e2e-update: $(FORAGE_BIN) ## e2e: re-import after a zone change touches only what changed
	$(E2E_ENV) ./tests/e2e/forage-e2e.sh update

e2e-all: $(FORAGE_BIN) ## Run every e2e suite on one cluster (CI runs them in parallel)
	$(E2E_ENV) ./tests/e2e/forage-e2e.sh all

e2e-clean: ## Delete e2e kind clusters and the e2e work dir
	./tests/e2e/forage-e2e.sh clean

e2e-diagnostics: ## Dump cluster state for a failed suite (usage: make e2e-diagnostics CONTEXT=kind-forage-e2e-schema)
	@if [ -z "$(CONTEXT)" ]; then echo "Error: CONTEXT required"; exit 1; fi
	-kubectl --context "$(CONTEXT)" get nodes -o wide
	-kubectl --context "$(CONTEXT)" get crd
	-kubectl --context "$(CONTEXT)" get dnszones,arecords,aaaarecords,cnamerecords,mxrecords,txtrecords,srvrecords,caarecords -A
	-kubectl --context "$(CONTEXT)" get events -A --sort-by=.lastTimestamp

# ── SBOMs, signing inputs and SLSA provenance (ADR-0004) ──────────────────────
# CI generates binary SBOMs with firestoned/github-actions/rust/generate-sbom;
# these targets post-process, gate and package them the same way locally.
SBOM_DIR          ?= sbom
SBOM_SPEC_VERSION ?= 1.5

sbom-generate: ## Generate the forage SBOM locally (usage: make sbom-generate TARGET=x86_64-unknown-linux-gnu)
	@command -v cargo-cyclonedx >/dev/null 2>&1 || { echo "Error: cargo-cyclonedx not found. Run 'cargo install cargo-cyclonedx --locked --version 0.5.9'."; exit 1; }
	@cargo cyclonedx --all --describe crate --target $(TARGET) --spec-version $(SBOM_SPEC_VERSION) --format json
	@echo "✓ SBOM generated: $(BINARY_NAME).cdx.json"

sbom-stage: ## Stage, annotate and gate the binary SBOM (usage: make sbom-stage SBOM_NAME=forage-linux-amd64)
	@if [ -z "$(SBOM_NAME)" ]; then echo "Error: SBOM_NAME required, e.g. SBOM_NAME=forage-linux-amd64"; exit 1; fi
	@test -f $(BINARY_NAME).cdx.json || { echo "Error: $(BINARY_NAME).cdx.json not found; generate it first"; exit 1; }
	@mkdir -p $(SBOM_DIR)
	@cp $(BINARY_NAME).cdx.json $(SBOM_DIR)/$(SBOM_NAME).cdx.json
	@$(MAKE) --no-print-directory sbom-annotate SBOM=$(SBOM_DIR)/$(SBOM_NAME).cdx.json
	@$(MAKE) --no-print-directory sbom-check SBOM=$(SBOM_DIR)/$(SBOM_NAME).cdx.json

sbom-annotate: ## Add producer metadata (supplier, author) to an SBOM if absent (usage: make sbom-annotate SBOM=file.cdx.json)
	@if [ -z "$(SBOM)" ]; then echo "Error: SBOM required"; exit 1; fi
	@./scripts/sbom.sh annotate "$(SBOM)"

sbom-check: ## Fail unless an SBOM meets the NTIA minimum elements (usage: make sbom-check SBOM=file.cdx.json)
	@if [ -z "$(SBOM)" ]; then echo "Error: SBOM required"; exit 1; fi
	@./scripts/sbom.sh check "$(SBOM)"

release-tarball: ## Package one binary as NAME.tar.gz in DIR (usage: make release-tarball DIR=artifacts/x BINARY=forage NAME=forage-linux-amd64)
	@./scripts/release.sh tarball "$(DIR)" "$(BINARY)" "$(NAME)"

provenance-subjects: ## Write base64 SLSA subjects for every file in DIR (signature bundles excluded) to OUT (usage: make provenance-subjects DIR=subjects OUT=subjects.b64)
	@if [ -z "$(DIR)" ] || [ -z "$(OUT)" ]; then echo "Error: DIR and OUT required"; exit 1; fi
	@cd "$(DIR)" && find . -maxdepth 1 -type f ! -name '.*' ! -name '*.bundle' -print0 | sort -z \
		| xargs -0 sha256sum | sed 's| \./| |' > "$(CURDIR)/subjects.sha256"
	@cat "$(CURDIR)/subjects.sha256"
	@base64 -w0 < "$(CURDIR)/subjects.sha256" > "$(OUT)"
	@echo "✓ $$(wc -l < "$(CURDIR)/subjects.sha256" | tr -d ' ') provenance subjects written to $(OUT)"

release-assets: ## Sort downloaded release artifacts in DIR into release/ sboms/ signatures/ provenance/ + checksums (usage: make release-assets DIR=artifacts)
	@./scripts/release.sh assets "$(DIR)"

# ── Housekeeping ──────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cargo clean
	rm -rf target/ sbom/ licenses.json semgrep-results.sarif subjects.sha256 $(BINARY_NAME).cdx.json
	@echo "✓ Clean complete"
