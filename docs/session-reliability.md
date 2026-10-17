# Session reliability

## Connection evidence

- Process uptime comes from Windows creation time; it does not establish a connection.
- Connection uptime starts at a verified owned-log connection event and resets on disconnect.
- Managed running time includes recovery and retains the existing longest-run calculation.
- `last_disconnect` and cumulative `reconnection_attempts` are distinct from consecutive failures.
- Headless is an observed window-control mode, independent of log-derived connection status.

Repeated connection signals do not restart an already established connection timer.
Missing logs leave the connection Unknown; a live PID alone cannot produce Connected.
Restoring the window changes neither process identity nor the connection generation.

## Recovery

1. Existing verified logs and exit handles detect disconnects or exits.
2. Allow the existing 30-second reconnect grace for a live disconnected client.
3. Close only the managed identity and verify exit before replacement.
4. Retry with existing 10/20/40/80-second exponential delays, jitter and rate-limit handling; five consecutive failed attempts pause recovery.
5. Resolve that account's current provider and reapply its requested client mode/profile after verified process discovery.

Authentication, permission, missing ownership and unsupported-provider failures retain attention states.
Stop and generation changes cancel pending work; the existing retired-tracker window catches late launches.
Automatic reconnection starts a new connection and is never described as uninterrupted uptime.

## Idle behavior

[Roblox's Player documentation](https://github.com/Roblox/creator-docs/blob/main/content/en-us/reference/engine/classes/Player.yaml) describes idle notifications and automatic disconnect after at least 20 idle minutes.
Hiding or minimizing is not input and does not establish idle prevention.
No supported external idle-timer override or reliable hidden-window input mechanism was identified.

The pre-existing opt-in activity experiment uses foreground Windows input and skips locked/sleeping sessions or held modifiers.
Normal auto-hidden clients remain ineligible; Headless requires the separate `headless_foreground_activity` opt-in.
Headless attempts wait for 60 seconds without desktop input, temporarily restore the verified window, request foreground focus, submit normal input and minimize/hide again.
Busy desktops defer with one-minute retries; successful OS submission schedules the next attempt ten minutes later.
Use only in an authorized experience; Windows accepting an input event does not prove Roblox accepted it.

## Validation limits

Automated tests cover lifecycle, retries, cancellation, identity reuse, settings and owned-log parsing.
Live provider joins, two-account gameplay, forced recovery and 25/60-minute idle observations were not executed.
No authorized disposable account/experience pair was supplied for those checks; no indefinite-session guarantee is made.

The [live harness](client-acceptance.md) now records connection timers, last disconnect, retry counts and replacement identities separately.
Its Studio observer supplies independent connection nonces and progress acknowledgements; no observer was installed during this validation.
The user selected a 25-minute recovery-off baseline and a separate 60-minute recovery-on run; both remain BLOCKED.

## Headless activity safeguards

- Disabled by default, including accounts that previously enabled Normal activity.
- One managed activity at a time; no hidden-window messages or idle-timer changes.
- Identity, preferences, Pause, desktop input and lock/sleep state are checked before submission.
- Restore waits at most three seconds; denied focus, changed input or cancellation leave an explicit result.
- Window state is observed while temporarily visible; saved mode and process policies remain intact.
- Mode changes and shutdown cancel activity and preserve the existing restoration path.

`activity25` and `activity60` require uninterrupted server-observed connections with automatic recovery disabled.
The observer records bounded input/idle counts, without typed text, alongside connection nonces and progress acknowledgements.
The separate recovery scenarios report replacement processes and new connection intervals.

Windows can deny focus or input; desktop activity can defer attempts past Roblox's idle timeout.
No live idle-prevention result or indefinite-session guarantee is claimed.
