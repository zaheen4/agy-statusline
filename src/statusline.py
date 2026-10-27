#!/usr/bin/env python3
import sys
import json
import os
import shutil
import re

def get_terminal_width(payload):
    # 1. Native payload width
    tw = payload.get("terminal_width")
    if isinstance(tw, int) and tw > 0:
        return tw
    # 2. Environment COLUMNS
    cols_env = os.environ.get("COLUMNS")
    if cols_env and cols_env.isdigit():
        c = int(cols_env)
        if c > 0:
            return c
    # 3. Standard terminal size
    try:
        return shutil.get_terminal_size((120, 24)).columns
    except Exception:
        return 120

def format_model(model_raw):
    raw = (model_raw or "Gemini").strip()
    effort = ""
    if re.search(r'\((Extra High|XHigh)\)', raw, re.IGNORECASE):
        effort = " [XH]"
    elif re.search(r'\(Max\)', raw, re.IGNORECASE):
        effort = " [MAX]"
    elif re.search(r'\(High\)', raw, re.IGNORECASE):
        effort = " [H]"
    elif re.search(r'\(Medium\)', raw, re.IGNORECASE):
        effort = " [M]"
    elif re.search(r'\(Low\)', raw, re.IGNORECASE):
        effort = " [L]"
    elif re.search(r'\(Thinking\)', raw, re.IGNORECASE):
        effort = " [T]"

    clean = re.sub(r'\s*\([^)]*\)$', '', raw).strip()
    clean = re.sub(r'^(Gemini|Claude|Google)\s+', '', clean).strip()
    return f"{clean}{effort}"

def build_context_gauge(pct_float, pct_int, cols):
    if cols >= 85:
        filled = min(5, max(0, int(pct_int // 20)))
        blocks = ["█" if i < filled else "░" for i in range(5)]
        return f" \033[90m│\033[0m \033[36mCtx: [{''.join(blocks)}] {pct_float:.1f}%\033[0m"
    elif cols >= 70:
        return f" \033[90m│\033[0m \033[36mCtx: {pct_int}%\033[0m"
    return ""

def format_quota_colored(pct):
    if pct is None:
        return "\033[90m--%\033[0m"
    if pct >= 40:
        c = "\033[32m"
    elif pct >= 20:
        c = "\033[33m"
    else:
        c = "\033[31m"
    return f"{c}{pct}%\033[0m"

def extract_quota(payload, model_str):
    is_3p = bool(re.search(r'Claude|Gpt', model_str, re.IGNORECASE))
    key_5h = "3p-5h" if is_3p else "gemini-5h"
    key_wk = "3p-weekly" if is_3p else "gemini-weekly"
    target_group = "Claude and GPT models" if is_3p else "Gemini Models"

    # 1. Native payload quota
    quota_dict = payload.get("quota")
    h5_pct = None
    wk_pct = None

    if isinstance(quota_dict, dict):
        if key_5h in quota_dict and isinstance(quota_dict[key_5h], dict):
            frac = quota_dict[key_5h].get("remaining_fraction")
            if frac is not None:
                h5_pct = int(round(frac * 100))
        if key_wk in quota_dict and isinstance(quota_dict[key_wk], dict):
            frac = quota_dict[key_wk].get("remaining_fraction")
            if frac is not None:
                wk_pct = int(round(frac * 100))

    # 2. Secondary fallback to /tmp/agy_quota_cache.json
    if h5_pct is None or wk_pct is None:
        cache_path = os.path.join(os.environ.get("TMPDIR", "/tmp"), "agy_quota_cache.json")
        if not os.path.exists(cache_path):
            cache_path = os.path.join(os.environ.get("TEMP", "/tmp"), "agy_quota_cache.json")
        if os.path.exists(cache_path):
            try:
                with open(cache_path, "r") as f:
                    data = json.load(f)
                    groups = data.get("command", {}).get("data", {}).get("groups", [])
                    for g in groups:
                        if g.get("name") == target_group:
                            for b in g.get("buckets", []):
                                if b.get("window") == "5h" and h5_pct is None:
                                    h5_pct = int(round(b.get("remaining_fraction", 0) * 100))
                                elif b.get("window") == "weekly" and wk_pct is None:
                                    wk_pct = int(round(b.get("remaining_fraction", 0) * 100))
            except Exception:
                pass

    if h5_pct is not None or wk_pct is not None:
        return f" \033[90m│\033[0m 5h: {format_quota_colored(h5_pct)} \033[90m(Wk: {format_quota_colored(wk_pct)}\033[90m)"
    return " \033[90m│\033[0m 5h: \033[90m--%\033[0m \033[90m(Wk: \033[90m--%\033[0m\033[90m)"

def main():
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    try:
        raw_stdin = sys.stdin.read()
        payload = json.loads(raw_stdin) if raw_stdin.strip() else {}
    except Exception:
        payload = {}

    cols = get_terminal_width(payload)

    # 1. Identity & Reasoning
    model_obj = payload.get("model")
    if isinstance(model_obj, dict):
        raw_model = model_obj.get("display_name") or model_obj.get("id") or "Gemini"
    elif isinstance(model_obj, str):
        raw_model = model_obj
    else:
        raw_model = "Gemini"
    model_disp = format_model(raw_model)

    # 2. Execution Mode Badges
    mode = (payload.get("cycle_mode") or "default").lower()
    badges = {
        "auto": "\033[1;30;42m AUTO \033[0m",
        "plan": "\033[1;30;46m PLAN \033[0m",
        "accept-edits": "\033[1;30;43m EDITS \033[0m",
        "default": "\033[1;37;44m DEFAULT \033[0m"
    }
    mode_badge = badges.get(mode, f"\033[1;37;44m {mode} \033[0m")

    # 3. Sandbox
    sandbox_enabled = payload.get("sandbox", {}).get("enabled", False)
    sb_badge = " \033[32m\033[0m" if sandbox_enabled else ""

    # 4. Host / SSH Detection
    ssh_badge = ""
    if any(os.environ.get(k) for k in ("SSH_CLIENT", "SSH_TTY", "SSH_CONNECTION")):
        ssh_badge = "\033[38;2;166;173;200m\U000F08B9 SSH\033[0m "

    # 5. Git Metadata
    vcs = payload.get("vcs", {})
    branch = vcs.get("branch", "")
    git_segment = ""
    if branch:
        dirty_flag = "\033[33m*\033[0m" if vcs.get("dirty", False) else ""
        git_segment = f" \033[90m│\033[0m \033[35m {branch}{dirty_flag}"

    # 6. Quota
    quota_segment = extract_quota(payload, raw_model)

    # 7. Context Meter (3-tier)
    ctx_pct = 0.0
    if "context_window" in payload and isinstance(payload["context_window"], dict):
        ctx_pct = float(payload["context_window"].get("used_percentage", 0.0))
    elif "context" in payload and isinstance(payload["context"], dict):
        used = payload["context"].get("used_tokens", 0)
        max_t = payload["context"].get("max_tokens", 1) or 1
        ctx_pct = (used / max_t) * 100.0
    pct_int = int(round(ctx_pct))
    ctx_segment = build_context_gauge(ctx_pct, pct_int, cols)

    # Output assembly
    output = f"{ssh_badge}{mode_badge}{sb_badge} \033[90m│\033[0m \033[1m{model_disp}\033[0m{git_segment}{quota_segment}{ctx_segment}\n"
    try:
        sys.stdout.write(output)
        sys.stdout.flush()
    except UnicodeEncodeError:
        sys.stdout.buffer.write(output.encode("utf-8", errors="replace"))
        sys.stdout.buffer.flush()

if __name__ == "__main__":
    main()
