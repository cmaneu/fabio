# Fabio CLI installer for Windows — downloads the latest release binary.
#
# Usage:
#   irm https://raw.githubusercontent.com/iemejia/fabio/main/install.ps1 | iex
#
# Environment variables:
#   INSTALL_DIR  — override install location (default: %LOCALAPPDATA%\fabio)

$ErrorActionPreference = 'Stop'

$Repo = 'iemejia/fabio'
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'fabio' }

# --- Platform detection ---

$Arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x64' }
    'ARM64' { 'arm64' }
    default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

$Artifact = "fabio-windows-$Arch"

# --- Fetch latest release tag ---

Write-Host "==> fetching latest release..." -ForegroundColor Cyan
$Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
$Tag = $Release.tag_name

if (-not $Tag) {
    throw "Could not determine latest release tag"
}

Write-Host "==> installing fabio $Tag (windows/$Arch)" -ForegroundColor Cyan

# --- Download ---

$DownloadUrl = "https://github.com/$Repo/releases/download/$Tag/$Artifact.zip"
$ChecksumUrl = "$DownloadUrl.sha256"

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "fabio-install-$([System.Guid]::NewGuid().ToString('N').Substring(0,8))"
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

$ZipPath = Join-Path $TmpDir "$Artifact.zip"
$Sha256Path = Join-Path $TmpDir "$Artifact.zip.sha256"

Write-Host "==> downloading $DownloadUrl" -ForegroundColor Cyan
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
Invoke-WebRequest -Uri $ChecksumUrl -OutFile $Sha256Path -UseBasicParsing

# --- Verify checksum ---

Write-Host "==> verifying checksum" -ForegroundColor Cyan
$ExpectedHash = (Get-Content $Sha256Path -Raw).Trim().Split(' ')[0].ToLower()
$ActualHash = (Get-FileHash $ZipPath -Algorithm SHA256).Hash.ToLower()

if ($ExpectedHash -ne $ActualHash) {
    Remove-Item -Recurse -Force $TmpDir
    throw "Checksum mismatch (expected $ExpectedHash, got $ActualHash)"
}

# --- Install ---

Write-Host "==> installing to $InstallDir" -ForegroundColor Cyan
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force

# --- Clean up ---

Remove-Item -Recurse -Force $TmpDir

# --- Add to PATH ---

$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$InstallDir;$UserPath", 'User')
    Write-Host "==> added $InstallDir to user PATH (restart your terminal to use 'fabio')" -ForegroundColor Yellow
}

Write-Host "==> fabio installed successfully to $InstallDir\fabio.exe" -ForegroundColor Green
