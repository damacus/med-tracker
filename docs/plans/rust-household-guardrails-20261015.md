# Rust household architecture and guardrails delivery

## Morning delivery scope: 16 October 2026

The owner has resumed work with a smaller delivery target: have a useful result
to demonstrate by morning, and hand off anything estimated above six hours of
Sol work or two human working days. This supersedes the pause instruction below
for this bounded delivery only. The accepted architecture remains unchanged.

The target is 09:00 Europe/London on 16 October, interpreted from "by the morning".
Deliver a reproducible household-inventory demonstration, screenshots and a
published draft checkpoint. Do not describe it as a production-ready migration.

The demonstration must show:

1. A real maintained browser sign-in and selection of an authorised household.
2. Creation and use of medication inventory without a mandatory storage location.
3. Recording a dose, exact stock reduction, saved history and persistence after
   refresh. Reads must not create further doses or deduct stock again.
4. Active-household selection restricted to current memberships, with another
   household's inventory inaccessible and revoked access rejected.
5. Desktop and mobile screenshots inspected by the delivery owner, plus a short
   walkthrough naming the synthetic fixtures, launch command and limitations.

Use the existing browser and restricted-role request fixtures. One Sol owner
controls source and verification; the delivery owner maintains this plan and
publishes the result. No additional review team is planned. Account usage was
checked: the weekly ordinary allowance is exhausted; no reset is authorised.

Treat the following as spending limits, not duration promises: allow 45 minutes
for inventory and first proof, at most two evidence-based correction attempts
per failure, and at most six hours of Sol work for this delivery. Reserve time
inside that limit for visual inspection, documentation and publication. Stop a
work item early when its prerequisites or risk make the threshold unrealistic.
Record the actual failure and hand it off rather than weakening its checks.

The history browser/download checks currently fail with HTTP 503 rather than
200. They are outside this demonstration and must remain visible as failures.
Do not delete their tests, claim they passed or add preparation-audit machinery
to rescue the overnight scope. The account-wide history API's earlier focused
results are separate evidence. Existing Android changes remain deferred locally.

Completion of this morning delivery means the listed demonstration has current
evidence and its code/docs are pushed. Required full CI and independent security
acceptance still govern any future ready-for-merge claim. Do not waive lint,
dependency or security gates to meet the demonstration deadline.

### Expensive work disposition

The items below are conservatively treated as above the owner's threshold
because they require unresolved product decisions, several security-sensitive
workflows or application-wide repairs. This is a scope decision, not a measured
runtime estimate. Do not start them during the morning delivery. Detailed
handoffs appear in the final section of this document.

- Complete personal-history browser/download security and acceptance.
- Canonical personal care plans and disclosure across households.
- Person-allocated stock transfers between households.
- Rust-native encrypted portability and a verified restore rehearsal.
- Remaining application-wide strict lint and dependency remediation.
- Extraction and full acceptance of independently releasable foundation PRs.

## Owner delivery reset: pause at the next gate

The stop gate has now been reached: both focused browser-history checks returned
503 instead of 200. The implementation thread acknowledged the pause. Only
preservation, stopping existing jobs/agents, and the handoff remain authorised;
do not repair the 503s or start another review or implementation gate. Completion
of the handoff and an idle implementation thread are still to be confirmed.

The owner has instructed this implementation to pause at the next bounded
verification gate so follow-up work can be planned as smaller PRs with controlled
scope and model usage. This supersedes the open-ended continuation and single-PR
publication sequence below. It does not cancel accepted product decisions.

Stop after the currently implemented personal-history browser/download path has
completed its already-selected focused verification and bounded security review.
Do not begin shared-plan implementation, stock transfers, further portability,
broad typing/lint refactors or additional audit infrastructure. If that focused
gate fails, record the exact failure and pause; it is not authorisation to keep
expanding the implementation until every adjacent requirement is satisfied.

Ask existing agents to stop at safe boundaries and report their patches, jobs,
ownership and unverified work. Preserve all changes. Commit/push only a coherent
stage that satisfies applicable publication rules. Do not bypass hooks or the
previous automatic approval rejection. If publication is blocked, preserve a
recoverable checkpoint, report the precise blocker and pause anyway.

The handoff must state local versus published HEADs, touched capabilities,
passing/failing/unrun checks, outstanding jobs, Android work preserved for the
separate change, and the smallest safe next PR. No new implementation gate starts
until the owner resumes it. The coordination thread is preparing the follow-up
PR boundaries; this is a pause instruction, not acknowledgement of a completed
pause.

## Proposed follow-up gates and PRs

Delivery: serial focused PRs against main by default. Use a dependent branch only
when its prerequisite cannot land yet and the review benefit exceeds repeated
rebases and CI. Do not build a speculative stack of every remaining capability.
The union of accepted product decisions is unchanged; sequencing is not a claim
that every decision must ship in the present PR.

| Gate / PR | Bounded outcome | Required evidence / stop condition | Excluded work |
| --- | --- | --- | --- |
| G0: checkpoint and inventory | Freeze the current work and identify what is published, committed locally, uncommitted, generated, or deferred Android work. Map changes to capabilities and identify hard dependencies. | Current focused run results, exact HEADs, safe snapshot, stopped agents/jobs and publication blocker. Stop even if the current focused gate fails. | New behaviour, broad cleanup, rewriting the working tree to force a split. |
| P1: independent guidance/tooling | Publish already-complete documentation and verification wiring that can be independently built and checked from main. Reuse existing commits and evidence. | Inspect actual prerequisites of tooling commit 67442452a and earlier guidance/lint commits; run only changed-area required checks on the extracted candidate. If coupled to unfinished runtime work, report that dependency instead of adding a compatibility layer. | Application behaviour, global lint fixes, advisory exceptions or hook bypass. |
| P2: household foundation | Account-owned profile; people in multiple households; current-membership-only active household; authorised creation/invitations/removal; household inventory with optional storage labels. | Real browser/API journey, restricted-role cross-tenant tests, revocation and stale-form rejection, invitation atomicity, original attributable dose/stock behaviour. Required CI applies. | New global history/download capability, shared-plan editing, stock transfers, auth-plugin replacement. |
| P3: own medication history | Account holders can read and download their own original history after leaving households, preserving original provenance and excluding other people's records. | Focused pagination/snapshot checks, all supported credential allow/deny paths, account/session revocation before release, audit failure fail-closed, zero stock/reminder effects, browser and response contract evidence. | General audit-platform work, checkpoint signer/evidence-export worker, importable portability, shared-plan authoring. |
| P4: shared care continuity | A person's approved current treatment, schedule, pause and latest administration status follows their household participation, with explicit person/guardian editing authority. | Agreed authority bootstrap and canonical timezone first; cross-household disclosure/departure tests, last-dose visibility, explicit stock matching and preserved local history. | Stock transfer, unrelated model typing, generic policy/framework replacement. |
| P5: allocated-stock transfer | Person/authorised guardian explicitly transfers allocated stock between valid household participations. | Agree allocation semantics first; exact quantities, dual-sided transaction/audit, replay and concurrency tests, communal-stock exclusion and cross-tenant denial. | Automatic moves, generic inventory redesign, legacy import. |
| P6: remaining verification hardening | Address measured remaining lint/dependency/portability gaps in separately bounded changes. Preserve the requested strict policy. | Each change names the actual findings and its completion check. Independent advisory blockers are reported, not made into an unbounded rewrite. Run full required checks and the owned fresh Rust save/restore rehearsal on the integrated candidate. | New domain features, broad cleanup without measured findings, security exceptions without explicit approval. |

P2 and P3 extraction boundaries are provisional until G0 proves they can compile,
migrate, preserve security and pass required checks independently. A historical
snapshot is not permission to overwrite newer work. If their code is inseparable,
retain one coherent foundation PR and narrow its completion criteria rather than
manufacturing unsafe intermediate schemas. Android stays in its existing separate
OpenSpec change; preserve its current edits without continued implementation.

For each resumed gate, record one actor/outcome, explicit included/excluded work,
one accountable writer, specific acceptance checks and the stop condition. Sol
owns implementation. Astra is reserved for one bounded security review of the
frozen relevant change, with correction-only follow-up for verified findings.
Do not spawn more workers or start the next gate automatically. Run focused tests
first, reuse unchanged evidence, and run required full gates on the frozen PR
candidate. Failed required gates prohibit acceptance/publication as applicable;
they trigger a bounded handoff, not permission for an application-wide refactor.

New discoveries are classified before action: required for this gate's correctness
or security, independently actionable existing defect, or future enhancement.
Only the first category expands current implementation. Record the other two for
prioritisation. After two failed fixes on the same issue, stop and report the real
SQL/HTTP/runtime evidence and smallest proposed next action.

No reliable human-time or token estimate has been established. The earlier 2–4
week estimate was not derived from measured agent throughput and must not be used
as a delivery promise. At each checkpoint report completed checks, remaining
specific failures and observed effort; estimate only the next fixed gate once
its actual scope and prerequisites are known.

## Objective and authority

Deliver the user's four-step cleanup/tooling/skills/PR request, including the
subsequently approved application-wide refactor. The latest owner decisions
below supersede earlier Rails-parity plans and the initial proposal. Record
architectural decisions as ADRs, not a new OpenSpec change.

Android implementation is deferred by the owner to the separate
`complete-android-household-client` OpenSpec change. Do not continue Android UI,
fixtures, emulator tests or generated-client integration in this refactor.
Preserve existing work and evidence for separation; it does not count as
completed Rust delivery.

## Global Constraints

- Branch: `chore/codex-skills-and-rust-guardrails`; worktree:
  `/private/tmp/medtracker-rust-guardrails`. One implementation writer at a time.
- Include all initial local changes, including deletions and untracked files.
  The original checkout remains untouched. Inspect and fix invalid resulting
  configuration or command examples rather than publishing known breakage.
- Loco 1.2 is primary; Tera/DaisyUI views; PostgreSQL 18. Rails is read-only
  reference code. No legacy data import, credentials migration, database parity,
  reverse migration or production deployment. Fresh-schema changes must never
  reset or silently adopt an existing populated database.
- Account holds authentication, profile information and preferences, and has
  exactly one person. A person optionally has an account. No separate user or
  profile table. People may belong to multiple households and exist without
  accounts. Minors do not have logins. Preserve existing capacity rules.
- A household owns medication inventory and stock. Membership permits viewing
  and recording household care; managers additionally administer membership.
  Remove the old per-person view/record/manage gate within a household.
- A person can create a household, or accept a manager's invitation to join.
  Adult person or authorised guardian approves sharing an existing person's
  care with another household. Do not infer guardianship from management alone.
- Active household is per session/device, restricted to current memberships.
  Every form/write remains bound to its original household after switching.
  Reject revoked membership at every request and mutation boundary.
- Locations are optional physical storage labels within a household, with no
  membership or permissions. Household inventory works without creating a label.
- Personal care follows the person. Current households receive current
  treatments, schedules, active pauses and latest administration status, not
  automatically the detailed history or narrative notes of another household.
  The person keeps access to full personal history and controls additional
  disclosure. On leaving, the old household keeps events recorded there and
  immutable attribution, but receives no future updates.
- Explicit stock transfer is initiated by the moving person or authorised
  guardian and limited to stock allocated to that person. Communal stock stays
  put. Transfer exact selected quantities atomically and idempotently; source
  and destination must be valid household participations. Never auto-move stock.
- Only the person or an explicitly authorised guardian may change a shared
  treatment plan's medicine, dose, schedule or pause. A household manager may
  explicitly and auditably match local stock to the prescribed medicine; never
  infer a match from medicine names or dose text. These owner-approved rules
  resolve the shared-plan edit and stock-matching questions; guardian bootstrap,
  personal-stock allocation and canonical treatment timezone remain separate.
- Reuse maintained security/protocol libraries. Preserve live permission checks,
  RLS, audit attribution, replay protection, atomic stock deduction and mail
  outbox. Better Auth has not been proved faulty; its organisation adapters must
  not create a second source of membership truth. Retain atomic domain operation
  boundaries wherever the pinned plugin cannot supply them.
- Typed DTO -> validated command -> operation -> typed result -> DTO boundaries
  throughout root src. Distinct entity ID newtypes, enum dose sources, typed
  conflict codes, parsed timezone-aware timestamps and exact Decimal math.
  JSON remains legitimate at transport/configuration/template/document-storage
  boundaries; no internal struct -> Value -> struct command passing.
- TDD for executable changes. Use Task commands and Fish/RTK. Do not alter code
  comments unless needed for requested guardrails/documentation or lint fixes.
  No broad lint suppressions, including in tests or generated entities. Fix every
  violation of the requested policy while retaining meaningful assertions.
- Preserve API/MCP contracts except explicitly superseded access/location
  semantics; document deliberate changes in OpenAPI, clients and ADRs together.
- No merge/deploy/live data actions. Commits and feature-branch push/PR are
  authorised. Verify author, committer and signing identity before every commit;
  use commit-local signing fallback only if 1Password is unavailable.

## Task 1: Record ADRs and install Rust-specific agent guidance

Create sequential ADRs after existing 0012, following docs/adrs conventions:
accounts/people/households; household inventory and personal care continuity;
authentication/membership/tenancy responsibilities; fresh Rust without legacy
compatibility. Record the exact global constraints, alternatives, consequences,
superseded old assumptions and accepted decision versus implementation status.
Keep unresolved organisation-plugin replacement details Proposed, with the
known pinned invitation-acceptance separate-write limitation and current secure
adapter. Do not claim new application behaviour already works. Link the ADRs
from design documentation and documentation navigation.

Remove obsolete CLAUDE.md, .claude and .jules; consolidate root AGENTS.md and
delete agents.md. Move Ruby/Rails-specific rules to rails/AGENTS.md, marking
Rails read-only. Root guidance must be Rust-only and require the three local
skills. Create .agents/skills/rust-loco-architecture/SKILL.md,
rust-migration-and-tooling/SKILL.md and rust-drift-audit/SKILL.md with the user's
requested names/descriptions and complete actionable rules, corrected for the
latest decisions: no legacy import/parity requirement, no duplicate profile
table, household membership replaces intra-household per-person grants. Retain
Rust-native portability side-effect invariants and property-test requirements.
Preserve and review all initial local changes, correcting invalid mise TOML and
README executable names. Retain legacy rust crates; document actual consumers
and superseded application status. Update narrowly affected tests/references
that require duplicate guides or removed files, first proving relevant failures.
Run documentation build, focused tooling checks and diff whitespace checks.
Commit all Task 1 changes only; leave lefthook recreation for Task 4 explicitly.

## Task 2: Implement accounts, household participation and active tenancy

Read the accepted ADRs and current Loco boot/router/test harness. Replace the
fresh identity/tenancy schema without retaining the users/profile chain.
Accounts have one unique person reference; people have optional reverse account
association and many household participations. Preserve account-level profile
settings. Make login registration/onboarding, household creation, invitation
acceptance, membership removal, household list/selection and actual browser/API
journeys work on this model. Keep an authoritative membership store and reuse
Better Auth for the standard capabilities it provides. Do not bypass atomic
domain operations for invitation acceptance or duplicate role rules. Expose
whole-household member care access and manager-only member administration.
Represent guardian authority separately from household role; fail closed until
an adult/guardian has approved sharing. Remove obsolete person grants only
after all callers, RLS and tests are migrated. Session/device selection must not
retarget open forms. Update fixtures and model mappings with schema changes.
Write red real-router/PostgreSQL acceptance for accountless minors, unique
account/person, multi-household membership, authorised invitations, invalid and
revoked selections, cross-tenant denial, manager/member roles, and transactional
failure. Exercise desktop/mobile UI with retained styling and save screenshots
for visible changes. Verify focused gates and commit. Do not claim care-summary
or stock-transfer support until Task 3 integrates it.

## Task 3: Implement personal care continuity, stock ownership and typed operations

Implement approved care-summary visibility and person-controlled historical
disclosure. Separate person-owned treatment definitions/history from
household-owned inventory, preserving original event household and acting
person plus account credential attribution. Leaving revokes future access,
retains local historical events, and preserves the person's own full history.
Make locations optional stock labels and remove location membership/access
logic, updating API/client contracts for changed semantics.
Implement explicit assigned-stock transfers by the person/guardian with exact
amounts, atomic dual-sided inventory/audit writes, concurrency protection and
idempotency. No communal stock appropriation, negative inventory or automatic
transfer when joining/leaving. Preserve dose replay, latest shared dose status,
atomic outbox and actor provenance across households.
Apply typed command/result boundaries and entity ID wrappers throughout root
application domains and callers, not just dose writing. Remove internal JSON
round-trips and stringly domain codes/timestamps/amounts. Centralise privacy-safe
DbErr classification/tracing with generic responses and correlated request IDs.
Move SQL-worthy filtering to query boundaries. Bound CPU-heavy blocking work
using app-owned semaphores and keep transports/operators scoped to AppContext
lifecycles. Keep Rust-native export/restore safe; do not add old-system import.
Write failing acceptance/property tests for data-sharing and revocation,
double-dose replay/stock, restore without side effects, DST transitions, transfer
atomicity/concurrency, cross-household isolation and privacy-safe diagnostics.
Verify real API/browser journeys and commit integrated functionality.

### Self medication-history implementation boundary

Deliver authenticated self reads and downloads of original medication
administrations across all origin households, including with no current
membership. Derive the Person from the validated Account; do not accept a
target Person or household selector or require adult capacity for self access.
The API accepts account-bound first-party mobile OAuth with effective
`medtracker` scope; browser routes use the maintained browser session. Reject
household-bound stored sessions and app tokens, personal keys and integration
credentials on this global clinical reader. The broader account-profile
transaction helper does not itself establish this authority.
Reuse the typed immutable-history decoder and retain the existing local-only
household projection. Use bounded keyset pages with the history-entry ID as
the tie-breaker, preserving original labels and attribution without disclosing
credential references or another household's narrative notes.

Use a separate account-owned clinical-access audit source. Account transactions
clear the Person setting, so SQL must derive the Account-to-Person relationship
rather than trust that setting. Restricted scalar appenders must construct the
audit context themselves; a source-specific trigger feeds the existing global
ledger atomically with a null household. Grant the existing
`med_tracker_audit_verifier` explicit source read access. Retain the existing
retention review mechanism; it does not establish production retention approval.

Build downloads from one consistent database snapshot into a private bounded
spool, then close that snapshot. Revalidate the same credential, commit the
successful access audit, and only then release bytes. Post-render revocation
must release no bytes or success event; a denial-only appender may use previously
validated identity and credential provenance without restoring clinical access.
Invalid token claims cannot supply audit identity. Test rollback, spool cleanup,
foreign pooled contexts, all-page completeness and zero operational side effects.
This is personal medication-history export, distinct from importable portability.

The root Rust app has no wired ledger verifier, checkpoint signer or evidence
export worker. Database source/ledger/outbox tests cannot establish that external
evidence pipeline; record the gap until implemented and independently exercised.

## Task 4: Enforce strict Rust linting and nextest across local and hosted gates

Add inherited workspace rust/clippy lints for root, migration and all client
tools packages. Forbid unsafe_code. pedantic warn priority -1; deny dbg_macro,
todo, unimplemented, float_cmp, mem_forget and valid loss/precision conversion
lints (the requested lossy_float_conversions is not an existing lint; use
cast_precision_loss, cast_possible_truncation, cast_sign_loss and
lossy_float_literal). Warn unwrap_used, expect_used, indexing_slicing,
needless_pass_by_value and lowercase implicit_clone. Fix all resulting warnings
including test/generated code; do not suppress them or neuter tests.
Install pinned cargo-nextest, cargo-machete and cargo-deny through Mise. Add
minimal deny.toml covering advisories, duplicate-version warnings and sources;
allow only actually required pinned Git sources. Dependency security fixes must
be proven, not ignored. Keep formatting non-mutating in gates.
Update root/client Taskfiles, CI workflows and recreated lefthook.yml to run
fmt check, clippy --workspace --all-targets --all-features -- -D warnings,
machete, deny advisory/bans/sources checks, nextest --no-tests=fail and separate
cargo test --doc. Adapt current owned-database/storage preparation, sharding and
compiled-artifact reuse rather than bypassing them or running tests twice.
Cutover rehearsal remains explicit and tests fresh Rust save/restore, with no
Rails compatibility requirement. Fail zero-test selections; preserve browser
acceptance and cheap-first stage order. Add failing runner/CI regression tests,
run focused gates then complete relevant suites, and commit.

## Task 5: Integrated review, acceptance and publication

Resolve cross-task findings, update docs/contracts and confirm accepted ADRs
match code. Run task ci plus client tools gates and docs build on a frozen
candidate. Run explicit fresh Rust backup/restore rehearsal against disposable
owned data. Compare visible desktop/mobile journeys, not just screenshots.
Obtain broad independent review with complete branch diff, ADRs and evidence;
fix verified findings and re-review changed boundaries. Review all initial local
edits are included without invalid config/executable examples. Commit remaining
in-scope fixes, push feature branch, create and verify one PR against main.
PR description explains behavioural changes and retained legacy rust consumers;
omit routine CI test sections. Report published/hosted/merged/deployed states
accurately and identify any unfinished required acceptance rather than marking
partial work complete.

## Detailed handoffs for work outside the morning scope

These are executable handoffs, not permission to start the work automatically.
Take one bounded gate at a time after owner prioritisation. Begin from the
published checkpoint `0a426de97` and its subsequent morning commits; compare
current files before applying any retained patch. Never restore a historical
snapshot over newer work. The checkpoint is a draft, not an accepted baseline.

Each owner must name the source revision, included paths, actual test command,
first failing assertion and stop condition before implementation. Keep one source
writer and one Cargo/browser runner. Preserve restricted runtime credentials,
PostgreSQL RLS, immutable clinical attribution and maintained authentication.
Use the repository Task commands and nextest's nonzero-test requirement. Do not
reset populated databases or introduce advisory waivers or broad lint allows.

### H1: Finish personal-history browser and download

Why deferred: the ordinary browser path already fails; secure download release
also needs concurrent credential revocation, historical denial attribution and
several database/HTTP/browser proofs. Treat the combined work as above six Sol
hours or two human days until a bounded diagnostic establishes otherwise.

Starting evidence and files:

- `tests/requests/self_medication_history_browser.rs`: two actual failures,
  HTTP 503 versus 200, run `d7582316-d04a-4d62-a4a9-a618ab4002a4`.
- `src/controllers/profile/medication_history.rs`, the shared response DTO in
  `src/controllers/medication_history.rs`, `src/models/identity/browser.rs`,
  `src/models/care/account_clinical_access.rs`, and the profile Tera template.
- [ADR 0017](../adrs/0017-personal-history-download-authorisation-and-audit.md)
  is accepted design with implementation acceptance pending.
- Prepared SQL test and browser drafts are retained in the pause checkpoint
  `/private/tmp/medtracker-owner-pause.OQ9Wrp`; they are not executed evidence.

Ordered gates:

1. Reproduce only the two failing requests using
   `task slice:test TARGET=requests FILTER=api_v1::self_medication_history_browser:: NO_FAIL_FAST=true`.
   Capture privacy-safe error categories and actual PostgreSQL blockers before
   changing locks. Five-second timing alone does not prove a lock timeout.
2. Make the smallest demonstrated correction and rerun that pair. Do not widen
   RLS, duplicate session lifetime policy or replace maintained session deletion.
   Stop after two unsuccessful evidence-based fixes and hand off the observations.
3. Add the real maintained-session hard-delete race before the historical
   preparation change. Prove the request reached preparation/release boundaries;
   a setup failure or 404 is not evidence for the missing security invariant.
4. Implement the fixed preparation/terminal functions from ADR 0017 only after
   the boundary failure is reproduced. Preserve the existing source and ledger
   transaction, minimal metadata, private typed handle, narrow owner access,
   database-enforced single terminal outcome and fresh exact-session validation.
5. Cover success, deletion before release, account disable, Account-to-Person
   reassignment, both lock orderings, source/ledger failure, cancellation and
   multi-page output. No clinical bytes may precede the committed release audit.
6. Integrate the retained browser draft with real fixture writers. Check the
   exact four-field version-1 envelope and allowlisted 21-field entries, two own
   households, foreign-Person exclusion and unchanged stock/reminders/history.
   Inspect desktop/mobile screenshots and refresh/download behaviour.

Done means the focused request, restricted SQL, adjacent profile/API contract
and real browser checks all pass on one frozen revision, with bounded security
review. A success audit proves authorised release, not receipt of all bytes.

### H2: Canonical personal care and controlled household disclosure

Why deferred: current treatments, schedules and pauses are still household-local.
This is a new clinical authority and concurrency model, not a rendering change.
Unresolved guardian bootstrap and canonical timezone decisions prevent a reliable
small estimate; classify the full capability above the owner's threshold.

Use [ADR 0014](../adrs/0014-household-inventory-and-personal-care.md), the current
care-continuity SQL and `src/models/care/{treatments,doses,pause_periods}` as the
starting map. Inspect actual modules before deciding migration boundaries.

Before implementation obtain decisions on who grants/revokes guardian authority,
how existing care plans acquire a canonical identity without guessing from names,
and the timezone governing calendar schedules. A manager is never automatically
a guardian. An elapsed-hours as-needed slice cannot count as complete DST support.

Ordered gates:

1. Specify a Person-owned clinical medicine identity and plan revision, separate
   from each household's inventory. Add failing tests for explicit authoring
   authority and denial of manager-only plan edits.
2. Implement one as-needed plan journey with explicit audited manager stock
   matching. Do not infer equivalence from medication names or dose strings.
3. Prove cross-household latest-dose/safety visibility and simultaneous writes.
   Serialise the same canonical medicine across plans without locking a foreign
   household. Check self-edit and household-write lock ordering with real blockers.
4. Add current treatment/schedule/pause projections only for active participations.
   Exclude historical narrative by default; test departure cutoffs and revocation
   after lock waits. Preserve each original administration's source and revision.
5. Add calendar schedules with timezone/DST properties, then the person/guardian
   browser authoring and household stock-matching journeys. Update external
   contracts with the deliberate semantics and inspect desktop/mobile output.

Done means the real A/B household journeys, person/guardian denial cases,
concurrency, departure, immutable audit and DST checks pass. Keep this separate
from stock movement and general account/profile changes.

### H3: Explicit transfer of person-allocated stock

Why deferred: allocation is not yet a proved domain invariant, and a transfer
touches two household inventories, permissions, replay and attributable history.
Treat the complete transfer capability as above the threshold.

Before coding agree how quantities become allocated to a person, who may allocate
or release them, whether partial allocations are allowed and what happens on
departure. Reuse Decimal quantities and canonical stock operations; do not model
allocation as an editable medicine-name label.

Ordered gates:

1. Write the allocation invariant and explicit transfer command: actor, person,
   source/destination memberships, stock identities, exact amount and replay ID.
2. Add failing tests for communal-stock exclusion, foreign/revoked membership,
   insufficient allocation, incompatible stock, zero/negative amount and stale
   authority. No automatic transfer occurs when a person changes household.
3. Choose and document deterministic household/stock locking order. Recheck both
   sides after waits, then apply debit, credit, allocation and both audit records
   in one transaction. Any audit failure must roll back the entire transfer.
4. Prove replay, conflicting replay, simultaneous opposite-direction transfers,
   precision and conservation properties using real restricted-role operations.
5. Add a review/confirmation UI that makes both households and exact quantities
   explicit, then verify saved balances, cancellation and desktop/mobile use.

Done means stock is conserved, each transfer is attributable and atomic, and
replays cannot create or deduct stock twice. No clinical plan edit is implied.

### H4: Rust-native portability and restore rehearsal

Why deferred: original provenance, departed-household history and complete fresh
restore have not reached acceptance. Crypto/resource limits, transactional apply
and real restore proof make this larger than an overnight demonstration.

Inspect `src/models/care/portable_imports`, export code, `tests/rehearsal/cutover.rs`
and the existing owned cutover tasks. The accepted package pipeline remains
decrypt, validate, authorise, preflight, apply. FHIR remains an interoperability
adapter and MCP remains a trigger. No Rails credential/data migration is planned.

Ordered gates:

1. Inventory every exported entity and provenance field against current Rust
   consumers. Define a versioned package fixture with multiple households, a
   departed person and original administration/plan attribution.
2. Reproduce missing round-trip facts before changing the format. Specify version
   rejection/upgrade behaviour; never silently reset an existing populated DB.
3. Bound application-owned blocking crypto with a semaphore, use maintained
   primitives and remove demonstrated redundant buffers without broad rewrites.
4. Apply historical records without current stock deduction, live reminders/mail
   or replacement of original clinical timestamps/actors by restore metadata.
5. Add replay and failure-injection properties proving zero operational effects
   and transaction rollback, then run the real owned save/restore rehearsal on
   disposable PostgreSQL and storage. Confirm the test selection ran actual tests.

Done means a fresh restore can be read through real API/browser consumers with
original provenance, no duplicate writes and no operational side effects.
Schema-only tests or a package parser passing are insufficient.

### H5: Remaining strict Rust lint and dependency repairs

Why deferred: the last measured strict root result spans hundreds of files and
thousands of diagnostics; two dependency advisories are independently unresolved.
It is above the owner's threshold and must not turn the demonstration into an
application-wide cleanup. Keep the requested strict policy unchanged.

Known advisory observations are `rsa` RUSTSEC-2023-0071 and `unsafe-libyaml`
RUSTSEC-2024-0449. Re-resolve the current lockfile and maintained upstream fixes
before choosing remediation. No waiver or unsupported cargo-deny configuration
has been approved. Prior investigation did not establish a released upgrade
providing the proposed expiry/parent-restricted ignores.

Ordered gates:

1. Capture fresh root and client-tools diagnostics separately with existing Task
   gates and retain machine-readable results grouped by owned capability.
2. Split deterministic, behaviour-preserving lint fixes into bounded changes.
   Start with reviewed retained patches only after checking their preimages;
   each patch still needs compilation and relevant regression checks.
3. Handle typed domain changes as separate behaviour-sensitive work with public
   contract tests, exact Decimal/timestamp handling and no internal JSON bridges.
4. Trace each vulnerable dependency to direct consumers. Propose the smallest
   maintained upgrade/replacement with compatibility and negative security tests;
   ask before any policy exception rather than burying it in lint cleanup.
5. Run fmt, all-target/all-feature Clippy with warnings denied, machete, deny and
   nextest with no-tests failure, plus separate doctests, on the frozen candidate.
   Exercise installed local hooks and the actual hosted workflow independently.

Done means those exact gates pass without broad allows or hidden exclusions.
Ordinary `cargo check`, selected tests or the absence of a local hook file do not
prove strict CI acceptance.

### H6: Extract and accept the household foundation for release

Why deferred: the published checkpoint spans 358 files and shares fresh-schema,
identity and consumer changes. Safe extraction and full acceptance may exceed
the threshold; it must not be attempted by arbitrarily cherry-picking files.

Begin with a capability/path inventory of the published branch against current
main. Separate guidance/tooling, household foundation, own history and deferred
Android. Preserve all source and snapshots. P2/P3 in the table above remain
provisional until independently compiled and exercised.

Ordered gates:

1. Identify the actual schema/API/runtime dependency closure for account-owned
   profiles, person memberships, active sessions and optional storage labels.
2. Try one coherent extraction in an isolated owned worktree. If it needs the
   history schema to boot safely, retain that dependency explicitly; do not add
   compatibility tables or weaken RLS to manufacture a smaller PR.
3. Prove create, invite/accept, current-membership selection, stale-form rejection,
   removal, account-owned profile and attributable dose/stock writes through real
   browser/API journeys. Include restricted-role foreign/revoked denial and
   invitation/audit rollback. Native regeneration is a separate authorised gate.
4. Refresh main before the final frozen checks, reconcile contracts and obtain
   one bounded independent security review. Address only verified findings in
   that capability; classify unrelated discoveries for separate follow-up.
5. Run the required full gates and publish a ready PR only after acceptance.
   Otherwise keep it draft and name the exact blocker. No merge or deployment is
   implied by the morning demo or by pushing a checkpoint.

Done means the extraction has its own real acceptance evidence and can land
without relying on uncommitted work or another PR's unpublished schema.
