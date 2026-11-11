#!/usr/bin/env bash
# Builds the WASM game and assembles a static site in dist/.
# Usage: scripts/build-web.sh [--serve]   (--serve hosts dist/ on http://localhost:8000)
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build -p oculus-game --release --target wasm32-unknown-unknown
rm -rf dist && mkdir -p dist
cp web/index.html web/gl.js dist/
cp target/wasm32-unknown-unknown/release/oculus-battle.wasm dist/
echo "dist/ ready ($(du -h dist/oculus-battle.wasm | cut -f1) wasm)"

if [[ "${1:-}" == "--serve" ]]; then
  python3 -m http.server 8000 --directory dist
fi
