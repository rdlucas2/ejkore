APP_NAME      ?= ejkore
SIGNALING_IMG ?= $(APP_NAME)-signaling
SIGNALING_PORT ?= 3536
SERVE_PORT    ?= 8080
WASM_TARGET   ?= web

# Load .env if present (never commit .env — use .env.example)
ifneq (,$(wildcard .env))
  include .env
  export
endif

.PHONY: help setup build build-wasm test test-watch serve signaling dev clean lint fmt check
.PHONY: docker-build docker-test docker-up docker-down docker-logs docker-debug

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	  | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

## — Setup —————————————————————————————————————————

setup: ## Install required tools (wasm-pack, wasm32 target)
	@command -v cargo >/dev/null || (echo "ERROR: cargo not found — install Rust first" && exit 1)
	@command -v wasm-pack >/dev/null || cargo install wasm-pack
	rustup target add wasm32-unknown-unknown

## — Build —————————————————————————————————————————

build: ## Build game logic (native, for testing)
	cargo build -p ejkore-game

build-wasm: ## Build client WASM bundle
	wasm-pack build client --target $(WASM_TARGET) --out-dir ../dist/pkg

## — Test ——————————————————————————————————————————

test: ## Run all game logic tests
	cargo test -p ejkore-game

test-watch: ## Run tests on file change (requires cargo-watch)
	cargo watch -x 'test -p ejkore-game'

## — Lint / Format —————————————————————————————————

fmt: ## Format all Rust code
	cargo fmt --all

lint: ## Run clippy lints
	cargo clippy --all-targets -- -D warnings

check: fmt lint test ## Format, lint, and test (pre-commit check)

## — Local Run / Serve —————————————————————————————

serve: ## Serve the game locally (http://localhost:8080)
	@echo "Serving on http://localhost:$(SERVE_PORT)"
	python3 -m http.server $(SERVE_PORT) -d dist

signaling: ## Start matchbox signaling server (standalone Docker)
	docker run --rm -p $(SIGNALING_PORT):$(SIGNALING_PORT) \
	  --name $(SIGNALING_IMG) \
	  jhelsing/matchbox-server:latest \
	  0.0.0.0:$(SIGNALING_PORT)

dev: ## Build WASM, start signaling server, and serve (all in one)
	@echo "Starting signaling server in background..."
	@docker rm -f $(SIGNALING_IMG) 2>/dev/null || true
	@docker run -d --rm -p $(SIGNALING_PORT):$(SIGNALING_PORT) \
	  --name $(SIGNALING_IMG) \
	  jhelsing/matchbox-server:latest \
	  0.0.0.0:$(SIGNALING_PORT)
	@echo "Building WASM..."
	@$(MAKE) build-wasm
	@echo ""
	@echo "  Signaling server: ws://localhost:$(SIGNALING_PORT)"
	@echo "  Game server:      http://localhost:$(SERVE_PORT)"
	@echo ""
	@$(MAKE) serve

## — Docker Compose ————————————————————————————————

docker-build: ## Build all Docker images
	docker compose build

docker-test: ## Run tests in Docker (output to ./coverage/)
	docker compose run --rm test

docker-up: ## Start client + signaling server
	docker compose up -d
	@echo ""
	@echo "  Game:      http://localhost:$(SERVE_PORT)"
	@echo "  Signaling: ws://localhost:$(SIGNALING_PORT)"
	@echo ""

docker-down: ## Stop all containers
	docker compose down

docker-logs: ## Tail logs from all services
	docker compose logs -f

docker-debug: ## Open a shell in the client container
	docker compose exec client /bin/sh

## — Clean —————————————————————————————————————————

clean: ## Remove build artifacts and stop containers
	cargo clean
	rm -rf dist/pkg coverage
	@docker rm -f $(SIGNALING_IMG) 2>/dev/null || true
	@docker compose down 2>/dev/null || true
