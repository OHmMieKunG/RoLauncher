# Bootstrapper integration

Live provider acceptance remains BLOCKED; see the [host compatibility report](client-compatibility-report.md) and [provider batch procedure](client-acceptance.md).

Research reviewed 2026-10-17. Live provider gameplay remains unverified.

## Providers

| Selection | Launch method | Detection and limits |
| --- | --- | --- |
| Auto-detect | Existing `ShellExecuteW` adapter | Default for migrated accounts; displays and follows the registered launcher without changing handlers. |
| Official | `RobloxPlayerBeta.exe` with the existing URI as one argument | Registered official executable, installed local Versions directory, or explicit path. |
| Bloxstrap | Installed executable with one URI argument | Registered paths, uninstall records, standard local folder or explicit path; both protocols must already select it. |
| Fishstrap | Installed executable with one URI argument | Same guard; no automatic protocol reassignment. |
| Custom | Trusted local executable with one URI argument | Explicit absolute `.exe` path; it must support that contract and launch a discoverable installed player. |

`BootstrapperProvider` discovers installation/configuration and constructs a launch request.
Providers share the existing verified process discovery, ownership deadline, log connection verification and recovery status.
An executable starting successfully is not reported as a connected player.

## Source evidence

- [Bloxstrap LaunchSettings](https://github.com/bloxstraplabs/bloxstrap/blob/main/Bloxstrap/LaunchSettings.cs) accepts a first `roblox-player:` argument and also supports `-player` data.
- [Fishstrap LaunchSettings](https://github.com/fishstrap/fishstrap/blob/main/Bloxstrap/LaunchSettings.cs) implements the same first-argument URI handling.
- [Fishstrap startup](https://github.com/fishstrap/fishstrap/blob/main/Bloxstrap/App.xaml.cs) reads its uninstall registration and invokes protocol registration during startup.
- [Bloxstrap's switching guide](https://github.com/bloxstraplabs/bloxstrap/wiki/Switching-between-Roblox-and-Bloxstrap) describes launch-handler registration when launching Bloxstrap.

No invented `--headless`/`--no-render` argument is sent to a provider. Bootstrapper `nogpu` flags do not establish a Roblox rendering-off interface.

## Coexistence

RoLauncher does not rewrite the protocol registry or edit provider configuration.
It rejects a Bloxstrap/Fishstrap executable unless both `roblox-player` and `roblox` already resolve to it, preventing competing self-registration for either protocol.
Concurrent Bloxstrap plus Fishstrap launching is therefore **UNSUPPORTED** under this guard; use Official alongside the registered provider.

Changing a preference affects future launches/rejoins, while a running instance remains intact.
Recovery resolves the selected provider again; missing installations or a changed handler produce an explicit error, never silent fallback.
Restart RoLauncher after installing or relocating a provider because process-image roots are cached for efficient discovery.

## Trust and verification

Explicit executables require an absolute path, matching known-provider filename, readable DOS/PE headers and canonical path resolution.
PE validation is not Authenticode verification; only select trusted installed executables, especially Custom, which receives an authentication ticket.
The process must still pass tracker, PID, creation-time, generation, installed-image and Windows-session checks before control.

`GET /v1/bootstrappers` reports verified executable paths, launch availability, discovered settings paths and errors.
An executable may be detected while `available` is false because its protocol registration conflicts; launch requests still reject that provider.
Settings files are discovered read-only and never changed.

## Automatic detection

1. Open account details; the launcher list refreshes automatically and labels Auto-detect with the registered launcher name.
2. Leave the executable override blank to use discovery, or retain an explicit provider/path; refreshing never changes either saved choice.
3. Use Refresh launcher detection after installation changes; register a different provider through its own settings if required.

Bloxstrap/Fishstrap discovery checks matching protocol executables first, then current-user/system uninstall records, then the standard local folder.
Missing or invalid candidates are skipped; every detected executable passes the existing filename/PE/canonical-path checks.
An unregistered installation is shown with its limitation rather than silently launched or substituted.

Standard folders are documented by [Bloxstrap's installation guide](https://github.com/bloxstraplabs/bloxstrap/wiki/Change-where-Bloxstrap-is-installed) and [Fishstrap's installer source](https://github.com/fishstrap/fishstrap/blob/main/Bloxstrap/Installer.cs).
