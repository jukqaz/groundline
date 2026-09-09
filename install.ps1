# Run from a reviewed, binary-bearing stable distribution; requires Git and Codex.
param([string]$Codex = "codex")
$ErrorActionPreference = "Stop"
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Installation step failed." }
}
$installHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }
$architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$target = switch ($architecture) {
    "ARM64" { "aarch64-pc-windows-msvc" }
    "AMD64" { "x86_64-pc-windows-msvc" }
    default { throw "Unsupported host architecture." }
}
$source = Join-Path $PSScriptRoot "plugins/groundline"
$binary = Join-Path $source "bin/$target/groundline.exe"
if (!(Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "Use the binary-bearing stable distribution, not a source checkout."
}
Invoke-Checked $binary @("provider-smoke", "--plugin-root", $source, "--require-installed", "--json")
$version = & $binary --version
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^groundline ([0-9]+\.[0-9]+\.[0-9]+)$') { throw "Invalid distribution version." }
$version = $Matches[1]
Invoke-Checked $Codex @("plugin", "marketplace", "add", "https://github.com/jukqaz/groundline.git", "--ref", "stable", "--json")
Invoke-Checked $Codex @("plugin", "marketplace", "upgrade", "groundline", "--json")
Invoke-Checked $Codex @("plugin", "add", "groundline@groundline", "--json")
$cache = Join-Path $installHome "plugins/cache/groundline/groundline/$version"
$installed = Join-Path $cache "bin/$target/groundline.exe"
if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash -ne (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash) {
    throw "Installed artifact differs from this distribution."
}
Invoke-Checked $installed @("provider-smoke", "--plugin-root", $cache, "--require-installed", "--json")
$catalog = & $Codex debug models
if ($LASTEXITCODE -ne 0) { throw "Native model catalog unavailable." }
$catalog | & $installed setup --catalog - --apply
if ($LASTEXITCODE -ne 0) { throw "Setup failed; inspect the report and retain its private backup." }
Invoke-Checked $Codex @("--strict-config", "doctor", "--summary", "--no-color", "--ascii")
