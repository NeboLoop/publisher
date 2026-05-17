#!/bin/bash
# Build neboai for all supported platforms
# Requires: rustup targets installed (see below)
#
# Install targets first:
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin
#   rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl
#   rustup target add x86_64-pc-windows-gnu
#
# For Linux musl cross-compilation on macOS:
#   brew install filosottile/musl-cross/musl-cross
#
# For Windows cross-compilation on macOS:
#   brew install mingw-w64

set -euo pipefail

BINARY_NAME="neboai"
DIST_DIR="../dist"

declare -A TARGETS
TARGETS[darwin-arm64]="aarch64-apple-darwin"
TARGETS[darwin-amd64]="x86_64-apple-darwin"
TARGETS[linux-arm64]="aarch64-unknown-linux-musl"
TARGETS[linux-amd64]="x86_64-unknown-linux-musl"
TARGETS[windows-amd64]="x86_64-pc-windows-gnu"

rm -rf "$DIST_DIR"

for platform in "${!TARGETS[@]}"; do
  target="${TARGETS[$platform]}"
  echo "Building $platform ($target)..."

  if cargo build --release --target "$target" 2>/dev/null; then
    mkdir -p "$DIST_DIR/$platform"

    if [[ "$platform" == windows-* ]]; then
      cp "target/$target/release/${BINARY_NAME}.exe" "$DIST_DIR/$platform/${BINARY_NAME}.exe"
    else
      cp "target/$target/release/$BINARY_NAME" "$DIST_DIR/$platform/$BINARY_NAME"
      strip "$DIST_DIR/$platform/$BINARY_NAME" 2>/dev/null || true
    fi

    SIZE=$(du -h "$DIST_DIR/$platform/$BINARY_NAME"* | cut -f1)
    echo "  Done: $SIZE"
  else
    echo "  SKIPPED (target not installed or cross-compilation unavailable)"
  fi
done

echo ""
echo "Built binaries:"
find "$DIST_DIR" -type f | sort
