# Windows device identity feasibility proof

Experimental only; no licensing, installer, credential-store or startup integration.
Baseline: `0e993f315080589454e57193ff53a9e752638cf4` (`dev`, October 16, 2026).
Roadmap PR24 is unrelated to GitHub PR #24.

## Boundary and threat model

An installation allocation, an authorized Windows principal and a customer account are separate identities.
Neither a SID, a public installation label nor a container name proves private-key possession.
This experiment uses synthetic customer identifiers and local verification only.

The intended adversary is another ordinary Windows user, including one who copies all public PoC state.
Administrator/SYSTEM, code executing as the authorized user, kernel compromise, modified clients and privileged VM/disk snapshots are outside that boundary.
No result establishes legal/commercial readiness or changes existing allocation, transfer or offline policies.

## Baseline inspection

- `installer/RoLauncher.iss`: `{localappdata}\\Programs\\RoLauncher`, `PrivilegesRequired=lowest`; unchanged.
- `scripts/build-installer.ps1`: packages existing per-user payload; unchanged.
- `src/main.rs`: per-user application data; no experiment invocation.
- `src/platform.rs`: user DPAPI, without `CRYPTPROTECT_LOCAL_MACHINE`; unchanged.
- `src/store.rs`: existing credential/database/backup persistence; unchanged.
- Experiment is an explicit Cargo example; no production module export or new runtime dependency.

## Candidate comparison

| Dimension | A: software machine M + user U | B: user keys + machine evidence | C: Platform Crypto Provider |
| --- | --- | --- | --- |
| Standard-user bootstrap | Must measure machine create/finalize/ACL behavior | User-store provisioning must be measured | Must measure provider and supported algorithm |
| Windows-user isolation | Require U and current token SID, even if M is readable | Require user key and token SID | TPM does not replace user authorization or ACLs |
| Installation proof | M possession; stable discovery/enrollment remains separate | Public evidence is a risk signal, not cryptographic device proof | Hardware-backed possession if provider reports hardware; attestation separate |
| TPM dependence | None | None | Required; no silent provider fallback |
| Export resistance | Provider export policy; software boundary | Same; copying protected profile remains unproven | Policy plus hardware claim, not remotely attested here |
| Windows reinstall | Lost key requires reviewed recovery | Evidence/key may change; reviewed recovery | TPM persistence alone does not recover Windows authorization |
| Cloning | No privileged clone resistance claim | Copied/spoofed machine labels cannot establish installation possession | Physical TPM may resist ordinary file copying; VM/snapshot limits remain |
| Installer / UX | Stable machine registration and least-rights provisioning unresolved | Simpler per-user setup, weaker installation semantics | TPM absence/reset/lockout and provider support need recovery UX |
| Privileges | Exact observed result required; no inference from administrator success | No intended elevation | Exact observed result required |
| Maintenance | M/U loss, ACL drift, enrollment/recovery | Profile movement and risk-signal stability | Hardware replacement, firmware, TPM clear, provider changes |
| Assumptions | Trusted OS/provider, protected registration and future customer binding | Trusted OS; evidence is neither secret nor attested | Trusted provider report locally; remote assurance needs separate attestation design |

Candidate labels follow this task. Earlier architecture documents used different letters for some alternatives.
All three require future authenticated customer enrollment; a synthetic string here proves no account ownership.

## Observed Windows evidence

Local test: Windows 26H2, build **26300.9457**, x64; Rust 1.99.0.
The effective token was an unelevated, unrestricted, non-service process token in an interactive session.
This proves execution without elevation; it does not prove the account lacks administrator-group membership.

| Operation | Result and evidence class |
| --- | --- |
| Software provider | **Proven by real Windows test:** Microsoft Software Key Storage Provider, implementation flags `0x22`, version `65536`, descriptor support `1`, RSA supported |
| Software user keys | **Proven by real Windows test:** RSA-2048, SHA-256/PKCS#1 signatures, public verification, persisted reopen and separate-process restart |
| Software policy/ACL | **Proven by real Windows test:** export policy `0`, signing-only usage `2`, user scope, protected DACL allowing only creator and SYSTEM |
| Software ACL readback | Both allow ACEs read back `0xd01f01ff`; provider expands requested `GENERIC_ALL` |
| Software machine key / Candidate A | **Blocked by missing privileges on tested configuration:** create and policy calls succeeded, finalization returned `NTE_PERM (0x80090010)` |
| TPM provider | **Proven by real Windows test:** Microsoft Platform Crypto Provider, hardware implementation flag `1`, version `65536`, descriptor support `1`, RSA supported |
| TPM user-key operations | **Proven by real Windows test:** creation, persistence, reopen, fixed-challenge signing, public verification and deletion |
| TPM user-key ACL | **Unsupported by strict PoC policy on tested configuration:** readback included allow ACE flag `0x10` (inherited), even after post-finalize descriptor reapplication |
| TPM machine key / Candidate C | **Blocked by missing privileges on tested configuration:** finalization returned `NTE_PERM (0x80090010)`; no software substitution |
| Candidate B | **Proven by real Windows test:** two user keys plus public MachineGuid digest, live SID binding, separate-process challenge proof and cleanup |
| Two genuine Windows users | **Not tested:** no second authenticated user session was supplied; mocks and synthetic SID changes are not ACL proof |
| Reboot, elevated continuity, profile replacement | **Not tested:** require isolated manual validation below |
| TPM missing/disabled configuration | **Not tested:** this host reports an available hardware provider; no device/firmware configuration was changed |
| Unavailable provider | **Proven by real Windows test:** nonexistent provider returns `0xc0000225`; error remains provider-unavailable, with no fallback |
| Hardware attestation / physical TPM identity | **Not tested:** provider flags are local claims, not a remotely verified attestation |

`0x22` is a provider capability report, including software and virtual-isolation flags; no per-key VBS assurance is claimed.
TPM measurement deliberately reports signing capability with **ACL requirements unmet** and exits unsuccessfully; strict enrollment never accepts that key.
No SID, key blob, key-container name or machine identifier is included in the uploadable evidence report.

## Recommendation and Owner Decision

**Partially feasible.** User-bound CNG possession and synthetic context binding work without elevation on this host.
The approved installation-allocation model is not proven under the existing per-user installer: standard-token machine finalization fails, and Candidate B cannot supply equivalent installation proof.

Do not adopt B as an equivalent substitute or claim A/C work after an administrator creates a key.
A reviewed provisioning follow-up must first demonstrate protected stable machine registration, minimum key-use rights for the intended user, ordinary-user reopen/sign, and genuine User B denial.
No service, elevation helper, installer change or automatic provider migration is justified or implemented by this PoC.

Proposed Licensing Identity ADR amendments, pending owner review:

1. Record A as conditional on machine provisioning and genuine cross-user acceptance, rather than assuming standard-user creation.
2. Record B as a user-possession/risk-signal design with weaker installation semantics; changing allocation identity requires an explicit Owner Decision.
3. Separate software non-export policy, local hardware-provider evidence and remote hardware attestation; require explicit recovery for provider/key changes.
4. Require enrollment to pin M, U, the authorized SID reference and authenticated customer together; public descriptors remain untrusted inputs.
5. Leave provisioning/elevation UX, trusted descriptor discovery, least-rights ACLs, privacy/retention and hardware-failure recovery as unresolved production gates.

## Harness and cryptographic contract

The example is included in default Cargo tests but is absent from production `lib.rs`, startup and application packages.
Windows-only development features reuse the locked `windows-sys` version; no dependency or lockfile upgrade is needed.
Linux compiles the portable challenge tests and reports native execution as unsupported.

```powershell
. ./scripts/environment.ps1
cargo test --locked --example windows-device-identity
cargo run --locked --example windows-device-identity -- context
cargo run --locked --example windows-device-identity -- capabilities
./scripts/test-identity-poc.ps1
```

Run native tests from the ordinary, unrestricted Windows token, not a restricted automation sandbox.
The script records each native operation's exit code; an available provider does not imply successful key provisioning or acceptable ACLs.
Only `target/identity-poc-evidence/measurements.json` is uploadable; failed cleanup state must remain local.

Commands use an explicitly selected provider; there is no automatic fallback:

```powershell
$poc = './target/debug/examples/windows-device-identity.exe'
& $poc key-probe software user
& $poc key-probe tpm user
& $poc create B ./target/identity-B.json
& $poc check ./target/identity-B.json
& $poc inspect ./target/identity-B.json
& $poc probe ./target/identity-B.json
& $poc cleanup ./target/identity-B.json
```

A uses software machine M plus software user U; B uses two user keys plus public machine evidence; C requests TPM machine M plus TPM user U.
Each test enrollment gets random UUID containers; this intentionally does **not** implement globally unique installation discovery or cross-user enrollment serialization.
An existing public state path or key name is never overwritten.

The local challenge uses a fixed domain, big-endian schema version, five fixed 32-byte fields and length-prefixed UTF-8 customer/purpose strings.
Both keys sign the SHA-256 digest of the entire canonical challenge; the verifier pins distinct public keys and independently supplied expected context.
Only successful verification consumes the outstanding nonce; later reuse or context substitution fails.

Expected context must come from trusted enrollment/backend state in a future design; it cannot be trusted merely because it came from a public PoC file.
The current verifier has no network, customer authentication, real license issuer, lease or cross-process nonce database.
Fresh verifier instances require fresh nonces; reconstructing a verifier with an already spent expected nonce is outside this in-memory replay boundary.

## Security limits and failure handling

- SID references and MachineGuid digests are linkable public metadata, not secrets, anonymization or cryptographic installation evidence; keep state local.
- Copying public state cannot reproduce its original private-key proof; it can relabel a newly created synthetic identity because no trusted backend registration exists.
- Non-exportability is established by policy readback, not extraction testing, file/profile-copy resistance or privileged snapshot resistance.
- PoC owner/SYSTEM full-control ACLs simplify cleanup; they do not prove production minimum-use-rights provisioning.
- TPM ACL repair is diagnostic only, remains rejected on this host, and must not become an implicit weaker enrollment path.
- `NTE_BAD_KEYSET (0x80090016)` can mean absent or unreadable key storage; report ambiguity and recovery rather than inventing a corruption/deletion distinction.
- Corrupt public metadata, policy/scope mismatch, wrong token, changed machine evidence and missing signatures fail closed without minting replacement keys.
- Thread impersonation, anonymous, service and restricted token contexts cannot enroll identities; token handles and SID/security-descriptor allocations are released.
- Key-probe signs a fixed synthetic digest only; there is no arbitrary-data signing command, remote API or exported private key.
- Machine/key loss requires reviewed recovery; no Roblox credentials, backups or installed-user data are migrated.
- Only explicitly named PoC containers are deleted; cleanup verifies namespace, expected public key, current owner and ACL before deletion.
- After partial loss, cleanup attempts remaining matching keys and returns an explicit incomplete/absent-keyset error; preserve public state until reviewed.
- Initial sandbox/TPM exploratory cleanup failures were recovered locally; final namespace inventory found zero remaining owned user keys before regression tests.

## Reproducible manual Windows matrix

Use a disposable Windows test installation with two existing genuine accounts A and B; do not replace production profiles or change TPM/firmware state.
Build the example once, place its executable where both users can execute it, and record each user's sanitized `context` output.
Do not upload public state files, SID/machine digests, private-key files or any authenticated application data.

1. As unelevated A, run `create A STATE-A.json`, `create B STATE-B.json` and `create C STATE-C.json` using separate fresh paths; retain actual errors and never substitute providers.
2. For successful enrollment, run `check`, exit all PoC processes, start a new process and repeat; compare retained public key references locally.
3. Reboot the disposable machine, sign in as A, and repeat `check`/`inspect`; only this executed sequence can establish reboot continuity.
4. Copy **only** successful public state to a shared test folder; grant B read access to that file and executable without changing any key ACL.
5. Sign in genuinely as B. Run `probe SHARED_STATE.json`; require `owner_sid_matches=false`, then record raw native open/sign outcomes for M and U independently.
6. Run `check` as B; require user-mismatch denial even if machine public information is readable. If B exercises U, isolation failed; machine readability alone cannot grant authorization.
7. Sign in as A again, run elevated A's `context`/`check`, then unelevated A's `check`; verify the same SID and keys locally. Over-the-shoulder elevation as another account is a different principal and must be denied.
8. On a separately provisioned disposable machine, test TPM absent/disabled and unsupported configurations; record nonzero operations with no provider substitution. Do not disable TPM on the developer's machine.
9. Copy public state to another disposable installation and run `check`/`probe`; require evidence/key mismatch or unavailable original keys. Create a fresh identity there and compare public references locally.
10. In the disposable environment only, remove one **PoC-owned** key with Windows CNG tooling using its exact recorded name; require recovery, never automatic replacement. Run `cleanup` to remove the remaining matching key and retain any incomplete result.
11. Replace/copy a disposable test profile only from an isolated snapshot; test both same-SID profile replacement and different-SID import, and record provider-specific limits. Never claim ordinary file-copy resistance without executing this test.
12. Upgrade, uninstall and reinstall only a disposable RoLauncher test installation, without modifying its installer; run the external PoC before/after. CNG state is external to installation files, but retention is only code-inspected until this sequence runs.
13. As A, run `cleanup` for every successful state and remove local public files only after confirming deletion. Never enumerate/delete unrelated provider keys or manually remove cryptographic backing files.

Ordinary upgrades have no PoC hooks and cannot intentionally migrate/reset these keys.
Uninstall/reinstall therefore has no designed PoC cleanup hook; explicit cleanup or Windows/profile loss controls retention, subject to manual verification.
The experiment does not establish a production uninstall or recovery policy.

## Independent security-review checklist

- [ ] Confirm example-only reachability, unchanged production startup/installer/DPAPI, and zero version/release-intent changes.
- [ ] Trace effective token selection, denial without process fallback, allocation alignment and handle/allocation cleanup.
- [ ] Trace create/finalize/open/sign/verify/property/delete calls, random namespaces, collision handling and failure cleanup.
- [ ] Review public-file trust, canonical encoding, two-key pinning, purpose/customer/SID/evidence binding and nonce-consumption boundary.
- [ ] Reproduce reported machine permission denial and TPM ACL mismatch; distinguish provider capability from accepted enrollment.
- [ ] Execute genuine A/B ACL tests and reboot/elevated continuity before accepting those guarantees.
- [ ] Inspect ACLs for unexpected principals, inheritance and minimum-use rights; PoC owner full control is not production acceptance.
- [ ] Check provider absence, TPM loss, corruption and recovery errors never enroll replacement or weaker identities.
- [ ] Confirm CI artifacts contain only sanitized reports, never local public identity state, raw SID/HWID, private material or credentials.
- [ ] Require an Owner Decision and reviewed provisioning proof before using these findings in licensing implementation.

## Validation record

Approved test seams: offline challenge verifier, effective Windows-token identity and isolated CNG lifecycle/ACL harness.
Fourteen PoC tests pass: two portable encoding/context tests and twelve native Windows tests; none are ignored.
SID substitution and copied-state tests simulate hostile inputs; they do not replace genuine cross-user or copied-profile validation.

Red/green checkpoints covered canonical encoding, incomplete context, token retrieval, native signing, verified reopen/deletion, two-key verification, state persistence and partial cleanup.
Additional native negative tests first exposed anonymous-token acceptance and an unmapped provider-not-found status; both now fail closed.
The initial machine-success assumption was rejected by real `NTE_PERM`; the portable persistence contract tests B while A remains a recorded feasibility failure.

Existing local Windows regression passed: 111 default Rust tests, the separately invoked bootstrap test, 74 desktop checks and the WinUI smoke workflow.
Formatting, all-target Clippy and unchanged-version validation passed after implementation; final hosted evidence and exact source identities belong in the Draft PR body and retained review package.
Linux and hosted results must be read from those final runs, not inferred from local Windows success.

## Microsoft references

- [Persisted-key creation, scope and collision behavior](https://learn.microsoft.com/en-us/windows/win32/api/ncrypt/nf-ncrypt-ncryptcreatepersistedkey).
- [Key properties: provider implementation, scope, export policy, usage and security descriptors](https://learn.microsoft.com/en-us/windows/win32/seccng/key-storage-property-identifiers).
- [Property configuration](https://learn.microsoft.com/en-us/windows/win32/api/ncrypt/nf-ncrypt-ncryptsetproperty), [open](https://learn.microsoft.com/en-us/windows/win32/api/ncrypt/nf-ncrypt-ncryptopenkey), [sign](https://learn.microsoft.com/en-us/windows/win32/api/ncrypt/nf-ncrypt-ncryptsignhash).
- [Effective thread token](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openthreadtoken), [token information](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation).
- [TPM key attestation](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/component-updates/tpm-key-attestation).

Software protection, persisted non-exportability, a hardware provider flag and hardware attestation are different claims.
The local provider/property tests cannot authenticate those claims to a remote licensing backend.
An attestation service is outside this experiment.
