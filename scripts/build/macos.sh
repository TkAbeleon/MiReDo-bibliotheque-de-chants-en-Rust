#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_DIR="$ROOT_DIR/apps/miredo"
DIST_DIR="$ROOT_DIR/dist/macos"
BIN_NAME="miredo"

mkdir -p "$DIST_DIR"

if ! command -v rustc >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
  echo "Rust n'est pas installé. Installation automatique..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Ce script est conçu pour macOS."
  echo "Sur Linux, la cross-compilation macOS est possible mais nécessite un outil comme osxcross ou zig."
  echo "Exemple : rustup target add x86_64-apple-darwin aarch64-apple-darwin"
  exit 1
fi

rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --manifest-path "$APP_DIR/Cargo.toml" --release --target x86_64-apple-darwin
cp "$APP_DIR/target/x86_64-apple-darwin/release/$BIN_NAME" "$DIST_DIR/${BIN_NAME}-macos-x86_64"

echo "Binaire généré : $DIST_DIR/${BIN_NAME}-macos-x86_64"
