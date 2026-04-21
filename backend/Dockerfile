# Stage 1: Build
FROM rust:1.92-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for dev hot-reload
RUN cargo install cargo-watch --locked

WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source and build
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
# Keep runtime on the same distro line as the Rust toolchain image so glibc stays compatible.
FROM rust:1.92-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/mia-backend ./mia-backend

EXPOSE 8080

CMD ["./mia-backend"]
