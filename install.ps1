# NeboAI Publisher — Windows Install Script
# Usage: irm https://raw.githubusercontent.com/NeboLoop/publisher/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "NeboLoop/publisher"
$binary = "neboai"
$installDir = "$env:LOCALAPPDATA\Programs\neboai"

Write-Host "Installing neboai..." -ForegroundColor Cyan

# Get latest release
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$version = $release.tag_name
Write-Host "Latest version: $version"

# Download
$url = "https://github.com/$repo/releases/download/$version/${binary}-windows-amd64.exe"
$tmpFile = [System.IO.Path]::GetTempFileName() + ".exe"

Write-Host "Downloading..."
Invoke-WebRequest -Uri $url -OutFile $tmpFile

# Install
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Move-Item -Force $tmpFile "$installDir\$binary.exe"

# Add to PATH if not already there
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$installDir", "User")
    Write-Host "Added $installDir to PATH (restart terminal to use)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Installed $binary $version to $installDir\$binary.exe" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  neboai auth login    # Authenticate with NeboLoop"
Write-Host "  neboai --help        # See all commands"
