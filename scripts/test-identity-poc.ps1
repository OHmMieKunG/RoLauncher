param([string]$EvidenceDirectory = 'target/identity-poc-evidence')
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $taskRoot
try {
    . "$PSScriptRoot/environment.ps1"
    $taskEvidence = [IO.Path]::GetFullPath((Join-Path $taskRoot $EvidenceDirectory))
    New-Item -ItemType Directory -Force -Path $taskEvidence | Out-Null
    & cargo test --locked --example windows-device-identity
    if ($LASTEXITCODE -ne 0) { throw 'PoC contract tests failed' }
    & cargo build --locked --example windows-device-identity
    if ($LASTEXITCODE -ne 0) { throw 'PoC harness build failed' }
    $taskHarness = Join-Path $taskRoot 'target/debug/examples/windows-device-identity.exe'
    function Invoke-Poc([string[]]$Arguments) {
        $taskInfo = [Diagnostics.ProcessStartInfo]::new($taskHarness)
        $taskInfo.UseShellExecute = $false
        $taskInfo.CreateNoWindow = $true
        $taskInfo.RedirectStandardOutput = $true
        $taskInfo.RedirectStandardError = $true
        foreach ($taskArgument in $Arguments) { $taskInfo.ArgumentList.Add($taskArgument) }
        $taskProcess = [Diagnostics.Process]::Start($taskInfo)
        try {
            $taskOutput = $taskProcess.StandardOutput.ReadToEndAsync()
            $taskErrors = $taskProcess.StandardError.ReadToEndAsync()
            $taskProcess.WaitForExit()
            return [ordered]@{ exit_code=$taskProcess.ExitCode; output=$taskOutput.GetAwaiter().GetResult(); error=$taskErrors.GetAwaiter().GetResult() }
        } finally { $taskProcess.Dispose() }
    }
    $taskWindows = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
    $taskReport = [ordered]@{
        status='Measurements, not a security acceptance verdict; failures and missing contexts remain explicit'
        windows=[ordered]@{ build=$taskWindows.CurrentBuildNumber; revision=$taskWindows.UBR; display_version=$taskWindows.DisplayVersion }
        context=(Invoke-Poc @('context'))
        capabilities=(Invoke-Poc @('capabilities'))
        cross_user='Not tested: requires genuine User B; follow manual procedure'
        reboot='Not tested: follow manual procedure'
        measurements=[ordered]@{}
    }
    foreach ($taskProvider in @('software','tpm')) {
        foreach ($taskScope in @('user','machine')) {
            $taskReport.measurements["$taskProvider-$taskScope"] = Invoke-Poc @('key-probe',$taskProvider,$taskScope)
        }
    }
    foreach ($taskCandidate in @('A','B','C')) {
        $taskState = Join-Path $taskEvidence ("private-local-public-state-$taskCandidate-" + [guid]::NewGuid().ToString() + '.json')
        $taskTrial = [ordered]@{ create=(Invoke-Poc @('create',$taskCandidate,$taskState)) }
        # A create command can persist state before a later reopen/inspection failure.
        # Any retained state still requires cleanup, even when command exit status is nonzero.
        if (Test-Path -LiteralPath $taskState) {
            try {
                if ($taskTrial.create.exit_code -eq 0) {
                    $taskTrial.restart = Invoke-Poc @('check',$taskState)
                    $taskTrial.inspect = Invoke-Poc @('inspect',$taskState)
                    $taskTrial.owner_native_probe = Invoke-Poc @('probe',$taskState)
                }
            } finally {
                $taskTrial.cleanup = Invoke-Poc @('cleanup',$taskState)
                if ($taskTrial.cleanup.exit_code -eq 0) { Remove-Item -LiteralPath $taskState }
            }
        }
        $taskReport.measurements["candidate-$taskCandidate"] = $taskTrial
    }
    # Only this sanitized report is uploadable. Never upload PoC public state, SID references,
    # registry evidence, names or blobs. Keep failed-cleanup state local for recovery.
    $taskReport | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $taskEvidence 'measurements.json') -Encoding utf8
    Write-Output 'PoC measurements saved; inspect operation exit codes. Missing hardware/contexts are not passes.'
    foreach ($taskTrial in $taskReport.measurements.Values) {
        if ($taskTrial.Contains('cleanup') -and $taskTrial.cleanup.exit_code -ne 0) { throw 'PoC cleanup failed; retain local public state for recovery' }
        if ($taskTrial.Contains('error') -and $taskTrial.error.Contains('cleanup requires retry')) { throw 'PoC probe cleanup failed; do not treat this as hardware unavailability' }
    }
} finally { Pop-Location }
