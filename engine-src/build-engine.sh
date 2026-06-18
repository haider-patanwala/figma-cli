#!/usr/bin/env bash
# Bundle the JSX->Plugin-API codegen engine into a single QuickJS-loadable file.
#
# figma-client.js imports `ws` and `./figma-patch.js`. Neither is needed for
# codegen, so we copy figma-client.js next to a stub figma-patch.js and alias
# `ws` to a stub. Output: rust/assets/engine.js (embedded via include_str!).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
OUT="$HERE/../assets/engine.js"

# Copy the real codegen source next to our stub so its relative
# `./figma-patch.js` import resolves to the stub (not the real, fs-using one).
cp "$REPO/src/figma-client.js" "$HERE/figma-client.js"
cp "$HERE/figma-patch.stub.js" "$HERE/figma-patch.js"

npx --no-install esbuild "$HERE/entry.mjs" \
  --bundle \
  --format=iife \
  --platform=neutral \
  --target=es2020 \
  --alias:ws="$HERE/ws.stub.js" \
  --outfile="$OUT"

echo "engine bundled -> $OUT ($(wc -c < "$OUT") bytes)"
