# PR02 validation and independent review

Base: `000c175fafa8ff6e763dd28bf80509cbe702eab2`; branch `codex/pr02-supply-chain-security` targets `dev`.
This report distinguishes local executions from hosted CI; no merge or release is authorized.

## Local evidence — October 16, 2026

| Check | Evidence/result |
|---|---|
| TDD CLI/policy seams | User approved scanner CLI, audit decisions and workflow trust boundary; red/green cycles captured under ignored `target/pr02/tdd-red-*.log` |
| Security suite | 13 tests pass on both Windows and native Arch WSL with all three real pinned scanners; no cases skipped |
| Clean source/history | No Gitleaks matches in initial tracked tree, staged PR02 files or 62 available historical commits |
| Synthetic clean/false positives | Harmless environment references and documentation URL remain clean |
| Synthetic credential detection | PAT, Roblox cookie and authenticated launch URI detected; complete values absent from CLI output; removed-in-range secret still detected |
| Lockfile vulnerabilities | Real RustSec scan reports 10 pairs; synthetic advisory fixture retains ID and patched range without inventing merge authority |
| Advisory unavailable | Actual cargo-audit fixture with unreachable local DB source returns unverified; missing/corrupt report cases do likewise |
| License policy | Actual cargo-deny fixtures distinguish MIT, missing license and GPL-2.0-only outside reviewed allowlist |
| Scanner/process errors | Absent executable, bad process output and tampered tool archive return unverified; tampered executable is not extracted/executed |
| Untrusted PR safety | Parsed workflow test checks PR events, read-only permissions, immutable Action pins, nonpersisted credentials and no custom secrets/artifact upload |
| Windows Rust | 111 passed, plus explicit ignored bootstrap passed; no PR07 safety test skipped |
| Linux Rust | 108 passed, one existing ignored disposable Flatpak test; formatting and Clippy pass |
| Desktop | 74 checks pass on Windows and native Linux; initial Linux environment failures resolved with locked restore and checksum-verified official .NET 8.0.31 runtime |
| WinUI | Build and isolated full smoke pass; first smoke failed animation time-zero assertion; no application code changed |
| Rust static checks | Formatting and Clippy pass on Windows and Linux |
| Release governance | Existing 30 promotion tests pass on Linux; Windows governance/versioning/beta tests passed before UI stage; version check remains 1.7.0 |
| Hosted security CI | All three jobs passed on initial head `64242d10045bd62691d77d1d2fbd5c39926fd5fb`: [run 38854244395](https://github.com/oangsa/RoLauncher/actions/runs/38854244395); final-head evidence follows in the PR handoff |

The initial full Windows command stopped at smoke; it is not recorded as a completely passing full run.
Private local logs stay under ignored `target/pr02`; scanner raw secret matches are temporary and are not uploaded.
No live Roblox sessions or production credentials were used.
Existing local Windows payload, source ZIP and representative historical Linux payload scans also found no detector matches.
Actionlint 1.7.12 passed the new workflow; `check-release.py` verified unchanged 1.7.0 metadata.

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

## Exact changed files

All 16 files are new; no existing application, dependency lock or PR01 file is modified.

```text
.github/workflows/security.yml
docs/security/DEPENDENCY-INVENTORY.md
docs/security/PROVENANCE-AND-LICENSE-REGISTER.md
docs/security/SECRET-SCAN-REPORT.md
docs/security/SECURITY-GATES.md
docs/security/SUPPLY-CHAIN-THREAT-MODEL.md
docs/security/VALIDATION.md
docs/security/VULNERABILITY-BASELINE.md
docs/security/dependency-inventory.json
docs/security/vulnerability-baseline.json
scripts/security-deny.toml
scripts/security-gitleaks.toml
scripts/security-inventory.py
scripts/security-tools.py
scripts/security.py
scripts/test-security.py
```
