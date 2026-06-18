#!/usr/bin/env bash
# Install the figma-cli binary onto the PATH.
#
# Strategy:
#   1. If a prebuilt binary for this platform is bundled in this dir, install it.
#   2. Otherwise, build from source if a Rust toolchain + the repo are present.
#
# Install target: ~/.local/bin (created if missing). Add it to PATH if needed.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
DEST="${FIGMA_CLI_BIN_DIR:-$HOME/.local/bin}"
mkdir -p "$DEST"

uname_s="$(uname -s)"
uname_m="$(uname -m)"
case "$uname_s-$uname_m" in
  Darwin-arm64)  TRIPLE="aarch64-apple-darwin" ;;
  Darwin-x86_64) TRIPLE="x86_64-apple-darwin" ;;
  Linux-x86_64)  TRIPLE="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64) TRIPLE="aarch64-unknown-linux-gnu" ;;
  *) TRIPLE="unknown" ;;
esac

PREBUILT="$HERE/figma-cli-$TRIPLE"
if [ -f "$PREBUILT" ]; then
  install -m 0755 "$PREBUILT" "$DEST/figma-cli"
  echo "Installed prebuilt figma-cli ($TRIPLE) -> $DEST/figma-cli"
elif [ -f "$HERE/figma-cli" ]; then
  install -m 0755 "$HERE/figma-cli" "$DEST/figma-cli"
  echo "Installed bundled figma-cli -> $DEST/figma-cli"
else
  # Fall back to building from source (skill lives at <repo>/rust/skill).
  REPO_RUST="$HERE/../.."
  if command -v cargo >/dev/null 2>&1 && [ -f "$REPO_RUST/Cargo.toml" ]; then
    echo "No prebuilt binary for $TRIPLE; building from source (this can take a few minutes)…"
    ( cd "$REPO_RUST" && cargo build --release )
    install -m 0755 "$REPO_RUST/target/release/figma-cli" "$DEST/figma-cli"
    echo "Built and installed figma-cli -> $DEST/figma-cli"
  else
    echo "No prebuilt binary for $TRIPLE and no Rust toolchain found." >&2
    echo "Install Rust (https://rustup.rs) and re-run, or drop a prebuilt binary in $HERE." >&2
    exit 1
  fi
fi

# Node fallback: a handful of heavy/niche commands (extract, import, spec,
# instantiate, blocks, recreate-url, screenshot-url, remove-bg, dev, section,
# grid, annotate, plugins, api, sizes, combos) forward to the original Node CLI.
# Install it next to the binary as `js/` so the Rust binary auto-locates it.
JS_SRC="$HERE/../js"
if [ -d "$JS_SRC" ]; then
  rm -rf "$DEST/js"
  cp -R "$JS_SRC" "$DEST/js"
  if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
    echo "Installing Node fallback dependencies (for extract/import/blocks/url-tools/…)…"
    ( cd "$DEST/js" && npm install --omit=dev --silent >/dev/null 2>&1 ) \
      && echo "Node fallback ready at $DEST/js" \
      || echo "Note: \`npm install\` in $DEST/js failed — those fallback commands need it; re-run manually."
  else
    echo "Note: Node/npm not found. The Rust-native commands work without Node;"
    echo "      the fallback commands (extract/import/blocks/url-tools/…) need Node 18+."
  fi
fi

case ":$PATH:" in
  *":$DEST:"*) ;;
  *) echo "Note: add $DEST to your PATH (e.g. echo 'export PATH=\"$DEST:\$PATH\"' >> ~/.zshrc)";;
esac

echo "Done. Next: open a design file in Figma Desktop, then run: figma-cli connect"
