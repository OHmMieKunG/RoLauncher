# Client compatibility and performance report

Development build: **1.8.1-dev.4f6ebf3c5023**, 2026-10-17.
HEAD: `4daae1bfc14db1336e1366134367ccb16efb7d4d`; existing uncommitted work preserved.
Source/build/script fingerprint: `7fbc1a5968daf1c3ffd39c991926a7ea62fcef48df2e9ae2cd6781d06b21268a` (113 files).

## Read-only host inventory

| Item | Observed value |
| --- | --- |
| Windows | Windows 11 Pro, 10.0.26300.9457, 64-bit |
| CPU | AMD Ryzen 9 5900HS, 16 logical processors |
| Integrated GPU | AMD Radeon Graphics, driver 31.0.21925.1001 |
| Discrete GPU | NVIDIA GeForce RTX 3050 Ti Laptop GPU, driver 32.0.16.1742 |
| Fishstrap | 3.1.2; both Roblox protocols point to Fishstrap.exe |
| Roblox player found | Fishstrap distribution, file version 0.743.0.7431134 |
| Official / Bloxstrap | Not found in inspected standard roots; launches not attempted |
| Active processes at preflight | One Roblox player and one launcher; ownership, game progress and mode unverified |
| Presentation tooling | Pinned official PresentMon 2.6.0; no live capture performed |
| Observer tooling | Three scripts compiled with official Luau 0.743; no Studio execution |

The fresh preflight only inspected installations and process metadata.
No live Roblox process was changed, launched, rejoined or terminated.
Dedicated test accounts and an authorized experience remain unavailable.

## Feature status

| Feature | Status and evidence |
| --- | --- |
| Per-account Normal/Headless controls | IMPLEMENTED; existing reversible policies and native fixtures TESTED |
| Normal graphics preservation | IMPLEMENTED; shared graphics files and driver profiles are never edited |
| Private allocation / page-fault telemetry | IMPLEMENTED / TESTED with native fixture; faults include soft faults |
| Headless idle activity | IMPLEMENTED / EXPERIMENTAL; default off, separate opt-in; cancellation/restoration tested without keyboard submission |
| Timer-resolution / EcoQoS-off comparisons | IMPLEMENTED in test harness only; native policy restoration TESTED; Roblox results BLOCKED |
| Connection and recovery accounting | IMPLEMENTED / TESTED with fixtures; live continuous uptime BLOCKED |
| Provider matrix and simultaneous clients | BLOCKED pending dedicated accounts, experience and providers |
| Per-process 1 FPS / rendering-off / Network-only | UNSUPPORTED under the established permitted interfaces |

Headless activity temporarily shows the window only after 60 seconds without desktop input.
Busy desktops and denied focus can defer activity beyond Roblox's idle timeout; no prevention guarantee is made.
Normal accounts retain their existing 120 FPS/quality 10 settings; reduced Headless GPU or RAM use has not been measured.

## Roblox measurements

| Condition | CPU / working/private RAM / GPU / frames / restore latency |
| --- | --- |
| Visible Normal/Default | BLOCKED / not measured |
| Pure Minimized/Default | BLOCKED / not measured |
| Pure Hidden/Default | BLOCKED / not measured |
| Optimized Headless | BLOCKED / not measured |
| Visible Balanced | BLOCKED / not measured |
| Minimized Low Resource | BLOCKED / not measured |
| Headless EcoQoS disabled | BLOCKED / not measured |
| Headless ignore-timer-resolution | BLOCKED / not measured |
| External opaque occlusion | BLOCKED / not measured |

All provider launches, two-account comparisons, restoration latency and 25/60-minute idle/activity observations remain BLOCKED.
No launcher-only or uncontrolled process snapshot is used as a Roblox savings comparison.
Zero presentation events alone cannot establish rendering suppression.

## Executed checks

- 98 regular Rust tests passed; six tests are explicitly ignored by default.
- Three isolated native Windows fixtures passed; they do not certify Roblox behavior.
- 104 desktop assertions, native WinUI smoke and authenticated Rust/UI bridge passed.
- Six report, three archive and six mocked publication fixtures passed.
- Installer/update fixture passed checksum rejection, cancellation, exit handles, complete installation, restart and data preservation.
- Strict all-target Clippy, formatting, release metadata and offline adapter checks passed.
- Updated observer scripts compiled; Roblox input acceptance and DataStore behavior were not tested.

The sandboxed Rust run had 32 DPAPI encryption failures (Windows error 2); the isolated suite passed with the normal Windows user token.
Credential protection was not weakened.
Linux runtime acceptance was not executed in this Windows environment.

See [acceptance workflow](client-acceptance.md), [resource measurements](performance-benchmarks.md), [session reliability](session-reliability.md) and [rendering research](NO-RENDER-RESEARCH.md).
Package integrity, exact source correspondence and packaged-shell startup are checked after packaging; local results are recorded in `target/client-manager-checks/verification.md`.
