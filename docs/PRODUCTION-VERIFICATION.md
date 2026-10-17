# Development verification

Target: 1.8.4 development, with public candidate 1.8.4-beta.1 remaining upcoming.
Implementation and automated evidence are recorded in the [compatibility report](client-compatibility-report.md).
Both public-beta and stable publication remain NO-GO without the plan's authorized live evidence.

## Required gates

1. Run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test.ps1` for Rust, script, desktop, WinUI and authenticated bridge checks.
2. Run `cargo clippy --locked --all-targets -- -D warnings` and `cargo fmt --check`.
3. Run the three ignored isolated native fixtures individually with `--ignored --test-threads=1`; do not enable live Roblox acceptance without dedicated accounts.
4. Run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-installer.ps1` and `python scripts/check-release.py`.
5. After stamping a fresh development identity for a new build, run `scripts/package.ps1 -UseCurrentVersion -OutputRootDirectory dist/stabilization`.
6. Run `python scripts/verify-package.py --dist-directory dist/stabilization --smoke-shell` for SHA256, exact source bytes and isolated production-shell startup.

Local logs live under `target/stabilization-*.log`; final artifact evidence is in `target/stabilization-verification.md`.
The offline adapter check now compiles the real Windows code and cached dependencies.
Fixture tests use isolated data and the normal Windows token for DPAPI; they never weaken credential protection.

## Release decision

- IMPLEMENTED and TESTED describe code and automated fixtures; neither implies live Roblox validation.
- EXPERIMENTAL activity and harness comparisons remain opt-in and unmeasured.
- UNSUPPORTED controls remain Network-only, external per-process 1 FPS and rendering-off.
- BLOCKED live gates include provider coverage, two-account continuity, performance comparisons and 25/60-minute observations.

The publisher keeps its literal disabled guard, empty signing-key enrollment and null owner inbox.
No tags or releases are created by this stabilization work.
