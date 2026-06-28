#!/usr/bin/env bash
# Install the latest fabio binary from GitHub Releases.
# Supports Linux (x64/arm64), macOS (arm64), and Windows (x64/arm64 via WSL).
#
# Usage:
#   bash install.sh                    # Install to ~/.local/bin
#   bash install.sh /usr/local/bin     # Install to /usr/local/bin
#   FABIO_VERSION=v0.6.0 bash install.sh  # Install specific version

set -euo pipefail

# If fabio is already installed and no specific version was requested,
# use the built-in upgrade command instead of re-downloading.
if [ -z "${FABIO_VERSION:-}" ] && command -v fabio &>/dev/null; then
  echo "fabio is already installed: $(fabio --version 2>/dev/null || echo 'unknown version')"
  echo "Checking for updates..."
  if fabio upgrade; then
    echo "fabio is up to date."
    exit 0
  else
    echo "Upgrade failed (exit $?). Falling back to fresh install..." >&2
  fi
fi

REPO="iemejia/fabio"
INSTALL_DIR="${1:-${HOME}/.local/bin}"
VERSION="${FABIO_VERSION:-}"

# Detect OS
case "$(uname -s)" in
  Linux*)   OS="linux" ;;
  Darwin*)  OS="macos" ;;
  MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
  *)        echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64|amd64)  ARCH="x64" ;;
  aarch64|arm64)  ARCH="arm64" ;;
  *)              echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

# Determine file extension
if [ "$OS" = "windows" ]; then
  EXT="zip"
else
  EXT="tar.gz"
fi

# Get latest version if not specified
if [ -z "$VERSION" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | \
    grep '"tag_name"' | cut -d'"' -f4)
  if [ -z "$VERSION" ]; then
    echo "Failed to determine latest version" >&2
    exit 1
  fi
fi

ASSET="fabio-${OS}-${ARCH}.${EXT}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
CHECKSUM_URL="${URL}.sha256"

echo "Installing fabio ${VERSION} (${OS}/${ARCH})..."
echo "Download URL: ${URL}"

# Create install directory
mkdir -p "${INSTALL_DIR}"

# Download to temp directory
TMPDIR=$(mktemp -d)
trap 'rm -rf "${TMPDIR}"' EXIT

echo "Downloading..."
curl -fsSL -o "${TMPDIR}/${ASSET}" "${URL}"

# Verify checksum if available
if curl -fsSL -o "${TMPDIR}/${ASSET}.sha256" "${CHECKSUM_URL}" 2>/dev/null; then
  echo "Verifying checksum..."
  EXPECTED=$(cat "${TMPDIR}/${ASSET}.sha256" | awk '{print $1}')
  if command -v sha256sum &>/dev/null; then
    ACTUAL=$(sha256sum "${TMPDIR}/${ASSET}" | awk '{print $1}')
  elif command -v shasum &>/dev/null; then
    ACTUAL=$(shasum -a 256 "${TMPDIR}/${ASSET}" | awk '{print $1}')
  else
    echo "Warning: No sha256 tool found, skipping verification" >&2
    ACTUAL="${EXPECTED}"
  fi
  if [ "${EXPECTED}" != "${ACTUAL}" ]; then
    echo "Checksum mismatch!" >&2
    echo "  Expected: ${EXPECTED}" >&2
    echo "  Actual:   ${ACTUAL}" >&2
    exit 1
  fi
  echo "Checksum verified."
fi

# Extract
echo "Extracting..."
if [ "$EXT" = "zip" ]; then
  unzip -q -o "${TMPDIR}/${ASSET}" -d "${TMPDIR}/extract"
else
  mkdir -p "${TMPDIR}/extract"
  tar xzf "${TMPDIR}/${ASSET}" -C "${TMPDIR}/extract"
fi

# Install binary
BINARY="fabio"
if [ "$OS" = "windows" ]; then
  BINARY="fabio.exe"
fi

if [ -f "${TMPDIR}/extract/${BINARY}" ]; then
  cp "${TMPDIR}/extract/${BINARY}" "${INSTALL_DIR}/${BINARY}"
elif [ -f "${TMPDIR}/extract/fabio-${OS}-${ARCH}/${BINARY}" ]; then
  cp "${TMPDIR}/extract/fabio-${OS}-${ARCH}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
else
  # Find the binary wherever it was extracted
  FOUND=$(find "${TMPDIR}/extract" -name "${BINARY}" -type f | head -1)
  if [ -z "${FOUND}" ]; then
    echo "Could not find ${BINARY} in archive" >&2
    exit 1
  fi
  cp "${FOUND}" "${INSTALL_DIR}/${BINARY}"
fi

chmod +x "${INSTALL_DIR}/${BINARY}"

echo ""
echo "fabio ${VERSION} installed to ${INSTALL_DIR}/${BINARY}"
echo ""

# Check if install dir is in PATH
if ! echo "${PATH}" | tr ':' '\n' | grep -q "^${INSTALL_DIR}$"; then
  echo "Note: ${INSTALL_DIR} is not in your PATH."
  echo "Add it with:"
  echo "  export PATH=\"${INSTALL_DIR}:\${PATH}\""
  echo ""
fi

# Verify installation
if command -v fabio &>/dev/null; then
  echo "Verification:"
  fabio auth status 2>/dev/null || echo "  fabio installed successfully (run 'fabio auth login' to authenticate)"
else
  echo "Run 'fabio auth status' to verify installation after adding to PATH."
fi
