# Protected release promotion — PR01

Status: implementation prepared on `codex/pr01-protected-release-promotion`, October 16, 2026.
Publication, live settings changes, branch merges and application version changes were not authorized or performed.

## Release Intent contract

Ordinary `main` merges, version edits, changelog edits, workflow dispatches and pushed stable tags cannot authorize stable publication.
A qualifying release adds exactly one `.github/release-intents/<UUIDv4>.json` in an owner-merged `dev → main` PR.
Merging this record means the owner explicitly authorizes publication of that resulting merge commit after checks pass.

```json
{
  "schema": 1,
  "id": "<fresh lowercase UUIDv4 matching the filename>",
  "version": "<intentionally prepared MAJOR.MINOR.PATCH>",
  "channel": "stable"
}
```

The placeholders are deliberately invalid; this PR creates no executable intent and selects no release version.
The four fields are mandatory; extra fields, duplicate JSON fields, development suffixes and reused/edited paths are rejected.
Keep historical records unchanged and create the record in one commit, after preparation is complete.

1. Follow AGENTS.md: verify latest stable, prepare the intentional version once, and finish README, lockfile, release notes and changelogs.
2. Add the fresh record through a `feature/*` or `codex/*` PR to `dev`; pass all checks.
3. The repository owner merges the subsequent `dev → main` PR, using a merge commit as established by project convention.
4. Post-merge Windows, Ubuntu package, Ubuntu container and Arch container checks must succeed at the exact source SHA.
5. The trusted publisher verifies authority and artifacts, finishes a matching draft, then publishes automatically once.

Authority comes from GitHub's authenticated PR `merged_by.id` matching the personal repository owner's numeric ID.
Author names, commit messages, labels, record-supplied identities and arbitrary branch/tag names confer no authority.
Repository transfer, organization ownership or a changed default branch requires a reviewed transition; the gate currently accepts only `oangsa/RoLauncher` with `main` as default.

The source SHA comes from the owner-merged PR and GitHub push run, avoiding a self-referential commit field.
The source must still be current `main` before writes and final promotion; an older unconsumed intent needs a fresh, separately authorized promotion.
GitHub path history must show a single creation, preventing deletion/reintroduction or reuse of an old intent.

## Trust and artifact boundary

`ci-release.yml`, `beta-release.yml` and `linux.yml` retain their Windows/Linux build, test, smoke and packaging jobs.
They have read-only tokens, do not persist checkout credentials, and contain no publishing jobs or custom publishing secrets.
`codex/**` now follows the same development path as `feature/**`; only same-repository `dev` PRs target `main`.

`release-promotion.yml` runs on workflow completion from the default branch with a single serialized publication job.
It checks out the event's trusted default-branch SHA using a pinned checkout action, never the candidate `workflow_run.head_sha`.
The write token is passed only to the trusted Python publisher; it executes no candidate scripts, builds, installers or downloaded programs.

The publisher independently reads GitHub API evidence for the source workflow, latest attempt, exact jobs and artifacts.
It requires successful named jobs, matching source identities and active branch/tag rulesets before writing.
It repeats controls and run checks before final promotion, including whether the publishing workflow was disabled.

| Evidence | Required behavior |
|---|---|
| Source run | Official repository, `push`, exact workflow path/branch, completed successfully |
| Main checks | Branch flow; Windows checks; Linux checks and package (Ubuntu 24.04) |
| Separate Linux workflow | Same main SHA; successful Linux checks (Ubuntu) and Linux checks (Arch) |
| Run attempts | Re-read current attempt and job evidence before publishing; a changed attempt is not a mutation retry |
| Artifact selection | Exact Windows/Linux names suffixed `-attempt-N`, source run ID/SHA and current attempt; no expired, duplicate or missing artifacts |
| Archive integrity | GitHub API `sha256:` archive digest required and verified; missing digest fails closed |
| ZIP handling | Fixed basenames; bounded directory and decompression; no paths, duplicates, symlinks, encryption, split ZIPs or ZIP64 |
| Payload integrity | Existing Windows/Linux checksum validator requires exact payload sets and verified SHA256 values |
| Version/channel | Intent, Cargo, lockfile, README and notes agree; source CI retains all existing changelog/version checks |
| Stable ordering | A new stable version must exceed latest published stable; no application version is chosen by this change |
| Version tag | Exact checked commit; stable tags are lightweight; existing wrong/ambiguous tags are rejected |

The publisher adds `release-promotion.json` with intent, source, channel, run/attempt, artifact IDs/archive digests and five final asset hashes.
A hash of this receipt binds the draft body before uploads; the receipt becomes the sixth public release asset.
The receipt uses authenticated GitHub evidence, not a new cryptographic signing key or signed-update scheme; PR03/PR04 remain separate.

Each downloaded archive is capped at 256 MiB, including bounded HTTP reads when Content-Length is absent or false.
Expanded data across both archives is capped at 512 MiB; individual entries are capped at 256 MiB and checksum files at 1 MiB.
Extraction copies 1 MiB chunks to temporary files; final in-memory payloads remain bounded by the aggregate cap.

Before ZIP parsing, the standard footer must declare the exact expected entry count and a central directory no larger than 64 KiB.
Only stored/deflated entries are accepted; nested release payloads remain opaque and are never unpacked or executed.
JSON responses are capped at 8 MiB; request timeouts and the existing 15-minute publishing-job timeout bound stalled work.

These are application input budgets, not a guaranteed process-RSS ceiling or an OS sandbox.
Current local Windows payloads total about 156 MiB; a prior Linux package is about 64 MiB, within these budgets.
Larger packages or GitHub-generated ZIP64 archives fail closed and require a reviewed budget/format change before activation.

## Idempotency and failures

The matching release identity and receipt are the consumption ledger; no new database is needed.
A matching published release returns `consumed` and performs no writes.
A matching interrupted draft uploads only missing assets, verifies all asset sizes/digests, then promotes.

| Scenario | Result |
|---|---|
| Lost draft/tag creation response | Rerun reconciles existing identity before attempting creation again |
| Lost asset-upload response | Rerun recognizes identical uploaded bytes; never deletes/replaces them |
| Lost final-publish response | Rerun recognizes a complete consumed release; no second publication |
| Different draft marker, source, channel or asset bytes | Deny and quarantine for owner investigation |
| Historical release or pre-transition draft without receipt | Preserve it; deny automatic adoption or replacement |
| CI still running, failed, canceled, skipped or missing | Deny; completion of the other required workflow provides another reconciliation opportunity |
| Main advanced, artifact expired, or CI rebuilt as another attempt | Deny stale identity; do not rewrite the pending draft to fit different evidence |
| Invalid/truncated API evidence or unavailable GitHub | Fail closed; no automatic retry of ambiguous mutations |
| Emergency disable during upload | Keep draft; final gate checks workflow state before promotion |

Rerun the **Protected release promotion** run to retry the same source evidence; rebuilding CI is a different operation.
If an intent is stale, prepare a fresh owner-authorized intent and version/disposition as needed, rather than overwriting existing artifacts.
API evidence collections at 100 entries are rejected rather than partially trusted; split oversized promotion diffs.
GitHub can replace a pending concurrency run when more completion events queue; replay the trusted promotion run if an eligible event never executed.
This is a retry of existing authorization, not a new manual Publish approval.

## Beta and historical GPL transition

The existing `beta-vMAJOR.MINOR.PATCH` push still builds Windows/Linux/source artifacts.
Publication migrates to the common trusted publisher: the original tag-push actor must be the owner, the tag must still resolve to the checked SHA, and that SHA must belong to `dev` history.
Beta remains a prerelease with `make_latest=false`; it does not require a stable JSON intent or promote stable/latest.

Beta publication pauses during transition until the new default-branch publisher and separately approved controls/activation exist.
No beta tag was created or moved here; the existing successful beta run's actual `push`, tag-name `head_branch`, workflow path and owner actor were checked read-only for compatibility.
Legacy helper tests remain; both publishing command entry points now use the common workflow-run guard.

**Historical workflow hazard:** old tags contain old workflow code, including privileged publishing jobs.
Restrict creation of `beta-v*` tags to the owner before activation; new beta tags must point to the migrated workflow revision in `dev`.
Cancel old privileged runs and do not rerun pre-transition CI/publish jobs; their old code cannot be retroactively secured by this PR.

Historical GPL releases, tags, source archives, license terms and notices remain unchanged.
Existing source packaging still includes tracked build inputs and AGENTS.md; the new scripts/workflow/docs are tracked source inputs too.
No paid gating, signing credential or proprietary feature is added.

## GitHub settings — separate owner approval required

Read-only inspection on October 16 found a public repository, default branch `main`, unprotected `main` and zero rulesets.
No live setting was changed; deployment readiness is blocked until configuration and an isolated GitHub integration proof pass.
GitHub Free supports branch/tag rulesets for public repositories; availability must be reassessed before any private-repository transition. [GitHub ruleset documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/creating-rulesets-for-a-repository).

1. Keep the repository's write/admin/Actions-management authority with the trusted owner; audit collaborators, Apps, deploy keys and bypasses.
2. Set default workflow permissions to read repository contents; keep fork-PR write tokens and fork secret access disabled.
3. Add active rulesets targeting `refs/heads/main` and `refs/heads/dev`: require PRs, prevent deletion/force pushes, and require all five checks listed above.
4. Select GitHub Actions as expected check source and require branches up to date; permit the established merge-commit workflow.
5. Keep branch bypasses empty in normal operation; a sole owner does not need an unavailable second reviewer or a separate Publish reviewer.
6. Add an active tag ruleset for `refs/tags/v*` restricting updates and deletion, with no ordinary bypass; allow initial creation by the publisher token.
7. Add an active tag ruleset for `refs/tags/beta-v*` restricting creation, updates and deletion; limit creation bypass to the trusted owner/admin role and ensure no other account has that role.
8. Enable GitHub release immutability for future releases after testing its draft/upload behavior; preserve existing releases. [GitHub immutability setup](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes).
9. Cancel pre-transition privileged runs, complete isolated controls/permission/digest tests, then separately authorize `RELEASE_PROMOTION_ENABLED=true` as a repository Actions variable.

The runtime gate reads active effective branch rules, their source definitions and tag-rule definitions.
All five required checks must specify GitHub Actions App ID `15368`; strict up-to-date checking must be explicitly true on each required-status rule.
Missing/foreign integration IDs, missing/false strictness, truncated rule evidence and inactive/incorrect source rulesets deny promotion.

Visible branch bypasses must be empty; visible stable-tag bypasses must be empty.
Visible beta-tag bypasses permit only `RepositoryRole=5` (admin), `bypass_mode=always`; a separate owner/admin audit must confirm the owner is the only administrator.
Use separate stable and beta tag rulesets, since the beta creation exception must not apply to stable tags.

GitHub returns `bypass_actors` only to callers with write access to the ruleset; contents-write does not grant ruleset-write.
An omitted field is explicitly reported as **unverified**, never interpreted as an empty list; the publisher cannot enforce a hidden list using its least-privilege token.
Before enabling promotion, the owner/admin must separately inspect/export every applicable bypass list, collaborator/App/deploy-key permission and native immutability setting.

Record ruleset IDs, scope, check integration IDs/strictness, bypass actors and the reviewer/date in the activation evidence.
Reverify after protection, access, ownership or workflow changes; setting `RELEASE_PROMOTION_ENABLED=true` attests this separate verification has passed.
The checklist/attestation is an administrative prerequisite, not an enforced GitHub security boundary or a substitute for actual rules.

Rules and integration/strictness fields are readable with Metadata-read; no new publisher permissions are added.
The GitHub Actions identity was checked against actual hosted check runs on the reviewed branch. [GitHub rules API fields and bypass visibility](https://docs.github.com/en/rest/repos/rules).
Classic branch protection alone does not satisfy this implementation; configure the documented rulesets.

GitHub's immutable-release settings endpoint requires Administration read permission, unavailable to the proposed minimal workflow token.
The publisher therefore guarantees it never replaces published assets; native GitHub immutability and bypass/access configuration remain independently verified activation prerequisites.
Do not add a production admin token just to work around this limitation. [GitHub API permissions](https://docs.github.com/en/rest/repos/repos#check-if-immutable-releases-are-enabled-for-a-repository).

No repository-wide production secrets should be available to code-building workflows.
This patch references only the ephemeral `GITHUB_TOKEN`; trusted publication has contents-write, Actions-read and PR-read, with no Administration permission.
Untrusted fork PRs use `pull_request`, never `pull_request_target`; `workflow_run` authority stays on trusted default-branch code. [GitHub event and security model](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run).

## Emergency disable and rollback

1. Disable **Protected release promotion** in Actions and cancel its running/queued jobs; set `RELEASE_PROMOTION_ENABLED=false` or remove the variable.
2. Retain matching drafts, receipts, tags and artifacts for investigation; do not publish manually or overwrite conflicting assets.
3. Investigate credentials/source/controls, fix on a feature branch, and validate through dev; any live permission or credential action needs its own authorization.
4. Resume only after explicit owner authorization and revalidation of exact pending source/artifact identity.

Workflow disable is re-read before writes/final promotion; changing the variable alone does not cancel an already-running process.
GitHub cannot atomically combine a settings change with a release PATCH; cancel in-flight work for an emergency, and account for an already-completed publication.
Corrections to published bytes require a new version under AGENTS.md.

For code rollback, retain CI-only build jobs and the disabled guarded publisher; revert the new promotion path through a reviewed PR.
Do not blindly restore the old main-auto-publish or tag-local publisher jobs.
Do not delete, re-tag or replace historical releases as a rollback mechanism.

## Local validation and remaining proof

| Check | Result on October 16, 2026 |
|---|---|
| `python scripts/test-release-promotion.py` | 30 behavioral tests passed, including denial matrices, resource budgets and interruption/race cases |
| `python scripts/test-publish-beta.py` | 6 existing compatibility/checksum tests passed |
| `python scripts/test-versioning.py` | Development identity and stable preparation fixtures passed; repository version unchanged |
| `python scripts/test-linux-update.py` | 3 archive/update tests passed on Windows; not a native Linux run |
| `python scripts/check-release.py` | Existing 1.7.0 metadata/notes remain consistent |
| Python compilation | New/common publisher, tests and both entry points compile |
| Actionlint 1.7.12 | All four workflow files passed; official download SHA256 verified; shellcheck disabled/unavailable |

The first ordinary-merge test failed before implementation; later owner-authority, promotion, history, race, beta, CLI, tag-control, event and duplicate-field slices also ran red before green.
Tests use an external GitHub API fixture and synthetic archives; they exercise the actual promotion code without credentials or network writes.
No Windows application/UI/installer build, native Linux build, hosted CI run, production publish, ruleset edit or credential provisioning was performed.

Before activation, commission a disposable public-repository proof with the same Free-plan rules and owner-only access model.
Verify real GITHUB_TOKEN endpoint permissions, artifact and release digest availability, immutable draft completion, rule bypasses, token isolation, old-tag fencing, rerun/cancellation ordering and source association.
No sandbox repository or release was created under this task's prohibition on publishing releases.

GitHub-hosted runners, GitHub API correctness, the trusted owner/default-branch publisher and pinned checkout action remain trusted.
An administrator, compromised owner, trusted workflow edit or another write-capable workflow can defeat repository-local conventions; this implementation does not claim protection against those actors.
Final checks narrow race windows but cannot make API checks and publication a single atomic transaction; no separate signing/notarization or reproducible-build claim is made.

## Exact changed files

| File | Change |
|---|---|
| `.github/workflows/ci-release.yml` | CI-only main path, codex compatibility, read-only checkout and promotion tests |
| `.github/workflows/beta-release.yml` | Retain beta build/checks; move publication to trusted workflow |
| `.github/workflows/linux.yml` | Retain Ubuntu/Arch checks; add promotion tests and nonpersisted checkout credentials |
| `.github/workflows/release-promotion.yml` | Disabled-by-default trusted automatic publisher |
| `scripts/release_promotion.py` | Intent, evidence, artifact, rule, draft/idempotency and CLI boundary |
| `scripts/test-release-promotion.py` | Credential-free behavioral GitHub fixture and failure scenarios |
| `scripts/publish-release.py` | Stable entry point delegates to common guard |
| `scripts/publish-beta.py` | Beta entry point delegates to common guard; legacy helper compatibility preserved |
| `scripts/test.ps1` | Run promotion suite in Windows checks |
| `docs/DEVELOPMENT.md` | CI-only main behavior, intent workflow and codex branches |
| `.github/pull_request_template.md` | Explicit release-intent authorization guidance |
| `docs/RELEASE-GOVERNANCE.md` | Contract, controls, validation, transition and recovery record |

## Review-ready PR summary

**Title:** Require owner release intent before trusted automatic promotion

Ordinary main merges previously published automatically from a job executing candidate repository code with write authority.
Build workflows now remain read-only; a separate default-branch publisher requires a fresh owner-merged intent, passing exact-source Windows/Linux checks and verified artifacts before stable publication.
Matching retries resume drafts without replacing bytes; consumed intents perform no writes, and beta stays a prerelease through the same guarded transition.

Validation: 30 promotion behavioral tests, six existing beta tests, three archive/update tests, version fixtures, metadata checks, Python compilation and actionlint pass locally.
Live protection and automatic publication remain disabled/pending separate approval; no application version, GPL terms, release artifact or application behavior changed.
Review the settings/transition prerequisites and real-GitHub proof limitations above before authorizing activation.

## Follow-up review disposition

The comparison base is `dev` at `f61f9e06e4b5531b43b4af7a25a8ef1cec638af7`.
The feature branch was rebased onto that tip; no merge into `dev` or `main` was performed.
The unrelated Phase 0 audit was removed from the net PR diff and preserved outside the repository; the complete comparison is the 12 files listed above.

Follow-up code changes are limited to `scripts/release_promotion.py`, its behavioral suite and this governance record.
The original workflows, application version and application/licensing code were not changed by the follow-up.
Red tests reproduced weak integration/strictness, bypass and resource acceptance before the corresponding guards were added.

New coverage includes both branches, incorrect/missing App IDs, missing/false strictness, incomplete rule pages, visible bypasses and hidden-bypass reporting.
Artifact coverage includes overstated metadata, understated download size, aggregate expansion across archives, entry/checksum limits, central-directory growth and bounded HTTP reads.
Separate beta owner/admin creation bypass and rejected App/incorrect-role/PR-only exceptions are covered without creating a real tag.

Hosted results and exact workflow execution SHAs must accompany the final review package; local tests do not prove hosted compatibility.
For PR events GitHub builds a synthetic merge SHA; distinguish it from the PR head SHA and verify checkout's resolved HEAD in the logs.
For push events checkout uses the run head SHA; trusted workflow-run publication still uses the default-branch event SHA, never candidate code.

No release-intent record, live setting change, production credential or release is created by this follow-up.
Live repository-variable access is an administrative permission boundary; do not infer a missing/false variable from an unauthenticated 404.
The disabled-default CLI/workflow guards are tested; owner/admin verification of live activation state remains required before deployment.
