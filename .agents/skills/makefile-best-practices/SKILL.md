---
name: makefile-best-practices
description: Write Makefiles following best practices: phony targets, .env file loading, self-documenting help, Docker workflow targets (build, test, scan, run, debug), and git hooks (gitleaks secret scanning). Use when creating or editing a Makefile, or when the user asks about project automation, make targets, build scripts, or secret scanning.
---

# Makefile Best Practices

## Non-negotiables

- **Always declare phony targets** — prevents conflicts with files of the same name
- **Always include a `help` target** — self-documenting via `##` comments
- **Load `.env` safely** — include only if the file exists; never commit `.env`
- **`.env` must be in `.gitignore` and `.dockerignore`** — always provide `.env.example`
- **Variables in UPPER_SNAKE_CASE**, targets in `kebab-case`

## Structure Template

```makefile
APP_NAME ?= myapp
IMAGE_TAG ?= latest

# Load .env if present (never commit .env — use .env.example)
ifneq (,$(wildcard .env))
  include .env
  export
endif

.PHONY: help build test package run debug sonar-start sonar-scan trivy-scan

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	  | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## — Build ——————————————————————————————————————————

build: ## Build the artifact image
	docker build -t $(APP_NAME):$(IMAGE_TAG) .

build-test: ## Build the test image
	docker build -t $(APP_NAME)-test:$(IMAGE_TAG) . --target test

## — Test ———————————————————————————————————————————

test: build-test ## Run tests and write coverage to ./coverage/
	docker run --rm -v $(PWD)/coverage:/out $(APP_NAME)-test:$(IMAGE_TAG)

## — Package ————————————————————————————————————————

package: build ## Tag and push image to registry
	docker tag $(APP_NAME):$(IMAGE_TAG) $(REGISTRY)/$(APP_NAME):$(IMAGE_TAG)
	docker push $(REGISTRY)/$(APP_NAME):$(IMAGE_TAG)

## — Run / Debug ————————————————————————————————————

run: build ## Run the artifact container
	docker run --rm --env-file .env -p $(PORT):$(PORT) $(APP_NAME):$(IMAGE_TAG)

debug: build ## Open a shell in the artifact container
	docker run -it --rm --env-file .env --entrypoint /bin/sh $(APP_NAME):$(IMAGE_TAG)

debug-test: build-test ## Open a shell in the test container
	docker run -it --rm --entrypoint /bin/sh $(APP_NAME)-test:$(IMAGE_TAG)

## — Static Analysis & CVE Scanning ————————————————

sonar-start: ## Start local SonarQube (http://localhost:9000, admin/admin)
	docker run -d --name sonarqube -p 9000:9000 sonarqube
	@echo "Waiting for SonarQube to start — run: docker logs -f sonarqube"

sonar-scan: ## Run SonarQube analysis (requires SONAR_TOKEN env var)
	docker run -it --rm \
	  -e SONAR_HOST_URL="http://host.docker.internal:9000" \
	  -e SONAR_TOKEN="$(SONAR_TOKEN)" \
	  -v "$(PWD):/usr/src" \
	  sonarsource/sonar-scanner-cli

trivy-scan: build ## Scan image for CVEs with Trivy (table + JSON output)
	docker run --rm \
	  -v /var/run/docker.sock:/var/run/docker.sock \
	  -v "$(PWD):/output" \
	  aquasec/trivy image \
	    --format table --output /output/trivy-report.txt \
	    --scanners vuln \
	    $(APP_NAME):$(IMAGE_TAG)
	docker run --rm \
	  -v /var/run/docker.sock:/var/run/docker.sock \
	  -v "$(PWD):/output" \
	  aquasec/trivy image \
	    --format json --output /output/trivy-report.json \
	    --scanners vuln \
	    $(APP_NAME):$(IMAGE_TAG)
```

## Rules

### Phony Targets
Always declare every non-file target as `.PHONY`. Missing `.PHONY` causes silent failures if a file with the same name exists.

### .env Loading Pattern
```makefile
ifneq (,$(wildcard .env))
  include .env
  export
endif
```
- `wildcard` check prevents an error when `.env` doesn't exist
- `export` makes variables available to child processes (docker run, scripts)
- Always provide `.env.example` with dummy values documenting every variable

### Self-Documenting Help
```makefile
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	  | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
```
Add `## Description` after any target to have it appear in `make help`.

### Variable Defaults
Use `?=` for overridable defaults:
```makefile
APP_NAME ?= myapp      # overridable: make build APP_NAME=foo
IMAGE_TAG ?= latest
PORT      ?= 8080
REGISTRY  ?= ghcr.io/myorg
```

### Coverage & Scan Output
Mount a host directory to capture output from containers:
```makefile
test:
	docker run --rm -v $(PWD)/coverage:/out $(APP_NAME)-test:$(IMAGE_TAG)
```
Add `coverage/`, `trivy-report.*` to `.gitignore`.

## Git Hooks — Gitleaks Secret Scanning

Store hooks in `.githooks/` (committed to the repo) and configure git to use them:

```bash
git config core.hooksPath .githooks
```

Provide an `install-hooks` Makefile target so all contributors run it once after cloning:

```makefile
install-hooks: ## Install git hooks from .githooks/ (run once after cloning)
	git config core.hooksPath .githooks
	chmod +x .githooks/pre-commit
```

### `.githooks/pre-commit` pattern

```bash
#!/bin/bash
set -euo pipefail
GITLEAKS_VERSION="v8.21.2"
REPO_ROOT="$(git rev-parse --show-toplevel)"

if command -v gitleaks &>/dev/null; then
  gitleaks protect --staged --no-banner -v
elif command -v docker &>/dev/null; then
  docker run --rm -v "${REPO_ROOT}:/repo" -w /repo \
    "ghcr.io/gitleaks/gitleaks:${GITLEAKS_VERSION}" protect --staged --no-banner -v
else
  echo "ERROR: gitleaks or docker required"; exit 1
fi
```

- Local binary is used when available (faster); Docker is the fallback
- A `.gitleaks.toml` in the repo root configures allowlists for false positives
- Add `# gitleaks:allow` inline for single-line exceptions
- Add a `gitleaks-scan` target to scan full repo history on demand

## Checklist

- [ ] `.PHONY` declared for all non-file targets
- [ ] `help` target present with `##` comments on all targets
- [ ] `.env` loaded with `ifneq (,$(wildcard .env))` guard
- [ ] `.env` in `.gitignore` and `.dockerignore`
- [ ] `.env.example` committed with all variable names and dummy values
- [ ] Variables use `?=` for overridable defaults
- [ ] `coverage/` and scan output files in `.gitignore`
- [ ] `trivy-scan` and `sonar-scan` targets present
- [ ] `debug` target available for local troubleshooting
- [ ] `.githooks/pre-commit` committed with gitleaks scan
- [ ] `install-hooks` target present; hooks directory executable
- [ ] `.gitleaks.toml` present with allowlist for `.env.example` and vendor paths
- [ ] `gitleaks-scan` target for full history scan on demand
