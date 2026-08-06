# ---- Stage 1: build frontend ----
FROM node:22-bookworm-slim AS frontend-builder

RUN corepack enable
WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY frontend/ ./frontend/

RUN pnpm install --frozen-lockfile
RUN pnpm --filter frontend build

# ---- Stage 2: build Rust binary ----
FROM rust:1-bookworm AS rust-builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release

# ---- Stage 3: runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /app/target/release/yggr /app/yggr
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

RUN mkdir -p /app/data

VOLUME /app/data
VOLUME /app/config

EXPOSE 8080

CMD ["/app/yggr"]
