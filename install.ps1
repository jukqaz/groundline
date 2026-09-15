# Run from the reviewed binary-bearing stable distribution. No global Git edits.
[CmdletBinding()]
param(
    [string]$Codex = "",
    [ValidateSet("core", "insights", "both")][string]$Profile = "core",
    [ValidateSet("preserve", "astra")][string]$Preset = "preserve",
    [string]$Model = "", [string]$Effort = "",
    [ValidateSet("", "default", "fast")][string]$ServiceTier = "",
    [switch]$RestoreNativeContext,
    [string]$InsightsProfile = "", [string]$InsightsEndpoint = "",
    [string]$EnrollmentTokenFile = "",
    [switch]$EnableInsights
)
$ErrorActionPreference = "Stop"
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$script:Stages = [ordered]@{}
$script:ActiveStage = "preflight"
$script:Failed = $false
$script:Pending = $false
$script:CodexAppSelected = $false
function Set-Stage([string]$Name, [string]$Status, [int]$Code) {
    $script:Stages[$Name] = [ordered]@{ status = $Status; exit_code = $Code }
    if ($Status -eq "FAIL") { $script:Failed = $true }
    if ($Status -eq "ACTION_REQUIRED") { $script:Pending = $true }
}
function Invoke-Step([string]$Name, [string]$Executable, [string[]]$Arguments) {
    $script:ActiveStage = $Name
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        Set-Stage $Name "FAIL" $LASTEXITCODE
        throw "Resolve the failed stage and rerun the same installer."
    }
    Set-Stage $Name "PASS" 0
}
function Get-ArtifactSha256([string]$Path) {
    $hash = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        try { return [System.BitConverter]::ToString($hash.ComputeHash($stream)) }
        finally { $stream.Dispose() }
    } finally { $hash.Dispose() }
}
function Find-Codex {
    # Resolve installed App packages from Windows metadata, not a versioned path.
    $candidates = @()
    if (Get-Command Get-AppxPackage -ErrorAction SilentlyContinue) {
        $packages = @(Get-AppxPackage | Where-Object { $_.Name -match '(?i)OpenAI.*(Codex|ChatGPT)|(Codex|ChatGPT).*OpenAI' })
        foreach ($package in $packages) {
            $candidates += @(Get-ChildItem -LiteralPath $package.InstallLocation -Filter codex.exe -Recurse -File -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName)
        }
    }
    if ($candidates.Count -eq 1) { $script:CodexAppSelected = $true; return $candidates[0] }
    $command = Get-Command codex -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    throw "Codex could not be resolved uniquely. Install Codex or pass -Codex with its executable path."
}
function Invoke-InsightsSetup([string]$Executable, [string[]]$Arguments) {
    $selectApp = $script:CodexAppSelected -and !(Test-Path Env:GROUNDLINE_RUNTIME_FAMILY) -and !(Test-Path Env:CODEX_INTERNAL_ORIGINATOR_OVERRIDE)
    try {
        if ($selectApp) { $env:GROUNDLINE_RUNTIME_FAMILY = "codex_app" }
        & $Executable setup --verify @Arguments
    } finally {
        if ($selectApp) { Remove-Item Env:GROUNDLINE_RUNTIME_FAMILY }
    }
}
try {
    $setupArgs = @()
    if ($Model) { $setupArgs += @("--model", $Model) }
    if ($Effort) { $setupArgs += @("--effort", $Effort) }
    if ($ServiceTier) { $setupArgs += @("--service-tier", $ServiceTier) }
    if ($RestoreNativeContext) { $setupArgs += "--restore-native-context" }
    $insightsArgs = @()
    if ($InsightsProfile) { $insightsArgs += @("--input", $InsightsProfile) }
    if ($InsightsEndpoint) { $insightsArgs += @("--endpoint", $InsightsEndpoint) }
    if ($EnrollmentTokenFile) { $insightsArgs += @("--enrollment-token-file", $EnrollmentTokenFile) }
    if ($EnableInsights) { $insightsArgs += "--enable" }
    if (($Profile -eq "core" -and $insightsArgs.Count -gt 0) -or ($Profile -eq "insights" -and ($setupArgs.Count -gt 0 -or $Preset -ne "preserve"))) {
        throw "Options must match the selected -Profile core, insights, or both."
    }
    if (!(Get-Command git -ErrorAction SilentlyContinue)) { throw "Install Git before running setup." }
    if (!$Codex) { $Codex = Find-Codex }
    & $Codex plugin add --help | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Codex plugin support is required." }
    & $Codex debug models --help | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Codex debug models support is required." }
    & $Codex doctor --help | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Codex doctor support is required." }
    $installHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }
    $architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $target = switch ($architecture) {
        "ARM64" { "aarch64-pc-windows-msvc" }
        "AMD64" { "x86_64-pc-windows-msvc" }
        default { throw "Unsupported host architecture." }
    }
    Set-Stage "preflight" "PASS" 0
    $products = switch ($Profile) { "core" { @("groundline") } "insights" { @("groundline-insights") } "both" { @("groundline", "groundline-insights") } }
    $installVersion = ""
    foreach ($product in $products) {
        $source = Join-Path $PSScriptRoot "plugins/$product"
        $binary = Join-Path $source "bin/$target/$product.exe"
        Invoke-Step "distribution_$product" $binary @("provider-smoke", "--plugin-root", $source, "--require-installed", "--json")
        $version = & $binary --version
        if ($LASTEXITCODE -ne 0 -or $version -notmatch ('^' + [regex]::Escape($product) + ' ([0-9]+\.[0-9]+\.[0-9]+)$')) { throw "Invalid distribution version." }
        $version = $Matches[1]
        if ($installVersion -and $installVersion -ne $version) { throw "Use one complete release distribution." }
        $installVersion = $version
    }
    Invoke-Step "marketplace_add" $Codex @("plugin", "marketplace", "add", "https://github.com/jukqaz/groundline.git", "--ref", "stable", "--json")
    Invoke-Step "marketplace_refresh" $Codex @("plugin", "marketplace", "upgrade", "groundline", "--json")
    foreach ($product in $products) {
        Invoke-Step "install_$product" $Codex @("plugin", "add", "$product@groundline", "--json")
        $cache = Join-Path $installHome "plugins/cache/groundline/$product/$installVersion"
        $installed = Join-Path $cache "bin/$target/$product.exe"
        $script:ActiveStage = "verify_$product"
        if ((Get-ArtifactSha256 (Join-Path $PSScriptRoot "plugins/$product/bin/$target/$product.exe")) -ne (Get-ArtifactSha256 $installed)) {
            throw "Installed artifact differs. Obtain the complete current stable distribution and retry."
        }
        Invoke-Step "verify_$product" $installed @("provider-smoke", "--plugin-root", $cache, "--require-installed", "--json")
    }
    if ($Profile -ne "insights") {
        $script:ActiveStage = "catalog"
        $catalog = & $Codex debug models
        if ($LASTEXITCODE -eq 0) {
            Set-Stage "catalog" "PASS" 0
            $script:ActiveStage = "settings"
            $installed = Join-Path $installHome "plugins/cache/groundline/groundline/$installVersion/bin/$target/groundline.exe"
            $catalog | & $installed setup --catalog - --preset $Preset @setupArgs --apply
            if ($LASTEXITCODE -eq 0) { Set-Stage "settings" "PASS" 0 }
            elseif ($LASTEXITCODE -eq 2) { Set-Stage "settings" "ACTION_REQUIRED" 2 }
            else { Set-Stage "settings" "FAIL" $LASTEXITCODE }
            Remove-Variable catalog
        } else {
            Set-Stage "catalog" "FAIL" $LASTEXITCODE
            Set-Stage "settings" "NOT_RUN" 0
        }
    } else { Set-Stage "settings" "NOT_SELECTED" 0 }
    $script:ActiveStage = "native_doctor"
    & $Codex --strict-config doctor --summary --no-color --ascii
    if ($LASTEXITCODE -eq 0) { Set-Stage "native_doctor" "PASS" 0 }
    else { Set-Stage "native_doctor" "ACTION_REQUIRED" $LASTEXITCODE }
    if ($Profile -ne "core") {
        $script:ActiveStage = "insights_setup"
        $installed = Join-Path $installHome "plugins/cache/groundline/groundline-insights/$installVersion/bin/$target/groundline-insights.exe"
        Invoke-InsightsSetup $installed $insightsArgs
        if ($LASTEXITCODE -eq 0) { Set-Stage "insights_setup" "PASS" 0 }
        elseif ($LASTEXITCODE -eq 2) { Set-Stage "insights_setup" "ACTION_REQUIRED" 2 }
        else { Set-Stage "insights_setup" "FAIL" $LASTEXITCODE }
    } else { Set-Stage "insights_setup" "NOT_SELECTED" 0 }
} catch {
    if (!$script:Stages.Contains($script:ActiveStage) -or $script:Stages[$script:ActiveStage].status -ne "FAIL") {
        Set-Stage $script:ActiveStage "FAIL" 1
    }
    Write-Warning "The reported stage failed. Check the preceding diagnostic and rerun after resolving it."
} finally {
    $status = if ($script:Failed) { "FAIL" } elseif ($script:Pending) { "ACTION_REQUIRED" } else { "PASS" }
    [ordered]@{ kind = "groundline-installation"; schema = 1; status = $status; profile = $Profile; stages = $script:Stages; resume = "rerun_same_installer_after_resolving_reported_actions" } | ConvertTo-Json -Depth 5
}
if ($script:Failed) { exit 1 }
if ($script:Pending) { exit 2 }
exit 0
