# syntax=docker/dockerfile:1

# Build stage
FROM ubuntu:latest AS builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Rust stable toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /src

# Copy manifests first to cache dependency builds
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

# Create a dummy main to pre-build dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy the actual source code
COPY src/ src/

# Touch main.rs so cargo rebuilds with actual source
RUN touch src/main.rs

# Build the release binary
RUN cargo build --release

# Runtime stage
FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the build stage
COPY --from=builder /src/target/release/fabio /usr/local/bin/fabio

ENTRYPOINT ["fabio"]
