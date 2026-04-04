# =============================================================================
# Stage 1 — base: build game logic and WASM client
# =============================================================================
FROM rust:1.94.1-alpine3.21 AS base
RUN apk add --update musl-dev curl

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Add WASM target
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Cache dependency layer — copy manifests first
COPY Cargo.toml ./
COPY game/Cargo.toml game/Cargo.toml
COPY client/Cargo.toml client/Cargo.toml

# Create stub sources to build deps (client stub must use wasm_bindgen)
RUN mkdir -p game/src client/src && \
    echo "pub fn stub() {}" > game/src/lib.rs && \
    echo "use wasm_bindgen::prelude::*; #[wasm_bindgen] pub fn stub() {}" > client/src/lib.rs && \
    cargo build --release -p ejkore-game && \
    wasm-pack build client --target web --out-dir /tmp/pkg-stub 2>/dev/null || true && \
    rm -rf game/src client/src /tmp/pkg-stub

# Copy real source
COPY game/src game/src
COPY client/src client/src

# Remove stale stub artifacts so cargo recompiles with real source
RUN rm -rf target/release/deps/ejkore* target/release/.fingerprint/ejkore* \
    target/wasm32-unknown-unknown/release/deps/ejkore* \
    target/wasm32-unknown-unknown/release/.fingerprint/ejkore*

# Build game logic (native)
RUN cargo build --release -p ejkore-game

# Build WASM client
RUN wasm-pack build client --target web --out-dir /app/dist/pkg

# =============================================================================
# Stage 2 — test: run game logic tests
# =============================================================================
FROM base AS test
ENTRYPOINT ["sh", "-c", "cargo test -p ejkore-game 2>&1 | tee /out/test-results.txt"]

# =============================================================================
# Stage 3 — artifact: serve static files via nginx
# =============================================================================
FROM nginx:1.27.3-alpine AS artifact

# Remove apk attack surface
RUN rm -f /sbin/apk && \
    rm -rf /etc/apk /lib/apk /usr/share/apk /var/lib/apk

# Copy static web files (index.html etc.) first
COPY dist/index.html /usr/share/nginx/html/index.html

# Copy WASM bundle (overwrites any stale pkg from dist/)
COPY --from=base /app/dist/pkg /usr/share/nginx/html/pkg

# Custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

# Nginx runs as non-root via its own config
ENTRYPOINT ["nginx", "-g", "daemon off;"]
