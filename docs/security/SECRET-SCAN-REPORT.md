# Sanitized secret scan — PR02

Date: October 16, 2026. Gitleaks 8.30.1, default rules plus Roblox cookie/authenticated-launch rules.
No real credential value is included in this report, scanner output, tests or CI artifacts.

## Scope and findings

| Scope | Result |
|---|---|
| Current tracked Rust/C#/scripts/workflows/configuration/docs/installer/assets | No detector matches at the reviewed base or staged PR02 source scan |
| All locally available refs/history, 62 commits at initial scan | No detector matches; includes PR01/PR07 and historical source |
| Existing local `dist/rolauncher-v1.7.0` Windows payload | No detector matches; local output, not proof about published release bytes |
| Existing local `dist/rolauncher-v1.7.0-source.zip` | No detector matches, including archive traversal |
| Existing local Linux `1.6.0-dev.80c754cda96d` payload | No detector matches; historical representative output, not current release certification |
| Synthetic clean fixture | Clean |
| Generated PAT/cookie/authenticated-launch canaries | Detected; output contains rule/path/line/commit only |
| Secret introduced then removed within a PR range | Detected in history; removal from the final tree does not hide it |
| Environment-reference/documentation false positives | Synthetic harmless references remain clean; no broad exclusions added |

Confirmed exposures: **none detected**. Suspected active exposures: **none identified** within this scope.
Known synthetic webhook/cookie/token examples exist in tests (for example `src/discord.rs`); context identifies them as fixtures, not active credentials.
No credential was tested against an external service, revoked or rotated; unreachable Git objects, remote-only refs and private developer data are outside this audit.

## Handling audit

Inspected examples discard tickets/launch URLs and emit counts/verified identity only.
Roblox cookie headers are marked sensitive; sessions use DPAPI or Linux Secret Service-backed encryption; desktop bootstrap passes API credentials through stdin.
This is a focused review of tracked diagnostic and packaging paths, not proof against every future logging change or same-user compromise.

Packaging copies complete source directories, so untracked local credentials placed under those directories could enter a local source ZIP.
Keep secrets outside package inputs; use a clean checkout and scan the actual package tree/archive before distribution.
PR02 documents that existing packaging risk without changing PR01 publication authority or build packaging.

## Operational rules

1. Run `python scripts/security.py secrets --gitleaks .tools/security/gitleaks` for tracked content.
2. Run with `--history` for all available refs, or immutable `--base SHA --head SHA` for added history.
3. Run with `--directory PATH` for explicit generated package inputs/outputs, including supported archives to depth two.
4. Keep sensitive investigation records private; share sanitized locations, rule IDs and commit SHAs only.
5. Treat any new high-confidence credential match as blocking pending triage; do not contact the credential's service automatically.

The wrapper captures tool stdout/stderr, uses temporary reports, strips matched lines/values/authors and deletes raw reports.
`--redact=100` is defense in depth; no scan artifact is uploaded by the new workflow.
Directory snapshots include tracked files regardless of .gitignore; inline Gitleaks allows and repository ignore fingerprints are disabled.

Heuristics can miss short, fragmented, encoded or unsupported secrets; archive scanning is bounded and opaque installer formats are not a complete forensic scan.
Git history scans available Git patches, not deleted unreachable objects; native tool errors remain unverified.
If an active credential is confirmed later, urgently rotate/revoke and investigate use through separately authorized owner account actions; do not print it or rewrite history automatically.
