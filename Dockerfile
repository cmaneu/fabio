# syntax=docker/dockerfile:1

# Build stage — Alpine uses musl natively, producing a fully static binary.
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev git

WORKDIR /src

# Copy manifests and build script first to cache dependency builds
COPY Cargo.toml Cargo.lock rust-toolchain.toml build.rs ./

# Copy data files needed by build.rs at compile time
COPY src/commands/context/data/best_practices/ src/commands/context/data/best_practices/
COPY src/commands/context/data/workflows/ src/commands/context/data/workflows/

# Create a dummy main to pre-build dependencies
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy the actual source code
COPY src/ src/

# Touch main.rs so cargo rebuilds with actual source
RUN touch src/main.rs

# Build the release binary (statically linked via musl — zero runtime deps)
RUN cargo build --release

# Runtime stage — scratch: empty image, just the binary + CA certs (~8MB)
FROM scratch

# CA certificates for HTTPS (Fabric API, OneLake, Azure auth endpoints)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the compiled static binary from the build stage
COPY --from=builder /src/target/release/fabio /usr/local/bin/fabio

USER 65534

ENTRYPOINT ["fabio"]
