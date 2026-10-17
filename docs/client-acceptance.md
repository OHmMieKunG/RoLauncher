# Live Roblox acceptance

## Prerequisites

The live harness is an ignored Windows Rust test, excluded from release executables.
It refuses non-loopback APIs, duplicate account IDs, running accounts, mismatched places and unavailable selected providers.
It never imports credentials, changes protocol registrations or falls back to another provider.

1. Start RoLauncher with a fresh test directory and port: `RoLauncher.exe --data-dir C:\RoLauncher\data\acceptance --port 38472`.
2. Sign in through the existing UI using exactly two dedicated accounts; configure the authorized destination and explicit providers, then leave both stopped.
3. Install the observer through Studio in an owned test experience; never use an executor or inject scripts into a player.
4. Copy `scripts/acceptance/config.example.json` into an ignored local directory and replace its IDs, place, output and authorization flag.
5. Keep the desktop unlocked and on AC power; close unrelated graphics workloads and record resolution, graphics/FPS settings, power mode and foreground behavior.

Use a fresh absolute output path under an existing parent for every run.
Its final folder name is the observer RunId: 1â€“64 ASCII letters, digits, hyphens or underscores.
Do not change account settings while a run is active.

## Run and collect

```powershell
# Inventory only; no accounts or experience needed. Use a fresh output directory.
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/client-acceptance.ps1 -PreflightOnly -OutputDirectory target\preflight-new

# Prompts privately for the isolated launcher's API token, never a Roblox cookie.
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/client-acceptance.ps1 -ConfigPath target\acceptance-config.json

python scripts/client-report.py --journal target\fishstrap-switches-001\journal.jsonl --server-export target\server-export.json --output target\switches-report.json
```

The wrapper verifies the official PresentMon 2.6.0 x64 console SHA256 when `presentmon` is configured.
Download that executable from its [official release](https://github.com/GameTechDev/PresentMon/releases/tag/v2.6.0); no service, overlay or client modification is needed.
Expected SHA256: `b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af`.

| Scenario | Behavior |
| --- | --- |
| `switches` | Ten hide/restore round trips per account; peer identity/settings/window remain unchanged. |
| `benchmark` | Six baseline conditions plus EcoQoS-off and timer-policy experiments, three rotated blocks per account; 30-second settling and 60-second measurements. |
| `occlusion` | Visible-before, opaque external cover, visible-after; exploratory presentation/GPU comparison. |
| `isolation` | Different modes/profiles; Restart, Stop and Start the first account while checking its peer. |
| `recovery` | Terminate only the verified first managed process; verify replacement, selected provider and Headless reapplication. |
| `server_disconnect` | Wait for the owner to disconnect the first account using ordinary server controls; verify recovery. |
| `idle25` | Both clients hidden; Balanced/Low Resource respectively; recovery off for 1,500 seconds. |
| `idle60` | Same profiles; recovery on for 3,600 seconds; connections remain separate. |

Each benchmark block waits for a displayed checkpoint filename.
Arrange both windows visibly, interact normally in **both** accounts, then create that empty checkpoint file.
This manual input is excluded from idle observations and keeps the peer from timing out during a long benchmark batch.

Pure minimized/hidden conditions use Normal/Default with test-only window controls; Headless is measured separately with automatic efficiency enabled.
Raw samples include power-policy masks, memory priority, iconic state, private allocation and cumulative page faults.
Transitions verify exact power/memory-policy restoration as well as priority and affinity.

The harness starts only its selected stopped accounts, disables experimental activity, and stops them before restoring saved preferences.
Ctrl+C requests cleanup; hard termination cannot run it, so inspect both dedicated accounts afterward.
Ownership failures remain explicit; unavailable measurements during recovery remain missing.

## Studio observer

1. Install `observer.server.luau` in ServerScriptService and `observer.client.luau` in StarterPlayerScripts.
2. Set server-script attributes `RunId`, `PlaceId`, `FirstAccountId` and `SecondAccountId`; the RunId must match the output folder.
3. Publish only to the authorized test experience and start a fresh server for each run configuration.
4. Record every printed `runId:jobId` export key, including replacement servers.
5. After stopping the run, fill those keys into `export-observer.luau` and execute it in Studio's server Command Bar with authorized DataStore access.
6. Copy the selected StringValue's JSON Value into a local UTF-8 file; remove the export instance before publishing.

The observer stores bounded joins, departures, connection nonces and ten-second progress/acknowledgement samples.
It uses ordinary game networking without generating input or overriding idle behavior.
DataStore failures, late saves, absent exports or progress gaps leave evidence incomplete; see [Roblox DataStores](https://create.roblox.com/docs/cloud-services/data-stores).
Exports must contain a server observation watermark covering the measured interval; stale snapshots cannot establish an uninterrupted session.

## Provider batches and acceptance

Run Official, Bloxstrap and Fishstrap independently, then same-provider pairs and Official+Bloxstrap / Official+Fishstrap; swap account roles.
Only the user installs/registers providers between stopped batches; restore the original Fishstrap handlers afterward.
Concurrent Bloxstrap+Fishstrap remains unsupported because both self-register the same protocols.

Pass a live round trip only after full identity continuity, unchanged server connection, observed restoration and manual responsive gameplay are all confirmed.
Journal/report completion alone remains UNVERIFIED; it cannot certify gameplay, capture quality or zero rendering.
Review actual player versions, both GPU adapters, missing/lost telemetry, first displayed frame and window-state polling resolution before accepting measurements.

Raw journals, traces, server exports and local configuration stay in ignored directories.
Share only reviewed summaries; never package raw authenticated logs, cookies, tickets, launch URLs or API tokens.

## Activity and policy experiments

- `activity25`: 25 minutes with Headless activity enabled and recovery disabled.
- `activity60`: 60 minutes with Headless activity enabled and recovery disabled.
- Existing `idle25`/`idle60` remain activity-disabled baseline/recovery observations.
- `benchmark` adds test-only `headless_ecoqos_off` and `headless_ignore_timer` conditions; both restore the exact captured power policy.
- PresentMon writes selected and `-peer` captures; record both clients' actual 120 FPS/quality 10 settings before starting.

Activity runs require an unlocked desktop left idle, the updated Studio observer, complete-duration server exports and input-event counts.
The client observer only counts input/idle notifications; it never generates activity or resets the idle timer.
Manual activity during an activity run invalidates attribution; Windows submission alone is insufficient.
A complete report remains UNVERIFIED until independent gameplay/capture-quality review; no fixture can establish a Roblox pass.
