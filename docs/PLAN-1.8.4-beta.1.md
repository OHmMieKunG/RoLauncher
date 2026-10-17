# RoLauncher 1.8.4-beta.1 — Windows stabilization candidate

## 1. Baseline and agreed scope

Continue from clean HEAD `486f619`. [Windows CI run 39045909632 passed](https://github.com/OHmMieKunG/RoLauncher/actions/runs/39045909632), including Branch flow and Windows checks.

- Stabilize existing controllers, diagnostics and benchmark tooling; defer the in-app benchmark wizard.
- Keep Windows-only support and the current Headless Low Resource policy. Balanced Headless measurements remain isolated harness experiments.
- Target public version **1.8.4-beta.1**; local development builds retain fresh `1.8.4-dev.<12 hex>` identities.
- Preserve Normal graphics settings, existing account data and bootstrapper selections.
- Dedicated accounts and an authorized experience remain unavailable unless supplied later. Do not control existing personal sessions.

Audit findings to address: restoration baselines exist only in memory; Normal can be reported during incomplete restoration; monitoring performs repeated WMI ownership queries; diagnostics omit current control details; successful recoveries lack a separate counter; publishing remains bound to upstream.

## 2. Process control, restoration and monitoring

- Add regression fixtures before changing restoration behavior. Cover recreated windows, denied priority/power restoration and rapid mode changes.
- Re-enumerate a recreated window before attempting Normal restoration. Report Normal as applied only after the required window and process settings are verified.
- Persist a validated, versioned restoration journal atomically before changing captured settings. Keep it separate from account preferences and authentication data.
- Recover journal entries only after verifying account ownership, generation, PID, creation time, tracker and Windows session. Remove entries after verified restoration or confirmed process exit.
- Treat missing or corrupt legacy baselines as unavailable; report the limitation without guessing original settings or automatically restarting a client.
- Reuse verified process handles while ownership remains unchanged. Move shared GPU collection into one bounded background collector so slow WMI queries cannot stall ongoing window transitions.
- Preserve two-second sampling and nominal 100 ms transition/cancellation checks. Reject stale telemetry and maintain explicit unavailable values.
- Add per-process dedicated GPU-memory measurements where supported, retaining adapter attribution and avoiding invalid device-wide sums.

Keep existing reversible defaults. Do not introduce suspension, forced working-set trimming, hard CPU caps, shared graphics changes or driver profiles. Microsoft documents that [EcoQoS controls scheduling/power policy](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setprocessinformation); it does not establish rendering suppression.

## 3. Reliability, diagnostics and benchmark evidence

- Add backward-compatible account fields for successful recoveries and the last disconnect reason/code. Increment successful recoveries once when a managed recovery attempt reaches a verified connection; exclude duplicate signals and manual launches.
- Preserve existing backoff, retry limits, cancellation, provider reuse and Headless reapplication.
- Display connection uptime, process uptime, managed-run duration, recovery attempts and successful recoveries separately.
- Extend the existing authenticated diagnostics endpoint and export button with requested/effective/applied controls, restoration state, fresh metrics, environment versions and bounded failure history.
- Use explicit export allowlists. Exclude credentials, authenticated URLs, private server destinations, trackers, raw logs and user-specific paths. Collect environment information outside application locks.
- Add nullable dedicated-GPU-memory and sampling timestamps to runtime DTOs; preserve existing API fields and deserialize older settings with defaults.

Extend the existing harness rather than replacing it:

- Retain Normal, pure Minimized, pure Hidden and optimized Headless conditions; add a clearly identified test-only hidden Balanced comparison.
- Keep three rotated blocks, 30-second settling and 60-second measurement windows. Preserve the existing mean, peak, median and p95 reporting.
- Record effective policies, both clients’ presentation evidence, connection intervals, restore latency and missing telemetry. Compare equal workloads only.
- Extend acceptance coverage for different-game isolation, launcher restart and restoration after interrupted supervision.
- Document single-account checks separately from two-account batches.
- Keep idle activity opt-in and experimental. Perform 25/60-minute baseline, activity and recovery observations separately; [Roblox documents idle disconnection after at least 20 minutes](https://create.roblox.com/docs/reference/engine/classes/Player).
- Retain Network-only, external per-process 1 FPS and renderer-off as Unsupported unless supported functionality is demonstrated.

## 4. Beta identity and fork publishing preparation

- Extend existing Rust, C# and script version handling to accept `MAJOR.MINOR.PATCH-beta.N`.
- Ensure beta numbering sorts numerically, stable supersedes its matching beta, published beta/stable versions can replace matching local dev builds, and beta discovery remains opt-in.
- Preserve recognition of historical `beta-v<version>` releases. Use `v1.8.4-beta.1` for the new candidate.
- Carry the full beta identity through runtime strings, installer labels, filenames and source archives; retain numeric Windows metadata `1.8.4.0`.
- Consolidate unpublished notes under numeric target 1.8.4 and update the repository convention for public beta identities.

Retarget publishing consistently to fork repository ID `1422574717` and owner ID `180964824`: workflow guards, API/upload allowlists, owner policy, fixtures and documentation.

Future beta provenance must reference reviewed fork `main` and successful checks for that exact commit. Preserve owner authorization, artifact validation and attended evidence protections. Leave publication disabled, keys/inbox unenrolled, and stable publication blocked; create no tags or releases during this work.

## 5. Verification and release decisions

Run the Windows suite, strict Clippy, formatting, native fixtures, authenticated UI bridge, installer tests and package verification. Add focused tests for:

- Crash-journal persistence, stale identities, recreated windows and incomplete restoration.
- Slow collectors, stale GPU samples and unsupported GPU-memory counters.
- Recovery-counter deduplication, migration and per-account isolation.
- Diagnostic secret redaction.
- Beta ordering, stable-over-beta updates, metadata and legacy release compatibility.
- Fork publication boundaries, upstream rejection and disabled publishing.

Package the executable, loader, native desktop, notices, corresponding source and SHA256 checksums. Verify exact source correspondence and isolated production-shell startup, then push verified code to the fork’s `main`.

Update the compatibility report and classify each feature as **IMPLEMENTED / TESTED / EXPERIMENTAL / UNSUPPORTED / BLOCKED**.

- **Public beta GO:** automated/package gates pass and two authorized live accounts demonstrate launch, repeated switching, restoration, unchanged peer settings and connection continuity.
- **Stable GO:** additionally complete the claimed provider matrix, live performance comparisons, recovery tests and 25/60-minute observations without critical unresolved regressions.
- Missing live prerequisites permit a local candidate, but mean **NO-GO for public beta and stable publication** under the selected release gate.

No resource savings, rendering suppression or uninterrupted uptime will be claimed without executed measurements.
