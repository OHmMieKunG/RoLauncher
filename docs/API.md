# Local API reference

## Development fork additions

- Account/bulk PATCH and preset entries accept `resource.mode` (`normal`, `balanced`, `low_resource`) and optional hexadecimal-string `affinity`; `anti_afk` accepts `enabled`, `action` (`space`, `camera`) and `headless_foreground_activity` (default false). Both booleans must be true for the Headless activity experiment. Defaults preserve original affinity and disable activity.
- `client` accepts `mode` (`normal`, `headless`), `bootstrapper` (`system`, `official`, `bloxstrap`, `fishstrap`, `custom`), optional absolute `executable`, `auto_hide`, `start_on_open` and `monitoring`. Omitted `client` preserves preferences; an object replaces that account's full client preferences with safe defaults for omitted members.
- Migrated accounts use Normal, the installed protocol, no auto-hide/startup and enabled monitoring. Settings survive presets/backups; changing one account leaves others intact.
- `network_only` requests are rejected explicitly; `network_only_reason` explains the limitation. No client launch substitutes for an unsupported request.
- `POST /v1/accounts/USER_ID/client/hide` and `/restore` save requests and return HTTP 202; runtime `client_mode` confirms actual application separately. Restore selects Normal/Default and clears auto-hide; neither action rejoins.
- `GET /v1/bootstrappers` reports availability, executable, discovered settings path and errors; detection does not launch providers or alter the registry.
- Detection checks registered executables, uninstall records and standard local folders. A detected executable with `available:false` cannot be launched; its reason explains the registration conflict.
- The UI labels `system` as Auto-detect and refreshes discovery when account details open; persisted explicit provider/path choices remain unchanged.
- Runtime entries add nullable `client_mode`, `process_uptime_seconds`, and observed `window_state`; missing, stale or unverified values must not be shown as applied modes.
- Status/events include `metrics` with active/connected counts, measured CPU/RAM and measured-client count. Aggregate GPU stays null because busiest per-process engines cannot be summed as device utilization.
- Accounts expose `last_disconnect` and cumulative `reconnection_attempts`; connection uptime, process uptime and managed uptime are distinct.
- `PATCH /v1/settings/anti-afk` accepts `{"paused":true}` or `{"paused":false}` with the existing authentication and origin rejection. Pause is runtime-only and affects scheduled input.
- Status/events add `anti_afk_paused`, `session_suspended`, `no_render_reason`, and `runtime_controls`, keyed by account ID. Entries contain process identity, sample time, nullable normalized CPU/working-set/private-allocation/page-fault-rate/busiest-engine GPU metrics, unavailable reasons, and separate control/input results. `roblox_acceptance` remains `unverified`.
- Telemetry is excluded from account databases, backups, and presets. Ten-minute input deadlines reset on pause, disable, generation changes, lock/sleep, and long sampling interruptions.
- The compatibility route `/v1/updates/official/check` now checks `OHmMieKunG/RoLauncher`; the desktop validates that publisher's exact release URLs and checksums.

[Implementation status and limits](headless-architecture.md#delivery-status).

## Access

- Base: `http://127.0.0.1:38471`.
- Get the token from **Tools â†’ Support â†’ API token**; send `Authorization: Bearer TOKEN` on every route.
- Browser Origin and cross-site requests are rejected. Endpoints never export account credentials; keep the token private.

## Game joining

- Status includes `server_selection` (`smart`, `lowest`, `highest`) and `join_type` (`friend`, `random`).
- Preferences apply on future starts/rejoins. Smart uses reported ping/population; region and local latency are unavailable.
- Selection scans at most 500 candidates within 30 seconds. Job IDs/private links take priority.
- Assign saved games by PATCHing `game_profile_id` with matching `target`; stale/mismatched assignments fail atomically.
- A custom target without an ID, or `clear_target`, clears the assignment. Omitted destinations preserve it.
- Presets/backups retain assignments. Friend groups explicitly assigned accounts with equal saved destinations.
- Presence comes from the joining account or managed peer's saved session and must match the place. Roblox enforces access/capacity.

## Routes

| Method | Route | Body / behavior |
| --- | --- | --- |
| PATCH | `/v1/settings/game` | `{ "server_selection": "smart" / "lowest" / "highest", "join_type": "friend" / "random" }`; both required, saved together, used on subsequent launches. |
| GET | `/v1/status` | Accounts, saved game profiles, close preference, connectivity suspension, compatibility status |
| POST | `/v1/login` | Open the isolated browser sign-in window; HTTP 202 means queued |
| GET | `/v1/accounts` | Safe account/status records |
| POST | `/v1/accounts` | `{"cookies":["SESSION_VALUE"]}`; per-index import errors |
| PATCH | `/v1/accounts/USER_ID` | Alias, target, `auto_recovery`, `group`, `fallback_policy`, or explicit `clear_target` |
| DELETE | `/v1/accounts/USER_ID` | Remove a stopped account with no pending owned instance |
| POST | `/v1/accounts/USER_ID/start` | Queue launch; return operation ID |
| POST | `/v1/accounts/USER_ID/stop` | Cancel pending recovery and close verified client |
| POST | `/v1/accounts/USER_ID/restart` | Replace the current verified client |
| GET | `/v1/operations/UUID` | Queued/completed/failed/cancelled operation |
| GET | `/v1/events` | SSE snapshots; reconnect receives current snapshot |
| PATCH | `/v1/accounts/bulk` | `account_ids` (1â€“50 distinct IDs), `patch`; atomic settings update, alias patterns |
| POST | `/v1/accounts/USER_ID/retry` | Advance an eligible retry; retain failure count and respect cooldowns |
| POST | `/v1/accounts/USER_ID/repair` | Queue isolated sign-in requiring the same account ID |
| GET / POST | `/v1/profiles` | List / save current settings for `name`, `account_ids`, optional existing `id` |
| POST | `/v1/game-profiles` | Create/update `{id,name,game_name,thumbnail_url,target}`; UUID identifies the game profile, up to 100 |
| DELETE | `/v1/game-profiles/UUID` | Delete saved game and clear assignments; retain account destinations |
| GET | `/v1/games/PLACE_ID` | Authenticated Roblox game name and optional thumbnail lookup using a saved session |
| PATCH | `/v1/settings/window` | Save `{ "close_to_tray": true }` or `false`; snapshots include `close_to_tray` |
| POST | `/v1/profiles/import` | JSON array of credential-free profile definitions; atomic import |
| DELETE | `/v1/profiles/UUID` | Remove saved profile |
| POST | `/v1/profiles/UUID/apply` or `/start` | Apply profile settings / apply then Start; per-account launch results |
| GET | `/v1/activity?account_id=USER_ID` | Persistent newest-first history; account filter optional |
| GET | `/v1/diagnostics` | Sanitized status/history report |
| GET / POST | `/v1/backups` | List encrypted backup filenames / create a backup |
| POST | `/v1/backups/FILENAME/restore` | Restore validated backup while all accounts are stopped |
| PATCH | `/v1/updates` | Legacy API discovery repository; blank uses OHmMieKunG/RoLauncher. Does not change the desktop publisher allowlist. |
| POST | `/v1/updates/check` | Discover the legacy configured public repository's release in the saved update channel |
| POST | `/v1/updates/official/check` | Compatibility endpoint: check OHmMieKunG/RoLauncher; used by automatic desktop checks |
| PATCH | `/v1/settings/updates` | Save `{"include_beta":true}` to include beta releases, or `false` for stable releases only (default). The preference appears as `include_beta_updates` in snapshots. |
| GET | `/v1/settings/discord` | Enabled/configured flags, recovery preference, delivery status, recent activity; no webhook URL |
| PATCH | `/v1/settings/discord` | Optional `enabled`, `notify_recovery`, `webhook_url`; empty URL removes it (also set `enabled:false`) |
| POST | `/v1/settings/discord/test` | Queue a test to the saved webhook, even with notifications off; HTTP 202 means queued |

## Records and operations

- Account IDs and Windows creation ticks are strings; PIDs are integers.
- Process records include PID, creation time, tracker, and generation.
- Operations are session-local, capped at 1,000 completed entries; desired account state persists.
- Target edits apply at the next launch. Start/Stop are idempotent; conflicting pending Restart is rejected.

```powershell
$headers = @{ Authorization = 'Bearer YOUR_TOKEN' }
Invoke-RestMethod 'http://127.0.0.1:38471/v1/accounts' -Headers $headers
$target = @{ target = @{ place_id = 1818 }; auto_recovery = $true } | ConvertTo-Json
Invoke-RestMethod 'http://127.0.0.1:38471/v1/accounts/USER_ID' -Method Patch -Headers $headers -ContentType 'application/json' -Body $target
Invoke-RestMethod 'http://127.0.0.1:38471/v1/accounts/USER_ID/start' -Method Post -Headers $headers
```

### Discord bot settings

- `PATCH /v1/settings/discord` accepts `bot`: `enabled`, `token`, `guild_id`, `allowed_users` (user ID strings).
- Omitted fields retain values; an empty token removes it. Enabling requires token, server ID, and a nonempty allowlist.
- GET returns configuration flags, guild/users, status, and `connection_state` (`online`, `starting`, `offline`); never the token.
- Save does not start a stopped bot. POST `/v1/settings/discord/bot/start`, `/stop`, or `/restart` controls its saved connection.
- Start/Restart persist startup intent. Stop retains credentials but clears that intent; disabling/removing also clears it.
- `/status-register channel` saves channel/message/page and refreshes changed data every 15 seconds.
- Re-registering reuses the message or replaces a deleted one. Commands are opt-in and use supervised Stop/Restart.
- The same loopback authentication and origin protections apply.

## Account timing

- `running_since`: UTC start of the managed run; null before confirmed connection or after Stop/manual Restart.
- Disconnects and automatic rejoins retain it.
- `longest_streak_seconds`: saved best elapsed run; display the maximum of this and the current run.
- Both fields survive app restarts.

[Back to README](../README.md)

### Automatic Headless efficiency

Headless requests Low Resource controls regardless of the saved `resource.mode`; preferences and wire fields are unchanged.
`runtime_controls.resource_result` reports the effective policy, observed EcoQoS/memory priority, pending window stage and partial failures.
`client_mode` reports Headless only after minimization and hiding are observed; this does not certify rendering suppression or all efficiency controls.
Normal/Default restores captured process/window settings. Normal auto-hide retains hide-only behavior.

Runtime entries add `private_bytes`, `page_faults_per_second`, `last_activity_attempt_at` and `last_input_submitted_at`.
Page faults include soft faults; timestamps distinguish attempts from Windows input submission and never certify Roblox acceptance.
