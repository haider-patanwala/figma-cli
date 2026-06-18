#!/usr/bin/env bash
# Bundle the pure-JS "tools" modules (gradient analysis, etc.) into a single
# QuickJS-loadable file. pngjs/jpeg-js/fs are stubbed — image decoding happens
# in the Rust host, which injects pixels as opts.__img.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
OUT="$HERE/../assets/tools.js"

# Refresh the gradient-extractor copy from upstream, patching loadImage so the
# Rust host can inject decoded pixels via opts.__img.
python3 - "$REPO/src/gradient-extractor.js" "$HERE/gradient-extractor.js" <<'PY'
import sys
src, dst = sys.argv[1], sys.argv[2]
s = open(src).read().replace('const img = loadImage(path);', 'const img = opts.__img || loadImage(path);')
open(dst, 'w').write(s)
PY

# Pure modules copied verbatim from upstream.
cp "$REPO/src/shadcn.js" "$HERE/shadcn.js"

npx --no-install esbuild "$HERE/entry.mjs" \
  --bundle --format=iife --platform=neutral --target=es2020 \
  --alias:pngjs="$HERE/pngjs.stub.js" \
  --alias:jpeg-js="$HERE/jpegjs.stub.js" \
  --alias:fs="$HERE/fs.stub.js" \
  --alias:path="$HERE/path.stub.js" \
  --outfile="$OUT"

echo "tools bundled -> $OUT ($(wc -c < "$OUT") bytes)"
