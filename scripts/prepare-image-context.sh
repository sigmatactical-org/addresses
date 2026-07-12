#!/usr/bin/env bash
# Populate build/image/ for `docker build -f Dockerfile build/image`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/sigma-addresses"
if [[ ! -f "$BIN" && -f "$ROOT/../target/release/sigma-addresses" ]]; then
  BIN="$ROOT/../target/release/sigma-addresses"
fi
if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release" >&2
  exit 1
fi

mkdir -p "$ROOT/build/image"
rm -f "$ROOT/build/image/sigma-addresses"
cp "$BIN" "$ROOT/build/image/sigma-addresses"
chmod 555 "$ROOT/build/image/sigma-addresses"
