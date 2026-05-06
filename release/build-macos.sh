#!/usr/bin/env bash
#
# Build a universal (aarch64 + x86_64) macOS release artifact for lz11
# and package it into ./dist/.
#
# Usage:  ./release/build-macos.sh
#
# Requires: macOS host with rustup and Xcode command-line tools (for `lipo`).
#
# Uses --remap-path-prefix to scrub the build machine's home directory and
# project path out of the binary's embedded debug info and panic messages.

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

VERSION="$(awk -F\" '/^version[[:space:]]*=/ {print $2; exit}' Cargo.toml)"

ARM_TRIPLE="aarch64-apple-darwin"
X86_TRIPLE="x86_64-apple-darwin"
UNIVERSAL_NAME="lz11-$VERSION-universal-apple-darwin"

# ---------------------------------------------------------------------------
# 1. Make sure both Apple targets are installed in the active toolchain.
# ---------------------------------------------------------------------------
rustup target add "$ARM_TRIPLE" "$X86_TRIPLE"

# ---------------------------------------------------------------------------
# 2. Build both architectures with path remapping.
#
# Order matters: rustc applies --remap-path-prefix rules with last-match-wins
# semantics, so put the broadest rule (PWD) first and progressively more
# specific rules after it. Otherwise a general rule could shadow a more
# specific one (e.g. if CARGO_HOME lived under PWD).
# ---------------------------------------------------------------------------
export RUSTFLAGS="\
  --remap-path-prefix=$PWD=/build \
  --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup \
  --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"

cargo build --release --features cli --target "$ARM_TRIPLE"
cargo build --release --features cli --target "$X86_TRIPLE"

# ---------------------------------------------------------------------------
# 3. Combine into a universal binary and package.
# ---------------------------------------------------------------------------
mkdir -p dist
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

UNIV_DIR="$STAGE/$UNIVERSAL_NAME"
mkdir -p "$UNIV_DIR"

lipo -create \
  "target/$ARM_TRIPLE/release/lz11" \
  "target/$X86_TRIPLE/release/lz11" \
  -output "$UNIV_DIR/lz11"

cp README.md LICENSE-MIT LICENSE-APACHE "$UNIV_DIR/"

tar -C "$STAGE" -czf "dist/$UNIVERSAL_NAME.tar.gz" "$UNIVERSAL_NAME"

echo
echo "Built artifact:"
ls -lh dist/
