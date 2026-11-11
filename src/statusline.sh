#!/usr/bin/env bash

# Read JSON state payload from stdin
input=$(cat)

# Extract telemetry via jq with safe fallbacks
model=$(jq -r 'if (.model | type) == "object" then (.model.display_name // .model.id // "Gemini") elif (.model | type) == "string" then .model else "Gemini" end' <<< "$input" 2>/dev/null)
mode=$(jq -r 'if (.cycle_mode // "") == "" then "default" else .cycle_mode end' <<< "$input" 2>/dev/null)
ctx_pct=$(jq -r '.context_window.used_percentage // (.context.used_tokens / (.context.max_tokens // 1) * 100) // 0' <<< "$input" 2>/dev/null | awk '{printf "%.1f", $1}')
pct_int=$(jq -r '.context_window.used_percentage // (.context.used_tokens / (.context.max_tokens // 1) * 100) // 0' <<< "$input" 2>/dev/null | awk '{printf "%d", $1}')
branch=$(jq -r '.vcs.branch // empty' <<< "$input" 2>/dev/null)
dirty=$(jq -r '.vcs.dirty // false' <<< "$input" 2>/dev/null)

# Fallback: if .vcs.branch is omitted by Antigravity CLI, read directly from .git/HEAD
if [ -z "$branch" ]; then
  cwd=$(jq -r '.cwd // .workspace.current_dir // "."' <<< "$input" 2>/dev/null)
  [ -z "$cwd" ] && cwd="."

  git_dir="$cwd/.git"
  if [ -f "$git_dir" ]; then
    git_dir_path=$(sed 's/^gitdir: //' "$git_dir" 2>/dev/null)
    [[ "$git_dir_path" = /* ]] && git_dir="$git_dir_path" || git_dir="$cwd/$git_dir_path"
  fi

  if [ -d "$git_dir" ] && [ -f "$git_dir/HEAD" ]; then
    head_content=$(< "$git_dir/HEAD")
    if [[ "$head_content" == ref:\ refs/heads/* ]]; then
      branch="${head_content#ref: refs/heads/}"
    else
      branch="${head_content:0:7}"
    fi
  else
    branch=$(git -C "$cwd" rev-parse --abbrev-ref HEAD 2>/dev/null || true)
  fi

  if [ -n "$branch" ] && [ "$dirty" = "false" ]; then
    if ! git -C "$cwd" diff --quiet 2>/dev/null; then
      dirty="true"
    fi
  fi
fi

sandbox=$(jq -r '.sandbox.enabled // false' <<< "$input" 2>/dev/null)

# Terminal column width detection directly from native payload
cols=$(jq -r '.terminal_width // empty' <<< "$input" 2>/dev/null)
if ! [[ "$cols" =~ ^[0-9]+$ ]] || [ "$cols" -le 0 ]; then
  cols=${COLUMNS:-120}
fi
[[ "$cols" =~ ^[0-9]+$ ]] || cols=120

# Dynamic Model & Effort Normalizer
format_model_name() {
  local raw="$1"

  # 1. Parse effort levels
  local effort=""
  if [[ "$raw" =~ \([Ee]xtra[[:space:]]+[Hh]igh\) ]] || [[ "$raw" =~ \([Xx][Hh]igh\) ]]; then
    effort=" [XH]"
  elif [[ "$raw" =~ \(([Mm]ax)\) ]]; then
    effort=" [MAX]"
  elif [[ "$raw" =~ \(([Hh]igh)\) ]]; then
    effort=" [H]"
  elif [[ "$raw" =~ \(([Mm]edium)\) ]]; then
    effort=" [M]"
  elif [[ "$raw" =~ \(([Ll]ow)\) ]]; then
    effort=" [L]"
  elif [[ "$raw" =~ \(([Tt]hinking)\) ]]; then
    effort=" [T]"
  fi

  # 2. Strip trailing parenthetical text
  local clean="${raw%% (*}"

  # 3. Strip redundant provider brand prefix
  clean="${clean#Gemini }"
  clean="${clean#Claude }"
  clean="${clean#Google }"

  echo "${clean}${effort}"
}

model_disp=$(format_model_name "$model")

# Native Quota Telemetry (with cache fallback)
if [[ "$model" =~ [Cc]laude|[Gg]pt ]]; then
  key_5h="3p-5h"
  key_wk="3p-weekly"
  target_group="Claude and GPT models"
else
  key_5h="gemini-5h"
  key_wk="gemini-weekly"
  target_group="Gemini Models"
fi

# 1. Direct from JSON payload
five_h_pct=$(jq -r --arg k "$key_5h" '.quota[$k].remaining_fraction // empty' <<< "$input" 2>/dev/null | awk '{printf "%d", ($1 * 100) + 0.5}')
weekly_pct=$(jq -r --arg k "$key_wk" '.quota[$k].remaining_fraction // empty' <<< "$input" 2>/dev/null | awk '{printf "%d", ($1 * 100) + 0.5}')

# 2. Secondary fallback with lazy on-demand background refresh
CACHE_FILE="${TMPDIR:-/tmp}/agy_quota_cache.json"
[ ! -f "$CACHE_FILE" ] && CACHE_FILE="${TEMP:-/tmp}/agy_quota_cache.json"

if [ -z "$five_h_pct" ] || [ -z "$weekly_pct" ]; then
  # Trigger non-blocking detached refresh only if cache is stale (>30 mins)
  now=$(date +%s 2>/dev/null || echo 0)
  mtime=0
  [ -f "$CACHE_FILE" ] && mtime=$(stat -c %Y "$CACHE_FILE" 2>/dev/null || stat -f %m "$CACHE_FILE" 2>/dev/null || echo 0)
  lock_file="${TMPDIR:-/tmp}/agy_quota_fetch.lock"

  if (( now - mtime >= 1800 )); then
    lock_age=9999
    [ -f "$lock_file" ] && lock_age=$(( now - $(stat -c %Y "$lock_file" 2>/dev/null || stat -f %m "$lock_file" 2>/dev/null || echo 0) ))
    if (( lock_age >= 300 )); then
      fetcher=""
      for candidate in \
        "$HOME/.config/agy-statusline/fetch-quota.sh" \
        "$HOME/.config/agy/fetch-quota-cache.sh" \
        "$HOME/.local/bin/agy-fetch-quota"; do
        if [ -x "$candidate" ]; then
          fetcher="$candidate"
          break
        fi
      done
      if [ -n "$fetcher" ]; then
        touch "$lock_file" 2>/dev/null || true
        ( "$fetcher" >/dev/null 2>&1 & )
      fi
    fi
  fi

  if [ -f "$CACHE_FILE" ]; then
    [ -z "$five_h_pct" ] && five_h_pct=$(jq -r --arg g "$target_group" '(.command.data.groups[]? | select(.name==$g) | .buckets[]? | select(.window=="5h") | (.remaining_fraction * 100 | round)) // empty' "$CACHE_FILE" 2>/dev/null)
    [ -z "$weekly_pct" ] && weekly_pct=$(jq -r --arg g "$target_group" '(.command.data.groups[]? | select(.name==$g) | .buckets[]? | select(.window=="weekly") | (.remaining_fraction * 100 | round)) // empty' "$CACHE_FILE" 2>/dev/null)
  fi
fi

quota_info=""
if [ -n "$five_h_pct" ] && [ -n "$weekly_pct" ]; then
  [ "$five_h_pct" -ge 40 ] && c_5h=$'\033[32m' || { [ "$five_h_pct" -ge 20 ] && c_5h=$'\033[33m' || c_5h=$'\033[31m'; }
  [ "$weekly_pct" -ge 40 ] && c_wk=$'\033[32m' || { [ "$weekly_pct" -ge 20 ] && c_wk=$'\033[33m' || c_wk=$'\033[31m'; }
  quota_info=$' \033[90m│\033[0m 5h: '"${c_5h}${five_h_pct}%"$'\033[0m \033[90m(Wk: '"${c_wk}${weekly_pct}%"$'\033[0m\033[90m)'
else
  quota_info=$' \033[90m│\033[0m 5h: \033[90m--%\033[0m \033[90m(Wk: \033[90m--%\033[0m\033[90m)'
fi

# Context Bar Tiering (Prioritizing Quota Over Context)
# Tier 1 (cols >= 85): Full 5-block meter with 1-decimal percentage
# Tier 2 (70 <= cols < 85): Compact percentage only (no block bar)
# Tier 3 (cols < 70): Context omitted completely to preserve quota single-line density
ctx_info=""
if (( cols >= 85 )); then
  filled=$(( pct_int / 20 ))
  (( filled > 5 )) && filled=5
  (( filled < 0 )) && filled=0
  empty=$(( 5 - filled ))
  bar=""
  for ((i=0; i<filled; i++)); do bar+="█"; done
  for ((i=0; i<empty; i++)); do bar+="░"; done
  ctx_info=$' \033[90m│\033[0m \033[36mCtx: ['"$bar"$'] '"$ctx_pct"$'%\033[0m'
elif (( cols >= 70 )); then
  ctx_info=$' \033[90m│\033[0m \033[36mCtx: '"$pct_int"$'%\033[0m'
fi

# Execution mode badge
case "$mode" in
  auto)         mode_badge=$'\033[1;30;42m AUTO \033[0m' ;;
  plan)         mode_badge=$'\033[1;30;46m PLAN \033[0m' ;;
  accept-edits) mode_badge=$'\033[1;30;43m EDITS \033[0m' ;;
  default)      mode_badge=$'\033[1;37;44m DEFAULT \033[0m' ;;
  *)            mode_badge=$'\033[1;37;44m '"$mode"$'\033[0m' ;;
esac

# Compact sandbox status
[ "$sandbox" = "true" ] && sb=$' \033[32m\033[0m' || sb=""

# Host indicator (SSH detection)
host_info=""
if [ -n "${SSH_TTY:-}" ] || [ -n "${SSH_CONNECTION:-}" ] || [ -n "${SSH_CLIENT:-}" ]; then
  host_info=$'\033[38;2;166;173;200m\U000F08B9 SSH\033[0m '
fi

# VCS status strictly from JSON payload (no external git subshell execution)
git_info=""
if [ -n "$branch" ]; then
  [ "$dirty" = "true" ] && git_status=$'\033[33m*\033[0m' || git_status=""
  git_info=$' \033[90m│\033[0m \033[35m󰘬 '"$branch$git_status"
fi

# Render formatted status line
printf "%s%s%s \033[90m│\033[0m \033[1m%s\033[0m%s%s%s\n" \
  "$host_info" "$mode_badge" "$sb" "$model_disp" "$git_info" "$quota_info" "$ctx_info"
