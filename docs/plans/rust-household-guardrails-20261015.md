# Rust household delivery plan: ten outcome stacks

**Status:** Approved delivery plan; code acceptance remains pending. The ten rows are outcome groups, not ten mandatory GitHub branches. Use native ghstack only for real dependencies. Ship independent outcomes against main when they can be reviewed and verified independently.

## Authority, scope and completion

The accepted decisions in ADRs 0012–0017 and the owner refinements recorded here govern implementation; choices marked Proposed remain open. In particular, ADR 0015 does not approve wholesale removal or restriction of the Better Auth organisation plugin. Account identity, household authority, care continuity, stock and exact dose behavior are product decisions; this plan does not claim they already work. Preserve current Rust-native restore and sync contracts while adapting them to the accepted model.

A fresh Rust database is the target. Do not import Rails data or credentials, adopt a populated database, add an adoption guard, require Rails parity, or deploy. Preserve legacy Rust crates while they have active consumers. Do not add Android or iOS code, builds, generation or validation; new export/download products or export expiry; canary work; deployment; or legacy import. PR #2381, the open release item, is outside this plan.

Useful work from checkpoint PR #2735 still needs exact hunk accounting even though that PR has since been closed externally. The supplied audit snapshot covered 487 paths: 66 delivered, 14 explicit drops and 407 pending review. Those counts are historical, not current delivery state. Revalidate each path against current main. Every useful hunk needs a replacement or follow-up PR, or an explicit approved drop. A partial replacement never accounts for the rest of a mixed file. Keep the per-hunk evidence in the existing checkpoint ledger; do not infer delivery from filenames or issue closure.

Previously merged API, UI, profile, worker-expiry and Nextest #2770 work must not be recreated. Verify the exact current head and PR state before planning an overlapping change. Current-state decisions in the accepted ADRs are not evidence of application acceptance.

## Approved product contract

- An Account has exactly one Person; a Person may have no Account and may participate in multiple Households. Minors do not log in. Preserve existing Person type and capacity rules. Household membership grants household care access; manager authority administers membership only.
- When an adult creates a new accountless dependent, that adult declares themselves the initial guardian. Record a declaration, not verified legal status. A successor guardian must be authorised by an existing guardian. Disputed guardian recovery is deferred and fails closed. Household management never implies guardianship.
- A person or authorised guardian must explicitly approve sharing that person's care with another household. Only the person or an explicitly authorised guardian may change their shared plan.
- Households own medication inventory. Locations are optional storage labels, never permission boundaries. Care follows the person; detailed history and narrative notes do not automatically follow into another household. Preserve local events and immutable attribution after departure, with no future updates to the old household.
- Medicine matching requires the exact medicine, formulation, strength and stock unit. Do not infer equivalence from names or convert units. A stock transfer moves only an exact selected quantity, atomically and idempotently; communal stock stays put.
- Keep automatic notifications for dependants with explicit guardian authority and retain each account's self-preferences. Notifications about another non-dependent person require an explicit per-person opt-in by the receiving account. Household manager status alone never authorises notifications.
- Preserve replay protection, RLS, audit attribution, atomic stock deduction, mail outbox and privacy-safe error responses. Do not introduce temporary caches or trigger-based workarounds for these workflows.

## Ten outcome groups

| # | Outcome and ordered PR boundaries | Dependencies and acceptance |
| --- | --- | --- |
| 1 | **Browser identity and household selection.** Preserve split browser Cookie headers; add a per-session active-household selector; then bind OAuth consent to the selected household, account, session and permissions version. | Cookie handling is the first useful slice. Selection revalidates active membership and household lifecycle on every request; switching affects only that session and never retargets an open form. OAuth is a later dependent boundary. Test malformed/multiple Cookie headers, foreign/revoked selections, session isolation and consent expiry/replay. |
| 2 | **Canonical identity and household authority.** Keep the four identity PRs ordered: (a) household query/access-context preparation; (b) care and report household-context preparation; (c) canonical Account.person_id with global Person, removing redundant Person household/account and membership account fields together; (d) remove User actors only after real IDs flow end to end. | One-way schema and caller dependency: do not land generated entities or schema alone. Add the platform support/recovery SQL and actual caller UI, role and race fixes as an identity follow-up after the canonical schema, not as unused SQL or a schema-PR sidecar. Test account/person cardinality, accountless minors, cross-household access, revocation and transaction rollback. |
| 3 | **Optional location labels and retirement.** Make inventory work with no location label; then retire labels without location permissions or membership. Retiring a label detaches its medication association, preserves stock and historical references, and audits atomically. | Independent of a person's household membership. Update API/OpenAPI and current consumers together. No stock transfer or deletion is implied. Test no-location inventory, retired-label reads, name reuse, audit rollback and preservation of stock/history. |
| 4 | **Guardian declaration and succession.** Record a creating adult's declaration that they are the initial guardian of a new accountless dependent; support succession authorised by an existing guardian. Disputed recovery remains deferred and fails closed. | Depends on canonical identity. Guardian rights are separate from household role. Test declaration provenance, successor approval, manager-without-guardian denial and disputed recovery fail-closed behavior. |
| 5 | **Typed dose and person-care operations.** First add the exact-number Tera bridge with arbitrary-precision JSON and every affected renderer; then typed dose request/command/result; person-owned treatment plans; approved cross-household sharing with projection and departure behavior in the same boundary; and exact explicit stock matching. | Depends on identity and guardian authority. Keep request parsing, source provenance, replay conflict precedence, timing, persistence and stock writes within coherent boundaries. Use ID newtypes, a dose-source enum, Decimal amounts, timezone-aware times and typed conflicts; JSON stays at transport/configuration/template/document boundaries only. Do not infer stock matches or convert formulation, strength or unit. Test explicit approval and revocation boundaries. |
| 6 | **Person allocation and explicit stock transfer.** Represent person-allocated stock separately from communal stock; then allow a person or authorised guardian to transfer exact selected quantities between valid participating households. | Depends on identity, guardian authority and typed dose/stock operations. Writes and audits are atomic, concurrency-safe and idempotent. Never move stock on join/departure or transfer communal stock. Prove concurrent cross-household doses, rollback and no double deduction. |
| 7 | **Personal history and report denial audit.** Preserve the person's own history after departure from every household while keeping each household's local events and original attribution immutable. Add denied-report auditing with verified credential provenance and an append path that remains observable after the denied request rolls back. | Depends on identity, care history and credential context. Keep report route/payload adaptations with the model and audit contract. Test history after all departures, foreign narrative isolation, revocation boundaries, allowed/denied attribution and audit rollback. Do not add new download products. |
| 8 | **Notification preferences and worker rechecks.** First adapt account preferences while preserving automatic notifications for explicitly authorised dependants and requiring other participants' opt-in; then update worker recipient paging, revocation/lockout rechecks, stale-claim cleanup and audit behavior. | Depends on identity and guardian authority. Keep preference API/UI/RLS together, followed by restricted worker SQL/scheduler/cleanup and its tests. Worker queries expose only eligible recipient identifiers, not broad clinical/profile data. Test consent defaults, revoked membership, disabled/locked accounts, stale sending claims and atomic cleanup/audit. |
| 9 | **Existing Rust-native restore and sync.** Preserve current typed restore/sync behavior while adapting identity and operation references. Successful historical restore preserves original provenance and causes no stock deduction, live reminder or mail; invalid restore and sync validation also have zero side effects. | This is preservation and contract adaptation, not a new export product. Keep the client API contract already delivered by #2746. Retain existing Rust callers. Do not reject legacy portable links or dose provenance until an explicit compatibility policy covers the data and an alternative workflow. |
| 10 | **Dependency, runtime and strict quality gates.** First remove verified unused dependencies; then fix privacy-safe request tracing and DbErr classification, runtime/test lint findings and required generator reproducibility checks; finally unify strict local and hosted gates. | Add UUID/span/header correlation without clinical values. Keep app-owned resources within AppContext. Preserve meaningful tests and generated-code behavior. Use one Rust compiler at a time. Forbid unsafe_code; warn on pedantic at priority -1; deny dbg_macro, todo, unimplemented, float_cmp, mem_forget, cast_precision_loss, cast_possible_truncation, cast_sign_loss and lossy_float_literal; warn on unwrap_used, expect_used, indexing_slicing, needless_pass_by_value and implicit_clone. Do not configure the invalid lossy_float_conversions lint. Run `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`, `cargo machete`, `cargo deny --workspace --locked --all-features check advisories bans sources`, `cargo nextest run --locked --workspace --all-features --no-tests=fail` and `cargo test --locked --workspace --all-features --doc`; keep documentation, browser and Rust-native consumers in their owning gates. |

A row may contain several pull requests only when each boundary leaves a working behavior or useful, verified preparation and the next boundary has a real dependency. Do not split API, UI, migration, or tests into layers that cannot run together. Keep fixes with the owning contract; do not add upstack workarounds.

## Mixed checkpoint hunks and ownership

The checkpoint report is path-oriented and many changed files mix behaviors. Assign and review hunks as follows:

- Identity/access preparation owns only household/account context changes in shared callers. In particular, account identity changes inside report controllers/models precede the report-specific changes in outcome 7.
- Outcome 3 owns optional-label and retirement hunks in location models/controllers, location foreign keys, fixtures and browser/API tests. It does not own guardian, membership or stock-transfer hunks in the same files.
- Outcome 5 owns the number-to-Tera adapter and renderers, dose command path and person-plan/current-status consumers. The dm+d importer delta is style-only; its controller template bridge belongs to the exact-number preparation. There is no new dm+d feature.
- Outcome 6 owns only explicit allocation and transfer behavior. It does not own dose recording or location labels merely because they share inventory files.
- Outcome 7 owns the report denial snapshot, SQL append capability, report handlers and audit-isolation regressions. History schema/writer invariants remain with the immutable-history boundary; report projection adaptations remain with identity where they only replace household/person context.
- Outcome 8 owns preference/subscription rows, browser/API behavior and scheduler/cleanup functions. Account and household foreign keys remain with canonical identity; do not mark the whole notification SQL file delivered from one hunk.
- Outcome 9 owns portable import/sync adaptations and zero-side-effect properties. Existing Rust restore code remains active. Dedicated exports, downloads and export expiry are deferred; do not turn the checkpoint's candidate import rejections into an unapproved compatibility break.
- Outcome 10 owns unused-dependency work, error/request tracing preparation, lint fixes and generator reproducibility tooling. Generated entities belong with the schema or behavior that consumes them. Fresh-schema wiring remains with its owning feature; the rejected populated-database adoption guard and its regression stay dropped.
- Attach notification-delivery and record-lifecycle operational documentation to the behavior it describes. Keep proposed medication/person retirement and unsupported location reactivation documented as proposals. Screenshots support the matching UI outcome and never count as behavior on their own.
- Preserve the seven historical reports, Serena configuration, Docker pointer guide and product spelling decisions recorded in the checkpoint dispositions. Do not edit repository skill definitions; use the maintained global skills.

For every mixed file, compare the exact hunk to current main and record the owning outcome plus replacement, follow-up or explicit-drop evidence in the existing #2735 accounting ledger. This plan does not change that ledger or manufacture new drop decisions.

## Shared acceptance and release control

### Canonical identity implementation checkpoint

The controller has released the coupled canonical identity cut on the accepted
query preparations: mandatory unique `Account.person_id`, global Person, and
removal of `Person.account_id`, `Person.household_id` and
`household_memberships.account_id` together. Participation is unique per household
and Person and permits accountless people. Signup reserves a new server Person ID
and inserts the Account and Person atomically under a deferred restrictive FK.
Existing-account invitation acceptance reuses the canonical Person. No identity
claim, email matching, temporary cache, compatibility view or fabricated
Account/User is authorised. Actual User IDs and audit references remain until
the separate actor-removal slice.

The bounded security review approved own-Person and independently authorised
household-participant SELECT policies, plus the maintained support exception.
The reviewed support SQL SHA-256 is
`75aa1d1f32c4c0777139c384fd278e8d7bc3cd8db87f2a9dcb3b2599a523798d`.
Only the approving owner's row is visible before activation; the live roster
requires activation. Keep approval capacity/version, deadline and caller
rechecks, and retain all support write/export denials. The graph must remain
acyclic, with no blanket administrator visibility or membership impersonation.

The approved recovery eligibility SQL SHA-256 is
`5d8efa10815fb07c0b8477aa9326fd8e31935f24662a398dde4fe97d4fcac65b`.
Its fixed-owner, fixed-search-path function returns only a specified membership's
eligibility in the current household under live administrator checks. The owner
policy does not apply to the app role. Existing recovery authorisation, self/role
restrictions, locks, post-wait rechecks and atomic promotion/audit remain required.
This minimal caller conversion does not deliver the later recovery-hardening
follow-up.

Architecture and SQL design approval are not runtime acceptance. Focused local
restricted-PostgreSQL proofs cover accountless participation in two households,
foreign denial and live revocation; existing-account joining with unchanged
Person identity and replay; genuine invited-signup creation provenance and audit
rollback; and all eleven clinical references to retained participation, including
executed foreign-subject rejection and departed-history retention. Member
rosters, dose replay, canonical self plan editing, capability/assignment selection
and stock creation pass without Person grants, while other-person plan editing
and new manager-created authority remain denied. The rejected grant write retains
its domain audit, ledger and delivery atomically. Retained grant and membership
response schemas pass with unavailable profile fields and actual historical User
IDs preserved. Null/relinked membership Person IDs are rejected while unchanged
IDs remain accepted.

The final integrated review found two remaining grant-dependent readers. Actual
medication-detail and report-selector routes reached focused failures; their
participation-based corrections pass both focused browser-route regressions.
The retained relationship label/CSRF removal and immutable membership contract
regressions also pass (four actual tests across the two focused runs). Detailed
runs and logs are recorded in `/private/tmp/medtracker-global-person-boundary.md`.
Maintained support (32 cases) and recovery (18 cases) runtime proofs pass. The
new recovery helper also passes fixed-owner/search-path/ACL/FORCE-RLS and
restricted no-roster-access/denied-context checks. Mandatory/unique Account-Person
and participation constraints, deferred missing-Person commit rejection and atomic
provisioning/rollback pass. The existing expiry suite has twelve unchanged passes
and its one corrected canonical revocation fixture passes separately. Fresh
catalogue equality now passes against an explicit canonical identity delta over
the retained historical catalogues. SeaORM 2.0.4 generated the affected identity
mapping; its repeated composite-target-column limitation was corrected only for
the eleven participation relations against the actual FK order, with a real
restricted relation JOIN proof. Adjacent signup, invitation acceptance, profile,
account-session and credential checks pass after canonical fixture corrections.
Membership-only invitations reject obsolete care-authority fields, including
array, object and null forms, before any invitation, grant or mail effects. Their
standard mail worker and reached enqueue-failure rollback proofs pass separately.

The adjacent fixture pass has sixty-one valid distinct cases:
schedule and location boundaries, avatar ownership and post-wait revocation,
household-local self metadata, retained carer reads/removal and administration.
The saved-denial fixture now proves cached 403 replay with both explicit and
absent client request IDs: the replay marker remains present, each error ID matches
its own response header, normalising only that ID preserves the envelope, and no
additional rejected-domain event is written. The earlier ambiguous combined
assertion remains historical evidence, not a current missing-header claim; no
production replay correction was needed. A separate real detail-page regression proves the authorised relationship
read/removal link was hidden by the blocked assignment capability; its narrow
read-capability correction passes the full existing read/removal/denied-write
regression (run `acb4ff1c-e3bf-4073-a9e3-0d136188bc3b`). The smallest
maintained adult Person creation request reaches 500: creation still lacks subject
participation and produces new Manage authority. Do not add participation merely
to preserve that authority writer; this and the existing dependent-creation path
remain isolated pending the owner decision. The existing portable-import request
also reaches 500 after its test-only ownership query was corrected. Its production
Person index/upsert/snapshot still assumes household ownership and its importer
link creates or escalates Manage. That coupled compatibility repair is paused
pending the same authority decision; no automatic grant has been restored.

Real CSRF care-route negatives pass for ordinary-member removal and person-scoped
parent invitations, including unchanged authority and zero queued jobs. Maintained
invitation browser journeys pass six desktop/phone cases. Existing-care browser
journeys pass all eight case/project combinations across the retained five valid
results and four corrected focused results (one overlaps). The first attempt
stopped at an ambiguous H1/H2 locator; later test corrections preserve stale-version
409 separately from fresh-version restore 403. A synthetic PostgreSQL fraction
`.4518` versus Chrono `.451800` was normalised without Date conversion. No
application permission change followed from these test prerequisites.

Desktop/phone light/dark care screenshots and invitation/acceptance screenshots
are preserved under `docs/screenshots/canonical-identity-20261017/`, with hashed
originals and runtime reports in the private boundary record. Comparison against
the existing Rust screenshots confirms the deliberate membership-only and
read/removal-only controls; old image baselines remain unchanged. Existing
navigation differences and the invitation full-page fixed-bottom-bar capture
caveat belong to the separate UI checkpoints. This is not rendered Rails parity.
The existing contrast samples pass, but do not include the red Remove carer
button. Its contrast still needs an actual measured C4 acceptance check; these
screenshots and samples do not prove every control meets the contrast target.

Strict all-target/all-features Clippy passes on the mechanically formatted
candidate after removing three redundant Expr conversions. Formatting and the
documentation build pass. The dependency check runs and reports only the two
inherited unused dependencies, root `csv` and migration `serde_json`; neither
was copied from another PR or suppressed. These local checks do not resolve
the known creation/import failures.

Remaining gates are residual adjacent compatibility checks, inherited dependency
cleanup through its existing slice, creation/import disposition, delta freeze
and required full CI/browser acceptance. Full CI remains
incomplete while the two maintained creation/import paths are known failures.
Earlier all-target compilation is not acceptance of the current complete candidate.
Manage-producing writers must not infer guardian authority from a household role
or relationship label. The owner choice for the existing Add dependent flow is
still pending: no new declaration or claim flow is authorised by this checkpoint.

The user explicitly holds publication. Keep all canonical changes local; no
commit, push, draft PR, merge or deployment follows from local evidence alone.

### Separate checkpoint brief: initial dependent guardian declaration

The owner approved a separate declaration checkpoint on 2026-10-17. This is a
plan for a new accountless minor or dependent adult only. It does not activate
succession, disputes, claims or cross-household sharing from the wider outcome 4.
No production or executable test change is authorised by this brief alone.

Current writers explain the remaining failures: `people/creation.rs` creates a
Person without participation and automatically creates creator Manage, including
for capable adults; dependent creation additionally infers a family relationship
and synthetic self grant. `portable_imports/identity.rs`, `writing.rs` and
`links.rs` retain household-owned Person queries and automatically grant or
escalate importer Manage. Neither writer records an explicit declaration.

**Proposed complete journey.** Retain the existing creation entry permission
(household manager or existing valid Manage authority), then independently require
the authenticated canonical actor to be an adult with capacity, with live account,
credential and household participation. Entry permission is not guardian proof.
For a NEW minor/dependent adult, the existing Tera form presents an unchecked,
required attestation: “I declare that I am this new person's initial guardian.”
Explain that this records a declaration, not verified legal status. Existing
Person IDs, email matches and manager-selected relationship labels cannot satisfy
it. Missing/false/null/wrong-type declarations fail validation without writes.
Validate kind, age and capacity together: `adult` plus `has_capacity:false`
cannot bypass dependent eligibility. This is a proposed invariant to prove,
not acceptance of the current validator's classification.

Use one authoritative, versioned declaration record linking the new subject and
the principal-derived guardian Person, originating household, server-owned
statement version, declaration time and actual actor/provenance. The exact table
and restrictive/RLS constraints need the bounded schema review before coding.
Create Person, its initial participation, declaration and any displayed existing
carer relationship in one transaction with attributable versions, domain audit,
ledger and supported change event. Do not create an Account, User, credentials,
synthetic self grant or independent new Manage grant. For these new records,
Manage and automatic dependent reminders derive from the same current explicit
declaration; a relationship label is display data, never an authority source.
Local clinical actions additionally require current actor and subject participation.

Household suspension/departure denies local actions without rewriting the personal
declaration or implying that a household role determines guardianship. The
existing manager-authorised Remove carer action must not automatically withdraw
personal guardian authority: a household role cannot remove that global authority.
Its eventual effect on the local care link and notification eligibility must be
explicit and atomic without falsely rewriting the declaration. The owner has
confirmed guardian-only personal withdrawal and household-local manager removal.
This bounded checkpoint includes that guardian's withdrawal, not succession,
disputes, claims or recovery.
Existing unrelated valid authority/removal behavior remains intact.

Browser, API and MCP reach the same typed validated command. Proposed create-only
field is `person.initial_guardian_declaration: true`; the statement version and
guardian identity are server-owned. Preserve existing update and unrelated
unknown-field contracts. Update the existing Person POST OpenAPI contract, Tera
labels/errors and five locale files together. The maintained generic Rust client
and MCP resource writer must forward the field without a second authority path;
add adapter contract proof rather than empty source edits. No native work.

**Bounded acceptance and ownership.** Start with real restricted-App REDs for
unchecked/forged attestation and the current missing-participation create, then
prove minor/dependent-adult save, refresh, actual guardian plan edit and automatic
reminder eligibility; manager/non-declarant View/Record but no Manage; minor or
incapable declarant denial; foreign/current-person injection denial; revoked
actor/subject and post-wait checks; same-key replay exactly once and changed-body
conflict; reached declaration/audit/queue failure rolls back every write. Add
restricted RLS/FK/immutability proofs and desktop/phone light/dark keyboard/error
screenshots, including actual Remove-button contrast. Owned boundaries are
`care/people/{creation,validation}.rs`, a purpose-specific declaration model and
fresh schema/entity mappings, exact access/notification/removal consumers,
existing people browser/API controllers and form, OpenAPI/client-MCP contracts,
and existing people/care/browser test modules. No general guardian framework.

**Owner answers, 2026-10-17.** Capable adults join through their own Account;
manual creation and import must not create a capable accountless adult. Each NEW
dependent introduced by import requires a fresh current declaration from the
authenticated actor. Existing records require current authority; file contents,
uploader status, imported grants and relationship labels cannot supply it.
An import declaration is a new attributable security event, preserving original
clinical timestamps/provenance and producing no operational reminder or mail
side effects. Only the guardian can withdraw their personal declaration;
household managers can remove only the local care link. These are settled product
answers; the new schema/RLS and exact commands still need bounded security review.

**Small-stack acceptance proposal, requiring review.** Keep the two accepted
query preparations, then the canonical identity PR, then this declaration PR as
its mandatory child. The lower identity PR must reject non-declaring dependent
creation through a typed validation response with zero effects, rather than
retain the current 500 or grant authority. Its form must explain that the separate
declaration workflow is unavailable at that layer. Its tests would deliberately
prove this safe boundary; they must not claim successful dependent creation.
The child adds the complete declaration read/write/replay journey and restores
that capability. Final release must contain both. This is a deliberate temporary
contract change for owner approval, not permission to rewrite existing success
tests merely to make the lower PR green.

Imports must implement the settled fresh-declaration/current-authority rules
before the complete stack can claim acceptance. The smallest safe lower-layer
option is a typed, zero-effect preflight
denial for Person creation/authority-changing imports, with an explicit mandatory
declaration child that restores the approved existing restore workflow. Do not
delete the importer, relabel that denial as working portability, or indefinitely
land the lower layer without its child. Unaffected already-authorised import
operations must retain their existing behavior if separated safely. If the owner
does not approve the temporary boundary, keep the canonical and declaration work
stacked locally until the complete import child is accepted. No branch or PR
topology has been changed; the controller will manage native gh-stack publication
when the hold is released.

The actual rewrite is dirty local source above prep2 HEAD
`98c0c4fd9cd0e25dda6b04a50cc012f9a7824393`, not an accepted rewrite commit. Its
pre-brief tracked binary diff SHA is
`7cfda2e6a287022cd5979ce687c3a20da01e3959387ce6943b917e44c3e764e5`;
that hash excludes untracked files and subsequent brief-only edits. The current
15-file untracked manifest (sorted path/SHA256 JSON) hashes to
`4eb4f64e8ff32cd4026f531d387440ea9d3a68661d5b1bf8f6e8c2766af6bdde`:
ten screenshot proofs, the canonical catalogue, recovery eligibility SQL, two
global-Person test modules and the existing-carer fixture. Existing screenshot
archives retain the image bytes/hashes. Member management and final canonical
acceptance remain blocked on the actual creation/import disposition and required
whole-candidate gates. Publication is held; this brief has no runtime acceptance.

**Read-only consumer map for the eventual reviewed command.**

- `care/person_carers.rs::change` and
  `controllers/people/carer_relationships.rs` implement actual CSRF/version-bound
  removal, with household management, live target and stale-version checks.
  `care/administration/delegation.rs::deactivate` locks the local relationship,
  revokes only its linked grants, changes `active` and writes its version;
  `reactivate` calls the currently blocked `assign`. None proves permission to
  withdraw a personal declaration.
- `access.rs::{can_access_person,has_person_access,people_with_access}` and
  `authorization.rs::{own_person_manage,granted_person_manage}` cover direct and
  aggregate Manage. The separate grant-based `access.rs::managed_people` feeds
  `care/care_warning.rs`. New explicit authority must reach these consumers
  consistently while preserving grant-free View/Record and current self authority.
- `notification_preferences/managed.rs::{read,update}` selects live Manage grants
  and active subject participation; non-dependent opt-in is stored on those
  grants. `care/browser_push/reminders.rs` consumes that projection for subject
  eligibility. `reminders/delivery.rs::claim` opens a fresh access transaction and
  re-runs `plans` before delivery. The eventual declaration projection must keep
  automatic authorised-dependent notifications distinct from adult opt-in and
  household roles, and deny the correct queued delivery after local removal,
  departure or authorised personal withdrawal without deleting history or
  unrelated recipient preferences.
- `care/people/creation.rs` is the manual writer to replace; its
  `ensure_self_grant` and unconditional creator Manage are not declaration proof.
  `portable_imports/links.rs::grant_importer_access` is a separate unresolved
  writer. `care/profile_export.rs` exposes managed-preference data and needs
  truthful projection alignment if its storage changes. No restore may rewrite
  original clinical provenance or trigger operational reminders.

This map identifies current coupled consumers only. Guardian-only personal
withdrawal is included in the approved checkpoint scope; its proposed command
and new schema/RLS still require review before executable changes. Succession,
disputes and claims remain out of scope.

Every PR must prove its own behavior or useful preparation. Required evidence includes replay without duplicate writes, concurrent cross-household dose safety, audit rollback, membership/guardian revocation boundaries, personal history after all departures, restore with zero side effects, DST boundaries and desktop/mobile web screenshot comparison for visible journeys. Use real PostgreSQL/RLS and actual account, guardian, member and manager roles; do not substitute administrator credentials for runtime paths.

Strict quality gates must run against the exact reviewed head. A cancelled, skipped or empty test selection is not a green result. The root delivery controller alone authorises merges, after review and required green checks on that exact head. Publish focused PRs only through the authorised delivery workflow. Deployment, canary activation and destructive cutover remain separately unauthorised.
