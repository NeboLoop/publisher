# NeboAI publisher installer for Windows: the neboai CLI plus the publisher skill.
#
#   irm https://raw.githubusercontent.com/NeboLoop/publisher/main/install.ps1 | iex
#
# Installs from the latest GitHub release:
#   - neboai-windows-amd64.exe -> %LOCALAPPDATA%\Programs\neboai\neboai.exe (added to your PATH)
#   - neboai-skill.tar.gz      -> %USERPROFILE%\.claude\skills\neboai
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$base = "https://github.com/NeboLoop/publisher/releases/latest/download"
$installDir = "$env:LOCALAPPDATA\Programs\neboai"
$skillsDir = "$env:USERPROFILE\.claude\skills"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("neboai-" + [System.Guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

Write-Host "NeboAI publisher installer (windows-amd64)"
Write-Host ""
Write-Host "-> Downloading..."
foreach ($f in @("neboai-windows-amd64.exe", "neboai-skill.tar.gz", "SHA256SUMS")) {
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$f" -OutFile (Join-Path $tmp $f)
}

# Verify both downloads against the release checksums.
$sums = @{}
Get-Content (Join-Path $tmp "SHA256SUMS") | ForEach-Object {
    $parts = $_ -split "\s+", 2
    if ($parts.Count -eq 2) { $sums[$parts[1].TrimStart("*")] = $parts[0] }
}
foreach ($f in @("neboai-windows-amd64.exe", "neboai-skill.tar.gz")) {
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $f)).Hash.ToLower()
    if ($sums[$f] -ne $actual) { throw "Checksum verification failed for $f" }
}

# --- CLI ---
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Move-Item -Force (Join-Path $tmp "neboai-windows-amd64.exe") "$installDir\neboai.exe"
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if (-not (($userPath -split ";") -contains $installDir)) {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
    $env:PATH += ";$installDir"
}
Write-Host "   $(& "$installDir\neboai.exe" --version) installed to $installDir\neboai.exe"

# --- Skill ---
New-Item -ItemType Directory -Force -Path $skillsDir | Out-Null
if (Test-Path "$skillsDir\neboai") { Remove-Item -Recurse -Force "$skillsDir\neboai" }
tar -xzf (Join-Path $tmp "neboai-skill.tar.gz") -C $skillsDir
if ($LASTEXITCODE -ne 0) { throw "Could not unpack the publisher skill" }
Write-Host "   Publisher skill installed to $skillsDir\neboai"
Remove-Item -Recurse -Force $tmp

Write-Host ""
Write-Host "Next: sign in with your NeboAI account (open a new terminal first)"
Write-Host "   neboai auth login"
Write-Host ""
Write-Host "Then ask your AI tool, for example:"
Write-Host '   "I have an idea for a skill that..."'
Write-Host '   "Publish this to NeboAI"'
