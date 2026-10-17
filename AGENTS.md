# Repository instructions

## Versioning and releases

Use MAJOR.MINOR.PATCH for published stable releases. Apply these rules to all future releases:

- New feature: increment MINOR and reset PATCH to zero. For example, 0.1.3 becomes 0.2.0; generally M.X.Y becomes M.(X+1).0.
- Bug fixes, performance optimizations, and upgrades to an existing feature: increment PATCH. For example, 1.8.0 becomes 1.8.1, then 1.8.2; generally M.X.Y becomes M.X.(Y+1). Reserve MINOR increments for genuinely new features.
- A deliberately huge product milestone or breaking change: increment MAJOR and reset MINOR and PATCH to zero. For example, 0.4.7 becomes 1.0.0, and 1.4.7 becomes 2.0.0. N means the next major version, not an arbitrary number.
- A release containing both features and fixes uses the feature increment. A major milestone takes precedence over both.
- Bump once per release, not once per commit or individual fix. Respect an explicitly requested release version.
- For a new feature release, verify and anchor the target to the latest published stable GitHub release. For a subsequent update optimizing or upgrading an existing feature, increment PATCH from the latest delivered numeric version, including development deliveries. Following the delivered 1.8.0 development build, the next such update targets 1.8.1, followed by 1.8.2.
- Builds intended only for local development use MAJOR.MINOR.PATCH-dev.<12 random hexadecimal digits>, for example 1.4.0-dev.a1b2c3d4e5f6. Generate a fresh identity per local build/package, even at the same commit or with uncommitted changes. Keep the numeric target fixed while building and testing one update; increment PATCH for the next delivered existing-feature update. The suffix is an opaque identity, not a chronological ordering.
- Intentionally requested public betas use MAJOR.MINOR.PATCH-beta.N with a positive numeric sequence and tag vMAJOR.MINOR.PATCH-beta.N. Prepare with scripts/prepare-release.py KIND --beta N; local builds still use fresh dev identities. Beta discovery is opt-in; beta numbers sort numerically, stable supersedes its matching beta, and public beta/stable builds supersede matching local dev builds. Historical beta-vMAJOR.MINOR.PATCH releases remain recognized.
- Use scripts/prepare-release.py KIND to select a development target from GitHub latest stable (or an explicitly verified --base for offline work); for existing-feature updates, use the latest delivered numeric version as the base so the PATCH rule above takes precedence. scripts/build.ps1 and scripts/package.ps1 stamp a fresh development identity by default. Direct Cargo or desktop builds reuse the current identity; run scripts/versioning.py before a new local build. Use -UseCurrentVersion for nested builds, tests, and CI to keep all outputs consistent.
- Only prepare a plain stable version for an intentionally requested GitHub release: scripts/prepare-release.py KIND --stable, then package with -UseCurrentVersion. Publishing scripts must reject development identities. The final stable version of the same numeric target must be offered as an update to its development builds.
- Release notes describe net user-visible changes compared with the latest published stable release. Omit intermediate development iterations, fixes to features that have never shipped, incidental cosmetic adjustments, and capabilities already present in the stable baseline.
- Consolidate all unpublished iterations under docs/RELEASE-MAJOR.MINOR.PATCH.md and one upcoming entry in both changelogs. Do not create release-note files per random identity or present local iterations as separate stable releases. Preserve already published release history and artifacts.
- Do not replace already-released artifacts with changed contents under the same version; release the change with a new version.
- Keep Cargo.toml, the root package entry in Cargo.lock, README, release notes, and artifact filenames consistent. Runtime version strings should derive from CARGO_PKG_VERSION.
- Development runtime strings, desktop informational versions, installer labels, source archives and artifact filenames must carry the full suffix. Windows numeric file/assembly metadata uses the numeric core. Release notes use the numeric target and clearly mark it upcoming until published.
- Run required checks, package the Windows executable with WebView2Loader.dll and license notices, include corresponding source and AGENTS.md, and generate SHA256 checksums.

This is the project's release convention. It is inspired by Semantic Versioning; a large milestone alone is not necessarily a breaking API change under strict SemVer.

## Documentation and changelog style

- Keep README and docs concise. Use short sections, numbered steps for workflows, and bullets for independent facts.
- Limit prose paragraphs to three short sentences (about 60 words). Split longer explanations; remove repetition and filler.
- Preserve required commands, API contracts, prerequisites, security constraints, and known limitations when shortening.
- Write changelog bullets as one short sentence per user-visible change; aim for 25 words or fewer. State the action and user impact.
- Keep versions newest first, retain release dates/status, and use Added, Changed, or Fixed only when needed. Mark the upcoming release Unreleased/Upcoming.
- Describe net changes since the latest published stable. Omit local iterations, commit narratives, implementation details, test logs, and fixes to unshipped features.
- Keep CHANGELOG.md and desktop/CHANGELOG.txt aligned. Put detailed setup and validation in linked docs instead of repeating them in release history.
- Shorten historical wording without dropping released changes, version entries, dates, or compatibility limits. Never modify published release artifacts.

## Roblox integration constraint

Read-only inspection of Roblox process memory is permitted. Do not write to or modify Roblox process memory, inject code, patch executables or client files, or use executors. Authenticated Roblox requests, the installed launch protocol, Windows process metadata and exit handles, and existing log files may also be used. Never expose session cookies, tickets, or authenticated launch URLs in diagnostics.
