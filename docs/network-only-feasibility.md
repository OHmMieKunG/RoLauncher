# Network-only feasibility

**UNSUPPORTED — research reviewed 2026-10-17. No prototype is provided.**

## Findings

| Interface | Supported purpose | Player-session limitation |
| --- | --- | --- |
| [Open Cloud](https://create.roblox.com/docs/cloud) | Authorized access to creator resources | No standalone player-session transport identified. |
| [OAuth](https://create.roblox.com/docs/cloud/auth/oauth2-overview) | Grant scoped access to Open Cloud resources | An access token does not create a connected game player. |
| Installed `roblox-player` protocol | Pass an authenticated join request to an installed client | Requires the actual graphical Roblox client. |
| [Client-server runtime](https://github.com/Roblox/creator-docs/blob/main/content/en-us/projects/client-server.md) | Client data-model replication, scripts and physics simulation | The engine performs more than an authenticated HTTP join. |
| [Player](https://create.roblox.com/docs/reference/engine/classes/Player) | Represents a connected engine client | HTTP presence alone is not evidence of player membership. |

## Technical blockers

- No documented standalone player SDK or supported wire specification was identified in the reviewed official interfaces.
- Authentication, join authorization, transport validation, replication, scripts, physics ownership and heartbeat behavior would all need supported implementations.
- Experience compatibility depends on client scripts and simulation; exchanging authentication or presence requests cannot substitute for them.
- No documented external interface to disable the player's idle timer or implement a standalone session heartbeat was identified.
- Studio testing and creator service APIs are useful testing tools, but do not supply ordinary public player-session semantics.

This is a feasibility conclusion from the reviewed documentation, not proof that every private Roblox interface lacks such functionality.
No protected networking/security mechanism was reverse engineered, and no sessions were forged or public servers contacted for load testing.

## Product behavior

The UI displays a disabled Network-only option. API settings and mode transitions reject it explicitly before launching anything.
Normal and Headless retain the actual Roblox process; neither is substituted for a Network-only request.

The mode enum provides a future integration point without an empty backend.
A prototype requires an officially supported player-session interface, authorization for an owned test experience, and observable server-side player membership before enablement.
Any future transition between that backend and the official client must establish a new connection.
