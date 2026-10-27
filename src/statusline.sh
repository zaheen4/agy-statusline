#!/usr/bin/env bash
set -euo pipefail

# Read stdin
INPUT="$(cat)"
[ -z "$INPUT" ] && INPUT="{}"

# Terminal width
COLS="${COLUMNS:-}"
if [ -z "$COLS" ]; then
    COLS=$(tput cols 2>/dev/null || echo 120)
fi

# Parsing via python3 or jq or basic sed fallback
if command -v jq >/dev/null 2>&1; then
    MODEL=$(echo "$INPUT" | jq -r '.model // "Default"')
    CYCLE_MODE=$(echo "$INPUT" | jq -r '.cycle_mode // "default"' | tr '[:upper:]' '[:lower:]')
    BRANCH=$(echo "$INPUT" | jq -r '.vcs.branch // ""')
    DIRTY=$(echo "$INPUT" | jq -r '.vcs.dirty // false')
    USED_TOKENS=$(echo "$INPUT" | jq -r '.context.used_tokens // 0')
    MAX_TOKENS=$(echo "$INPUT" | jq -r '.context.max_tokens // 1')
else
    # Fallback parsing if jq is unavailable
    PARSED=$(python3 - "$INPUT" <<'PYEOF'
import sys, json
try:
    p = json.loads(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1] else {}
except Exception:
    p = {}
model = p.get("model", "Default")
mode = p.get("cycle_mode", "default").lower()
vcs = p.get("vcs", {})
branch = vcs.get("branch", "")
dirty = "true" if vcs.get("dirty", False) else "false"
ctx = p.get("context", {})
used = ctx.get("used_tokens", 0)
max_t = ctx.get("max_tokens", 1)
print(f"{model}\t{mode}\t{branch}\t{dirty}\t{used}\t{max_t}")
PYEOF
)
    IFS=$'\t' read -r MODEL CYCLE_MODE BRANCH DIRTY USED_TOKENS MAX_TOKENS <<< "$PARSED"
fi

# Model cleaning
MODEL_CLEAN=$(echo "$MODEL" | sed -E \
    -e 's/^(Gemini|Claude|Google)[[:space:]]+//' \
    -e 's/\(Extra High\)|\(XHigh\)/[XH]/I' \
    -e 's/\(Max\)/[MAX]/I' \
    -e 's/\(High\)/[H]/I' \
    -e 's/\(Medium\)/[M]/I' \
    -e 's/\(Low\)/[L]/I' \
    -e 's/\(Thinking\)/[T]/I')

# Mode badge
case "$CYCLE_MODE" in
    auto) MODE_BADGE=$'\033[42;30m AUTO \033[0m' ;;
    plan) MODE_BADGE=$'\033[46;30m PLAN \033[0m' ;;
    accept-edits) MODE_BADGE=$'\033[43;30m EDITS \033[0m' ;;
    *) MODE_BADGE=$'\033[44;30m DEFAULT \033[0m' ;;
esac

# SSH check
SSH_BADGE=""
if [ -n "${SSH_CLIENT:-}" ] || [ -n "${SSH_TTY:-}" ] || [ -n "${SSH_CONNECTION:-}" ]; then
    SSH_BADGE=$'\033[38;5;244m󰢹 SSH\033[0m '
fi

# Git segment
GIT_SEG=""
if [ -n "$BRANCH" ]; then
    DIRTY_FLAG=""
    [ "$DIRTY" = "true" ] && DIRTY_FLAG="*"
    GIT_SEG=$'\033[34m󰘬 '"$BRANCH$DIRTY_FLAG"$'\033[0m'
fi

# Context meter
PCT=$(awk -v u="$USED_TOKENS" -v m="$MAX_TOKENS" 'BEGIN { if (m <= 0) printf "0.0"; else printf "%.1f", (u/m)*100 }')
FILLED=$(awk -v p="$PCT" 'BEGIN { f = int(p / 20); if (f > 5) f = 5; if (f < 0) f = 0; print f }')
BLOCKS=""
for i in 0 1 2 3 4; do
    if [ "$i" -lt "$FILLED" ]; then
        BLOCKS="${BLOCKS}█"
    else
        BLOCKS="${BLOCKS}░"
    fi
done
CTX_SEG=$'\033[35m['"$BLOCKS"$'] '"$PCT"$'%\033[0m'

# Quota check
QUOTA_SEG=""
CACHE_PATH="${TMPDIR:-/tmp}/agy_quota_cache.json"
[ ! -f "$CACHE_PATH" ] && CACHE_PATH="${TEMP:-/tmp}/agy_quota_cache.json"
if [ -f "$CACHE_PATH" ]; then
    H5=$(grep -o '"5h_pct"[^,}]*' "$CACHE_PATH" 2>/dev/null | grep -o '[0-9]\+' || true)
    WK=$(grep -o '"week_pct"[^,}]*' "$CACHE_PATH" 2>/dev/null | grep -o '[0-9]\+' || true)
    if [ -n "$H5" ] && [ -n "$WK" ]; then
        QUOTA_SEG=$'\033[36mQ:'"$H5"'%/'"$WK"'%\033[0m'
    fi
fi

# Assembly
DIV=$' \033[38;5;239m│\033[0m '
LINE="${SSH_BADGE}"$'\033[32m\033[0m '"${MODE_BADGE} "$'\033[1m'"${MODEL_CLEAN}"$'\033[0m'

if [ -n "$GIT_SEG" ]; then
    LINE="${LINE}${DIV}${GIT_SEG}"
fi

if [ "$COLS" -ge 85 ]; then
    if [ -n "$QUOTA_SEG" ]; then
        LINE="${LINE}${DIV}${QUOTA_SEG}"
    fi
    LINE="${LINE}${DIV}${CTX_SEG}"
fi

printf " %s \n" "$LINE"
