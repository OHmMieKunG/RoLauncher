$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskOutput = Join-Path $taskRoot 'target/desktop-model-checks'
New-Item -ItemType Directory -Path $taskOutput -Force | Out-Null
@'
{"sdk":{"version":"9.0.100","rollForward":"latestFeature","allowPrerelease":false}}
'@ | Set-Content -LiteralPath (Join-Path $taskOutput 'global.json') -Encoding ascii
# This validates shared C# sources with the installed SDK, not the net8 WinUI package.
@'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net9.0</TargetFramework><Nullable>enable</Nullable><ImplicitUsings>enable</ImplicitUsings><TreatWarningsAsErrors>true</TreatWarningsAsErrors><EnableDefaultCompileItems>false</EnableDefaultCompileItems><NuGetAudit>false</NuGetAudit></PropertyGroup>
  <ItemGroup>
    <Compile Include="../../desktop.tests/*.cs" />
    <Compile Include="../../desktop/Models.cs;../../desktop/EditorDraft.cs;../../desktop/BulkDraft.cs;../../desktop/AccountFilter.cs;../../desktop/UpdateDownloader.cs;../../desktop/CookieBatch.cs" />
  </ItemGroup>
</Project>
'@ | Set-Content -LiteralPath (Join-Path $taskOutput 'Checks.csproj') -Encoding utf8
Push-Location -LiteralPath $taskOutput
try {
    & dotnet restore Checks.csproj --ignore-failed-sources
    if ($LASTEXITCODE -ne 0) { throw 'Offline desktop validation restore failed' }
    & dotnet run --project Checks.csproj --no-restore
    if ($LASTEXITCODE -ne 0) { throw 'Offline desktop model checks failed' }
} finally { Pop-Location }
