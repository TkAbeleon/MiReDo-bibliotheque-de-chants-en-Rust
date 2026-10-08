#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_DIR="$ROOT_DIR/apps/miredo"
DIST_DIR="$ROOT_DIR/dist/linux-fedora"
TARGET="x86_64-unknown-linux-gnu"
BIN_NAME="miredo"

mkdir -p "$DIST_DIR"

if ! command -v rustc >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
  echo "Rust n'est pas installé. Installation automatique..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! command -v dnf >/dev/null 2>&1; then
  echo "Ce script est destiné à Fedora/RHEL/CentOS."
  exit 1
fi

sudo dnf install -y gcc make pkgconf-pkg-config curl git
rustup target add "$TARGET"

cargo build --manifest-path "$APP_DIR/Cargo.toml" --release --target "$TARGET"
cp "$APP_DIR/target/$TARGET/release/$BIN_NAME" "$DIST_DIR/${BIN_NAME}-linux-fedora-x86_64"

echo "Binaire généré : $DIST_DIR/${BIN_NAME}-linux-fedora-x86_64"
