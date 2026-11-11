#!/usr/bin/env bash
set -euo pipefail

# Ensure standard user binary locations exist in PATH
export PATH="$HOME/.local/bin:$HOME/.local/share/mise/shims:$HOME/.local/share/mise/installs/antigravity-cli/latest:$PATH"

# 1. Pre-flight connectivity check: exit 0 quietly when offline
if command -v curl >/dev/null 2>&1; then
  if ! curl -s --connect-timeout 2 -m 4 -I https://www.googleapis.com >/dev/null 2>&1; then
    exit 0
  fi
elif command -v ping >/dev/null 2>&1; then
  if ! ping -c 1 -W 2 8.8.8.8 >/dev/null 2>&1; then
    exit 0
  fi
fi

# 2. Suppress browser launch under all circumstances
export BROWSER=/bin/true
export DISPLAY=""
export WAYLAND_DISPLAY=""
export CI=1

# Shadow GUI browser openers with a no-op shim
SHIM_DIR=$(mktemp -d "${TMPDIR:-/tmp}/agy-nobrowser.XXXXXX" 2>/dev/null || mktemp -d)
trap 'rm -rf "$SHIM_DIR"' EXIT
cat << 'EOF' > "$SHIM_DIR/xdg-open"
#!/bin/sh
exit 0
EOF
chmod +x "$SHIM_DIR/xdg-open"
export PATH="$SHIM_DIR:$PATH"

# 3. Locate agy executable
AGY_BIN=$(command -v agy 2>/dev/null || true)
if [ -z "$AGY_BIN" ]; then
  exit 0
fi

# 4. Fetch usage quota with bounded execution timeout
CACHE_FILE="${TMPDIR:-/tmp}/agy_quota_cache.json"
TMP_FILE=$(mktemp "${CACHE_FILE}.XXXXXX" 2>/dev/null || mktemp)

if timeout 15s "$AGY_BIN" -p "/usage" --output-format json > "$TMP_FILE" 2>/dev/null; then
  # 5. Atomic write only upon valid SUCCESS payload
  if grep -q '"status"[[:space:]]*:[[:space:]]*"SUCCESS"' "$TMP_FILE" 2>/dev/null; then
    mv -f "$TMP_FILE" "$CACHE_FILE"
    chmod 644 "$CACHE_FILE" 2>/dev/null || true
  else
    rm -f "$TMP_FILE"
  fi
else
  rm -f "$TMP_FILE"
fi
