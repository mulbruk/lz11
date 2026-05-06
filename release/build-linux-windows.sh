#!/usr/bin/env bash
#
# Build Linux (musl) and Windows (MinGW) release artifacts for lz11
# in a Podman container, then package them into ./dist/.
#
# Usage:  ./release/build-linux-windows.sh
#
# Requires: podman (rootless is fine).

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

VERSION="$(awk -F\" '/^version[[:space:]]*=/ {print $2; exit}' Cargo.toml)"
IMAGE="lz11-release-builder:latest"

LINUX_TRIPLE="x86_64-unknown-linux-musl"
WINDOWS_TRIPLE="x86_64-pc-windows-gnu"

# ---------------------------------------------------------------------------
# 1. Build the container image.
#    Cached on subsequent runs unless release/Containerfile changes.
# ---------------------------------------------------------------------------
podman build \
  --tag "$IMAGE" \
  --file release/Containerfile \
  release/

# ---------------------------------------------------------------------------
# 2. Compile both targets inside the container.
#
#    --userns=keep-id      maps the host user into the container so files
#                          written into the bind mount are owned by us, not
#                          root.
#    :Z on the bind mount  applies an SELinux private label so the container
#                          can read the project source on Fedora.
#    CARGO_HOME=/build/.cargo  keeps the registry and build cache in the
#                          project directory (gitignored) so it persists
#                          between runs without leaking the host's CARGO_HOME.
#    RUSTFLAGS=--remap-path-prefix  rewrites embedded paths in debug info
#                          and panic messages so nothing about the build
#                          environment leaks into the released binary.
# ---------------------------------------------------------------------------
podman run --rm \
  --userns=keep-id \
  --volume "$PROJECT_ROOT:/build:Z" \
  --workdir /build \
  --env CARGO_HOME=/build/.cargo \
  --env LINUX_TRIPLE="$LINUX_TRIPLE" \
  --env WINDOWS_TRIPLE="$WINDOWS_TRIPLE" \
  "$IMAGE" \
  bash -euo pipefail -c '
    # Order matters: rustc applies --remap-path-prefix rules with last-match-
    # wins semantics, so put the broadest rule (PWD) first and progressively
    # more specific rules after it. Otherwise a general rule like
    # "/build -> /build" would shadow a more specific one like
    # "/build/.cargo -> /cargo".
    export RUSTFLAGS="\
      --remap-path-prefix=${PWD}=/build \
      --remap-path-prefix=${RUSTUP_HOME:-/usr/local/rustup}=/rustup \
      --remap-path-prefix=${CARGO_HOME}=/cargo"

    cargo build --release --features cli --target "$LINUX_TRIPLE"
    cargo build --release --features cli --target "$WINDOWS_TRIPLE"
  '

# ---------------------------------------------------------------------------
# 3. Package the artifacts.
# ---------------------------------------------------------------------------
mkdir -p dist
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

# Linux: tar.gz with the bare ELF + docs.
LINUX_NAME="lz11-$VERSION-$LINUX_TRIPLE"
LINUX_DIR="$STAGE/$LINUX_NAME"
mkdir -p "$LINUX_DIR"
cp "target/$LINUX_TRIPLE/release/lz11" "$LINUX_DIR/"
cp README.md LICENSE-MIT LICENSE-APACHE "$LINUX_DIR/"
tar -C "$STAGE" -czf "dist/$LINUX_NAME.tar.gz" "$LINUX_NAME"

# Windows: zip with the .exe + docs.
WINDOWS_NAME="lz11-$VERSION-$WINDOWS_TRIPLE"
WINDOWS_DIR="$STAGE/$WINDOWS_NAME"
mkdir -p "$WINDOWS_DIR"
cp "target/$WINDOWS_TRIPLE/release/lz11.exe" "$WINDOWS_DIR/"
cp README.md LICENSE-MIT LICENSE-APACHE "$WINDOWS_DIR/"
( cd "$STAGE" && zip -qr "$PROJECT_ROOT/dist/$WINDOWS_NAME.zip" "$WINDOWS_NAME" )

echo
echo "Built artifacts:"
ls -lh dist/
