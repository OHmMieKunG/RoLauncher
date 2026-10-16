# Security gates and operation

The new **Security inventory** workflow runs on PRs to `dev`/`main` and explicit read-only dispatches.
It adds source/Rust evidence and Windows/Linux NuGet jobs without renaming existing checks or changing repository settings.
Workflow JSON is valid YAML, allowing dependency-free trust-boundary tests with Python's JSON parser.

## Decisions implemented now

| Result | CLI exit | CI treatment |
|---|---|---|
| New high-confidence secret pattern | 1 | Fail; no exemption or baseline suppression exists |
| Tool absent, timeout, corrupt/missing report, advisory fetch failure, filtered Rust audit, invalid/stale/future database time | 2 | Fail as unverified, independently from a confirmed finding |
| Completed Rust/NuGet vulnerability audit with findings | 0 | Visible baseline evidence; no unapproved permanent severity gate |
| Completed Rust license/source audit with missing/disallowed license | 0 | Visible finding; legal/license disposition remains required |
| Completed audit without known findings | 0 | Clean only within tool/feed/scope limits |
| Reviewed empty desktop test NuGet lock/configuration | 0 | Not-applicable; explicitly does not audit framework/runtime packs |

No detector match proves that a credential is active; blocking prevents publication of plausible new credentials while the owner investigates privately.
Current tracked source/history have no detector matches, so no pre-existing secret allowlist was needed.
Unavailable scans must be rerun or explicitly handled through a separately reviewed owner decision, not relabeled clean.

## Reviewed zero-package project

Only `desktop.tests/RoLauncher.Desktop.Tests.csproj` may return not-applicable for an empty graph.
Its reviewed `Microsoft.NET.Sdk`/`net8.0` configuration contains no PackageReference or custom imports; LF-normalized SHA256 is `26eb8bcbca31356d9436a2c4afb31a3a4929b56b9a9806d79a4b83d8d828c19d`.
The lock must be exactly schema 1 with one empty `net8.0` group.

Changed project contents, additional/different frameworks, or ancestor `Directory.Build.props/targets/rsp`, `Directory.Packages.props/targets`, `MSBuild.rsp` or `global.json` controls invalidate this exception.
Missing/malformed lock graphs and unexpected empty Windows/Linux production graphs are unverified, even if the native tool would report clean.
This narrow declaration/lock assessment assumes the existing trusted SDK environment; it does not audit SDK/runtime packs or certify all MSBuild behavior.

## RustSec freshness assessment

The database's `last-updated` must parse as a timezone-aware ISO timestamp and be at most **30 days** old.
Up to **five minutes** ahead of the UTC assessment clock is tolerated for clock skew; anything farther ahead is unverified.
Exactly 30 days and exactly five minutes are inclusive boundaries.

Each evaluated report records assessment time, age, maximum age, future tolerance and fresh/stale/future/invalid status.
Stale, malformed, timezone-less or implausibly future evidence returns unverified/exit 2 while retaining parsed advisories and informational warnings.
Fresh reports with advisories still return the existing baseline-only exit 0; this is a data-quality gate, not a vulnerability-severity merge gate.

Thirty days tolerates quiet advisory periods while refusing indefinitely old evidence; five minutes limits ordinary clock skew without trusting future-dated commits.
The normal audit command still fetches through cargo-audit; commit age is a bounded freshness proxy, not proof of complete/latest advisory coverage.
CI trusts its UTC clock; there is no CLI date override or stale-data bypass, and changing this window requires reviewed policy changes.

## Proposed policy requiring owner approval

1. Keep all new verified secrets blocking; promptly rotate confirmed active exposures through authorized owner account actions.
2. After baseline triage, block new critical/high advisories in shipped dependencies, with explicit disposition for unknown severity and exploitable lower-severity findings.
3. Track development/build-only advisories separately; do not automatically dismiss code executed by CI.
4. Block new missing/unreviewed or incompatible license obligations only after the existing license baseline is dispositioned.
5. Decide which new status checks become required through separately authorized repository settings work.

These proposed dependency/license thresholds are **not** silently installed by PR02.
The current Rust permissive/GPL allowlist is evidence policy, not commercial clearance; cargo-deny evaluates SPDX AND/OR expressions.
Ms-PL/GPL compatibility, custom Microsoft/Uno terms and native redistribution need human review beyond that Rust allowlist.

## False positives and exceptions

No active suppression or accepted risk exception is added.
Tests generate synthetic credentials at runtime; harmless environment references and documentary URLs do not match the high-confidence rules.
Default detector heuristics can still miss credentials; this is not proof of absence.

Every future exception must record ID/fingerprint, exact path/package/version/target scope, rationale/evidence, owner, reviewer/date, expiry and reassessment trigger.
New versions, changed usage/target, new exploit evidence, changed provenance or expired review invalidate the prior disposition.
For secrets, retain only sanitized fingerprint/location metadata; never store a matched line/value in an allowlist or PR comment.

An owner must approve each exception through review; no wildcard path/package exclusion or undocumented tool ignore is acceptable.
Raw `gitleaks:allow` comments and `.gitleaksignore` are disabled by the wrapper.
Rust reports with advisory ignores, severity thresholds or target filtering are rejected as unverified.

## Local commands

```sh
python scripts/security-tools.py --directory .tools/security
python scripts/security.py secrets --gitleaks .tools/security/gitleaks
python scripts/security.py secrets --gitleaks .tools/security/gitleaks --history
python scripts/security.py audit --kind rust --tool .tools/security/cargo-audit --database .tools/security/advisory-db
cargo fetch --locked
python scripts/security.py audit --kind licenses --tool .tools/security/cargo-deny
python scripts/security.py audit --kind nuget --project desktop/RoLauncher.Desktop.csproj
python scripts/security.py audit --kind nuget --project desktop.linux/RoLauncher.Linux.csproj
```

On Windows use executable `.exe` suffixes and load `scripts/environment.ps1`; run Linux NuGet commands with the native Linux SDK.
Restore each desktop project with its existing NuGet.Config, `--locked-mode` and Release configuration before auditing.
For behavioral tests set `GITLEAKS`, `CARGO_DENY`, `CARGO_AUDIT` to pinned executables, then run `python scripts/test-security.py`; unset tools cause explicit local skips, while hosted source CI sets all three.

Sanitized advisory/metadata reports may be retained after review; raw secret scan reports/logs must remain private and ephemeral.
The new workflow uploads no artifacts. Existing builds remain responsible for their own package/diagnostic hygiene.

## Stop and review

PR02 stops at a draft PR with [validation evidence](VALIDATION.md).
Owner decisions: advisory remediation/thresholds, license/provenance dispositions, actual payload obligations and separately approved required-check settings.
No legal clearance, Roblox approval, paid feature authorization or release authorization follows from passing these checks.
