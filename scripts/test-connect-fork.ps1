$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskNativeGit = (Get-Command git -CommandType Application).Source
$taskFixture = Join-Path $taskRoot ('target/fork-tests-' + [guid]::NewGuid().ToString('N'))
$taskRepository = Join-Path $taskFixture 'upstream'
New-Item -ItemType Directory -Path $taskRepository -Force | Out-Null

function Invoke-TestGit {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$TaskArguments)
    & $taskNativeGit @TaskArguments
    $global:LASTEXITCODE = $LASTEXITCODE
    if ($LASTEXITCODE -ne 0) { throw 'Native Git fixture command failed' }
}
Invoke-TestGit init -b main $taskRepository | Out-Null
Set-Content -LiteralPath (Join-Path $taskRepository 'tracked.txt') -Value 'baseline' -Encoding ascii
Invoke-TestGit -C $taskRepository add tracked.txt
Invoke-TestGit -C $taskRepository -c user.name=ForkFixture -c user.email=fixture@example.invalid commit -m baseline | Out-Null
Invoke-TestGit -C $taskRepository tag v1.7.0
$taskExpectedCommit = [string](Invoke-TestGit -C $taskRepository rev-parse HEAD)

$global:forkTestMode = 'success'
$global:forkTestCalls = [Collections.Generic.List[string]]::new()
function global:gh {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$TaskArguments)
    $global:LASTEXITCODE = 0
    $global:forkTestCalls.Add(($TaskArguments -join ' '))
    switch ($TaskArguments -join ' ') {
        'api user --jq .login' { if ($global:forkTestMode -eq 'wrong-account') { 'SomeoneElse' } else { 'OHmMieKunG' } }
        'repo fork oangsa/RoLauncher --clone=false' { 'https://github.com/OHmMieKunG/RoLauncher' }
        'api repos/OHmMieKunG/RoLauncher --jq .parent.full_name' { if ($global:forkTestMode -eq 'wrong-parent') { 'another/repository' } else { 'oangsa/RoLauncher' } }
        default { throw 'Unexpected GitHub call in fixture' }
    }
}
function global:git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$TaskArguments)
    if ($TaskArguments[0] -eq 'clone') {
        if ($global:forkTestMode -eq 'transport-failure') { $global:LASTEXITCODE = 128; return }
        Invoke-TestGit clone --no-checkout $taskRepository $TaskArguments[-1]
        Invoke-TestGit -C $TaskArguments[-1] remote set-url origin https://github.com/OHmMieKunG/RoLauncher.git
    } elseif ($TaskArguments.Count -gt 2 -and $TaskArguments[2] -eq 'fetch') {
        Invoke-TestGit -C $TaskArguments[1] fetch $taskRepository $TaskArguments[-1]
    } else { Invoke-TestGit @TaskArguments }
}

function New-ForkFixture {
    param([string]$TaskName)
    $taskDirectory = Join-Path $taskFixture $TaskName
    New-Item -ItemType Directory -Path (Join-Path $taskDirectory 'scripts') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'connect-fork.ps1') -Destination (Join-Path $taskDirectory 'scripts/connect-fork.ps1')
    Set-Content -LiteralPath (Join-Path $taskDirectory 'tracked.txt') -Value 'feature changes must survive' -Encoding ascii
    Set-Content -LiteralPath (Join-Path $taskDirectory 'new-feature.txt') -Value 'new source must survive' -Encoding ascii
    $taskDirectory
}
function Assert-ForkCheck {
    param([bool]$TaskCondition, [string]$TaskMessage)
    if (-not $TaskCondition) { throw $TaskMessage }
    $script:taskPassed++
}
$taskPassed = 0
$taskSuccess = New-ForkFixture success
$taskOriginal = [Convert]::ToBase64String([IO.File]::ReadAllBytes((Join-Path $taskSuccess 'tracked.txt')))
& (Join-Path $taskSuccess 'scripts/connect-fork.ps1') | Out-Null
Assert-ForkCheck (([string](Invoke-TestGit -C $taskSuccess rev-parse HEAD)).Trim() -eq $taskExpectedCommit.Trim()) 'Upstream history must be preserved'
Assert-ForkCheck (([string](Invoke-TestGit -C $taskSuccess branch --show-current)).Trim() -eq 'feature/resource-controls-anti-afk') 'Feature branch must use the upstream tag'
Assert-ForkCheck (([string](Invoke-TestGit -C $taskSuccess remote get-url origin)).Trim() -eq 'https://github.com/OHmMieKunG/RoLauncher.git') 'Origin must target the fork'
Assert-ForkCheck (([string](Invoke-TestGit -C $taskSuccess remote get-url upstream)).Trim() -eq 'https://github.com/oangsa/RoLauncher.git') 'Upstream must retain the source repository'
Assert-ForkCheck ([Convert]::ToBase64String([IO.File]::ReadAllBytes((Join-Path $taskSuccess 'tracked.txt'))) -eq $taskOriginal) 'Existing source must not be overwritten'
Assert-ForkCheck (Test-Path -LiteralPath (Join-Path $taskSuccess 'new-feature.txt')) 'New feature files must survive'
Assert-ForkCheck (([string](Invoke-TestGit -C $taskSuccess diff -- tracked.txt)).Contains('feature changes must survive')) 'Source changes must remain reviewable'

foreach ($taskMode in @('wrong-account','wrong-parent','transport-failure')) {
    $global:forkTestMode = $taskMode
    $global:forkTestCalls.Clear()
    $taskFailure = New-ForkFixture $taskMode
    $taskFailed = $false
    try { & (Join-Path $taskFailure 'scripts/connect-fork.ps1') | Out-Null } catch { $taskFailed = $true }
    Assert-ForkCheck $taskFailed "$taskMode must stop setup"
    Assert-ForkCheck (-not (Test-Path -LiteralPath (Join-Path $taskFailure '.git'))) "$taskMode must not attach incomplete metadata"
    if ($taskMode -eq 'wrong-account') { Assert-ForkCheck ($global:forkTestCalls.Count -eq 1) 'Wrong account must not create a fork' }
}
$global:forkTestCalls.Clear()
$taskFailed = $false
try { & (Join-Path $taskSuccess 'scripts/connect-fork.ps1') | Out-Null } catch { $taskFailed = $true }
Assert-ForkCheck ($taskFailed -and $global:forkTestCalls.Count -eq 0) 'Existing repositories must remain untouched'
Write-Output "$taskPassed fork setup checks passed; GitHub mocked, real local Git history and source preservation verified."
