# Isolated stable hotfix release lane

Status: governance preparation, October 16, 2026; **not activated**.
Application remains `1.7.0`; no bug fix, 1.7.1 metadata, release intent, tag or release is prepared.
The Draft PR must receive independent security review before any merge or activation.

## Verified baseline and separation

| Reference | Verified value |
|---|---|
| Latest published stable | `v1.7.0` |
| Stable tag and live main | `4daae1bfc14db1336e1366134367ccb16efb7d4d` |
| Premium dev | `c4d1c73d12dd4a4307871e62db79fe82fda7002a` |
| Draft PR #27 head | `49026dc7a470e1d8ea814a60ec7986a31c656f87` |
| Preparation branch | `codex/hotfix-release-lane`, based directly on stable SHA above |
| Proposed future patch | `1.7.1`, only after a specific bug is approved |

Live release, branch/tag and PR evidence were refreshed before editing.
Local `main` was stale; it was not used as the preparation base or moved.
Premium local/remote branch references and PR #27 remain untouched; Premium work is paused in this task.

`dev` contains unfinished identity/licensing work and cannot supply immediate stable hotfixes.
Feature releases retain `feature/* → dev → main`; isolated patches use verified stable history, reviewed governance, then `hotfix/* → main`.
After transition, a hotfix base may follow stable only through reviewed governance commits, never application changes from dev.

## Legacy versus guarded behavior

| Concern | Stable v1.7.0 workflow | Proposed lane |
|---|---|---|
| Main updates | Successful main builds invoke stable publisher automatically | CI-only; missing intent returns without writes |
| Write credentials | Candidate build checkout runs publication code | Separate trusted default-branch publisher; candidate data only |
| Main PR flow | Only dev accepted | Dev unchanged; narrow preparation/hotfix scope checked independently |
| Release authority | Main event and successful builds | Owner-merged, fresh source-specific intent |
| Hotfix source | No isolated policy | Published baseline, governance ancestry, exact commits/patch and semantic review |
| Retry | Legacy draft asset replacement possible | Matching receipt resumes missing assets; published/conflicting bytes preserved |
| Activation | Legacy publishing already present | Literal false guard plus separate owner variable and control verification |

## Selective PR01 audit and port

Reference: dev at `c4d1c73d12dd4a4307871e62db79fe82fda7002a`.
Files were read individually with `git show`; no dev merge/cherry-pick or whole-tree import occurred.

| Component | Decision |
|---|---|
| CI, Linux and beta workflows | Port removal of write publishers, nonpersisted credentials, attempt-qualified artifacts and tests; add scoped hotfix admission |
| release-promotion workflow | Port trusted workflow-run separation and serialization; add hard-disabled activation guard |
| release_promotion.py | Port intent, owner, exact-source/check/artifact, Ruleset, bounded archive and idempotency controls; add schema 2 |
| test-release-promotion.py | Port all 30 synthetic regressions, including unchanged feature/beta contracts |
| Publishing entry points/test.ps1 | Port common trusted gate and Windows policy regression hook |
| prepare-release.py/check-release.py | Reviewed and retained byte-for-byte from stable; existing patch/version consistency behavior sufficient |
| release_files.py | Retained stable checksum/payload validator; reused by trusted publisher |
| package.ps1 | Narrow stable-source archive change: exact committed Git tree; development packaging preserved |
| Governance documentation | Replace dev-specific assumptions with concise dual-source contracts and this transition plan |
| PR02 inventory, security tools, identity PoC, provisioning, Premium app code | Excluded; no runtime/dependency/license behavior imported |

## Safe main transition

1. Independently review this exact preparation head, source diff and both policy suites; retain `1.7.0` throughout.
2. Owner audits/cancels queued or running legacy privileged jobs and forbids reruns of pre-transition jobs/tags.
3. Owner separately authorizes required repository protections below; keep publisher hard-disabled and variable false/unset.
4. Review PR's Branch flow, Windows and both Linux-platform checks; merge only through reviewed PR, with no direct-push/bypass.
5. Confirm merged main has no stable/beta write job, unchanged version and no intent; ordinary completion events cannot publish.
6. Complete disposable-repository permission/digest/immutability/rerun proof and independent security review.
7. Only a later owner-approved activation PR may remove literal `false`; enable the variable only after verified controls and approval.
8. Approve a specific bug before creating hotfix branch, version metadata or executable patch intent.

The first main push uses the replacement CI-only workflow; the new workflow-run publisher remains hard-disabled even if a variable already exists.
Old queued/rerun workflows retain old code and cannot be retroactively secured by this change.
If owner cannot fence those runs or satisfy reviewed PR checks, stop migration; do not restore automatic publishing or bypass protection.

## Owner settings and activation blockers

Read-only GitHub inspection found `main.protected=false` and zero Rulesets.
Repository-variable state was not available through the connector; do not infer false/unset from missing administrative access.
No setting was changed, and activation remains blocked.

1. Audit owner/admins, collaborators, Apps, deploy keys, workflow-edit rights and repository secrets; keep build jobs free of production credentials.
2. Set default Actions token to contents-read; disable write tokens/secrets for fork PRs.
3. Create active branch Rulesets for `refs/heads/main` and `refs/heads/dev`: require PR, prevent force-push/deletion, require up-to-date branches.
4. Require Branch flow, Windows checks, Linux checks and package (Ubuntu 24.04), Linux checks (Ubuntu), Linux checks (Arch), each from GitHub Actions App ID `15368`.
5. Use merge commits for hotfix promotion; keep branch bypass lists empty and separately review workflow/governance changes.
6. Create stable tag Ruleset `refs/tags/v*`: restrict updates/deletion, empty bypass list, permit initial publisher tag creation.
7. Create separate beta tag Ruleset `refs/tags/beta-v*`: restrict creation/update/deletion; creation bypass only sole trusted owner/admin role (`RepositoryRole=5`, `always`).
8. Enable/test native release immutability separately; preserve historical releases/assets and record owner verification.
9. Export effective Ruleset IDs, scope, checks, integration IDs, strictness, bypass lists, reviewer and date; reverify after access/control changes.
10. Prove runtime token can read required evidence without Administration/write secrets; then separately approve activation PR and `RELEASE_PROMOTION_ENABLED=true`.

Hotfix runtime denies omitted branch/tag bypass evidence, unlike PR01 feature-mode's explicit administrative-attestation boundary.
GitHub documents that [bypass lists require ruleset write access](https://docs.github.com/en/rest/repos/rules#get-a-repository-ruleset); minimal publisher credentials may therefore be insufficient.
**Unresolved activation blocker:** prove a least-privilege evidence route in a disposable repository; do not add an admin token or weaken this guard to force activation.

## Future patch procedure — commands not executed

After approved governance migration, verified unchanged stable and specific bug approval:

```powershell
git fetch origin main --tags
git rev-parse v1.7.0
git log --oneline v1.7.0..origin/main
git diff --name-status v1.7.0..origin/main
git switch --create hotfix/1.7.1 origin/main
```

Require `v1.7.0` to resolve to the verified SHA above and latest stable still to be `v1.7.0`.
Review every post-stable main commit as governance-only; if stable/main changed unexpectedly, stop and reassess.
Never branch from or merge dev, or reuse another Premium branch for this work.

1. Implement only approved fix and related tests; inspect prerequisites before any selective backport/cherry-pick.
2. Use `python scripts/prepare-release.py fix` for local development, retaining target `1.7.1` with fresh development identities per AGENTS.md.
3. When an actual release is authorized, use `python scripts/prepare-release.py fix --stable`; complete README, both changelogs and consolidated `docs/RELEASE-1.7.1.md`.
4. Verify Cargo/lockfile, desktop informational version and artifact references; desktop runtime derives full version from existing build metadata.
5. Open reviewed hotfix PR; Branch flow prints exact commits, paths and patch hash, while missing intent cannot publish.
6. Owner reviews full patch/blobs, call paths, tests and dependencies; record approved bug, prerequisite analysis and Premium exclusion.
7. Add schema-2 intent alone in final commit, using preceding reviewed SHA and exact main base; no edits/reuse of its UUID path.
8. Obtain passing exact-source checks; owner merges with unchanged reviewed tree and authorized base; activated publisher independently verifies promotion.
9. Verify published receipt, source, checksums and package contents; stable must supersede development builds of same numeric target.

The complete intent contract and hash algorithm are in [Release governance](RELEASE-GOVERNANCE.md).
Version selection still anchors latest published stable, not development identities or unfinished features.
No release-note entry is added for this preparation because it changes no shipped application behavior.

## Backport, rollback and recovery

1. After publication, create a separate dev-targeted branch for approved application fix/tests only.
2. Review prerequisites and cherry-pick those commits individually; resolve compatibility and test against dev independently.
3. Do not merge hotfix release metadata, intent or governance history blindly into Premium development.

For emergency disable, disable Protected release promotion, cancel running/queued publishers, and remove/set activation variable false.
Keep CI-only workflows; revert faulty governance through reviewed PR without reinstating legacy publishers.
Retain drafts, tags, receipts and artifacts for investigation; never delete/move published versions as rollback.

Rerun only trusted promotion with the same source run/attempt to reconcile matching interrupted publication.
Changed main, attempts, expired artifacts, conflicting tags/assets or stale intent require owner investigation and fresh reviewed authority.
Published corrections require a new patch; partial drafts with mismatched receipt stay quarantined.

## Independent security-review checklist

- [ ] Verify live stable/tag/main and exact Draft PR head; compare every changed file against stable.
- [ ] Confirm Premium branch/PR references, runtime code, dependencies, version and published history remain unchanged.
- [ ] Trace PR admission versus publication authority; verify ordinary main/hotfix merges cannot authorize themselves.
- [ ] Review both intent schemas, UUID freshness, merge parents/tree, full patch blobs and dependency/prerequisite attestations.
- [ ] Check each commit and net diff for hidden/reverted Premium changes and unrelated refactors.
- [ ] Trace build-token isolation, trusted checkout, redirects, archive bounds, source-tree hashes and payload checksum sets.
- [ ] Exercise missing/failed Windows/Linux evidence, stale/replayed events/tags, races, interrupted uploads and consumed receipts.
- [ ] Review legacy queued/rerun/tag fencing and hard-disabled migration with no 1.7.1 metadata or intent.
- [ ] Independently inspect all active Rulesets/bypasses, token permissions, native immutability and activation evidence.
- [ ] Keep activation blocked until real least-privilege/digest/immutable-draft proof succeeds; no production release for testing.

Remaining trust includes GitHub/hosted runners, trusted owner/default-branch governance and pinned checkout.
Pattern checks and review attestations cannot establish arbitrary program semantics or reproducible binary provenance.
API check/write races cannot be made atomic; final rechecks and native protections narrow that risk.

## Validation evidence

| Local check | Result |
|---|---|
| Existing promotion fixtures | 30 passed; feature/beta authority and replay behavior retained |
| Hotfix policy fixtures | 14 passed, including 27 denial scenarios and migration/source/history checks |
| Existing beta/archive fixtures | 6 beta and 3 Linux update/archive tests passed |
| Version fixtures/check-release | Passed; repository remains 1.7.0 |
| Rust tests | 75 passed; one WinUI bridge test initially ignored, then passed separately |
| Desktop and native WinUI | 74 checks, UI smoke and Rust/WinUI bridge passed |
| Formatting/Clippy | Passed, locked all-target Clippy with warnings denied |
| Workflow/Python validation | Four workflows pass actionlint 1.7.12; Python modules compile; shellcheck unavailable |
| Linux-only Sober adapter locally | Windows cannot execute its POSIX entry point (`WinError 193`); native hosted Linux required |
| Production publication/settings | Not run or changed; synthetic API fixtures only |

New hotfix authority, merge-tree, per-commit scope, source-byte, branch-admission and hidden-bypass cases were exercised red before their guards passed.
Native Linux/Windows packaging and exact hosted checkout evidence belong with the Draft PR's current head checks.
Local fixtures do not prove real publisher token permissions, artifact digest availability or immutable-release completion.

## Exact preparation file list

- `.github/workflows/ci-release.yml`
- `.github/workflows/linux.yml`
- `.github/workflows/beta-release.yml`
- `.github/workflows/release-promotion.yml`
- `scripts/release_promotion.py`
- `scripts/hotfix_policy.py`
- `scripts/branch-flow.py`
- `scripts/test-release-promotion.py`
- `scripts/test-hotfix-policy.py`
- `scripts/publish-release.py`
- `scripts/publish-beta.py`
- `scripts/test.ps1`
- `scripts/package.ps1`
- `docs/RELEASE-GOVERNANCE.md`
- `docs/HOTFIX-RELEASE-STRATEGY.md`
