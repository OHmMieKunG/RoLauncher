param([string]$OutputRootDirectory = 'dist', [switch]$UseCurrentVersion)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $taskRoot
try {
    . "$PSScriptRoot\environment.ps1"
    if (-not $UseCurrentVersion) {
        & python "$PSScriptRoot\versioning.py"
        if ($LASTEXITCODE -ne 0) { throw 'Development version stamping failed' }
    }
    $taskManifest = Get-Content -LiteralPath 'Cargo.toml' -Raw
    $taskVersion = [regex]::Match($taskManifest, '(?m)^version = "(\d+\.\d+\.\d+(?:-dev\.[0-9a-f]{12}|-beta\.[1-9][0-9]*)?)"').Groups[1].Value
    if (-not $taskVersion) { throw 'Cannot determine release version' }
    $taskName = "rolauncher-v$taskVersion"
    $taskOutput = if ([IO.Path]::IsPathRooted($OutputRootDirectory)) { [IO.Path]::GetFullPath($OutputRootDirectory) } else { [IO.Path]::GetFullPath((Join-Path $taskRoot $OutputRootDirectory)) }
    $taskBinaryZip = Join-Path $taskOutput "$taskName-windows-x64.zip"
    $taskSourceZip = Join-Path $taskOutput "$taskName-source.zip"
    $taskInstaller = Join-Path $taskOutput "$taskName-setup-x64.exe"
    $taskChecksums = Join-Path $taskOutput "$taskName-SHA256SUMS.txt"
    foreach ($taskArtifact in @($taskBinaryZip, $taskSourceZip, $taskInstaller, $taskChecksums)) {
        if (Test-Path -LiteralPath $taskArtifact) { throw "Release artifact already exists: $taskArtifact. Bump the version; do not overwrite a release." }
    }
    $taskDistribution = Join-Path $taskOutput $taskName
    if (Test-Path -LiteralPath $taskDistribution) { throw "Distribution directory already exists: $taskDistribution" }
    & "$PSScriptRoot\build.ps1" -UseCurrentVersion -OutputDirectory $taskDistribution
    Copy-Item -LiteralPath 'AGENTS.md' -Destination $taskDistribution
    & python "$PSScriptRoot\create-archive.py" $taskDistribution $taskBinaryZip --include-root
    if ($LASTEXITCODE -ne 0) { throw 'Portable ZIP creation failed' }
    # Stable source must correspond exactly to the committed build inputs.
    if ($taskVersion -notmatch '-dev\.') {
        & git diff --quiet HEAD --
        if ($LASTEXITCODE -ne 0) { throw 'Stable source packaging requires a clean committed tree' }
        & git archive --format=zip "--output=$taskSourceZip" HEAD
        if ($LASTEXITCODE -ne 0) { throw 'Exact-commit source archive failed' }
    } else {
        $taskSource = Join-Path $taskRoot ('target\source-package-' + [guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $taskSource | Out-Null
        foreach ($taskItem in @('Cargo.toml','Cargo.lock','build.rs','README.md','CHANGELOG.md','AGENTS.md','LICENSE','THIRD_PARTY_NOTICES.md','Directory.Build.props','global.json','.gitignore','.gitattributes','.github','assets','installer','src','examples','scripts','docs','licenses')) {
            Copy-Item -LiteralPath $taskItem -Destination $taskSource -Recurse
        }
        foreach ($taskProject in @('desktop','desktop.tests')) {
            $taskProjectSource = Join-Path $taskSource $taskProject
            New-Item -ItemType Directory -Path $taskProjectSource | Out-Null
            Get-ChildItem -LiteralPath $taskProject -File | Copy-Item -Destination $taskProjectSource
        }
        & python "$PSScriptRoot\create-archive.py" $taskSource $taskSourceZip
        if ($LASTEXITCODE -ne 0) { throw 'Source ZIP creation failed' }
    }
    & "$PSScriptRoot\build-installer.ps1" -DistributionDirectory $taskDistribution -OutputDirectory $taskOutput
    $taskLines = foreach ($taskArtifact in @($taskBinaryZip, $taskSourceZip, $taskInstaller)) {
        $taskHash = Get-FileHash -LiteralPath $taskArtifact -Algorithm SHA256
        "$($taskHash.Hash.ToLowerInvariant())  $(Split-Path -Leaf $taskArtifact)"
    }
    $taskLines | Set-Content -LiteralPath $taskChecksums -Encoding ascii
    Write-Output "Packaged $taskName with source, notices and SHA256 checksums."
} finally { Pop-Location }
