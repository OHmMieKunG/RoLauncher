# Development

## Build and test

Windows prerequisites: Rust 1.99.0, .NET 8 SDK, Python 3.13, Inno Setup 6.7+, Visual Studio C++ Build Tools, and Windows SDK.
Dependencies are pinned. Only Windows builds and packages are supported.

```powershell
.\scripts\build.ps1
.\scripts\test.ps1
cargo clippy --locked --all-targets -- -D warnings
.\scripts\test-installer.ps1
```

Output: `dist\rolauncher`; UI previews: `target\ui-preview`. Tests use isolated apps and simulated accounts.

## Git flow

Remote: `git@github.com:oangsa/RoLauncher.git`.

1. Branch from `dev` as `feature/description`.
2. Open a PR into `dev`; pass CI.
3. Promote `dev` into `main` through a PR with a merge commit.

Recommended protection: require PRs, **Branch flow**, and **Windows checks** on dev/main; disable force pushes and deletion.

## Releases

Follow [AGENTS.md](../AGENTS.md). Documentation-only edits need no app release.

| KIND | Increment |
| --- | --- |
| `feature` | MINOR; reset PATCH |
| `fix` | PATCH |
| `major` | MAJOR; reset MINOR/PATCH |

- Run `python scripts/prepare-release.py KIND` against the latest published stable. Offline: use an explicitly verified `--base VERSION`.
- Local versions use `MAJOR.MINOR.PATCH-dev.<12 random hex digits>`.
- Build/package scripts refresh the identity; the upcoming numeric target stays fixed.
- Cargo/desktop builds reuse it. Run `python scripts/versioning.py` before a new direct local build.
- Use `-UseCurrentVersion` for checks, nested builds, and CI.
- Keep manifest, lockfile, README, runtime/desktop versions, installer labels, source, and filenames consistent.

For an intentional stable release:

1. Run `python scripts/prepare-release.py KIND --stable`.
2. Finish consolidated release notes and both changelogs.
3. Run `python scripts/check-release.py`.
4. Package with `scripts/package.ps1 -UseCurrentVersion`.

Publishing rejects dev identities. Main CI tests and packages the Windows installer, portable/source ZIPs, and SHA256 checksums; publication remains separately disabled.
Published artifacts stay unchanged; interrupted drafts resume only at the same commit.

## Local packaging

```powershell
.\scripts\package.ps1
python scripts/verify-package.py --smoke-shell
```

Alternate output: `-OutputRootDirectory FOLDER` and verifier `--dist-directory FOLDER`.
Existing artifacts are never overwritten. The installer is unsigned.

[Validation](VALIDATION.md) · [Performance](PERFORMANCE.md) · [API](API.md)

[Live Roblox acceptance](client-acceptance.md) uses two dedicated accounts and an owned experience; its tests are never enabled by the normal test suite.
Run `python scripts/test-client-report.py` for offline evidence fixtures without Roblox.
