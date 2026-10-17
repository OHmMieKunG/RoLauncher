# Development verification

Updated 2026-10-17. Baseline: published upstream stable 1.7.0; target: 1.8.1 development.
This is a development extension with fixture validation, not a completed live Roblox acceptance run.

## Executed checks

| Check | Result |
| --- | --- |
| Full Rust tests | 88 passed; two interactive tests excluded by default and executed separately. |
| Native Windows fixture | Passed minimize, hide/restore, original priority, process sampling and lock/sleep handling. |
| Rust/WinUI bridge | Passed inherited-pipe bootstrap, authenticated real Rust API and clean shell exit. |
| Shared desktop tests | 95 assertions passed on .NET 8. |
| WinUI native build and smoke | Passed; covers per-account mode/provider/profile persistence and independent account defaults. |
| Clippy / formatting | Strict all-target lint and formatting passed; final test additions are checked again before delivery. |
| Offline Windows adapter | Type check passed with stubbed dependencies; full application tests above provide stronger evidence. |
| Version and release consistency | Passed manifest/lockfile/README/notes/changelog alignment and development-identity tests. |
| Publication fixtures | Six mocked tests passed; nothing published. |
| Linux updater archive tests | Three passed on Windows. |
| Installer fixtures | Passed rejection, cancellation, exit waiting, complete installation, restart and data preservation. |
| Linux Sober isolation | Could not execute on Windows: requires POSIX execution, a C compiler and Flatpak. |

Rust 1.98.0 and .NET SDK 8.0.426 were used; the existing guide recommends Rust 1.99.0.
Earlier missing-package, .NET 8 and native-build blockers were resolved in this environment.
Inno Setup 6.7.3 was publisher-signature verified and installed only under ignored `.tools/inno` for packaging.

## Delivery and remaining gates

`scripts/package.ps1 -UseCurrentVersion -OutputRootDirectory dist/client-manager` creates the development executable, loader, native desktop, notices, corresponding source, installer and SHA256 checksums.
Verify with `python scripts/verify-package.py --dist-directory dist/client-manager --smoke-shell` before using those outputs.
Published stable artifacts remain unchanged; the new development files are not a GitHub release.

Live Official/Bloxstrap/Fishstrap joins, independent two-account gameplay, forced recovery, resource comparisons and 25/60-minute idle observations remain unperformed.
No disposable account/authorized experience pair was supplied, and no existing user-managed sessions were controlled for those tests.
See [feature status](headless-architecture.md#delivery-status) and [benchmark limits](performance-benchmarks.md).
