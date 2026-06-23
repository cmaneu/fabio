# syntax=docker/dockerfile:1

# Build stage — official Rust image (avoids rustup install overhead)
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    make \
    perl \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Copy manifests first to cache dependency builds
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

# Create a dummy main to pre-build dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release --features vendored-openssl 2>/dev/null || true && \
    rm -rf src

# Copy the actual source code
COPY src/ src/

# Touch main.rs so cargo rebuilds with actual source
RUN touch src/main.rs

# Build the release binary with vendored OpenSSL (no runtime libssl needed)
RUN cargo build --release --features vendored-openssl

# Runtime stage — distroless: glibc + libgcc + CA certs, nothing else (~20MB)
FROM gcr.io/distroless/cc-debian12

# Copy the compiled binary from the build stage
COPY --from=builder /src/target/release/fabio /usr/local/bin/fabio

USER nonroot

ENTRYPOINT ["fabio"]
