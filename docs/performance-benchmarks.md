# Performance benchmarks

## Current evidence

| Measurement | Result |
| --- | --- |
| Live Normal versus Minimized versus Hidden CPU/RAM/GPU | **BLOCKED / not measured**; no authorized disposable account/experience supplied. |
| Roblox launch and restoration latency | Not measured. |
| Two-account connection reliability | Not measured. |
| 25-minute and 60-minute idle observations | Not run. |
| Native Windows control fixture | Hide/restore and priority restoration passed; this is not a Roblox benchmark. |

Do not infer rendering suppression from a hidden window, a zero GPU sample or missing presentation events.
Roblox documents [graphics quality settings](https://en.help.roblox.com/hc/en-us/articles/203314310-Graphics-Quality), which remain under user control.
No resource-savings percentage is claimed.

Headless now requests Low Resource automatically: minimize then hide, Below Normal priority, EcoQoS and low memory priority.
[EcoQoS](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setprocessinformation) targets power efficiency; [memory priority](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-memory_priority_information) influences reclamation under pressure.
These controls do not free private allocation or disable rendering; no forced working-set clearing, hard CPU cap or suspension is used.

## Metrics and cost

- CPU uses deltas of process kernel/user time divided by elapsed time and active logical processors; values are whole-machine normalized.
- RAM reports working set and private allocation separately; page-fault rate includes soft faults.
- GPU uses one shared WMI engine query per two-second monitoring cycle and reports each process's busiest engine.
- Per-process GPU engines can overlap; total device GPU usage is unavailable rather than an invalid sum.
- Disabled monitoring skips account sampling; absent, first CPU, denied and stale measurements remain unavailable.
- Aggregate CPU/RAM reports the measured subset and its count; it never estimates missing clients.
- Process control runs outside the application state lock; process-image installation roots are cached.

## Reproducible live procedure

1. Supply authorized disposable accounts and an owned test experience with server-side join/leave and progress observations.
2. Record OS, CPU/GPU, graphics/FPS settings, provider/client versions, place, verified process identity and account generation.
3. Use six separate conditions: visible Normal/Default, pure Minimized/Default, pure Hidden/Default, optimized Headless/Default, visible Balanced, and minimized Low Resource.
4. Settle for 30 seconds and measure for 60 seconds; run three rotated blocks with ordinary manual interaction in both accounts between blocks.
5. Capture native Roblox CPU/working/private memory at one-second intervals; preserve GPU engine/adapter identities and PresentMon frame data separately.
6. Restore Normal/Default and verify full process identity, original priority/affinity/EcoQoS/memory priority, window state and independent server connection continuity.
7. Repeat provider/pair batches and recovery tests; observe 25 minutes without recovery and 60 minutes with recovery, with experimental activity disabled.

Use the [live acceptance harness](client-acceptance.md) and [current compatibility report](client-compatibility-report.md).
The older `scripts/benchmark.ps1` is a generic PID-only CPU/memory sampler; it does not establish ownership, GPU/render activity or connection continuity.
Keep API bearer tokens in memory and omit authenticated URIs, tickets, cookies and raw logs from artifacts.

See [architecture](headless-architecture.md), [session reliability](session-reliability.md) and [existing performance notes](PERFORMANCE.md).

## 1.8.1 isolated experiments

Normal peers keep their existing 120 FPS cap and graphics quality 10; do not lower shared settings during these runs.
Add `headless_ecoqos_off` and `headless_ignore_timer` to the six baseline conditions, with three rotated blocks and manual interaction between blocks.
Both selected and peer processes receive separate PresentMon captures; Normal priority, power policy and memory priority are checked against their captured originals.

The timer experiment sets only `PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION` for the verified selected process and restores the captured masks.
It remains test-only; denying the setter or failing restoration invalidates the measurement.
Neither experiment is a 1 FPS limiter or a renderer-off mechanism.

Report each block's CPU, working/private memory, page faults, GPU engines, frame evidence and restore latency; include the Normal peer's frame capture.
Do not promote a candidate based on missing frame events, lower working set alone, interrupted simulation or a replacement connection.
All additional Roblox measurements remain BLOCKED pending dedicated accounts and an authorized experience.
