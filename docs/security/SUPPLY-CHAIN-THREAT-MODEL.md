# Supply-chain threat model — PR02

Assets: credentials, source integrity, build inputs, distributable licenses, release artifacts and source archives.
Trust boundaries: PR code, hosted runner, registry/advisory services, trusted default-branch publisher, local developer workspace and installed runtimes.
PR01 authorization logic and existing required check names remain unchanged.

| Threat / evidence | Current control | Residual risk and disposition |
|---|---|---|
| Untrusted PR executes code/MSBuild | New workflow uses `pull_request`, read-only contents, no custom secrets, nonpersisted checkout; no publication/upload steps | Candidate code can alter its own checks or make network requests; independent review and separately configured rules remain necessary |
| PR text injected into shell | Only immutable base/head SHAs enter scanner through environment; CLI validates SHA syntax | Static matrix values are trusted; no PR title/body/branch interpolation |
| Workflow/Action compromise | New checkout/setup-dotnet actions use full commit SHAs | Existing build Actions use mutable major tags; document separate hardening, preserve existing PR01 workflows |
| Advisory outage or malformed tool result | Scanner exit 2, unverified; strict report/exit consistency | Availability blocks completion without pretending dependencies are clean; feeds can still have coverage/freshness gaps |
| Known dependency vulnerabilities | Exact lockfiles, RustSec and transitive NuGet evidence | Ten Rust baseline pairs open; no owner-approved permanent advisory severity gate yet |
| Dependency substitution | Cargo checksums, NuGet content hashes, locked restores, cargo-deny registry/source review | Registries, maintainers and native package repositories remain trusted; hashes are integrity, not safe-code proofs |
| SDK/runtime drift | Rust 1.99; Uno SDK version recorded; existing .NET locked package graphs | SDK channels/roll-forward, runtime packs, runner images and OS updates are not completely immutable |
| Build-script downloads | New scanner archives use fixed upstream URLs and SHA256; expected executable alone is copied | Existing rustup curl-to-shell, apt/pacman updates and Inno discovery are separately unpinned inputs; no bootstrap/publisher rewrite here |
| Native binary redistribution | Current packaging collects license files and metadata | NuGet file/legacy licenses, proprietary tool terms, native assets and GNU runtime exceptions require actual-payload disposition |
| Source archive leaks local files | New scanner snapshots tracked files and can scan explicit outputs; reports live under ignored target/temp | Existing source packaging recursively copies source directories, including possible untracked files; require clean-checkout and actual archive scan; document separately before packaging hardening |
| Diagnostic leaks | Scanner captures raw logs and drops matches/values/authors; location-only results; no new upload-artifact | Existing general build diagnostics/package uploads run independently; no claim that a new check cryptographically prevents a malicious workflow from uploading data |
| Forged artifact/checksum | Existing SHA256/package verifiers and PR01 receipt/digest/evidence checks | Hashes are not independent signing; code signing/update trust remains separate future work |
| Candidate poisons publisher | PR01 checks out trusted default-branch event SHA and treats artifacts as bounded data | Privileged promotion code, GitHub and owner remain trusted; PR02 adds no publisher trigger, write scope or secret |
| Historical privileged workflows/tags | PR01 governance explicitly records activation, owner/tag-control and old-run fencing requirements | PR02 does not attest live settings or activation; any newly confirmed PR01 defect must be escalated separately |

## Boundaries preserved

- Existing publisher, required check names, Release Intent contract and activation logic are unchanged.
- New scanners cannot tag, publish, push repository content, or provision credentials with their declared permissions.
- `workflow_dispatch` is read-only auditing, not release authorization; security workflow is not a publisher source workflow.
- No release intent, version edit, repository/ruleset change, production credential, signing key or commercial feature is introduced.

## Separate escalation items

Existing mutable build Action pins, recursive local source collection, runtime/compiler drift and independently uploaded build diagnostics warrant owner triage.
They are observations of existing build paths, not silent changes to PR01's publisher or an assertion that its authorization gate is broken.
Live controls, token policies and old-tag fencing require the separately authorized PR01 deployment proof described in [release governance](../RELEASE-GOVERNANCE.md).
