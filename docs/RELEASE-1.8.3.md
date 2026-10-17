# RoLauncher 1.8.3

Upcoming / Unreleased — changes since published stable 1.7.0.

- Configure Normal/Headless per account and restore managed windows without rejoining.
- Select an installed protocol, Official, Bloxstrap, Fishstrap or trusted custom provider independently for future launches and recovery.
- Detect installed launchers automatically when opening account details; preserve explicit provider and executable choices.
- Apply reversible Windows efficiency controls automatically in Headless, preserving saved resource profiles and restoring captured settings in Normal mode.
- Monitor CPU, working set, private allocation, page faults and available GPU engine usage and separate process/connection uptime in the existing native UI.
- Persist client/resource preferences through bulk edits, presets and encrypted backups; existing accounts retain their installed protocol and Normal mode.
- Manage all instances with batch launch, stop, hide and restore controls.
- Keep Network-only visibly unsupported; idle activity remains explicitly opt-in and experimental.

This fork supports Windows 10/11 x64 only; Linux/Sober builds, packages, updates and CI jobs are removed.
Published historical artifacts remain unchanged.

Bloxstrap/Fishstrap must already match the installed player handler; simultaneous competing provider registration is unsupported.
Headless means hidden background gameplay, not rendering disabled; live Roblox benchmarks and 25/60-minute idle validation remain outstanding.
See [implementation status](headless-architecture.md#delivery-status) and [performance evidence](performance-benchmarks.md).

Headless activity has a separate per-account opt-in: briefly activate only after 60 seconds without desktop input, then hide again.
Skipped attempts retry at most once per minute; Windows submission does not establish Roblox acceptance.
Normal accounts keep their saved graphics settings; no shared bootstrapper settings or driver profiles are edited.
