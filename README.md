# RoLauncher 1.8.4-dev.a7c40bd57487

Development source based on published stable 1.7.0, targeting the next feature release.
Adds per-account background client modes, provider selection, reversible resource profiles and monitoring in the existing launcher.
See [implementation status](docs/headless-architecture.md#delivery-status) for tested behavior and live-validation limits.

Manage Roblox accounts, save destinations, and rejoin disconnected sessions.
Supports Windows 10/11 x64 with the native WinUI desktop. Linux support has been removed.

## Download

Build this development tree using [Development](docs/DEVELOPMENT.md).
[Upstream baseline releases](https://github.com/oangsa/RoLauncher/releases/latest) do not include these fork changes.

- Windows 10 (1809+) or Windows 11, x64.
- Browser sign-in requires [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

## Get started

1. Add accounts through browser sign-in or cookie import.
2. Choose destinations and select accounts.
3. Press **Start**.

Keep cookies private; never include them in bug reports.

## Features

- Per-account Normal/Headless requests, live Hide/Restore, batch controls, optional affinity, and CPU/RAM/GPU monitoring.
- Independent provider selection and Default/Balanced/Low Resource profiles; see [provider coexistence limits](docs/bootstrapper-integration.md).
- Opt-in experimental foreground activity with interface/tray pause and Ctrl+Alt+P; idle prevention is unverified.
- New settings carried through bulk edits, presets, imports/exports, and encrypted backups.

- Saved games, account groups, presets, and bulk edits.
- Automatic rejoin, fallback policies, and activity Dashboard.
- Encrypted backups, optional Discord alerts and bot controls.
- System/light/dark themes, tray controls, and **Download → Relaunch** updates.
- Optional beta updates under **Settings → Updates**.

Headless retains the actual Roblox client; it does not prove rendering stops.
Network-only remains unsupported. See [upcoming changes](docs/RELEASE-1.8.4.md) and [feasibility research](docs/network-only-feasibility.md).

## Documentation

[User guide](docs/USAGE.md) · [API](docs/API.md) · [Development](docs/DEVELOPMENT.md) · [Changelog](CHANGELOG.md)

[Report bugs](https://github.com/OHmMieKunG/RoLauncher/issues) · [Validation and limits](docs/VALIDATION.md)

## License

[GPL-3.0-only](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
