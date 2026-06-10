FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    sudo \
    lld \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Install zig (needed by cargo-zigbuild for macOS/arm64 cross-compilation)
RUN curl -sSL "https://ziglang.org/download/0.16.0/zig-$(uname -m)-linux-0.16.0.tar.xz" \
    | tar -xJ -C /opt \
    && ln -s /opt/zig-$(uname -m)-linux-0.16.0/zig /usr/local/bin/zig

# Reuse existing ubuntu user (UID 1000) and grant passwordless sudo
RUN echo "ubuntu ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/ubuntu \
    && chmod 0440 /etc/sudoers.d/ubuntu

USER ubuntu
