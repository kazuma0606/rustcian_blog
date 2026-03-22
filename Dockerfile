# ---------------------------------------------------------------------------
# Stage 1: dependency cache
# Copy only Cargo manifests and build stub sources so Docker can cache the
# dependency layer separately from application source changes.
# ---------------------------------------------------------------------------
FROM rust:1.88-slim-bookworm AS deps

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY application/core/Cargo.toml        application/core/
COPY application/backend/Cargo.toml     application/backend/
COPY application/frontend/Cargo.toml   application/frontend/
COPY application/analytics/Cargo.toml  application/analytics/

# Create minimal stub sources so `cargo build` can resolve and cache deps.
RUN mkdir -p \
      application/core/src \
      application/backend/src \
      application/frontend/src \
      application/analytics/src && \
    echo "pub fn _stub() {}" > application/core/src/lib.rs && \
    echo "pub fn _stub() {}" > application/frontend/src/lib.rs && \
    printf 'fn main() {}' > application/analytics/src/main.rs && \
    printf 'fn main() {}\n#[allow(dead_code)]\npub fn _stub() {}' > application/backend/src/main.rs && \
    cargo build --release --locked --bin rustacian_blog_backend 2>/dev/null || true

# ---------------------------------------------------------------------------
# Stage 2: build
# ---------------------------------------------------------------------------
FROM deps AS builder

# Overwrite stubs with real source; touch to force rebuild of changed crates.
COPY application/ application/
RUN touch \
      application/core/src/lib.rs \
      application/backend/src/main.rs \
      application/frontend/src/lib.rs \
      application/analytics/src/main.rs && \
    cargo build --release --locked --bin rustacian_blog_backend

# ---------------------------------------------------------------------------
# Stage 3: runtime
# Only the binary + blog content. No compiler, no Cargo, no source.
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rustacian_blog_backend .
COPY content/ ./content/

ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
ENV CONTENT_ROOT=/app/content
ENV STORAGE_BACKEND=local

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=20s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

CMD ["./rustacian_blog_backend"]
