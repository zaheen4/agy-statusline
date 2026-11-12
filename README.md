# agy-statusline

A fast, clean status bar for the Google Antigravity CLI (`agy`).

```text
 AUTO  │ 3.8 Flash [H] │ 󰘬 main* │ 5h: 87% (Wk: 25%) │ Ctx: [█░░░░] 25.4%
 ─────   ─────────────   ───────   ─────────────────   ──────────────────
 Mode    Model & Think   Branch    Remaining Quota     Context Window
```

Engineered for terminal emulators, remote SSH sessions, and Android Termux.

## Highlights

- **Instant Execution:** Built in native Rust (<2ms execution time). Eliminates the ~45ms Python interpreter startup lag on every screen update.
- **Direct Updates:** Reads Git branch, terminal width, and quota limits directly from Antigravity's session stream without spawning slow subshells.
- **Smart Screen Sizing:** Automatically fits narrow screens and split viewports by keeping your quota visible and shortening or hiding the context bar.
- **Model Quota Limits:** Shows remaining 5-hour and weekly limits for Gemini, Claude, and GPT models with color-coded warnings (green above 40%, yellow 20–39%, red below 20%).
- **Clean Model Names:** Shortens vendor names (e.g., `Flash 3.8`) and thinking tiers into compact badges like `[H]` or `[MAX]`.
- **Battery-Friendly Cache:** When offline or outside active sessions, quota refreshes quietly in the background without draining laptop battery or opening unwanted OAuth browser pop-ups.
- **Portable:** Single precompiled binary, paired with standalone Bash (`src/statusline.sh`) and Python 3 (`src/statusline.py`) reference engines.

## Quickstart

### Linux, macOS, WSL, Android Termux

```bash
curl -fsSL https://raw.githubusercontent.com/zaheen4/agy-statusline/main/install.sh | bash
```

The installer detects your operating system and architecture, installs the native binary into `~/.local/bin/agy-statusline`, and enables it in Antigravity. If Cargo is not installed or precompiled binaries are unavailable, it automatically falls back to the Python reference engine.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/zaheen4/agy-statusline/main/install.ps1 | iex
```

## Setup Inside Antigravity

You can manage your statusline directly inside Antigravity CLI without editing configuration files by hand:

- **Set statusline command:**
  ```text
  /statusline ~/.local/bin/agy-statusline
  ```
- **Toggle on or off:**
  ```text
  /statusline off
  /statusline on
  ```
- **Revert to default:**
  ```text
  /statusline delete
  ```

### Manual Configuration

Alternatively, configure the statusline manually by adding this block to `~/.gemini/antigravity-cli/settings.json` (or `%USERPROFILE%\.gemini\antigravity-cli\settings.json` on Windows):

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

Using the Bash script:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/home/your-user/.config/agy/statusline.sh",
    "enabled": true
  }
}
```

Using the Python script:

```json
{
  "statusLine": {
    "type": "command",
    "command": "python3 /path/to/agy-statusline/src/statusline.py",
    "enabled": true
  }
}
```

## What Each Segment Shows

- **Mode Badge:** Displays the active agent mode (`AUTO`, `PLAN`, `EDITS`, `DEFAULT`) with high-contrast ANSI colors.
- **Model & Thinking Level:** Displays a shortened model name and bracketed reasoning effort (`[XH]`, `[MAX]`, `[H]`, `[M]`, `[L]`, `[T]`).
- **Security & Remote Badges:** Displays `` when the Antigravity sandbox is active, and `󰢹 SSH` during remote terminal sessions.
- **Git Branch:** Displays the current Git branch and an uncommitted changes marker (`*`). Reads directly from `.git/HEAD` with sub-millisecond fallback when Antigravity omits VCS metadata.
- **Quota Limits:** Displays your 5-hour and weekly quota remaining:
  - Green: 40% or more remaining
  - Yellow: 20% to 39% remaining
  - Red: Under 20% remaining
- **Context Meter:** 5-block gauge (`[██░░░] 45.2%`) tracking token capacity.

## Responsive Screen Sizing

To prevent long lines from wrapping on narrow screens or mobile viewports, the statusline automatically adapts based on terminal column width:

| Terminal Width | Displayed Segments | Context Bar Format |
| :--- | :--- | :--- |
| **85+ columns** | Mode, Model, Git, Quota, Context | Full 5-block gauge (`[██░░░] 45.2%`) |
| **70 to 84 columns** | Mode, Model, Git, Quota, Context | Number only (`Ctx: 45%`) |
| **Under 70 columns** | Mode, Model, Git, Quota | Context hidden to keep quota visible |

## Prerequisites

A patched Nerd Font is required to display status icons (shield, Git branch, and SSH badges). JetBrains Mono Nerd Font is recommended.

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

After installing, set your terminal font to `JetBrainsMono Nerd Font`:
- **Windows Terminal:** Settings > Defaults > Appearance > Font face > `JetBrainsMono Nerd Font`
- **WezTerm:** Add `font = wezterm.font("JetBrainsMono Nerd Font")` in `wezterm.lua`
- **Alacritty:** Set `font.normal.family = "JetBrainsMono Nerd Font"` in `alacritty.toml`
- **Kitty:** Set `font_family JetBrainsMono Nerd Font` in `kitty.conf`
- **iTerm2:** Preferences > Profiles > Text > Font > `JetBrainsMono Nerd Font`

## Quota Telemetry & Lazy Background Caching

During active Antigravity sessions, model quotas stream directly through standard input on every turn.

When quota data is not present in the stream, engines read from `/tmp/agy_quota_cache.json` using **Lazy On-Demand Background Refresh**:
- **Zero Idle Drain:** When Antigravity is closed or inactive, background execution stops completely, preserving laptop battery.
- **Non-Blocking Fetch:** When Antigravity is running and the cache is older than 30 minutes, an asynchronous worker (`fetch-quota.sh`) refreshes data in the background without pausing prompt rendering.
- **Offline Safe:** Performs a fast connectivity check before querying APIs. If your machine is offline, it exits quietly without opening browser windows.

### Optional Fixed-Interval Poller (Systemd)

If you prefer a scheduled background timer that updates the cache every 10 minutes even when Antigravity is not open, optional systemd user units are provided in `systemd/`:

```bash
mkdir -p ~/.config/systemd/user ~/.config/agy-statusline
cp src/fetch-quota.sh ~/.config/agy-statusline/fetch-quota.sh
chmod +x ~/.config/agy-statusline/fetch-quota.sh

cp systemd/agy-quota-cache.service ~/.config/systemd/user/
cp systemd/agy-quota-cache.timer ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now agy-quota-cache.timer
```

## Testing & Verification

Test statusline rendering by piping sample telemetry data through any engine:

```bash
echo '{"model":"Gemini 3.8 Flash (High)","cycle_mode":"auto","terminal_width":100,"vcs":{"branch":"main","dirty":true},"quota":{"gemini-5h":{"remaining_fraction":0.87},"gemini-weekly":{"remaining_fraction":0.25}},"context_window":{"used_percentage":25.4}}' | ./target/release/agy-statusline
```

Expected output:

```text
 AUTO  │ 3.8 Flash [H] │ 󰘬 main* │ 5h: 87% (Wk: 25%) │ Ctx: [█░░░░] 25.4%
```

Test the Python or Bash engines with the same command by substituting `python3 src/statusline.py` or `./src/statusline.sh`.

## Troubleshooting

- **Icons appear as question marks or boxes:** Verify that a Nerd Font is installed and selected in your terminal settings.
- **No Git branch appears:** Outside of a Git repository, the branch segment is hidden automatically. Inside a repository, branch names and uncommitted changes resolve automatically.
- **Permission denied:** Ensure executable permissions are granted (`chmod +x ~/.local/bin/agy-statusline`).

## License

MIT License. See LICENSE for details.
