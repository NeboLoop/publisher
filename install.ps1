# NeboAI Publisher — Windows Install Script
# Installs the neboai CLI binary AND the skill into Claude Code.
# Usage: irm https://raw.githubusercontent.com/NeboLoop/publisher/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "NeboLoop/publisher"
$binary = "neboai"
$installDir = "$env:LOCALAPPDATA\Programs\neboai"

Write-Host ""
Write-Host "  NeboAI Publisher Installer" -ForegroundColor Cyan
Write-Host ""

# --- Step 1: Install CLI binary ---

Write-Host "-> Installing neboai CLI..." -ForegroundColor White

$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$version = $release.tag_name
$url = "https://github.com/$repo/releases/download/$version/${binary}-windows-amd64.exe"

$tmpFile = [System.IO.Path]::GetTempFileName() + ".exe"
Invoke-WebRequest -Uri $url -OutFile $tmpFile

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Move-Item -Force $tmpFile "$installDir\$binary.exe"

# Add to PATH
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$installDir", "User")
    $env:PATH += ";$installDir"
}

Write-Host "  Done: $binary $version" -ForegroundColor Green

# --- Step 2: Install skill into Claude Code ---

Write-Host "-> Installing publisher skill..." -ForegroundColor White

$claudeSkillsDir = "$env:USERPROFILE\.claude\skills\neboai"

# Clone repo to temp
$tmpDir = [System.IO.Path]::GetTempPath() + "neboai-install-" + [System.Guid]::NewGuid().ToString("N").Substring(0,8)
git clone --depth 1 --quiet "https://github.com/$repo.git" $tmpDir 2>$null

# Copy skill files
New-Item -ItemType Directory -Force -Path $claudeSkillsDir | Out-Null
Copy-Item "$tmpDir\SKILL.md" $claudeSkillsDir -Force
Copy-Item "$tmpDir\references" $claudeSkillsDir -Recurse -Force
Copy-Item "$tmpDir\scripts" $claudeSkillsDir -Recurse -Force
Copy-Item "$tmpDir\examples" $claudeSkillsDir -Recurse -Force
Remove-Item $tmpDir -Recurse -Force

Write-Host "  Done: skill installed" -ForegroundColor Green

# --- Done ---

Write-Host ""
Write-Host "  Ready!" -ForegroundColor Green
Write-Host ""
Write-Host "  You can now publish to NeboLoop directly from Claude." -ForegroundColor White
Write-Host ""
Write-Host "  Just tell Claude what you want to build:" -ForegroundColor White
Write-Host '    "I have an idea for an agent that..."' -ForegroundColor Yellow
Write-Host '    "Build me a plugin that connects to..."' -ForegroundColor Yellow
Write-Host '    "Publish this to NeboLoop"' -ForegroundColor Yellow
Write-Host ""
Write-Host "  Claude handles everything - building, validating," -ForegroundColor Gray
Write-Host "  authenticating, and publishing. You just describe your idea." -ForegroundColor Gray
Write-Host ""
