#!/usr/bin/env bash

# Atrium automated installation script
# Supported platforms: macOS (x86_64, arm64), Linux (x86_64, aarch64)

set -e

echo "=== Atrium Installer ==="

# 1. Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        OS_NAME="macos"
        if [ "$ARCH" = "arm64" ]; then
            BINARY_ARCH="arm64"
        else
            BINARY_ARCH="x64"
        fi
        ;;
    Linux)
        OS_NAME="linux"
        if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
            BINARY_ARCH="arm64"
        else
            BINARY_ARCH="x64"
        fi
        ;;
    *)
        echo "Error: Atrium currently only supports macOS and Linux."
        exit 1
        ;;
esac

BINARY_NAME="atriumd-${OS_NAME}-${BINARY_ARCH}"
DOWNLOAD_URL="https://github.com/MrDawell/atrium/releases/latest/download/${BINARY_NAME}"

echo "Detected OS: $OS ($ARCH)"
echo "Target Binary: $BINARY_NAME"

# 2. Determine Installation Path
INSTALL_DIR="/usr/local/bin"
USE_SUDO=false

if [ ! -w "$INSTALL_DIR" ]; then
    if command -v sudo >/dev/null 2>&1; then
        USE_SUDO=true
    else
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
        echo "Notice: /usr/local/bin is not writeable and sudo is not available. Falling back to $INSTALL_DIR"
    fi
fi

# 3. Download atriumd Native Binary
TEMP_FILE="$(mktemp)"
trap 'rm -f "$TEMP_FILE"' EXIT

echo "Downloading atriumd binary..."
DOWNLOAD_SUCCESS=false

if command -v curl >/dev/null 2>&1; then
    if curl -fsSL -o "$TEMP_FILE" "$DOWNLOAD_URL"; then
        DOWNLOAD_SUCCESS=true
    fi
elif command -v wget >/dev/null 2>&1; then
    if wget -qO "$TEMP_FILE" "$DOWNLOAD_URL"; then
        DOWNLOAD_SUCCESS=true
    fi
else
    echo "Error: curl or wget is required to download the binary."
    exit 1
fi

if [ "$DOWNLOAD_SUCCESS" = false ]; then
    echo "Error: Failed to download the precompiled binary from $DOWNLOAD_URL."
    echo "This is likely because the repository or release is not yet published on GitHub."
    echo "You can build atriumd from source locally in this directory by running:"
    echo "  cargo build --release --bin atriumd"
    echo "And copying target/release/atriumd manually to ${INSTALL_DIR}/atriumd"
    exit 1
fi

chmod +x "$TEMP_FILE"

# 4. Copy to Installation Directory
echo "Installing atriumd to ${INSTALL_DIR}/atriumd..."
if [ "$USE_SUDO" = true ]; then
    sudo mv "$TEMP_FILE" "${INSTALL_DIR}/atriumd"
else
    mv "$TEMP_FILE" "${INSTALL_DIR}/atriumd"
fi

# 5. Handle Python environment preparation quietly
if command -v python3 >/dev/null 2>&1; then
    echo "python3 detected. Installing python requirements (grpcio, grpcio-tools)..."
    python3 -m pip install -q grpcio grpcio-tools || true
else
    echo "Warning: python3 not detected. The Python bridge CLI and MCP server will require python3 to run."
fi

echo "=== Installation Completed Successfully! ==="
echo "You can now run 'atriumd' to start the local background daemon."

# Warn if installed in local path not in PATH
if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
    case :$PATH: in
        *:"$INSTALL_DIR":*) ;;
        *)
            echo "Warning: $INSTALL_DIR is not in your PATH. Please add it to your shell config file (e.g. ~/.bashrc or ~/.zshrc):"
            echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
            ;;
    esac
fi
