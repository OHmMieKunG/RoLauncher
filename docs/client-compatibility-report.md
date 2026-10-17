# Client compatibility and performance report

Development build: **1.8.3-dev.3a1e4940f9af**, 2026-10-17.
Integrated upstream main: `193c465`; GitHub release publication remains disabled.
Source/build/script fingerprint: `69311b58fab251c02f42f1ee82cbe8c018ec44793c65c1bbf8a5d22713aa5b6d` (102 files).

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
| Active processes at preflight | Three Roblox players and one launcher; ownership, game progress and mode unverified |
| Presentation tooling | Pinned official PresentMon 2.6.0; no live capture performed |
| Observer tooling | Three scripts compiled with official Luau 0.743; no Studio execution |

The recorded preflight only inspected installations and process metadata.
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

Routine full-state reads now follow two-second samples instead of every 100 ms; window/input cancellation checks retain the faster cadence.
This removes redundant launcher state copies; no Roblox savings or CPU percentage is inferred.

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
- 107 desktop assertions, native WinUI smoke and authenticated Rust/UI bridge passed.
- Six report, three archive and seven mocked beta-publication fixtures passed.
- Upstream safeguards passed 30 promotion, 14 hotfix and 25 owner-evidence fixtures; nothing was published.
- Native regular/semibold and executable-input font checks passed; inspected normal/small-window screenshots show corrected punctuation and readable labels.
- Installer/update fixture passed checksum rejection, cancellation, exit handles, complete installation, restart and data preservation.
- Strict all-target Clippy, formatting, release metadata and offline adapter checks passed.
- Updated observer scripts compiled; Roblox input acceptance and DataStore behavior were not tested.

Tests ran with the normal Windows user token for DPAPI and native UI access.
Credential protection was not weakened.
Windows 10/11 x64 is the only supported platform; Linux/Sober source adapters, desktop, CI, packaging and updates are removed.
The production build script rejects non-Windows targets; remaining dependency versions are unchanged.
The UI fixture initially sampled before its first compositor tick; it passed after adding a bounded progress wait.

See [acceptance workflow](client-acceptance.md), [resource measurements](performance-benchmarks.md), [session reliability](session-reliability.md) and [rendering research](NO-RENDER-RESEARCH.md).
Package integrity, exact source correspondence and packaged-shell startup are checked after packaging; local results are recorded in `target/client-manager-checks/verification.md`.
