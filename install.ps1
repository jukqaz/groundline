# Run from the reviewed binary-bearing stable distribution. No global Git edits.
# Profiles select explicit setup; native refresh can update other installed marketplace plugins.
[CmdletBinding()]
param(
    [string]$Codex = "",
    [ValidateSet("core", "insights", "both")][string]$Profile = "core",
    [string]$Model = "", [string]$Effort = "",
    [ValidateSet("", "default", "fast")][string]$ServiceTier = "",
    [switch]$RestoreNativeContext,
    [string]$InsightsProfile = "", [string]$InsightsEndpoint = "",
    [string]$EnrollmentTokenFile = "",
    [switch]$EnableInsights
)
$ErrorActionPreference = "Stop"
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$script:PreviousConsoleEncoding = [Console]::OutputEncoding
[Console]::OutputEncoding = $OutputEncoding
$script:Stages = [ordered]@{}
$script:ActiveStage = "preflight"
$script:Failed = $false
$script:Pending = $false
$script:CodexAppSelected = $false
$script:SourceCommit = $null
$script:PreviousCommit = $null
$script:Rollback = "not_needed"
$script:MarketplaceUrl = "https://github.com/jukqaz/groundline.git"
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

function Get-GitValue([string]$Root, [string[]]$Arguments) {
    $output = & git -c core.fsmonitor=false --literal-pathspecs -C $Root @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) { throw "Git distribution validation failed." }
    return ($output -join "`n").Trim()
}
function Get-NormalPath([string]$Path) {
    return [System.IO.Path]::GetFullPath($Path).TrimEnd([char[]]"\/")
}
function Get-CleanCommit([string]$Root, [bool]$NativeSnapshot = $false) {
    $checkpoint = "inspect_paths"
    try {
        $directory = Get-Item -LiteralPath $Root -Force
        $gitDirectory = Get-Item -LiteralPath (Join-Path $Root ".git") -Force
        if (!$directory.PSIsContainer -or !$gitDirectory.PSIsContainer -or
            ($directory.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -or
            ($gitDirectory.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
            throw "Use a normal clean Git distribution."
        }
        $checkpoint = "current_owner"
        $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
        try {
            # Elevated Windows tokens may create files owned by their default owner
            # SID rather than the user SID; accept only these current-token owners.
            $currentOwners = @($identity.User.Value, $identity.Owner.Value)
            $checkpoint = "path_owner"
            foreach ($path in @($directory.FullName, $gitDirectory.FullName)) {
                $owner = (Get-Acl -LiteralPath $path).GetOwner([System.Security.Principal.SecurityIdentifier]).Value
                if ($owner -notin $currentOwners) { throw "Git distribution ownership is unsupported." }
            }
        } finally { $identity.Dispose() }
        $checkpoint = "git_root"
        $top = Get-GitValue $Root @("rev-parse", "--show-toplevel")
        $checkpoint = "root_alignment"
        if ((Get-NormalPath $top) -ne (Get-NormalPath $Root)) {
            throw "Use the complete clean distribution at its repository root."
        }
        $checkpoint = "git_status"
        $changes = Get-GitValue $Root @("status", "--porcelain", "--untracked-files=all")
        $checkpoint = "clean_status"
        foreach ($change in @($changes -split "`n")) {
            if (!$change) { continue }
            if (!$NativeSnapshot -or $change -cne "?? .codex-marketplace-install.json") {
                throw "Use the complete clean distribution at its repository root."
            }
        }
        $checkpoint = "git_revision"
        $revision = Get-GitValue $Root @("rev-parse", "--verify", "HEAD^{commit}")
        $checkpoint = "revision_format"
        if ($revision -cnotmatch '^[0-9a-f]{40}$') { throw "Invalid distribution commit." }
        return $revision
    } catch {
        # Emit only fixed checkpoints and exception types, never private paths or Git output.
        Write-Warning ("Git distribution validation failed: {0} ({1})." -f $checkpoint, $_.Exception.GetType().Name)
        throw
    }
}
function Get-NativeJson([string[]]$Arguments) {
    $output = & $Codex @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Native state inspection failed." }
    return (($output -join "`n") | ConvertFrom-Json)
}
function Test-ReleaseVersion([string]$Version) {
    return $Version -cmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'
}
function Assert-ForwardVersion([string]$Current, [string]$Candidate) {
    if (!(Test-ReleaseVersion $Current) -or !(Test-ReleaseVersion $Candidate)) {
        throw "Invalid native release version."
    }
    $oldParts = $Current.Split('.')
    $newParts = $Candidate.Split('.')
    for ($index = 0; $index -lt 3; $index++) {
        $oldNumber = [UInt64]::Parse($oldParts[$index])
        $newNumber = [UInt64]::Parse($newParts[$index])
        if ($newNumber -gt $oldNumber) { return }
        if ($newNumber -lt $oldNumber) { throw "A release downgrade requires separate review." }
    }
}
function Test-MarketplaceUrl([string]$Url) {
    return @("https://github.com/jukqaz/groundline.git", "git@github.com:jukqaz/groundline.git", "ssh://git@github.com/jukqaz/groundline.git") -ccontains $Url
}
function Get-NativeState([bool]$RequireVersionAlignment = $true) {
    $listing = Get-NativeJson @("plugin", "marketplace", "list", "--json")
    $plugins = Get-NativeJson @("plugin", "list", "--json")
    if ($listing.marketplaces -isnot [System.Array] -or
        $plugins.installed -isnot [System.Array]) { throw "Invalid native state response." }
    $markets = @($listing.marketplaces | Where-Object { $_.name -eq "groundline" })
    if ($markets.Count -gt 1) { throw "Ambiguous GroundLine marketplace source." }
    $market = $null
    $revision = $null
    if ($markets.Count -eq 1) {
        $market = $markets[0]
        if ($market.marketplaceSource.sourceType -cne "git" -or
            !(Test-MarketplaceUrl $market.marketplaceSource.source) -or
            $market.root -isnot [string] -or ![System.IO.Path]::IsPathRooted($market.root)) {
            throw "Unsupported GroundLine marketplace source."
        }
        $revision = Get-CleanCommit $market.root $true
        $metadataPath = Join-Path $market.root ".codex-marketplace-install.json"
        $metadataFile = $null
        try { $metadataFile = Get-Item -LiteralPath $metadataPath -Force -ErrorAction Stop }
        catch [System.Management.Automation.ItemNotFoundException] {}
        if ($metadataFile) {
            if ($metadataFile.PSIsContainer -or $metadataFile.Length -gt 16384 -or
                ($metadataFile.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) { throw "Invalid native source metadata." }
            $metadata = Get-Content -LiteralPath $metadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($metadata.source_type -cne "git" -or $metadata.source -cne $market.marketplaceSource.source -or
                $metadata.revision -cne $revision -or $metadata.ref_name -isnot [string] -or !$metadata.ref_name -or
                $metadata.sparse_paths -isnot [System.Array] -or $metadata.sparse_paths.Count -ne 0) {
                throw "Native source metadata does not match its snapshot."
            }
        }
    }
    $map = @{}
    foreach ($plugin in @($plugins.installed)) {
        if ($plugin.marketplaceName -ne "groundline") { continue }
        $name = $plugin.name
        if (!$market -or $name -cnotin @("groundline", "groundline-insights") -or
            $map.ContainsKey($name) -or $plugin.pluginId -cne "$name@groundline" -or
            $plugin.installed -isnot [bool] -or !$plugin.installed -or $plugin.enabled -isnot [bool] -or
            $plugin.marketplaceSource.sourceType -cne "git" -or
            $plugin.marketplaceSource.source -cne $market.marketplaceSource.source -or
            $plugin.source.source -cne "local" -or $plugin.source.path -isnot [string] -or
            (Get-NormalPath $plugin.source.path) -ne (Get-NormalPath (Join-Path $market.root "plugins/$name")) -or
            !(Test-ReleaseVersion $plugin.version)) { throw "Unsupported installed GroundLine state." }
        if ($RequireVersionAlignment) {
            $manifest = Get-Content -LiteralPath (Join-Path $market.root "plugins/$name/.codex-plugin/plugin.json") -Raw | ConvertFrom-Json
            if ($manifest.name -cne $name -or $manifest.version -cne $plugin.version) {
                throw "Native snapshot and installed version differ; review the partial update first."
            }
        }
        $map[$name] = [pscustomobject]@{ version = $plugin.version; enabled = $plugin.enabled }
    }
    return [pscustomobject]@{ market = $market; commit = $revision; installed = $map }
}
function Assert-InstalledMap($Actual, $Expected) {
    if ($Actual.Count -ne $Expected.Count) { throw "Installed product set changed unexpectedly." }
    foreach ($name in $Expected.Keys) {
        if (!$Actual.ContainsKey($name) -or $Actual[$name].version -cne $Expected[$name].version -or
            $Actual[$name].enabled -ne $Expected[$name].enabled) {
            throw "Installed version or enabled state changed unexpectedly."
        }
    }
}
function Assert-RestoredArtifacts($Restored) {
    foreach ($name in $Restored.installed.Keys) {
        $version = $Restored.installed[$name].version
        $source = Join-Path $Restored.market.root "plugins/$name/bin/$target"
        $cache = Join-Path $installHome "plugins/cache/groundline/$name/$version/bin/$target"
        foreach ($file in @("$name.exe", "$name.exe.sha256", "manifest.json")) {
            $sourcePath = Join-Path $source $file
            $cachePath = Join-Path $cache $file
            foreach ($path in @($sourcePath, $cachePath)) {
                $item = Get-Item -LiteralPath $path -Force
                if ($item.PSIsContainer -or ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
                    throw "Invalid restored artifact."
                }
            }
            if ((Get-ArtifactSha256 $sourcePath) -cne (Get-ArtifactSha256 $cachePath)) {
                throw "Restored artifact differs from the previous commit."
            }
        }
    }
}
function Restore-NativeSource($Previous, [string[]]$NewProducts) {
    $script:ActiveStage = "source_rollback"
    $current = Get-NativeState $false
    if ($current.market -and $current.commit -cne $script:SourceCommit -and
        $current.commit -cne $script:PreviousCommit) { throw "Unexpected source prevents automatic recovery." }
    foreach ($name in $NewProducts) {
        if ($current.installed.ContainsKey($name)) {
            if ($current.installed[$name].version -cne $installVersion) { throw "Unexpected new product prevents recovery." }
            $null = Get-NativeJson @("plugin", "remove", "$name@groundline", "--json")
        }
    }
    if ($current.market) { $null = Get-NativeJson @("plugin", "marketplace", "remove", "groundline", "--json") }
    if ($Previous.market) {
        $null = Get-NativeJson @("plugin", "marketplace", "add", $Previous.market.marketplaceSource.source, "--ref", $Previous.commit, "--json")
        $null = Get-NativeJson @("plugin", "marketplace", "upgrade", "groundline", "--json")
        $restored = Get-NativeState
        if (!$restored.market -or $restored.commit -cne $Previous.commit) { throw "Previous source commit was not restored." }
        Assert-InstalledMap $restored.installed $Previous.installed
        Assert-RestoredArtifacts $restored
        $script:Rollback = "previous_commit_pinned"
    } else {
        $restored = Get-NativeState
        if ($restored.market -or $restored.installed.Count -ne 0) { throw "Fresh registration was not removed." }
        $script:Rollback = "fresh_registration_removed"
    }
    Set-Stage "source_rollback" "PASS" 0
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
    if (($Profile -eq "core" -and $insightsArgs.Count -gt 0) -or ($Profile -eq "insights" -and $setupArgs.Count -gt 0)) {
        throw "Options must match the selected -Profile core, insights, or both."
    }
    if (!(Get-Command git -ErrorAction SilentlyContinue)) { throw "Install Git before running setup." }
    if (!$Codex) { $Codex = Find-Codex }
    & $Codex plugin add --help | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Codex plugin support is required." }
    & $Codex plugin marketplace upgrade --help | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Codex marketplace upgrade support is required." }
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
    $script:ActiveStage = "native_source"
    $previous = Get-NativeState
    $script:PreviousCommit = $previous.commit
    if ($previous.market) { $script:MarketplaceUrl = $previous.market.marketplaceSource.source }
    Set-Stage "native_source" "PASS" 0
    $products = switch ($Profile) { "core" { @("groundline") } "insights" { @("groundline-insights") } "both" { @("groundline", "groundline-insights") } }
    $updatedProducts = @(@($products) + @($previous.installed.Keys) | Sort-Object -Unique)
    $verifyProducts = @($updatedProducts)
    $checkInsights = $previous.installed.ContainsKey("groundline-insights")
    try {
        $null = Get-Item -LiteralPath (Join-Path $installHome "groundline/insights") -Force -ErrorAction Stop
        $checkInsights = $true
    } catch [System.Management.Automation.ItemNotFoundException] {}
    if ($checkInsights -and "groundline-insights" -notin $verifyProducts) { $verifyProducts += "groundline-insights" }
    $installVersion = ""
    foreach ($product in $verifyProducts) {
        $script:ActiveStage = "distribution_$product"
        $source = Join-Path $PSScriptRoot "plugins/$product"
        $binary = Join-Path $source "bin/$target/$product.exe"
        Invoke-Step "distribution_$product" $binary @("provider-smoke", "--plugin-root", $source, "--require-installed", "--json")
        $versionOutput = & $binary --version
        if ($LASTEXITCODE -ne 0 -or $versionOutput -cnotmatch ('^' + [regex]::Escape($product) + ' ([0-9]+\.[0-9]+\.[0-9]+)$')) { throw "Invalid distribution version." }
        $version = $Matches[1]
        if (!(Test-ReleaseVersion $version) -or ($installVersion -and $installVersion -cne $version)) { throw "Use one complete release distribution." }
        $installVersion = $version
    }
    $script:ActiveStage = "distribution_version"
    foreach ($name in $previous.installed.Keys) { Assert-ForwardVersion $previous.installed[$name].version $installVersion }
    $script:ActiveStage = "distribution_revision"
    $script:SourceCommit = Get-CleanCommit $PSScriptRoot
    foreach ($product in $verifyProducts) {
        $tracked = @("plugins/$product/.codex-plugin/plugin.json", "plugins/$product/bin/$target/$product.exe", "plugins/$product/bin/$target/$product.exe.sha256", "plugins/$product/bin/$target/manifest.json")
        $null = Get-GitValue $PSScriptRoot (@("ls-files", "--error-unmatch", "--") + $tracked)
    }
    Set-Stage "distribution_revision" "PASS" 0
    if ($checkInsights) {
        $candidate = Join-Path $PSScriptRoot "plugins/groundline-insights/bin/$target/groundline-insights.exe"
        Invoke-Step "insights_server_compatibility" $candidate @("worker", "check-server", "--json")
    } else { Set-Stage "insights_server_compatibility" "NOT_CONFIGURED" 0 }
    $expected = @{}
    foreach ($name in $updatedProducts) {
        $enabled = if ($previous.installed.ContainsKey($name)) { $previous.installed[$name].enabled } else { $true }
        $expected[$name] = [pscustomobject]@{ version = $installVersion; enabled = $enabled }
    }
    $newProducts = @()
    $sourceVerified = $false
    try {
        if ($previous.market) {
            Invoke-Step "marketplace_remove" $Codex @("plugin", "marketplace", "remove", "groundline", "--json")
        }
        Invoke-Step "marketplace_add" $Codex @("plugin", "marketplace", "add", $script:MarketplaceUrl, "--ref", $script:SourceCommit, "--json")
        Invoke-Step "marketplace_refresh" $Codex @("plugin", "marketplace", "upgrade", "groundline", "--json")
        $script:ActiveStage = "snapshot_revision"
        $current = Get-NativeState
        if (!$current.market -or $current.commit -cne $script:SourceCommit) { throw "Native source does not match the checked commit." }
        Set-Stage "snapshot_revision" "PASS" 0
        foreach ($product in $products) {
            if (!$previous.installed.ContainsKey($product)) {
                $newProducts += $product
                Invoke-Step "install_$product" $Codex @("plugin", "add", "$product@groundline", "--json")
            } else { Set-Stage "install_$product" "PASS" 0 }
        }
        $script:ActiveStage = "installed_state"
        $current = Get-NativeState
        if (!$current.market -or $current.commit -cne $script:SourceCommit) { throw "Native source changed during installation." }
        Assert-InstalledMap $current.installed $expected
        Set-Stage "installed_state" "PASS" 0
        foreach ($product in $updatedProducts) {
            $cache = Join-Path $installHome "plugins/cache/groundline/$product/$installVersion"
            $installed = Join-Path $cache "bin/$target/$product.exe"
            $script:ActiveStage = "verify_$product"
            if ((Get-ArtifactSha256 (Join-Path $PSScriptRoot "plugins/$product/bin/$target/$product.exe")) -ne (Get-ArtifactSha256 $installed)) {
                throw "Installed artifact differs from the checked distribution."
            }
            Invoke-Step "verify_$product" $installed @("provider-smoke", "--plugin-root", $cache, "--require-installed", "--json")
        }
        $sourceVerified = $true
    } finally {
        # Keep recovery armed through native source, state, and artifact checks.
        # Settings/consent operations below are outside this source transaction.
        if (!$sourceVerified) {
            $failedSourceStage = $script:ActiveStage
            if (!$script:Stages.Contains($failedSourceStage) -or $script:Stages[$failedSourceStage].status -ne "FAIL") {
                Set-Stage $failedSourceStage "FAIL" 1
            }
            try { Restore-NativeSource $previous $newProducts }
            catch {
                $script:Rollback = "failed"
                Set-Stage "source_rollback" "FAIL" 1
            }
            $script:ActiveStage = $failedSourceStage
        }
    }
    if ($Profile -ne "insights") {
        $script:ActiveStage = "catalog"
        $catalog = & $Codex debug models
        if ($LASTEXITCODE -eq 0) {
            Set-Stage "catalog" "PASS" 0
            $script:ActiveStage = "settings"
            $installed = Join-Path $installHome "plugins/cache/groundline/groundline/$installVersion/bin/$target/groundline.exe"
            $catalog | & $installed setup --catalog - @setupArgs --apply
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
    try {
        [ordered]@{ kind = "groundline-installation"; schema = 1; status = $status; profile = $Profile; source_commit = $script:SourceCommit; previous_commit = $script:PreviousCommit; rollback = $script:Rollback; stages = $script:Stages; resume = "rerun_same_installer_after_resolving_reported_actions" } | ConvertTo-Json -Depth 5
    } finally { [Console]::OutputEncoding = $script:PreviousConsoleEncoding }
}
if ($script:Failed) { exit 1 }
if ($script:Pending) { exit 2 }
exit 0
