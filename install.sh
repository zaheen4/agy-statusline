#!/usr/bin/env bash
set -euo pipefail

REPO="eyepatch/agy-statusline"
TARGET_DIR="${TARGET_DIR:-${HOME}/.local/bin}"
FALLBACK_DIR="${HOME}/.config/agy-statusline"
SETTINGS_DIR="${HOME}/.gemini/antigravity-cli"
SETTINGS_FILE="${SETTINGS_DIR}/settings.json"

echo "==> Installing agy-statusline..."

# Ensure target directories exist with fallback for sandboxed/restricted environments
if ! mkdir -p "$TARGET_DIR" 2>/dev/null || [ ! -w "$TARGET_DIR" ]; then
    TARGET_DIR="/tmp/bin"
    mkdir -p "$TARGET_DIR" 2>/dev/null || true
fi

if ! mkdir -p "$FALLBACK_DIR" 2>/dev/null || [ ! -w "$FALLBACK_DIR" ]; then
    FALLBACK_DIR="/tmp/agy-statusline"
    mkdir -p "$FALLBACK_DIR" 2>/dev/null || true
fi

mkdir -p "$SETTINGS_DIR" 2>/dev/null || true

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$ARCH" in
    x86_64|amd64) ARCH_TAG="x86_64" ;;
    aarch64|arm64) ARCH_TAG="aarch64" ;;
    *) ARCH_TAG="$ARCH" ;;
esac

INSTALLED_CMD=""

# 1. Check if built locally
if [ -f "./target/release/agy-statusline" ]; then
    echo "==> Using locally compiled release binary..."
    cp "./target/release/agy-statusline" "${TARGET_DIR}/agy-statusline"
    chmod +x "${TARGET_DIR}/agy-statusline"
    INSTALLED_CMD="${TARGET_DIR}/agy-statusline"
fi

# 2. If cargo is available and src/main.rs exists, build locally
if [ -z "$INSTALLED_CMD" ] && command -v cargo >/dev/null 2>&1 && [ -f "Cargo.toml" ]; then
    echo "==> Building native binary via Cargo..."
    cargo build --release
    cp "target/release/agy-statusline" "${TARGET_DIR}/agy-statusline"
    chmod +x "${TARGET_DIR}/agy-statusline"
    INSTALLED_CMD="${TARGET_DIR}/agy-statusline"
fi

# 3. Attempt download of precompiled release binary if installing via curl/remote
if [ -z "$INSTALLED_CMD" ]; then
    TARGET_NAME=""
    if [ "$OS" = "linux" ]; then
        TARGET_NAME="${ARCH_TAG}-unknown-linux-musl"
    elif [ "$OS" = "darwin" ]; then
        TARGET_NAME="${ARCH_TAG}-apple-darwin"
    fi

    if [ -n "$TARGET_NAME" ] && command -v curl >/dev/null 2>&1; then
        TARBALL_URL="https://github.com/${REPO}/releases/latest/download/agy-statusline-${TARGET_NAME}.tar.gz"
        TMP_DIR="$(mktemp -d)"
        echo "==> Fetching precompiled release binary for ${TARGET_NAME}..."
        if curl -fsSL "$TARBALL_URL" -o "${TMP_DIR}/archive.tar.gz" 2>/dev/null; then
            tar -xzf "${TMP_DIR}/archive.tar.gz" -C "$TMP_DIR"
            if [ -f "${TMP_DIR}/agy-statusline" ]; then
                mv "${TMP_DIR}/agy-statusline" "${TARGET_DIR}/agy-statusline"
                chmod +x "${TARGET_DIR}/agy-statusline"
                INSTALLED_CMD="${TARGET_DIR}/agy-statusline"
                echo "==> Successfully installed precompiled binary."
            fi
        fi
        rm -rf "$TMP_DIR"
    fi
fi

# 4. Fallback to Python reference engine
if [ -z "$INSTALLED_CMD" ]; then
    echo "==> Falling back to Python 3 reference engine..."
    if [ -f "src/statusline.py" ]; then
        cp "src/statusline.py" "${FALLBACK_DIR}/statusline.py"
    else
        curl -fsSL "https://raw.githubusercontent.com/${REPO}/main/src/statusline.py" -o "${FALLBACK_DIR}/statusline.py"
    fi
    chmod +x "${FALLBACK_DIR}/statusline.py"
    INSTALLED_CMD="python3 ${FALLBACK_DIR}/statusline.py"
fi

# 5. Safely patch settings.json
python3 - <<EOF
import json, os

path = os.path.expanduser("$SETTINGS_FILE")
data = {}
if os.path.exists(path):
    try:
        with open(path, "r") as f:
            data = json.load(f)
    except Exception:
        data = {}

data["statusLine"] = {
    "type": "command",
    "command": "$INSTALLED_CMD",
    "enabled": True
}

try:
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
    print("==> Configured agy settings.json successfully.")
except (PermissionError, OSError):
    print("==> Notice: Could not write directly to " + path + " (read-only filesystem).")
    print("==> Add the following block to your settings.json manually:")
    print(json.dumps({"statusLine": data["statusLine"]}, indent=2))
EOF

echo "==> Installation complete: ${INSTALLED_CMD}"
echo "==> Restart agy to enjoy your new statusline."
