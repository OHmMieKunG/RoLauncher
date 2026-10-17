$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot)).TrimEnd('\')
$taskGitDirectory = Join-Path $taskRoot '.git'
if (Test-Path -LiteralPath $taskGitDirectory) {
    throw 'This folder already has Git metadata. Inspect git remote -v and git status before reconnecting.'
}
foreach ($taskCommand in @('git','gh')) { Get-Command $taskCommand -ErrorAction Stop | Out-Null }

function Invoke-ForkTool {
    param([string]$TaskExecutable, [Parameter(ValueFromRemainingArguments = $true)][string[]]$TaskArguments)
    & $TaskExecutable @TaskArguments
    if ($LASTEXITCODE -ne 0) {
        throw "$TaskExecutable failed. Run this script in your normal network-enabled PowerShell. For authentication errors, first run: gh auth login --hostname github.com"
    }
}

$taskLogin = [string](Invoke-ForkTool gh api user --jq .login)
if ($taskLogin.Trim() -ne 'OHmMieKunG') { throw 'Authenticate gh as OHmMieKunG before creating this fork.' }
Invoke-ForkTool gh repo fork oangsa/RoLauncher --clone=false | Out-Host
$taskParent = [string](Invoke-ForkTool gh api repos/OHmMieKunG/RoLauncher --jq .parent.full_name)
if ($taskParent.Trim() -ne 'oangsa/RoLauncher') { throw 'OHmMieKunG/RoLauncher is not a fork of the expected upstream repository.' }

$taskStage = [IO.Path]::GetFullPath((Join-Path $taskRoot ('target/fork-checkout-' + [guid]::NewGuid().ToString('N'))))
if (-not $taskStage.StartsWith($taskRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Checkout outside workspace' }
New-Item -ItemType Directory -Path (Split-Path -Parent $taskStage) -Force | Out-Null
Invoke-ForkTool git clone --no-checkout https://github.com/OHmMieKunG/RoLauncher.git $taskStage
Invoke-ForkTool git -C $taskStage remote add upstream https://github.com/oangsa/RoLauncher.git
Invoke-ForkTool git -C $taskStage fetch upstream refs/tags/v1.7.0:refs/tags/v1.7.0
Invoke-ForkTool git -C $taskStage branch feature/resource-controls-anti-afk refs/tags/v1.7.0
Invoke-ForkTool git -C $taskStage symbolic-ref HEAD refs/heads/feature/resource-controls-anti-afk
# Populate the index without checking out files over the existing implementation.
Invoke-ForkTool git -C $taskStage reset --mixed HEAD | Out-Null
$taskBase = [string](Invoke-ForkTool git -C $taskStage rev-parse HEAD)
if ($taskBase.Trim() -notmatch '^[0-9a-f]{40,64}$') { throw 'Upstream commit identity unavailable' }

$taskMetadata = (Resolve-Path -LiteralPath (Join-Path $taskStage '.git')).Path
if ([IO.Path]::GetDirectoryName($taskMetadata) -ne $taskStage -or [IO.Path]::GetDirectoryName($taskGitDirectory) -ne $taskRoot) { throw 'Unexpected metadata move target' }
if ((Get-Item -LiteralPath $taskMetadata -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing linked Git metadata' }
Move-Item -LiteralPath $taskMetadata -Destination $taskGitDirectory
$taskConnected = [string](Invoke-ForkTool git -C $taskRoot rev-parse HEAD)
if ($taskConnected.Trim() -ne $taskBase.Trim()) { throw 'Connected history does not match the verified upstream base' }

Write-Output 'Fork: https://github.com/OHmMieKunG/RoLauncher'
Write-Output "Base commit: $($taskBase.Trim())"
Write-Output 'Branch: feature/resource-controls-anti-afk'
Write-Output 'Current source changes are preserved and uncommitted. Review before committing and pushing.'
Invoke-ForkTool git -C $taskRoot remote -v
Invoke-ForkTool git -C $taskRoot status --short
