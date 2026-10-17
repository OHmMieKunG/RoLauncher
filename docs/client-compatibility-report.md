# Client compatibility and performance report

Local candidate: **1.8.4-dev.a7c40bd57487**; public target: **1.8.4-beta.1**, upcoming.
Baseline: `486f619`; Windows 10/11 x64 only.
Publication remains disabled; **public beta NO-GO / stable NO-GO** until authorized live evidence is complete.

## Feature status

| Feature | Classification | Evidence and limit |
| --- | --- | --- |
| Versioned restoration journal | IMPLEMENTED / TESTED | Atomic persistence, stale identity rejection, interrupted native supervision and write-failure fixtures. |
| Recreated windows and Normal restoration | IMPLEMENTED / TESTED | Native window recreation, denied process restoration and rapid switching preserve baselines. |
| Missing legacy baselines | IMPLEMENTED / TESTED | Existing clients stay unavailable; no guessed settings or automatic restart. |
| Verified process handles | IMPLEMENTED / TESTED | Ownership is verified before attachment; cached handles retain creation-time/session and account checks. |
| Shared bounded GPU collector | IMPLEMENTED / TESTED | Nonblocking transition path; stale and unavailable samples remain missing. |
| Dedicated GPU memory | IMPLEMENTED / TESTED / BLOCKED | DTO and collector fixtures pass; live supported counters unavailable; adapter values remain separate. |
| Recovery attempts and successes | IMPLEMENTED / TESTED | Durable defaults, generation checks, deduplication, manual-launch exclusion and per-account isolation. |
| Authenticated diagnostics export | IMPLEMENTED / TESTED | Explicit fields, fresh metrics and bounded history; secrets, destinations, logs, trackers and paths excluded. |
| Numbered beta identities | IMPLEMENTED / TESTED | Rust/C# ordering, legacy recognition and beta opt-in; fixture Windows numeric metadata is 1.8.4.0. |
| Fork publishing preparation | IMPLEMENTED / TESTED / BLOCKED | Fork IDs and upstream rejection; reviewed-main checks; disabled workflow and unenrolled owner policy. |
| Hidden Balanced comparison | IMPLEMENTED / EXPERIMENTAL / BLOCKED | Harness-only condition; production Headless remains Low Resource; no live measurements. |
| Launcher restart and different-game isolation | IMPLEMENTED / BLOCKED | Harness scenarios added; native interrupted-supervision fixture passes; two-account acceptance outstanding. |
| Idle activity | EXPERIMENTAL / BLOCKED | Default off and separately opted in; 25/60-minute baseline, activity and recovery observations outstanding. |
| Provider matrix and peer continuity | BLOCKED | Dedicated accounts, authorized experiences and all claimed providers unavailable. |
| Network-only, external per-process 1 FPS, renderer-off | UNSUPPORTED | No supported functionality established under the permitted interfaces. |

## Executed automated checks

- 103 regular Rust tests pass; six explicit live/UI fixtures remain ignored by default.
- Three isolated native Windows fixtures cover policies, occlusion, metrics, restoration and session events.
- 119 desktop assertions, native WinUI smoke and authenticated Rust/UI bridge pass.
- Seven report, three archive, eight beta-publication, 32 fork-promotion, 14 hotfix and 25 owner-evidence fixtures pass.
- Isolated Rust/C# beta metadata retains full 1.8.4-beta.1 strings and numeric Windows version 1.8.4.0.
- Strict all-target Clippy, formatting, real-dependency offline adapter checks and installer/update fixtures pass.

The initial sandbox run could not access Windows DPAPI; the suite passed with the normal user token outside the sandbox.
Credential protection was unchanged; no personal Roblox session was controlled.
These are fixture results, not Roblox performance or continuity measurements.

## Packaging and source correspondence

Artifacts carry the full development identity and include the executable, native desktop, WebView2 loader, notices, source and AGENTS.md.
SHA256 and exact source/archive correspondence are verified with `scripts/verify-package.py --smoke-shell`.
The final package-gate result is recorded separately in `target/stabilization-package-verification.log` and `target/stabilization-verification.md`.

The source/configuration fingerprint and its exact scope are recorded in [machine-readable evidence](client-compatibility-report.json).
The retained host inventory was observed for the 1.8.3 baseline; it is not a fresh inventory or a benchmark.
Normal graphics, account data and bootstrapper choices are preserved; no shared graphics files or driver profiles are edited.

## Unexecuted live gates

Public beta requires two authorized accounts to demonstrate launch, repeated switching, restoration, unchanged peer settings and connection continuity.
Stable additionally requires the claimed provider matrix, equal-workload measurements, recovery tests and separate 25/60-minute observations.
No resource savings, rendering suppression or uninterrupted uptime is claimed.

See [saved plan](PLAN-1.8.4-beta.1.md), [acceptance workflow](client-acceptance.md) and [performance evidence](performance-benchmarks.md).
