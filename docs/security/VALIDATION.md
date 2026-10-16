# PR02 validation and independent review

Base: `000c175fafa8ff6e763dd28bf80509cbe702eab2`; branch `codex/pr02-supply-chain-security` targets `dev`.
This report distinguishes local executions from hosted CI; no merge or release is authorized.

## Local evidence — October 16, 2026

| Check | Evidence/result |
|---|---|
| TDD CLI/policy seams | User approved scanner CLI, audit decisions and workflow trust boundary; red/green cycles captured under ignored `target/pr02/tdd-red-*.log` |
| Security suite | 12 tests pass on Windows with all three real pinned scanners; Linux initial 11-test run passed, final rerun pending |
| Clean source/history | No Gitleaks matches in initial tracked tree or 62 available historical commits; final new files require staged-tree scan |
| Synthetic clean/false positives | Harmless environment references and documentation URL remain clean |
| Synthetic credential detection | PAT, Roblox cookie and authenticated launch URI detected; complete values absent from CLI output; removed-in-range secret still detected |
| Lockfile vulnerabilities | Real RustSec scan reports 10 pairs; synthetic advisory fixture retains ID and patched range without inventing merge authority |
| Advisory unavailable | Actual cargo-audit fixture with unreachable local DB source returns unverified; missing/corrupt report cases do likewise |
| License policy | Actual cargo-deny fixtures distinguish MIT, missing license and GPL-2.0-only outside reviewed allowlist |
| Scanner/process errors | Absent executable, bad process output and tampered tool archive return unverified; tampered executable is not extracted/executed |
| Untrusted PR safety | Parsed workflow test checks PR events, read-only permissions, immutable Action pins, nonpersisted credentials and no custom secrets/artifact upload |
| Windows Rust | 111 passed, one existing ignored bootstrap; standalone bootstrap rerun pending after UI smoke |
| Linux Rust | 108 passed, one existing ignored disposable Flatpak test; formatting and Clippy pass |
| Desktop | Windows 74 checks passed; Linux initial run failed missing targeting pack, locked restore/rerun underway |
| WinUI | Build passed; first smoke failed animation time-zero assertion; isolated rerun underway, no application code changed |
| Release governance | Existing 30 promotion tests pass on Linux; Windows governance/versioning/beta tests passed before UI stage; version check remains 1.7.0 |
| Hosted CI | Not yet claimed; exact run/head/checkout evidence will be added after draft creation |

The initial full Windows command stopped at smoke; it is not recorded as a completely passing full run.
Private local logs stay under ignored `target/pr02`; scanner raw secret matches are temporary and are not uploaded.
No live Roblox sessions or production credentials were used.

## Scope verification

Changes are limited to the new security workflow, scanner/configuration/tests and `docs/security` evidence.
PR01 workflows/publisher, application sources, lockfiles, versions, existing notices, release intents and repository settings remain unchanged.
The new workflow has no publishing/tagging path; packaging in existing CI is validation, not a release publication.

## Independent review checklist

- [ ] Confirm exact draft head/base and unchanged PR01/application/version/lockfile paths.
- [ ] Recheck scanner archive digests and full Action revisions against upstream evidence.
- [ ] Run all security tests with all three real scanners; verify no required cases skipped in hosted source job.
- [ ] Introduce synthetic credentials, remove them in a later commit, and verify location-only failure across the PR range.
- [ ] Simulate missing/corrupt/stale advisory data, tool crashes and filtered/ignored audit configuration; expect unverified.
- [ ] Confirm baseline advisories/license findings stay visible and do not silently become accepted exceptions.
- [ ] Review CommonServiceLocator, Windows App SDK, Uno custom/missing terms and actual LGPL/native payload obligations.
- [ ] Compare RAM-related functions and authorship evidence independently; keep platform-policy approval separate.
- [ ] Verify all existing Windows/Linux checks at the final head and inspect actual packaged source/notices before distribution.
- [ ] Confirm no release, merge, version bump, repository-setting change or production credential action occurred.

Commercial readiness remains blocked by unresolved rights/provenance, license obligations, runtime/component coverage, open advisories and separate platform-policy/release deployment requirements.
Owner approval is still needed for baseline remediation priorities, future severity/license gates, exceptions and separately configured required checks.
