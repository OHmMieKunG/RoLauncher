# Client management architecture

Implemented in the development tree; live Roblox validation is pending.

## Audit and dependency plan

- Rust 2024, Tokio/Axum local API, DPAPI persistence, native WinUI 3/.NET 8 desktop.
- Account targets already cover place, job and private-server destinations; presets and encrypted backups preserve account preferences.
- Launch ownership uses browser tracker, PID, creation time and account generation; log ownership establishes connection state.
- Existing serialized launches, exit watches, disconnect grace and bounded exponential retries remain the session supervisor.
- Extend account preferences through patches, bulk edits, presets, backups, API and desktop models; keep applied state in runtime snapshots.
- Extend the existing Windows automation worker for reversible mode/profile controls and shared resource sampling; perform Windows calls outside the application state lock.
- Route existing authenticated launch URIs through a provider; retain the installed protocol default for migrated accounts.
- Reuse existing Win32 bindings and desktop controls; no new runtime dependencies.

## Boundaries

Client Headless is reversible background window management, not rendering-off.
The launcher `--headless` flag still means running without its desktop UI.
Network-only remains unsupported unless a documented player-session interface is established.

No process-memory writes, injection, client patching, suspension or graphics-preference edits are permitted.
Authentication URLs, cookies and tickets must never appear in diagnostics.

## Lifecycle and restoration

The existing session state machine remains authoritative: Stopped, Queued, Launching, Unknown, Running, Reconnecting, Backoff and NeedsAttention.
Headless is a separate observed control mode; it does not manufacture a Connected session state.
Mode/profile requests persist immediately and are reconciled on the two-second sampling cycle.
Pending window transitions advance on the existing 100 ms worker loop, outside application locks, with a ten-second deadline.
A duplicated verified process handle pins identity during the transition; account generation/preferences are rechecked without repeated WMI queries.

Before controls, the worker verifies process image, tracker, PID, creation time, account generation and Windows session.
It clones current account preferences under the state mutex and releases it before blocking Windows calls.
Results are published only if the same account still owns that identity.

The controller enumerates a unique visible unowned top-level window belonging to the verified process and retains its handle while hidden.
It captures priority, affinity, EcoQoS policy, memory priority and full `WINDOWPLACEMENT` before changing each setting.
It retains those originals across profile changes and failed restoration, observes asynchronous window transitions, and discovers recreated visible windows.
Ambiguous/missing windows and permission failures remain explicit errors; no title matching is used.

Normal/Default restores captured window placement and process settings without relaunching.
Balanced requests Below Normal priority. Low Resource also requests EcoQoS execution-speed control, low memory priority and minimization.
Headless applies Low Resource automatically without changing the saved profile: minimize first, observe minimization, then hide.
An already lower CPU/memory priority is preserved; unrelated power-policy bits and timer-resolution settings are retained.

Normal auto-hide keeps its existing hide-only behavior unless Low Resource is selected.
Denied controls produce partial-optimization status; observed Headless does not mean every efficiency control succeeded.
Controls whose original values cannot be read are skipped; no guessed restoration defaults are written.
Restore actions select Normal/Default and clear auto-hide while preserving an explicitly selected affinity.

Dead or retired identities are cleaned separately; graceful shutdown retries restoration and records any errors in `control-cleanup.json`.
Forced launcher termination cannot execute cleanup, and original settings are not persisted across a crash; use Task Manager or restart the affected managed client if needed.
Neither visibility nor scheduling changes disable rendering or reduce memory allocation by definition.

## Usage and build

1. Open an account in the existing Instance Manager; configure its provider, client mode, resource profile and monitoring/startup preferences.
2. Save, then Start; existing place/job/private destinations and automatic-rejoin preferences still apply.
3. Use Hide/Restore for live changes, or the account editor for independent preferences; read requested and observed modes separately.
4. Use Launch/Stop/Hide/Restore All for saved accounts; Edit selected applies resource profiles through the existing atomic bulk editor.

Use [Development](DEVELOPMENT.md) for prerequisites and `scripts/build.ps1` / `scripts/test.ps1`.
`scripts/package.ps1` creates a fresh development identity, executable, loader, desktop, licenses, source, installer and checksums.
This fork supports Windows 10/11 x64 only; Linux/Sober is unsupported.

## Delivery status

| Feature | Status | Evidence / limit |
| --- | --- | --- |
| Per-account preferences and migration | IMPLEMENTED / TESTED | Rust persistence/isolation/preset tests and desktop serialization tests. |
| Normal/Headless and live restoration | IMPLEMENTED / TESTED at OS-fixture level | Isolated Windows window passed; live Roblox acceptance remains EXPERIMENTAL. |
| Multi-instance controls | IMPLEMENTED / TESTED with fixtures | Existing verified launch/restart/stop plus mode requests and batch controls; simultaneous Roblox gameplay untested. |
| Official/provider routing | IMPLEMENTED / EXPERIMENTAL | Source-verified URI construction and executable validation; live provider launch matrix pending. |
| Concurrent Bloxstrap plus Fishstrap | UNSUPPORTED | Competing protocol registration is rejected. |
| Default/Balanced/Low Resource | IMPLEMENTED / TESTED at OS-fixture level | Reversible priority/EcoQoS/memory/window controls; Roblox savings unmeasured. |
| Connection recovery | IMPLEMENTED / TESTED with simulated sessions | Existing owned-log state machine/backoff plus provider reuse and mode reconciliation. |
| CPU/RAM/GPU/process uptime | IMPLEMENTED / TESTED with fixtures | GPU availability is driver-dependent; total GPU remains unavailable. |
| Native WinUI manager | IMPLEMENTED / TESTED | Native build, editor/model tests, smoke fixture and authenticated Rust/UI bridge. |
| Idle activity experiment | EXPERIMENTAL | Separate Headless opt-in, idle-only foreground input and bounded retries; Roblox acceptance and idle prevention unverified. |
| Rendering-off and network-only | UNSUPPORTED | No supported implementation established; no substitute backend. |
| Live provider matrix, 25/60-minute idle checks and Roblox benchmarks | BLOCKED | Requires authorized disposable accounts and test experience. |

See [network feasibility](network-only-feasibility.md), [providers](bootstrapper-integration.md), [reliability](session-reliability.md) and [measurements](performance-benchmarks.md).

The [acceptance harness](client-acceptance.md) is compiled only for explicit Windows tests; it adds no production API or persisted settings.
It reuses verified process controls and the local API, separates six baseline benchmark conditions, and requires independent server evidence before any live pass.
[Current host and blocked cases](client-compatibility-report.md) remain separate from automated fixture results.

## Activity and measurement

Headless foreground activity is separately opt-in and uses the existing 100 ms worker, verified process handle and asynchronous window restoration.
The worker serializes activity, observes temporary visibility, rechecks cancellation before input, and keeps captured process/window originals intact.
Normal accounts retain 120 FPS/quality 10 when those are their saved settings; RoLauncher never changes shared graphics files or driver profiles.

Runtime memory sampling includes private allocation and page-fault rate; page faults include soft faults.
Timer-resolution policy and EcoQoS-off comparisons exist only in the explicit test harness, with exact policy restoration and peer frame capture.
No additional production throttling or external per-process 1 FPS/rendering-off control is claimed.
