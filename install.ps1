[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = "Stop"

Write-Host @"
  █████╗ ████████╗██████╗ ██████╗ ██╗   ██╗███╗   ███╗
 ██╔══██╗╚══██╔══╝██╔══██╗██╔══██╗██║   ██║████╗ ████║
 ███████║   ██║   ██████╔╝██████╔╝██║   ██║██╔████╔██║
 ██╔══██║   ██║   ██╔══██╗██╔══██╗██║   ██║██║╚██╔╝██║
 ██║  ██║   ██║   ██║  ██║██║  ██║╚██████╔╝██║ ╚═╝ ██║
 ╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝
 ─────────────────────────────────────────────────────
  [ v1.0.0-beta • Context Stabilization Protocol ]
 ─────────────────────────────────────────────────────

 🤨 DETECTING AGENT TOXICITY...
 ▸ Your AI agents have been absolutely robbing you blind.
 ▸ It's time to stop paying a car lease every month just to copy-paste boilerplate.
 ▸ Atrium is initializing. Get ready to save your jewelry money.

 ┌────────────────────────────────────────────────────────┐
 │  ◬  SYSTEM PROVISIONING MATRIX                         │
 ├────────────────────────────────────────────────────────┤
 │  [ .. ] Scanning host platform target architecture... │
"@


# 1. Detect Architecture
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq "ARM64") {
    $binArch = "arm64"
} else {
    $binArch = "x64" # Default to x64
}

$binaryName = "atriumd-windows-$binArch.exe"
$downloadUrl = "https://github.com/MrDawell/atrium/releases/latest/download/$binaryName"

Write-Host "Detected Architecture: $arch"
Write-Host "Target Binary: $binaryName"

# 2. Determine Installation Path
$installDir = Join-Path $env:LOCALAPPDATA "atrium\bin"
if (-not (Test-Path $installDir)) {
    Write-Host "Creating installation directory at $installDir..."
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$targetPath = Join-Path $installDir "atriumd.exe"

# 3. Download the Binary
Write-Host "Downloading atriumd.exe..."
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $targetPath -UseBasicParsing
} catch {
    Write-Host "Error: Failed to download the precompiled binary from $downloadUrl." -ForegroundColor Red
    Write-Host "This is likely because the repository or release is not yet published on GitHub." -ForegroundColor Yellow
    Write-Host "You can build atriumd from source locally in this directory by running:" -ForegroundColor Yellow
    Write-Host "  cargo build --release --bin atriumd" -ForegroundColor Green
    Write-Host "And then copy the binary manually to your installation path: $targetPath" -ForegroundColor Green
    Exit 1
}

# 4. Add to User Environment PATH permanently
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathElements = $userPath -split ';'

if ($pathElements -notcontains $installDir) {
    Write-Host "Adding $installDir to User PATH Environment Variable..."
    $newUserPath = "$userPath;$installDir"
    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    # Also update path in current active session
    $env:Path = "$env:Path;$installDir"
    Write-Host "Successfully registered PATH entry. You may need to restart your terminal for changes to fully propagate."
}

# 5. Quietly setup Python Dependencies
if (Get-Command python -ErrorAction SilentlyContinue) {
    Write-Host "python detected. Quietly installing requirements (grpcio, grpcio-tools)..."
    try {
        Start-Process -FilePath "python" -ArgumentList "-m pip install -q grpcio grpcio-tools" -NoNewWindow -Wait
    } catch {
        Write-Host "Warning: Failed to install python packages automatically. You may need to run 'pip install grpcio grpcio-tools' manually."
    }
} else {
    Write-Host "Warning: python was not found in your PATH. Note that Python is required to run the Atrium bridge CLI and MCP server."
}

# 6. Automatically register Atrium MCP settings in editors
Write-Host "Registering Atrium in Claude Code, Cursor, Cline, and Continue..."
$bridgePath = Join-Path $PSScriptRoot "api\bridge.py"
try {
    if (Test-Path $bridgePath) {
        Start-Process -FilePath $targetPath -ArgumentList "--register --bridge-path `"$bridgePath`"" -NoNewWindow -Wait
    } else {
        Start-Process -FilePath $targetPath -ArgumentList "--register" -NoNewWindow -Wait
    }
} catch {
    Write-Host "Warning: Failed to execute registration configuration automatically."
}

Write-Host "=== Installation Completed Successfully! ==="
Write-Host "You can now start the daemon by running: atriumd"
