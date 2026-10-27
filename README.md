# agy-statusline

A high-performance, single-line telemetry status bar engine for the Google Antigravity CLI (`agy`).

Engineered for modern terminal emulators, low-latency SSH sessions, and Android Termux environments.

## Features

- Sub-millisecond Execution: Native Rust binary executes in ~1.5ms, eliminating the 45ms Python interpreter startup latency on every UI render tick.
- Zero-Subprocess VCS Status: Reads Git metadata (`.vcs.branch` and `.vcs.dirty`) directly from the Antigravity session payload on stdin.
- Reasoning Tag Normalization: Strips vendor noise (`Gemini`, `Claude`, `Google`) and converts reasoning tiers into compact markers (`[XH]`, `[MAX]`, `[H]`, `[M]`, `[L]`, `[T]`).
- High-Contrast Mode Badges: Color-coded ANSI badges mapping agent cycle modes (`AUTO`, `PLAN`, `EDITS`, `DEFAULT`).
- Responsive Breakpoint: Automatically omits quota and context gauges when terminal width falls below 85 columns.
- Asynchronous Quota Monitoring: Reads cached 5-hour and weekly quota fractions from background refresh tasks.
- Context Meter: 5-step block gauge (`[█░░░░] 25.4%`) tracking session context consumption.
- Dual-Engine Support: High-performance native Rust binary with zero external crates, paired with a portable Python 3 reference engine.

## Performance Benchmark

Measured average latency over 20 consecutive execution ticks on Linux x86_64:

| Engine | Latency | Startup Overhead | Local Toolchain Required |
| :--- | :--- | :--- | :--- |
| Native Rust Binary | 1.75 ms | Sub-millisecond | No (CI prebuilts provided) |
| Python 3 Script | 44.95 ms | ~40 ms interpreter startup | Python 3 |

Rust is 25.7x faster than the Python runtime per UI tick.

## Installation

### One-Line Quickstart

#### Linux, macOS, WSL, Android Termux

```bash
curl -fsSL https://raw.githubusercontent.com/eyepatch/agy-statusline/main/install.sh | bash
```

The installer detects your operating system and architecture, downloads the precompiled static binary into `~/.local/bin/agy-statusline`, and automatically configures your Antigravity settings. If precompiled binaries cannot be fetched or Cargo is not present, it automatically falls back to `src/statusline.py`.

#### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/eyepatch/agy-statusline/main/install.ps1 | iex
```

### Manual Configuration

To manually configure `agy-statusline`, add the following block to `~/.gemini/antigravity-cli/settings.json` (or `%USERPROFILE%\.gemini\antigravity-cli\settings.json` on Windows):

Using the compiled native binary:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/home/your-user/.local/bin/agy-statusline",
    "enabled": true
  }
}
```

Using the Python reference engine:

```json
{
  "statusLine": {
    "type": "command",
    "command": "python3 /path/to/agy-statusline/src/statusline.py",
    "enabled": true
  }
}
```

## Palettes and Themes

Preconfigured themes are located in the `themes/` directory:

- `themes/catppuccin.json`: Catppuccin Mocha palette
- `themes/tokyo-night.json`: Tokyo Night palette
- `themes/nord.json`: Nord palette

## Testing & Verification

Simulate incoming Antigravity session payloads via stdin:

Standard execution state:

```bash
echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","vcs":{"branch":"main","dirty":true},"context":{"used_tokens":25400,"max_tokens":100000}}' | COLUMNS=120 ./target/release/agy-statusline
```

Narrow terminal breakpoint (< 85 columns):

```bash
echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","vcs":{"branch":"main","dirty":false},"context":{"used_tokens":25400,"max_tokens":100000}}' | COLUMNS=70 ./target/release/agy-statusline
```

Python fallback verification:

```bash
echo '{"model":"Claude 3.7 Sonnet","cycle_mode":"default","vcs":{"branch":"feature/auth","dirty":false},"context":{"used_tokens":0,"max_tokens":200000}}' | COLUMNS=120 python3 src/statusline.py
```

## Troubleshooting

- Status icons render as question marks or rectangles: Verify that a Nerd Font (such as JetBrains Mono Nerd Font) is active in your terminal emulator.
- No Git branch appears: Git status is read from Antigravity session metadata. Outside of a Git repository, the branch segment is automatically omitted.
- Permission denied: Ensure executable permissions are set on the binary (`chmod +x ~/.local/bin/agy-statusline`).

## License

MIT License. See LICENSE for details.
