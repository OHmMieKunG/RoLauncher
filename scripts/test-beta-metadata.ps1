$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
. "$PSScriptRoot\environment.ps1"
$taskFixture = Join-Path $taskRoot ('target\beta-metadata-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path "$taskFixture\src","$taskFixture\assets" | Out-Null
Copy-Item -LiteralPath "$taskRoot\build.rs" -Destination $taskFixture
Copy-Item -LiteralPath "$taskRoot\assets\RoLauncher.ico" -Destination "$taskFixture\assets"
@'
[package]
name = "rolauncher-beta-metadata-fixture"
version = "1.8.4-beta.1"
edition = "2024"
[build-dependencies]
winresource = "=0.1.31"
'@ | Set-Content -LiteralPath "$taskFixture\Cargo.toml" -Encoding ascii
'fn main() { println!("{}", env!("CARGO_PKG_VERSION")); }' | Set-Content -LiteralPath "$taskFixture\src\main.rs" -Encoding ascii
$taskRuntime = & cargo run --quiet --offline --manifest-path "$taskFixture\Cargo.toml" --target-dir "$taskRoot\target\beta-metadata-build"
if ($LASTEXITCODE -ne 0 -or $taskRuntime.Trim() -ne '1.8.4-beta.1') { throw 'Rust beta runtime identity lost' }
$taskRust = Join-Path $taskRoot 'target\beta-metadata-build\debug\rolauncher-beta-metadata-fixture.exe'
New-Item -ItemType Directory -Path "$taskFixture\desktop" | Out-Null
'<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework><Version>1.8.4-beta.1</Version><IncludeSourceRevisionInInformationalVersion>false</IncludeSourceRevisionInInformationalVersion></PropertyGroup></Project>' | Set-Content -LiteralPath "$taskFixture\desktop\Metadata.csproj" -Encoding ascii
'public class Metadata {}' | Set-Content -LiteralPath "$taskFixture\desktop\Metadata.cs" -Encoding ascii
& dotnet build "$taskFixture\desktop\Metadata.csproj" --configfile "$taskRoot\desktop\NuGet.Config" -o "$taskFixture\desktop-output" -p:NuGetAudit=false
if ($LASTEXITCODE -ne 0) { throw 'C# beta metadata build failed' }
foreach ($taskBinary in @($taskRust, "$taskFixture\desktop-output\Metadata.dll")) {
    $taskInfo = (Get-Item -LiteralPath $taskBinary).VersionInfo
    if ($taskInfo.ProductVersion -ne '1.8.4-beta.1' -or
        "$($taskInfo.FileMajorPart).$($taskInfo.FileMinorPart).$($taskInfo.FileBuildPart).$($taskInfo.FilePrivatePart)" -ne '1.8.4.0') {
        throw 'Beta informational or numeric Windows metadata mismatch'
    }
}
Write-Output 'Isolated Rust/C# beta metadata passed: full 1.8.4-beta.1 identity and numeric Windows version 1.8.4.0.'
