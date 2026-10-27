# agy-statusline

A high-performance, single-line telemetry status bar engine for the Google Antigravity CLI (`agy`).

Engineered for modern terminal emulators, low-latency SSH sessions, and Android Termux environments.

## Features

- Sub-millisecond Execution: Native Rust binary executes in ~1.5ms, eliminating the 45ms Python interpreter startup latency on every UI render tick.
- Zero-Subprocess Telemetry Pipeline: Ingests Git metadata (`.vcs.branch`, `.vcs.dirty`), terminal column count (`.terminal_width`), and quota fractions (`.quota`) directly from Antigravity's stdin JSON stream without running subshells.
- Inverted 3-Tier Responsive Layout: Prioritizes quota availability over context consumption so quota limits remain visible on narrow mobile SSH and Termux screens.
- Native Quota Routing: Ingests model-specific quota buckets (`gemini-5h` and `gemini-weekly` for Gemini; `3p-5h` and `3p-weekly` for Claude/GPT) with fallback to `/tmp/agy_quota_cache.json`.
- Dynamic Capacity Health Colors: Quota percentages dynamically render green (>= 40%), yellow (20% to 39%), or red (< 20%).
- Reasoning Tag Normalization: Strips vendor noise (`Gemini`, `Claude`, `Google`) and converts reasoning tiers into compact bracketed markers (`[XH]`, `[MAX]`, `[H]`, `[M]`, `[L]`, `[T]`).
- High-Contrast Mode Badges: Color-coded ANSI badges mapping agent cycle modes (`AUTO`, `PLAN`, `EDITS`, `DEFAULT`).
- Multi-Engine Architecture: High-performance native Rust binary with zero external crates, paired with a canonical Bash script (`src/statusline.sh`) and portable Python 3 reference engine (`src/statusline.py`).

## 3-Tier Responsive Layout

To prevent statusline wrapping on narrow viewports, layout priority is inverted to protect quota metrics:

| Tier | Viewport Width | Displayed Telemetry | Context Metric Format |
| :--- | :--- | :--- | :--- |
| Tier 1 | `>= 85` columns | Model, Mode, Git, Quota, Context | Full 5-block gauge (`[██░░░] 45.2%`) |
| Tier 2 | `70` to `84` cols | Model, Mode, Git, Quota, Context | Compact percentage only (`Ctx: 45%`) |
| Tier 3 | `< 70` columns | Model, Mode, Git, Quota | Context omitted to preserve single-line quota |

## Performance Benchmark

Measured average latency over 20 consecutive execution ticks on Linux x86_64:

| Engine | Latency | Startup Overhead | Local Toolchain Required |
| :--- | :--- | :--- | :--- |
| Native Rust Binary | 1.75 ms | Sub-millisecond | No (CI prebuilts provided) |
| Python 3 Script | 44.95 ms | ~40 ms interpreter startup | Python 3 |

Rust is 25.7x faster than the Python runtime per UI render tick.

## Prerequisites

A patched Nerd Font is required to display status icons (shield, Git branch, and SSH indicators). JetBrains Mono Nerd Font is recommended.

### Installing a Nerd Font

#### macOS
```bash
brew install --cask font-jetbrains-mono-nerd-font
```

#### Linux (Arch Linux)
```bash
sudo pacman -S ttf-jetbrains-mono-nerd
```

#### Linux (Ubuntu / Debian / Fedora)
```bash
mkdir -p ~/.local/share/fonts
curl -fLo ~/.local/share/fonts/JetBrainsMono.tar.xz https://github.com/ryanoasis/nerd-fonts/releases/latest/download/JetBrainsMono.tar.xz
tar -xf ~/.local/share/fonts/JetBrainsMono.tar.xz -C ~/.local/share/fonts/
fc-cache -fv
```

#### Windows (PowerShell / Windows Terminal)
```powershell
winget install DEVCOM.JetBrainsMonoNerdFont
```

#### Android Termux
```bash
mkdir -p ~/.termux
curl -fLo ~/.termux/font.ttf https://github.com/ryanoasis/nerd-fonts/raw/master/patched-fonts/JetBrainsMono/Ligatures/Regular/JetBrainsMonoNerdFont-Regular.ttf
termux-reload-settings
```

### Terminal Font Activation

After installing, set your terminal emulator font family to `JetBrainsMono Nerd Font`:
- Windows Terminal: Settings > Defaults > Appearance > Font face > `JetBrainsMono Nerd Font`
- WezTerm: Set `font = wezterm.font("JetBrainsMono Nerd Font")` in `wezterm.lua`
- Alacritty: Set `font.normal.family = "JetBrainsMono Nerd Font"` in `alacritty.toml`
- Kitty: Set `font_family JetBrainsMono Nerd Font` in `kitty.conf`
- iTerm2: Preferences > Profiles > Text > Font > `JetBrainsMono Nerd Font`

## Installation

### One-Line Quickstart

#### Linux, macOS, WSL, Android Termux

```bash
curl -fsSL https://raw.githubusercontent.com/zaheen4/agy-statusline/main/install.sh | bash
```

The installer detects your operating system and architecture, installs the native binary into `~/.local/bin/agy-statusline`, and automatically updates `~/.gemini/antigravity-cli/settings.json`. If precompiled binaries cannot be fetched or Cargo is not present, it automatically falls back to `src/statusline.py`.

#### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/zaheen4/agy-statusline/main/install.ps1 | iex
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

Using the canonical Bash script:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/home/your-user/.config/agy/statusline.sh",
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

Simulate incoming Antigravity session payloads via stdin across each responsive tier:

Tier 1 (Width >= 85 cols: full 5-block gauge):

```bash
echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","terminal_width":100,"vcs":{"branch":"main","dirty":true},"quota":{"gemini-5h":{"remaining_fraction":0.87},"gemini-weekly":{"remaining_fraction":0.25}},"context_window":{"used_percentage":25.4}}' | ./target/release/agy-statusline
```

Tier 2 (70 <= Width < 85 cols: compact numeric context):

```bash
echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","terminal_width":75,"vcs":{"branch":"main","dirty":false},"quota":{"gemini-5h":{"remaining_fraction":0.87},"gemini-weekly":{"remaining_fraction":0.25}},"context_window":{"used_percentage":25.4}}' | ./target/release/agy-statusline
```

Tier 3 (Width < 70 cols: context omitted, quota preserved):

```bash
echo '{"model":"Claude 3.7 Sonnet","cycle_mode":"default","terminal_width":60,"vcs":{"branch":"feature/auth","dirty":false},"quota":{"3p-5h":{"remaining_fraction":1.0},"3p-weekly":{"remaining_fraction":1.0}}}' | ./target/release/agy-statusline
```

Python fallback verification:

```bash
echo '{"model":"Claude 3.7 Sonnet","cycle_mode":"default","terminal_width":100,"vcs":{"branch":"feature/auth","dirty":false},"context_window":{"used_percentage":15.0}}' | python3 src/statusline.py
```

## Troubleshooting

- Status icons render as question marks or rectangles: Verify that a Nerd Font (such as JetBrains Mono Nerd Font) is active in your terminal emulator.
- No Git branch appears: Git status is read from Antigravity session metadata. Outside of a Git repository, the branch segment is automatically omitted.
- Permission denied: Ensure executable permissions are set on the binary (`chmod +x ~/.local/bin/agy-statusline`).

## License

MIT License. See LICENSE for details.
