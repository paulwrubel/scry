#!/bin/sh
set -eu

REPO="paulwrubel/scry"
INSTALL_DIR="${HOME}/.local/bin"

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)
        TARGET_OS="unknown-linux-gnu"
        ;;
    Darwin)
        TARGET_OS="apple-darwin"
        ;;
    *)
        echo "Unsupported operating system: $OS (only Linux and macOS are supported)"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH (only x86_64 and ARM64 are supported)"
        exit 1
        ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/scry-${TARGET}.tar.gz"

if command -v curl >/dev/null 2>&1; then
    FETCH="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    FETCH="wget -qO-"
else
    echo "Neither curl nor wget found. Please install one and try again."
    exit 1
fi

mkdir -p "$INSTALL_DIR"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading scry for ${TARGET}..."
$FETCH "$RELEASE_URL" | tar xz -C "$TMPDIR"

install "$TMPDIR/scry" "$INSTALL_DIR/scry"

echo ""
echo "scry installed to ${INSTALL_DIR}/scry"

case ":$PATH:" in
    *:"$INSTALL_DIR":*)
        ;;
    *)
        echo ""
        echo "Note: ${INSTALL_DIR} is not in your PATH."
        echo "Add the following to your shell config:"
        echo ""
        echo "    export PATH=\"\${HOME}/.local/bin:\${PATH}\""
        ;;
esac

echo ""
echo "Run 'scry --help' to get started!"
