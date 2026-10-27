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
