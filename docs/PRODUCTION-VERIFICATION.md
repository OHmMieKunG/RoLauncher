# Development verification

Updated 2026-10-17. Target: 1.8.3 development; published upstream stable: 1.7.0.
Upstream main `193c465` is integrated, including its disabled publisher and release safeguards.
This delivery has fixture validation; live Roblox acceptance remains blocked.

## Executed checks

| Check | Result |
| --- | --- |
| Rust suite | 98 passed; six ignored by default. |
| Native Windows fixtures | Three passed: reversible process policies, occlusion, window restoration, metrics and session/cancellation handling. |
| Rust/WinUI bridge | Authenticated real Rust API, inherited-pipe bootstrap and clean shell exit passed. |
| Shared desktop models | 107 assertions passed on .NET 8. |
| Native UI | Smoke passed, including provider/mode/profile isolation, native font metrics, executable input and narrow layouts. |
| Clippy / formatting | Strict all-target lint and formatting passed. |
| Offline adapter | Type check passed with stubbed dependencies; full Windows compilation also passed. |
| Version checks | Manifest, lockfile, README, consolidated notes and changelogs agree. |
| Report/archive/beta publication | Six, three and seven fixtures passed; no publication performed. |
| Upstream release safeguards | 30 promotion, 14 hotfix and 25 owner-evidence fixtures passed with synthetic evidence. |
| Installer/update | Isolated checksum, cancellation, exit handles, complete installation, restart and data preservation passed. |
| Windows-only support | Non-Windows build guard passed; only Windows application jobs remain in the three parsed workflows. |

Rust 1.98.0, .NET SDK 8.0.426 and Inno Setup 6.7.3 were used.
The upstream CI pins Rust 1.99.0; no compiler or dependency upgrade was introduced here.
See the [compatibility report](client-compatibility-report.md) for host versions, font validation and unsupported cases.

The initial UI run sampled before the compositor advanced; its animation assertion now waits for bounded progress, and the rerun passed.
All remaining dependency versions are preserved; 69 obsolete lock entries and the Linux-only source/build paths were removed.

## Packaging and limits

1. Build with `scripts/package.ps1 -UseCurrentVersion -OutputRootDirectory dist/client-manager` after stamping a fresh identity for a new build.
2. Verify with `python scripts/verify-package.py --dist-directory dist/client-manager --smoke-shell`.
3. Read local final results in `target/client-manager-checks/verification.md`; development artifacts are not GitHub releases.

Normal saved graphics remain 120 FPS and quality 10; shared graphics/bootstrapper settings were not edited.
Redundant launcher full-state reads were moved from 100 ms ticks to two-second samples; no Roblox resource-savings percentage is claimed.
No live Roblox process was controlled for acceptance, and no keyboard input was submitted by fixtures.

Provider joins, two-account gameplay, recovery, rendering/resource comparisons and 25/60-minute observations require dedicated accounts and an authorized experience.
Network-only, external per-process 1 FPS and rendering-off remain unsupported.
See [feature status](headless-architecture.md#delivery-status) and [measurement limits](performance-benchmarks.md).
