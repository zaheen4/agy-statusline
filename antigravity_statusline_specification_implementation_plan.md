# Project Specification & Implementation Plan: `agy-statusline`

This document details the architecture, code implementation, cross-platform compatibility, and deployment automation for `agy-statusline`—a dedicated, cross-platform status bar engine for the Google Antigravity CLI (`agy`).

---

## 1. Project Overview & Scope

The `agy-statusline` project packages a lightweight, keyboard-first status bar designed for the Antigravity CLI. It replaces the default verbose footer with a single-line telemetry display engineered for modern terminal emulators and mobile SSH environments (e.g., Android Termux).

### Functional Scope
* **Included:** Model normalization, reasoning effort extraction, execution mode badging, zero-subprocess VCS parsing, context window utilization metering, asynchronous quota monitoring, terminal breakpoint responsiveness, and automated configuration installers.
* **Excluded (Out of Scope):** LLM prompt engineering guidelines, agent persona rules, markdown styling constraints, and tool execution security hooks.

### Core Display Features
* **Model & Reasoning Tag Normalization:** Strips vendor branding (`Gemini `, `Claude `, `Google `) and transforms verbose reasoning tags into compact uppercase markers (`[XH]`, `[MAX]`, `[H]`, `[M]`, `[L]`, `[T]`).
* **Strict Single-Cell Visual Alignment:** Standardizes on single-cell Nerd Font glyphs (e.g., `` at `U+F132` for sandbox status, `󰘬` for Git, `󰢹` for SSH) to ensure columns never wrap or misalign on Android Termux or narrow SSH sessions.
* **Execution Mode Badges:** High-contrast ANSI badges mapping active agent states (`AUTO`, `PLAN`, `EDITS`, `DEFAULT`).
* **Zero-Subprocess VCS Status:** Consumes `.vcs.branch` and `.vcs.dirty` directly from incoming JSON payloads on `stdin`, eliminating the latency of running subshell Git commands on every keystroke/render cycle.
* **Context Usage Meter:** 5-block visual meter (`[█░░░░]`) calculated in 20% steps with 1-decimal percentage tracking.
* **Responsive Width Breakpoint:** Automatically detects terminal width and hides quota and context meters when width falls below 85 columns.

---

## 2. Ingestion & Runtime Flow

Antigravity executes the statusline command on every state transition or UI tick, streaming session state as a JSON payload over `stdin`. The statusline script parses this payload, inspects terminal dimensions, and outputs a single formatted ANSI line to `stdout`.

```text
                  [ Antigravity CLI (agy) ]
                             │
              Pipes session JSON payload via stdin
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
   Unix / macOS / WSL / Termux         Windows (Native)
  ┌───────────────────────────┐     ┌───────────────────┐
  │ statusline.py (or .sh)    │     │   statusline.py   │
  └───────────────────────────┘     └───────────────────┘
            │                                 │
            └────────────────┬────────────────┘
                             ▼
                 Formatted Single-Line ANSI
               (Rendered into agy bottom bar)
```

---

## 3. Repository File Structure

```text
agy-statusline/
├── README.md                 # Visual overview, screenshots, and setup instructions
├── LICENSE                   # MIT License
├── install.sh                # Automated installer for Linux, macOS, WSL, and Termux
├── install.ps1               # Automated installer for native Windows PowerShell
├── src/
│   ├── statusline.py         # Universal, zero-dependency Python 3 statusline engine
│   └── statusline.sh         # Pure POSIX/Bash alternative for minimal Unix environments
└── themes/
    ├── catppuccin.json       # Default Catppuccin Mocha palette
    ├── tokyo-night.json      # Tokyo Night palette
    └── nord.json             # Nord palette
```

---

## 4. Core Implementation: `src/statusline.py`

Using pure Python 3 allows the engine to run without platform-specific dependencies like `jq`, `awk`, or external `stat` binaries across Linux, macOS, and Windows.

```python
#!/usr/bin/env python3
import sys
import json
import os
import shutil
import re

def get_terminal_width():
    try:
        return shutil.get_terminal_size((120, 24)).columns
    except Exception:
        return 120

def format_model(model_raw):
    # Remove vendor noise
    clean = re.sub(r'^(Gemini|Claude|Google)\s+', '', model_raw or 'Default')
    # Standardize reasoning levels
    replacements = [
        (r'\(Extra High\)|\(XHigh\)', '[XH]'),
        (r'\(Max\)', '[MAX]'),
        (r'\(High\)', '[H]'),
        (r'\(Medium\)', '[M]'),
        (r'\(Low\)', '[L]'),
        (r'\(Thinking\)', '[T]')
    ]
    for pattern, tag in replacements:
        clean = re.sub(pattern, tag, clean, flags=re.IGNORECASE)
    return clean.strip()

def build_context_gauge(used_tokens, max_tokens):
    if max_tokens <= 0:
        return "[░░░░░] 0.0%"
    pct = (used_tokens / max_tokens) * 100.0
    filled = min(5, max(0, int(pct // 20)))
    blocks = ["█" if i < filled else "░" for i in range(5)]
    return f"[{''.join(blocks)}] {pct:.1f}%"

def read_quota_cache():
    cache_path = os.path.join(os.environ.get("TMPDIR", "/tmp"), "agy_quota_cache.json")
    if not os.path.exists(cache_path):
        cache_path = os.path.join(os.environ.get("TEMP", "/tmp"), "agy_quota_cache.json")
    
    if os.path.exists(cache_path):
        try:
            with open(cache_path, "r") as f:
                data = json.load(f)
                h5 = data.get("5h_pct", "--")
                wk = data.get("week_pct", "--")
                return f"Q:{h5}%/{wk}%"
        except Exception:
            pass
    return None

def main():
    cols = get_terminal_width()
    
    try:
        raw_stdin = sys.stdin.read()
        payload = json.loads(raw_stdin) if raw_stdin.strip() else {}
    except Exception:
        payload = {}

    # 1. Identity & Reasoning
    model_str = format_model(payload.get("model", "Default"))
    
    # 2. Execution Mode Badges
    mode = payload.get("cycle_mode", "default").lower()
    badges = {
        "auto": "\033[42;30m AUTO \033[0m",
        "plan": "\033[46;30m PLAN \033[0m",
        "accept-edits": "\033[43;30m EDITS \033[0m",
        "default": "\033[44;30m DEFAULT \033[0m"
    }
    mode_badge = badges.get(mode, badges["default"])

    # 3. Host / SSH Detection
    ssh_badge = ""
    if any(os.environ.get(k) for k in ("SSH_CLIENT", "SSH_TTY", "SSH_CONNECTION")):
        ssh_badge = "\033[38;5;244m󰢹 SSH\033[0m "

    # 4. Git Metadata (Payload-derived, no subprocess)
    vcs = payload.get("vcs", {})
    branch = vcs.get("branch", "")
    dirty = "*" if vcs.get("dirty", False) else ""
    git_segment = f"\033[34m󰘬 {branch}{dirty}\033[0m" if branch else ""

    # 5. Context Meter
    ctx = payload.get("context", {})
    gauge_str = build_context_gauge(ctx.get("used_tokens", 0), ctx.get("max_tokens", 1))
    ctx_segment = f"\033[35m{gauge_str}\033[0m"

    # Assemble segments based on responsive width
    segments = [f"{ssh_badge}\033[32m\033[0m {mode_badge} \033[1m{model_str}\033[0m"]
    
    if git_segment:
        segments.append(git_segment)

    # Hide meters in narrow viewports (< 85 columns)
    if cols >= 85:
        quota_str = read_quota_cache()
        if quota_str:
            segments.append(f"\033[36m{quota_str}\033[0m")
        segments.append(ctx_segment)

    output = " \033[38;5;239m│\033[0m ".join(segments)
    sys.stdout.write(f" {output} \n")
    sys.stdout.flush()

if __name__ == "__main__":
    main()
```

---

## 5. Cross-Platform Automated Installers

Both installers preserve existing configuration in `settings.json` by updating only the `"statusLine"` dictionary key.

### 5.1. Linux / macOS / WSL / Termux (`install.sh`)

```bash
#!/usr/bin/env bash
set -euo pipefail

TARGET_DIR="$HOME/.config/agy-statusline"
SETTINGS_DIR="$HOME/.gemini/antigravity-cli"
SETTINGS_FILE="$SETTINGS_DIR/settings.json"

echo "==> Installing agy-statusline..."

# Ensure target directories exist
mkdir -p "$TARGET_DIR" "$SETTINGS_DIR"

# Copy engine script
cp src/statusline.py "$TARGET_DIR/statusline.py"
chmod +x "$TARGET_DIR/statusline.py"

# Safely patch settings.json
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
    "command": "python3 " + os.path.expanduser("$TARGET_DIR/statusline.py"),
    "enabled": True
}

with open(path, "w") as f:
    json.dump(data, f, indent=2)

print("==> settings.json successfully updated.")
EOF

echo "==> Done. Restart agy to view your new statusline."
```

### 5.2. Windows PowerShell (`install.ps1`)

```powershell
$ErrorActionPreference = "Stop"

$TargetDir = Join-Path $HOME ".config\agy-statusline"
$SettingsDir = Join-Path $HOME ".gemini\antigravity-cli"
$SettingsFile = Join-Path $SettingsDir "settings.json"

Write-Host "==> Installing agy-statusline on Windows..." -ForegroundColor Cyan

if (!(Test-Path $TargetDir)) { New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null }
if (!(Test-Path $SettingsDir)) { New-Item -ItemType Directory -Path $SettingsDir -Force | Out-Null }

Copy-Item "src\statusline.py" -Destination (Join-Path $TargetDir "statusline.py") -Force

$ScriptTarget = (Join-Path $TargetDir "statusline.py").Replace("\", "/")

python -c @"
import json, os

path = r'$SettingsFile'
data = {}
if os.path.exists(path):
    try:
        with open(path, 'r') as f:
            data = json.load(f)
    except Exception:
        data = {}

data['statusLine'] = {
    'type': 'command',
    'command': 'python \"$ScriptTarget\"',
    'enabled': True
}

with open(path, 'w') as f:
    json.dump(data, f, indent=2)

print('==> settings.json successfully updated.')
"@

Write-Host "==> Done. Restart agy in Windows Terminal." -ForegroundColor Green
```

---

## 6. End-User Guide (`README.md` Content)

### Prerequisites
* **Antigravity CLI:** Installed and operational (`agy`).
* **Nerd Font:** A patched font (e.g., JetBrains Mono Nerd Font, Fira Code Nerd Font) active in your terminal emulator to display status glyphs.
* **Python 3:** Available on system path (`python3` on Unix/macOS, `python` on Windows).

### One-Line Quickstart

**Linux / macOS / Termux / WSL:**
```bash
curl -fsSL https://raw.githubusercontent.com/<username>/agy-statusline/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/<username>/agy-statusline/main/install.ps1 | iex
```

### Manual Configuration
If you prefer not to use the automated script, add this block to `~/.gemini/antigravity-cli/settings.json` (or `%USERPROFILE%\.gemini\antigravity-cli\settings.json` on Windows):

```json
{
  "statusLine": {
    "type": "command",
    "command": "python3 /path/to/src/statusline.py",
    "enabled": true
  }
}
```

---

## 7. Verification & Mock Test Suite

Run these mock inputs in your terminal to verify statusline formatting across different states before committing changes.

### 1. Standard Agent Run
```bash
echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","vcs":{"branch":"main","dirty":true},"context":{"used_tokens":25400,"max_tokens":100000}}' | python3 src/statusline.py
```
*Expected Display:*
`   AUTO  3.8 Flash [H] │ 󰘬 main* │ [█░░░░] 25.4% `

### 2. Narrow Responsive Breakpoint (<85 columns)
```bash
COLUMNS=70 echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","vcs":{"branch":"main","dirty":false},"context":{"used_tokens":25400,"max_tokens":100000}}' | python3 src/statusline.py
```
*Expected Display:*
`   AUTO  3.8 Flash [H] │ 󰘬 main `  
*(Context meter is automatically hidden to prevent wrapping)*

### 3. Idle / Default State with Clean Repo
```bash
echo '{"model":"Claude 3.7 Sonnet","cycle_mode":"default","vcs":{"branch":"feature/auth","dirty":false},"context":{"used_tokens":0,"max_tokens":200000}}' | python3 src/statusline.py
```
*Expected Display:*
`   DEFAULT  3.7 Sonnet │ 󰘬 feature/auth │ [░░░░░] 0.0% `

---

## 8. Troubleshooting & FAQ

* **Glyphs render as blank rectangles or question marks:**  
  The terminal emulator does not have a Nerd Font enabled. Configure the font family to a Nerd Font (such as `JetBrainsMono NF`) in your terminal emulator preferences (Foot, WezTerm, iTerm2, Alacritty, or Windows Terminal).
* **Double status bar displayed:**  
  If the default Antigravity bar appears stacked alongside the custom bar, open `settings.json` and set `"stack_with_default": false`.
* **No Git status displayed:**  
  The engine reads Git information directly from the Antigravity session payload. If you run `agy` outside of an initialized Git repository, the branch segment will automatically omit itself.