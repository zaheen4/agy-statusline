$ErrorActionPreference = "Stop"

$Repo = "zaheen4/agy-statusline"
$TargetDir = Join-Path $HOME ".local\bin"
$FallbackDir = Join-Path $HOME ".config\agy-statusline"
$SettingsDir = Join-Path $HOME ".gemini\antigravity-cli"
$SettingsFile = Join-Path $SettingsDir "settings.json"

Write-Host "==> Installing agy-statusline on Windows..." -ForegroundColor Cyan

if (!(Test-Path $TargetDir)) { New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null }
if (!(Test-Path $FallbackDir)) { New-Item -ItemType Directory -Path $FallbackDir -Force | Out-Null }
if (!(Test-Path $SettingsDir)) { New-Item -ItemType Directory -Path $SettingsDir -Force | Out-Null }

$InstalledCmd = ""
$BinaryPath = Join-Path $TargetDir "agy-statusline.exe"

# 1. Check local release build
if (Test-Path "target\release\agy-statusline.exe") {
    Copy-Item "target\release\agy-statusline.exe" -Destination $BinaryPath -Force
    $InstalledCmd = $BinaryPath.Replace("\", "/")
}

# 2. Check cargo locally
if ([string]::IsNullOrEmpty($InstalledCmd) -and (Get-Command cargo -ErrorAction SilentlyContinue) -and (Test-Path "Cargo.toml")) {
    Write-Host "==> Building native binary via Cargo..." -ForegroundColor Yellow
    cargo build --release
    Copy-Item "target\release\agy-statusline.exe" -Destination $BinaryPath -Force
    $InstalledCmd = $BinaryPath.Replace("\", "/")
}

# 3. Download precompiled binary from GitHub Releases
if ([string]::IsNullOrEmpty($InstalledCmd)) {
    $ZipUrl = "https://github.com/$Repo/releases/latest/download/agy-statusline-x86_64-pc-windows-msvc.zip"
    $TempZip = Join-Path $env:TEMP "agy-statusline.zip"
    try {
        Write-Host "==> Downloading precompiled Windows binary..." -ForegroundColor Yellow
        Invoke-WebRequest -Uri $ZipUrl -OutFile $TempZip
        Expand-Archive -Path $TempZip -DestinationPath $TargetDir -Force
        Remove-Item $TempZip -Force
        if (Test-Path $BinaryPath) {
            $InstalledCmd = $BinaryPath.Replace("\", "/")
        }
    } catch {
        Write-Host "==> Release asset download failed, falling back to Python..." -ForegroundColor DarkYellow
    }
}

# 4. Fallback to Python reference script
if ([string]::IsNullOrEmpty($InstalledCmd)) {
    $PyTarget = Join-Path $FallbackDir "statusline.py"
    if (Test-Path "src\statusline.py") {
        Copy-Item "src\statusline.py" -Destination $PyTarget -Force
    } else {
        $PyUrl = "https://raw.githubusercontent.com/$Repo/main/src/statusline.py"
        Invoke-WebRequest -Uri $PyUrl -OutFile $PyTarget
    }
    $CleanPyTarget = $PyTarget.Replace("\", "/")
    $InstalledCmd = "python `"$CleanPyTarget`""
}

# 5. Patch settings.json
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
    'command': r'$InstalledCmd',
    'enabled': True
}

with open(path, 'w') as f:
    json.dump(data, f, indent=2)

print('==> settings.json successfully updated.')
"@

Write-Host "==> Installation complete: $InstalledCmd" -ForegroundColor Green
Write-Host "==> Restart agy to enjoy your new statusline." -ForegroundColor Green
