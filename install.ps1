# Atrium Windows Native Installer Script
# Supported platform: Windows (x64, ARM64)

$ErrorActionPreference = "Stop"

Write-Host "=== Atrium Windows Installer ==="

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
Invoke-WebRequest -Uri $downloadUrl -OutFile $targetPath -UseBasicParsing

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

Write-Host "=== Installation Completed Successfully! ==="
Write-Host "You can now start the daemon by running: atriumd"
