param(
    [string]$ConfigPath,
    [switch]$PreflightOnly,
    [string]$OutputDirectory = 'target\live-client-preflight',
    [string]$PresentMon
)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $taskRoot
try {
    . "$PSScriptRoot\environment.ps1"
    $taskExpectedHash = 'b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af'
    if ($ConfigPath) {
        $taskConfigFile = (Resolve-Path -LiteralPath $ConfigPath).Path
        $taskConfig = Get-Content -LiteralPath $taskConfigFile -Raw | ConvertFrom-Json
        if ($taskConfig.presentmon) { $PresentMon = $taskConfig.presentmon }
    }
    if ($PresentMon) {
        if ((Get-FileHash -LiteralPath $PresentMon -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskExpectedHash) {
            throw 'Expected the pinned official PresentMon 2.6.0 x64 console binary; SHA256 mismatch'
        }
    }
    if ($PreflightOnly) {
        $taskOutput = [IO.Path]::GetFullPath((Join-Path $taskRoot $OutputDirectory))
        if (Test-Path -LiteralPath $taskOutput) { throw 'Use a fresh preflight output directory' }
        New-Item -ItemType Directory -Path $taskOutput | Out-Null
        $taskMissing = @()
        $taskOs = $null; $taskGpus = @(); $taskCpu = @()
        try {
            $taskOsRow = Get-CimInstance Win32_OperatingSystem
            $taskRevision = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').UBR
            $taskOs = @{ name=$taskOsRow.Caption; version=$taskOsRow.Version; revision=$taskRevision; architecture=$taskOsRow.OSArchitecture }
            $taskGpus = @(Get-CimInstance Win32_VideoController | ForEach-Object { @{ name=$_.Name; driver=$_.DriverVersion } })
            $taskCpu = @(Get-CimInstance Win32_Processor | ForEach-Object { @{ name=$_.Name; logical_processors=$_.NumberOfLogicalProcessors } })
        } catch { $taskMissing += 'CIM hardware inventory unavailable' }
        $taskProviders = @()
        foreach ($taskProvider in @('Bloxstrap','Fishstrap')) {
            $taskExe = Join-Path $env:LOCALAPPDATA "$taskProvider\$taskProvider.exe"
            $taskProviders += @{ name=$taskProvider; default_path_present=(Test-Path -LiteralPath $taskExe); version=$(if (Test-Path -LiteralPath $taskExe) { (Get-Item -LiteralPath $taskExe).VersionInfo.ProductVersion } else { $null }) }
        }
        $taskHandlers = @{}
        foreach ($taskProtocol in @('roblox-player','roblox')) {
            try {
                $taskCommand = (Get-Item -LiteralPath "Registry::HKEY_CLASSES_ROOT\$taskProtocol\shell\open\command").GetValue('')
                $taskMatch = [regex]::Match($taskCommand, '^\s*"([^"]+\.exe)"|^\s*(\S+\.exe)')
                $taskExecutable = if ($taskMatch.Groups[1].Success) { $taskMatch.Groups[1].Value } else { $taskMatch.Groups[2].Value }
                $taskHandlers[$taskProtocol] = [IO.Path]::GetFileName($taskExecutable)
            } catch { $taskHandlers[$taskProtocol] = $null }
        }
        $taskPlayers = @()
        foreach ($taskBase in @((Join-Path $env:LOCALAPPDATA 'Roblox\Versions'),(Join-Path $env:LOCALAPPDATA 'Fishstrap\Versions'),(Join-Path $env:LOCALAPPDATA 'Bloxstrap\Versions'))) {
            if (Test-Path -LiteralPath $taskBase) {
                $taskPlayers += @(Get-ChildItem -LiteralPath $taskBase -Filter RobloxPlayerBeta.exe -Recurse -File | ForEach-Object { @{ distribution=(Split-Path -Leaf (Split-Path -Parent $taskBase)); version=$_.VersionInfo.FileVersion } })
            }
        }
        $taskProcesses = @(Get-Process -Name RobloxPlayerBeta,RoLauncher,Bloxstrap,Fishstrap -ErrorAction SilentlyContinue | ForEach-Object { @{ name=$_.ProcessName; pid=$_.Id } })
        $taskInventory = @{
            schema=1; utc=[DateTime]::UtcNow.ToString('o'); live_tests='BLOCKED'; reason='Dedicated accounts and authorized experience are not ready'
            windows=$taskOs; cpu=$taskCpu; gpu=$taskGpus; providers=$taskProviders; protocol_handlers=$taskHandlers
            installed_players=$taskPlayers; active_processes=$taskProcesses; missing=$taskMissing
            tools=@{ wpr_available=[bool](Get-Command wpr.exe -ErrorAction SilentlyContinue); presentmon_verified=[bool]$PresentMon }
        }
        $taskInventory | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $taskOutput 'inventory.json') -Encoding utf8
        & python "$PSScriptRoot\client-report.py" --blocked (Join-Path $taskOutput 'inventory.json') --output (Join-Path $taskOutput 'report.json')
        if ($LASTEXITCODE -ne 0) { throw 'Blocked-case report generation failed' }
        Write-Output "Preflight only; no accounts launched or modified. Evidence: $taskOutput"
        return
    }
    if (-not $ConfigPath) { throw 'Supply -ConfigPath, or use -PreflightOnly without accounts' }
    $taskOldToken = $env:ROLAUNCHER_API_TOKEN
    $taskOldConfig = $env:ROLAUNCHER_ACCEPTANCE_CONFIG
    $taskSecureToken = Read-Host 'API token from the isolated test launcher (never a Roblox cookie)' -AsSecureString
    $taskPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($taskSecureToken)
    try {
        $env:ROLAUNCHER_API_TOKEN = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($taskPointer)
        $env:ROLAUNCHER_ACCEPTANCE_CONFIG = $taskConfigFile
        & cargo test --locked --lib controls::windows::acceptance::live_roblox_acceptance -- --ignored --exact --test-threads=1 --nocapture
        if ($LASTEXITCODE -ne 0) { throw 'Acceptance did not complete; inspect journal and cleanup status' }
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($taskPointer)
        $taskSecureToken.Dispose()
        $env:ROLAUNCHER_API_TOKEN = $taskOldToken
        $env:ROLAUNCHER_ACCEPTANCE_CONFIG = $taskOldConfig
    }
} finally { Pop-Location }
