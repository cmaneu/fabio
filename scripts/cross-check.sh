#!/usr/bin/env bash
# scripts/cross-check.sh — Validate compilation across all 6 CI targets from Linux.
#
# Prerequisites (run with --setup to install automatically):
#   System:  lld, clang, zig
#   Cargo:   cargo-xwin, cargo-zigbuild
#   Rustup:  targets for windows-msvc, apple-darwin, linux-gnu arm64
#
# Usage:
#   ./scripts/cross-check.sh              # quick check (cargo check) all 6 targets
#   ./scripts/cross-check.sh --full       # full release build all 6 targets
#   ./scripts/cross-check.sh --target windows-x64   # single target
#   ./scripts/cross-check.sh --setup      # install all prerequisites
#
# Supported --target values:
#   linux-x64, linux-arm64, macos-x64, macos-arm64, windows-x64, windows-arm64

set -euo pipefail

# ── Target definitions ──────────────────────────────────────────────────────
# Format: "name|rust_triple|tool"
# tool: native, zigbuild, xwin
TARGETS=(
  "linux-x64|x86_64-unknown-linux-gnu|native"
  "linux-arm64|aarch64-unknown-linux-gnu|zigbuild"
  "macos-x64|x86_64-apple-darwin|zigbuild"
  "macos-arm64|aarch64-apple-darwin|zigbuild"
  "windows-x64|x86_64-pc-windows-msvc|xwin"
  "windows-arm64|aarch64-pc-windows-msvc|xwin"
)

# ── Defaults ────────────────────────────────────────────────────────────────
MODE="quick"      # quick = check, full = build --release
FILTER=""          # empty = all targets
DO_SETUP=false
FMT_CHECK=true
PASSED=0
FAILED=0
SKIPPED=0
RESULTS=()

# ── Colors (disabled if not a terminal) ─────────────────────────────────────
if [ -t 1 ]; then
  GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[0;33m'
  BLUE='\033[0;34m'; BOLD='\033[1m'; RESET='\033[0m'
else
  GREEN=''; RED=''; YELLOW=''; BLUE=''; BOLD=''; RESET=''
fi

# ── Helpers ─────────────────────────────────────────────────────────────────
info()  { echo -e "${BLUE}[info]${RESET} $*"; }
ok()    { echo -e "${GREEN}[pass]${RESET} $*"; }
fail()  { echo -e "${RED}[FAIL]${RESET} $*"; }
warn()  { echo -e "${YELLOW}[warn]${RESET} $*"; }
header(){ echo -e "\n${BOLD}── $* ──${RESET}"; }

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Options:
  --quick           Type-check only, no codegen (default)
  --full            Full release build (slower, tests linking)
  --target <name>   Only check one target (e.g., windows-x64, macos-arm64)
  --no-fmt          Skip cargo fmt check
  --setup           Install all prerequisites and exit
  -h, --help        Show this help

Targets: linux-x64, linux-arm64, macos-x64, macos-arm64, windows-x64, windows-arm64
EOF
  exit 0
}

# ── Parse arguments ─────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case $1 in
    --quick)   MODE="quick"; shift ;;
    --full)    MODE="full"; shift ;;
    --target)  FILTER="$2"; shift 2 ;;
    --no-fmt)  FMT_CHECK=false; shift ;;
    --setup)   DO_SETUP=true; shift ;;
    -h|--help) usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

# ── Setup mode ──────────────────────────────────────────────────────────────
do_setup() {
  header "Installing prerequisites"

  info "Installing system packages (lld, clang)..."
  sudo apt update -qq && sudo apt install -y -qq lld clang

  if ! command -v zig &>/dev/null; then
    info "Installing zig via snap..."
    sudo snap install zig --classic --beta
  else
    info "zig already installed: $(zig version)"
  fi

  info "Installing cargo-xwin and cargo-zigbuild..."
  cargo install cargo-xwin cargo-zigbuild

  info "Adding rustup targets..."
  rustup target add \
    x86_64-pc-windows-msvc \
    aarch64-pc-windows-msvc \
    x86_64-apple-darwin \
    aarch64-apple-darwin \
    aarch64-unknown-linux-gnu

  echo ""
  ok "Setup complete. Run $(basename "$0") to validate all targets."
}

if $DO_SETUP; then
  do_setup
  exit 0
fi

# ── Prerequisite checks ────────────────────────────────────────────────────
check_prereqs() {
  local missing=0

  for cmd in lld clang; do
    if ! command -v "$cmd" &>/dev/null; then
      warn "Missing system package: $cmd"
      missing=1
    fi
  done

  if ! command -v zig &>/dev/null; then
    warn "Missing: zig (install via: sudo snap install zig --classic --beta)"
    missing=1
  fi

  for tool in cargo-xwin cargo-zigbuild; do
    # cargo-xwin binary is 'cargo-xwin', cargo-zigbuild binary is 'cargo-zigbuild'
    if ! command -v "$tool" &>/dev/null; then
      warn "Missing cargo tool: $tool (install via: cargo install $tool)"
      missing=1
    fi
  done

  if [ $missing -ne 0 ]; then
    echo ""
    warn "Some prerequisites are missing. Run: $(basename "$0") --setup"
    exit 1
  fi
}

# ── Build functions ─────────────────────────────────────────────────────────
run_native() {
  local triple=$1
  if [ "$MODE" = "quick" ]; then
    cargo check --target "$triple" 2>&1
  else
    cargo build --release --target "$triple" 2>&1
  fi
}

run_zigbuild() {
  local triple=$1
  if [ "$MODE" = "quick" ]; then
    # zigbuild doesn't support 'check', but a debug build without --release
    # is the fastest full compile
    cargo zigbuild --target "$triple" 2>&1
  else
    cargo zigbuild --release --target "$triple" 2>&1
  fi
}

run_xwin() {
  local triple=$1
  if [ "$MODE" = "quick" ]; then
    cargo xwin check --target "$triple" 2>&1
  else
    cargo xwin build --release --target "$triple" 2>&1
  fi
}

# ── Main ────────────────────────────────────────────────────────────────────
START_TIME=$SECONDS

header "Fabio cross-compilation check (mode: $MODE)"
check_prereqs

# Format check (once, target-independent)
if $FMT_CHECK; then
  header "cargo fmt --check"
  if cargo fmt -- --check 2>&1; then
    ok "Formatting OK"
  else
    fail "Formatting errors found (run: cargo fmt)"
    FAILED=$((FAILED + 1))
    RESULTS+=("fmt|FAIL")
  fi
fi

# Per-target checks
for entry in "${TARGETS[@]}"; do
  IFS='|' read -r name triple tool <<< "$entry"

  # Filter if --target was given
  if [ -n "$FILTER" ] && [ "$name" != "$FILTER" ]; then
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  header "$name ($triple) [$tool]"
  target_start=$SECONDS

  set +e
  case $tool in
    native)   output=$(run_native "$triple" 2>&1) ; rc=$? ;;
    zigbuild) output=$(run_zigbuild "$triple" 2>&1) ; rc=$? ;;
    xwin)     output=$(run_xwin "$triple" 2>&1) ; rc=$? ;;
  esac
  set -e

  elapsed=$((SECONDS - target_start))

  if [ $rc -eq 0 ]; then
    ok "$name passed (${elapsed}s)"
    PASSED=$((PASSED + 1))
    RESULTS+=("$name|PASS|${elapsed}s")
  else
    fail "$name failed (${elapsed}s)"
    echo "$output" | tail -30
    FAILED=$((FAILED + 1))
    RESULTS+=("$name|FAIL|${elapsed}s")
  fi
done

# ── Summary ─────────────────────────────────────────────────────────────────
TOTAL_TIME=$((SECONDS - START_TIME))

header "Results"
printf "  %-20s %s\n" "TARGET" "STATUS"
printf "  %-20s %s\n" "------" "------"
for r in "${RESULTS[@]}"; do
  IFS='|' read -r rname rstatus rtime <<< "$r"
  if [ "$rstatus" = "PASS" ]; then
    printf "  %-20s ${GREEN}%s${RESET}  %s\n" "$rname" "$rstatus" "$rtime"
  else
    printf "  %-20s ${RED}%s${RESET}  %s\n" "$rname" "$rstatus" "${rtime:-}"
  fi
done

echo ""
info "Passed: $PASSED  Failed: $FAILED  Skipped: $SKIPPED  Total: ${TOTAL_TIME}s"

if [ $FAILED -gt 0 ]; then
  exit 1
fi
