#!/bin/sh
# Fabio CLI installer — downloads the latest release binary for your platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/iemejia/fabio/main/install.sh | bash
#
# Environment variables:
#   INSTALL_DIR  — override install location (default: ~/.local/bin)

set -eu

REPO="iemejia/fabio"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# --- Helpers ---

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

need_cmd() {
    if ! command -v "$1" > /dev/null 2>&1; then
        error "need '$1' (command not found)"
    fi
}

# --- Platform detection ---

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  PLATFORM="linux" ;;
        Darwin) PLATFORM="macos" ;;
        *)      error "unsupported OS: $OS (use Linux or macOS)" ;;
    esac

    case "$ARCH" in
        x86_64|amd64)       ARCH="x64" ;;
        aarch64|arm64)      ARCH="arm64" ;;
        *)                  error "unsupported architecture: $ARCH" ;;
    esac

    # macOS x64 build is not currently published
    if [ "$PLATFORM" = "macos" ] && [ "$ARCH" = "x64" ]; then
        error "macOS x64 binaries are not available — use macOS arm64 (Apple Silicon) or build from source"
    fi

    ARTIFACT="fabio-${PLATFORM}-${ARCH}"
}

# --- Fetch latest release tag ---

fetch_latest_tag() {
    TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')

    if [ -z "$TAG" ]; then
        error "could not determine latest release tag"
    fi
}

# --- Checksum verification ---

verify_checksum() {
    EXPECTED=$(cat "$1" | awk '{print $1}')
    if command -v sha256sum > /dev/null 2>&1; then
        ACTUAL=$(sha256sum "$2" | awk '{print $1}')
    elif command -v shasum > /dev/null 2>&1; then
        ACTUAL=$(shasum -a 256 "$2" | awk '{print $1}')
    else
        info "warning: sha256sum/shasum not found — skipping checksum verification"
        return 0
    fi

    if [ "$EXPECTED" != "$ACTUAL" ]; then
        error "checksum mismatch (expected $EXPECTED, got $ACTUAL)"
    fi
}

# --- Install ---

install_binary() {
    NEEDS_SUDO=""
    if [ ! -w "$(dirname "$INSTALL_DIR")" ] && [ ! -d "$INSTALL_DIR" ]; then
        NEEDS_SUDO="sudo"
    elif [ -d "$INSTALL_DIR" ] && [ ! -w "$INSTALL_DIR" ]; then
        NEEDS_SUDO="sudo"
    fi

    if [ -n "$NEEDS_SUDO" ]; then
        info "elevated permissions required to install to $INSTALL_DIR"
    fi

    $NEEDS_SUDO mkdir -p "$INSTALL_DIR"
    $NEEDS_SUDO tar xzf "$TMPDIR_DL/${ARTIFACT}.tar.gz" -C "$INSTALL_DIR"
    $NEEDS_SUDO chmod +x "$INSTALL_DIR/fabio"
}

# --- Main ---

main() {
    need_cmd curl
    need_cmd tar

    detect_platform
    fetch_latest_tag

    info "installing fabio ${TAG} (${PLATFORM}/${ARCH})"

    TMPDIR_DL="$(mktemp -d)"
    trap 'rm -rf "$TMPDIR_DL"' EXIT

    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ARTIFACT}.tar.gz"
    CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

    info "downloading ${DOWNLOAD_URL}"
    curl -fsSL -o "$TMPDIR_DL/${ARTIFACT}.tar.gz" "$DOWNLOAD_URL"
    curl -fsSL -o "$TMPDIR_DL/${ARTIFACT}.tar.gz.sha256" "$CHECKSUM_URL"

    info "verifying checksum"
    verify_checksum "$TMPDIR_DL/${ARTIFACT}.tar.gz.sha256" "$TMPDIR_DL/${ARTIFACT}.tar.gz"

    info "installing to ${INSTALL_DIR}"
    install_binary

    info "fabio installed successfully to ${INSTALL_DIR}/fabio"

    # Check if INSTALL_DIR is in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            printf '\n'
            info "add fabio to your PATH by adding this to your shell profile:"
            printf '    export PATH="%s:$PATH"\n' "$INSTALL_DIR"
            ;;
    esac
}

main
