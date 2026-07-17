#!/usr/bin/env sh
set -e

REPO="haider-patanwala/figma-cli"
BIN="figma-cli"
INSTALL_DIR="/usr/local/bin"

# Detect OS + arch
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Darwin" ]; then
  echo "Error: only macOS is supported right now." >&2
  exit 1
fi

case "$ARCH" in
  arm64)  ASSET="figma-cli-macos-arm64" ;;
  x86_64) ASSET="figma-cli-macos-x86_64" ;;
  *)
    echo "Error: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

URL="https://github.com/$REPO/releases/latest/download/$ASSET"

echo "Downloading $BIN ($ARCH)..."
TMP="$(mktemp)"
curl -fsSL "$URL" -o "$TMP"
chmod +x "$TMP"

# Install — try /usr/local/bin, fall back to ~/bin
if [ -w "$INSTALL_DIR" ] || sudo -n true 2>/dev/null; then
  sudo mv "$TMP" "$INSTALL_DIR/$BIN"
  echo "Installed to $INSTALL_DIR/$BIN"
else
  mkdir -p "$HOME/bin"
  mv "$TMP" "$HOME/bin/$BIN"
  INSTALL_DIR="$HOME/bin"
  echo "Installed to $HOME/bin/$BIN"
  echo "Add ~/bin to your PATH if it isn't already:"
  echo "  export PATH=\"\$HOME/bin:\$PATH\""
fi

# Install the Figma plugin (writes files to ~/.figma-ds-cli/plugin/)
echo ""
echo "Installing Figma bridge plugin..."
"$INSTALL_DIR/$BIN" connect 2>/dev/null || true

echo ""
echo "Done. Run 'figma-cli connect' to start."
