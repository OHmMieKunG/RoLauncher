# Performance

Measure both Rust supervisor and WinUI desktop; measure Roblox and browser sign-in separately.
Repeat with 0, 10, and 50 clients:

```powershell
.\scripts\benchmark.ps1 -ProcessId PID -Seconds 30 -Output idle.json
```

- Historical headless 0.4.0: 14.6 MiB working set, 2.1 MiB private memory, 0.0% CPU over 15.24 seconds ([sample](idle-benchmark.json)).
- That sample excludes today's desktop; CPU was below counter resolution.
- Current total app usage is unmeasured.
