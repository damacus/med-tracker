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

Declaration-only test preparation now lives in the dedicated
`/private/tmp/medtracker-initial-guardian-declaration` checkout. Its staged parent
tree is `34bf716bc6d3fc4ffc5da4445160ea11a59ef7ae`, uncommitted and unchanged;
the declaration delta is unstaged/new files. Proposed design and unapplied SQL
are frozen for a NEW bounded security review at
`/private/tmp/medtracker-initial-guardian-design.md` (SHA256
`881d15ae05cc4d26832cde9524823f83c275312f830296206bb431d8c661111c`)
and `/private/tmp/medtracker-initial-guardian-proposed.sql` (SHA256
`6574710f3076283d280e33a0bb43f56f95e3ebe3a3800d073ad63200b7fb099a`).
No declaration production implementation or SQL has been applied.

The new bounded Astra review is now conditionally approved and the controller
has released the reviewed implementation after the actual REDs below. Preserve
the original packet hashes/copies; this is design approval, not runtime acceptance.
The corrected pre-provider guard takes Account FOR NO KEY UPDATE, Household
FOR UPDATE, then exact declaration FOR SHARE with explicit five-second lock
timeout. Refresh subscription, current authorised intent/payload and claimed
attempt/deadline after those locks; hold through the existing bounded provider
attempt and explicitly finish the guard before existing complete() opens its
transaction. Do not alter the shared Account FOR UPDATE helper or upgrade later.
Keep uncertain-send semantics and prove both lock orders, including implicit
audit-FK Account KEY SHARE. Current delegate_person is denied by Cedar;
assign/reactivate are not a demonstrated bypass.

Authority must use positive visible declaration-backed active local links OR
existing valid legacy grants. Never infer legacy status from absence under RLS.
New declared subjects receive no grants through manual creation/import; keep
direct creation and delegation denied, inspect all retained grant writers, and
prove hidden declarations in another household supply no Manage or reminders.
No marker/global visibility helper is authorised. Fresh import repeats that
include declarations for now-existing People are rejected; authorised repeats
omit declarations and recheck existing-record authority. Current import has no
idempotency ledger: do not add one. Preserve record/stock idempotency. Global
guardian withdrawal remains first-party/own-only after last household departure.
All later rollback, live-age, RLS/ACL, closure, foreign/revision/credential,
connection-reuse and concurrency acceptance remains required. The staged parent
tree is immutable and every declaration change remains unstaged or untracked.

Authorised initial HTTP RED handle9835 terminated201: nextest
`3f55b85f-6054-45b9-bdbf-0ee2f3597cf5`, three actual tests, zero passes.
Missing attestation returned500 vs422; true manual declaration500 vs201; valid
two-dependent external import declarations422 vs201. Setup/compilation succeeded;
new-table queries are guarded behind the unreached successful201 assertions.
Later invalid-identity cases now use distinct replay keys; those unreached tails
have no runtime acceptance yet. Log:
`/private/tmp/medtracker-guardian-initial-http-red.log`.

The maintained ApiClient import-shape RED handle1709 terminated201: nextest
`2c5af9ed-8bcd-49ac-916c-3be4b59dc59c`, one actual wiremock test failed with
`bundle_envelope=false`, `passphrase_header=false`, `body_secret_absent=false`.
No request/password values were logged. The focused wrapper is
`/private/tmp/medtracker-declaration-client-red.yml client-red`; it mirrors the
maintained client test command with an exact package/lib/filter and fail-on-empty,
not the complete client gate. Log:
`/private/tmp/medtracker-guardian-client-import-shape-red.log`.
All jobs are terminal and compiler/browser leases released. No client production,
schema or credential-scope fix follows before the declaration review approval.

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

Browser and API reach the same typed validated command. Proposed create-only
field is `person.initial_guardian_declaration: true`; the statement version and
guardian identity are server-owned. Preserve existing update and unrelated
unknown-field contracts. Update the existing Person POST OpenAPI contract, Tera
labels/errors and five locale files together. Current MCP is read-only (list,
export and sync); no manual/import write tool is being invented. The maintained
ApiClient import method needs the separately proven request-shape correction and
current external declaration inputs. Normal MCP household/personal-key/integration
credentials remain excluded from global withdrawal; no widened authentication or
cosmetic adapter changes. No native work.

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

### Guardian declaration review and first implementation stage, 17 October

The owner approved the separate bounded Astra review. It conditionally accepted
the proposed declaration schema and authority design, requiring the following
notification corrections before acceptance: Account `FOR NO KEY UPDATE`, then
Household `FOR UPDATE`, then the exact declaration `FOR SHARE`; an explicit
five-second lock timeout; refreshed subscription, intent, attempt and deadline;
and release of the guard before the existing completion transaction. Sent bytes
cannot be rolled back. Preserve uncertain-delivery handling.

Do not infer legacy status from an RLS-hidden declaration. New declared People
must receive no legacy grant through manual creation, import or restored
assignment. Existing assignment remains denied by Cedar. Authority may use a
positive valid declaration/local-link predicate or a genuinely existing valid
legacy grant. Import repeats containing declarations for now-existing People
are rejected; authorised repeats omit those declarations. No import replay
ledger or MCP write tool is introduced.

The retained Sol owner implemented the first local manual-creation stage, then
repeated model-capacity failures stopped its turn. The controller temporarily
took sequential ownership. The staged parent tree remains
`34bf716bc6d3fc4ffc5da4445160ea11a59ef7ae`; child changes remain uncommitted.
At that first-stage freeze, imports, client transport and reminder production
changes were not implemented; the later narrow import/client result is below.
The browser declaration form and all later authority/concurrency proofs remain.

Verification is incomplete:

- Job `76653` stopped at compilation; no tests executed.
- Job `50057` selected two tests but both stopped in database template setup.
  The function-creation role lacked schema CREATE. No HTTP assertions executed.
- The corrected role-separated migration passed on fresh disposable PostgreSQL
  through job `93109`; log:
  `/private/tmp/medtracker-guardian-migration-preflight.log`.
- Controller job `20431` stopped before tests because the global Rust build
  cache is on the full external volume (32 MiB available). Log:
  `/private/tmp/medtracker-guardian-manual-controller.log`.
- Controller job `43742` used the existing local target as the build directory,
  but this caused a dependency rebuild. It was deliberately interrupted before
  consuming the remaining local disk; terminal exit 130, no test result. Log:
  `/private/tmp/medtracker-guardian-manual-controller-local-cache.log`.

The immediate cache-recovery prerequisite was subsequently completed as recorded
below. These stopped commands are retained as historical evidence, not current
acceptance. Publication remains on hold, and the seeded review website has not
been restarted or migrated.

### Cache repaired and manual tests passed, 17 October

The owner authorised cache repair and retry. No Rust compiler or Nextest process
was active before cleanup. Removed only the two regenerable incremental caches
`81/b22ffe18fecb58/debug/incremental` and
`13/c0b1df432232ab/debug/incremental` beneath
`/Users/damacus/Library/Caches/rust-builds/intermediates/`.
External-volume free space increased from 32 MiB to 21 GiB. Compiled dependency
artifacts, application binaries, worktrees and the review website were retained.

Recovered the retained implementation owner's original command environment:
`CARGO_HOME=/private/tmp/medtracker-rust-tooling/cargo-home`,
`CARGO_TARGET_DIR=/private/tmp/medtracker-session-household-selection/target`,
`CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`.
Retried the maintained Task selector
`task slice:test TARGET=care_api FILTER=guardian_declarations_manual_create NO_FAIL_FAST=true`.

Controller job `7729` completed with exit 0. Nextest run
`fb7ec9d7-e53e-4c56-94a7-43a8841a81bc` executed exactly two tests: both passed,
582 unrelated cases were skipped. Compilation took 10m40s; tests took 3.252s.
The passing tests cover explicit manual attestation and atomic/replay-safe
manual creation. Log:
`/private/tmp/medtracker-guardian-manual-cache-repaired.log`.
This does not establish import, withdrawal, reminder, browser or full CI
acceptance. No application source changed during the retry. The staged parent
tree remains unchanged; publication remains on hold.

### Narrow import and client implementation, 17 October

The retained Sol owner resumed after the manual result. The shared typed creator
now provides the same new Person, participation, principal-derived declaration
and local relationship for manual creation and a newly imported dependent.
Import does not create a default location, independent grant, Account, User or
credential. Its external declarations must match exactly the new dependent
portable IDs; existing People cannot receive another initial declaration. The
importer loads existing People through live household participation, never email
attachment or a global Person lookup. Hidden portable-ID collisions remain an
insert conflict, not identity adoption.

The handler locks the validated creator Account before its existing Household
lock. Existing imported People require positive live self/legacy authority or a
visible, unwithdrawn declaration with an active local link and current dependent
age/capacity. Household manager status alone no longer authorises clinical
import writes. Both the manual and import grant-creation writers are removed;
direct grant creation and delegation remain closed. The former membership
permissions-version bump belonged to grant creation/restoration. No additional
bump is introduced without a consumer requiring it; declaration revision and
local relationship state will be checked live at the later authority boundary.

Job `93950`, Nextest `397fc11b-aaab-4a41-a6b0-79b47ce57483`, executed all three
focused HTTP tests. Both manual tests passed again after the shared extraction.
The import case reached new-dependent apply 201, current attributed declarations,
zero independent grants/credentials, zero operational notification/mail effects,
and fresh repeat-with-declarations 422. It then failed at the authorised repeat
without declarations. Log:
`/private/tmp/medtracker-guardian-import-manual-focused.log`. The diagnostic
assertion retains only HTTP status and a bounded standard error code. Exact
diagnostic job `99124` confirmed HTTP 422/unprocessable_content, Nextest
`b11fcef9-065a-4a89-bfe9-108e96207ce7`; log:
`/private/tmp/medtracker-guardian-import-repeat-diagnostic.log`.

Restricted-role job `14880`, Nextest
`db31bb6e-0eb6-48d8-b4bb-29c7678c6440`, proved that the existing-Person
INSERT/ON CONFLICT statement returned SQLSTATE 42501, while a participation-scoped
UPDATE succeeded. Both probe attempts were rolled back before the unchanged
HTTP failure. Log: `/private/tmp/medtracker-guardian-import-restricted-probe.log`.
No RLS or ACL widening was needed. The narrow correction updates only an
existing portable ID with live current-household participation, avoiding a new
Person candidate before conflict handling. New Person insertion and hidden
portable-ID collision behaviour are unchanged. The temporary statement-shaped
probe was removed after preserving this evidence.

Job `31412`, Nextest `f32906bf-5b68-4495-8197-2b7aacbbc17f`, then passed all three
focused HTTP tests in 4.855 seconds. Authorised repeat without declarations now
returns 201 and preserves clinical records, versions, change records and
operational state; a fresh repeat containing declarations still returns 422.
Log: `/private/tmp/medtracker-guardian-import-update-focused.log`. The compile
emitted the existing Darwin linker unwind-size warning; strict acceptance is
still pending.

The maintained ApiClient now sends the encrypted value as `bundle`, puts its
passphrase only in `x-medtracker-portable-passphrase`, and offers a typed external
declaration input. Existing calls omit that optional declaration collection.
Job `62024`, Nextest `0b243d30-abc3-45b5-ae00-ac8a93cfdd93`, passed the one
previously failing wiremock request-shape test through
`/private/tmp/medtracker-declaration-client-red.yml client-red`; log:
`/private/tmp/medtracker-guardian-client-import-shape-green.log`. This proves the
ordinary bundle/header request shape; the later focused review checks below
also cover explicit declaration forwarding. Full client acceptance remains
pending. MCP remains read-only; no new tool or credential scope is introduced.

All owned jobs are terminal and the shared compiler/cache was released back to
the delivery controller after the focused correction. The parent staged tree is still
`34bf716bc6d3fc4ffc5da4445160ea11a59ef7ae`; this checkpoint remains unstaged and
uncommitted. Remaining work includes independent delta review,
withdrawal/current-authority/reminder/rollback/concurrency proofs,
the explicit browser declaration journey, contracts/catalogue and required final
gates. No full CI, publication, deployment or review-site change is claimed.

### Focused review corrections and next-stage brief, 17 October

The creator's non-unique database failure already passes through
`OperationError::from`, which emits a fixed category and a new error trace UUID,
then returns a safe domain error. `create_imported` retains that conversion;
`PreparedDeclarations` maps its already-converted domain result, not a raw DbErr.
No second classifier was added. Job `16067`, Nextest
`f8183de6-30d5-4237-bdf8-e59d8425fd39`, passed the one creator-converter regression
using the existing test-only tracing capture. It verifies category, trace UUID
and absence of raw clinical/secret error text. Log:
`/private/tmp/medtracker-guardian-creator-diagnostics.log`.

Client job `96915`, Nextest `34922430-03c7-49f8-9810-b8c624194c04`, reached the
expected header privacy failures: the passphrase HeaderValue was not sensitive
and appeared in request Debug output; invalid header input returned a generic
builder error. Explicit declaration forwarding passed. The correction constructs
the maintained HeaderValue, marks it sensitive, and rejects invalid bytes with a
fixed `InvalidRequestHeader` error before network I/O. Job `42980`, Nextest
`927f86ad-00a9-4cf8-b9d1-fe5cbc4e5b3c`, then passed all three focused tests.
Logs: `/private/tmp/medtracker-guardian-sensitive-header-{red,green}.log`.
The scoped Task wrapper is `/private/tmp/medtracker-declaration-review-fixes.yml`;
both targets use nextest with fail-on-empty. All jobs are terminal and the
compiler/cache lease is returned to the controller. OpenAPI, CLI declaration
input, full client gates and later journey acceptance remain unfinished.

The next bounded stage is own-record withdrawal plus the actual declaration
browser journey. It does not add succession, dispute recovery, identity claims,
new sharing, MCP writes or legal verification. Before implementation, add the
following focused real-App REDs using the current declaration fixture, maintained
native token exchange and existing browser sign-in/CSRF helpers:

- Create a dependent through the existing 201 route, then require the new
  own-only list and withdrawal routes to work with the same first-party mobile
  OAuth principal. Repeat after its last household departure. List uses captured
  declaration names, bounded id-descending keyset pages (default25, maximum100),
  no current foreign Person profiles, and no-store on every outcome.
- PATCH `/api/v1/auth/guardian-declarations/{id}` accepts only typed
  `{guardian_declaration:{withdrawn:true,revision:"1"}}`. Lock Account NO KEY
  UPDATE, maintained exact credential, then declaration UPDATE; no Household
  lock or membership restoration. Verify canonical guardian ownership after
  waits, server withdrawal time and revision2, one global attributed version and
  ledger/delivery, and terminal replay without another event. Foreign IDs404,
  mismatched active/future revisions409, malformed/overflow inputs422; no
  withdrawn-to-active operation. An original revision1 withdrawal may replay its
  fixed terminal revision2 without a new authority write.
- Household-bound stored sessions/app tokens, personal keys and integration
  credentials must not gain global mutation authority. Prove each credential was
  genuinely valid at its maintained boundary before its denial. Preserve current
  inactive/revoked/locked account/session/grant checks and connection reuse.
  This exact source uses four context GUCs: Account, Household, membership and
  invitation digest. Clear all four; do not create two unused GUCs to match an
  older packet's six-context wording.
- Direct and aggregate plan authority must use the same positive declaration,
  active local link/participation and current-date eligibility, alongside self
  and existing valid legacy authority. Test before/at the eighteenth birthday;
  never silently relabel a minor. Manager Remove carer deactivates only its
  local link and stops local derived authority; personal declaration stays
  recorded. Only its guardian withdraws the personal declaration, including
  after departure. Hidden declarations provide no fallback authority.
- The browser form offers only new minor/dependent-adult creation, an initially
  unchecked required declaration and server-versioned statement. Derive guardian
  solely from the principal; no guardian ID, arbitrary statement or upload.
  Keep existing adult editing distinct. Prove missing declaration/error/retry,
  real create/refresh, own list/CSRF withdrawal/terminal refresh and distinct
  local Active/Inactive versus personal Recorded/Withdrawn labels, with keyboard,
  desktop/phone and selected-locale evidence.
- Force reached declaration/version/ledger failures and verify full rollback;
  prove withdrawal/write blocking in both commit orders, restrictive provenance
  retention, ordinary soft closure, foreign/accountless visibility and stale
  connection cleanup. Provider/reminder pre-send serialization remains the
  separate following stage: Account NO KEY UPDATE → Household UPDATE → exact
  declaration SHARE, five-second lock timeout, refresh subscription/intent/
  attempt/deadline, bounded provider attempt, then finish the guard before the
  existing completion transaction. Do not reuse the shared Account UPDATE helper
  or claim sent bytes can roll back.

Owned implementation boundaries for review are the declaration model, existing
direct/aggregate access predicates, narrow OAuth-only and browser transaction
helpers, API account routes, a small global browser controller, existing Person
form/rendering plus an own-list template, all five locale files, OpenAPI/route
pins and the focused API/browser tests. The API helper can OAuth-only gate before
delegating the maintained account transaction; the browser needs its narrow
Account-before-session helper around the maintained validator. Preserve safe DB
classification in newly added code. The existing one-way declaration table/RLS
remains the basis; any newly reached policy limitation must be reported before
widening it. The initial own-record API/browser source is now implemented and
focused-tested below; the broader authority/security matrix remains pending.

The controller subsequently authorised the initial two tests. Job `87072`,
Nextest `c762f29b-e9b9-4198-8d17-7779614e304b`, executed exactly two runtime REDs:
the existing browser new-Person form returned200 but lacked the explicit
declaration checkbox; genuine dependent creation201 followed by last-household
departure and native profile200 reached the new own-list route404. No setup or
compile failure occurred; withdrawal/audit tails remain unreached. Log:
`/private/tmp/medtracker-guardian-withdrawal-initial-red.log`.
The routine read-only Sol reviewer found the browser test's later303 assertions
needed a no-redirect client. That test-only correction is applied after terminal;
registration, native exchange, fixture identities and audit SQL had no other
static finding. The parent index remains unchanged. Production source work is
authorised under the reviewed brief. The controller then granted one bounded
two-test compiler window.

The first own-record source candidate passed both tests: job `78436`, terminal0,
Nextest `f75336a0-e24f-48f8-bbcc-8b236860f623`,2/2 in3.138seconds, log
`/private/tmp/medtracker-guardian-withdrawal-first-candidate.log`. The first test
reached zero-household native Account authentication, captured-name listing,
stale/future revision denial, withdrawal, global actual-User version/ledger/
delivery and terminal replay with unchanged identity. The second reached the
initial unchecked required checkbox, missing-attestation rejection, successful
creation, own-list rendering, missing-CSRF denial, valid withdrawal and a
withdrawn refresh without a withdrawal form. Routine read-only review found a
missing CSRF response-cookie on direct page entry; the source now returns the
CsrfToken response parts, following the maintained Person renderer. No other
bounded source defect was found. Scoped rustfmt, all-five-locale tree parity and
diff-check passed. The parent staged tree remains unchanged. Darwin emitted its
known linker unwind-size warning; this was not strict lint acceptance.

The compiler lease was returned immediately after terminal. Direct/aggregate
declaration authority, current-age transitions, the larger credential/revocation/
rollback/concurrency/context-cleanup matrix, selected-locale creation-form
plumbing, CLI/OpenAPI contracts and actual desktop/phone browser proof remain
unfinished. Reminder/provider pre-send serialization is a later stage. These two
focused passes are not full checkpoint or canonical acceptance. No commits or
publication are authorised.

The next access prerequisite has a reached RED: job `98601`, terminal201,
Nextest `8a336a6c-4534-4257-a6b7-3727f035cb01`, executed exactly one test.
Genuine dependent creation returned201 and the capability endpoint returned200;
the assertion that the active guardian appears in aggregate Manage without a
legacy grant failed. Direct schedule-write and withdrawal tails were not reached.
Log: `/private/tmp/medtracker-guardian-authority-initial-red.log`. The compiler
lease was returned immediately. No positive authority predicate has been changed;
the exact bounded direct/aggregate proposal is with the routine read-only reviewer.
This is declaration/access work only; no further import, provider or reminder
implementation is included in the current admin prerequisite.

The controller approved the resulting narrow source correction after that RED
and routine predicate review. The direct, existence and aggregate Manage paths
now add a positive declaration alternative while retaining canonical self and
valid legacy authority. The declaration helper locks exact positive local
declarations in ID order, rechecks the actor after waits, derives the current
date from the database clock and maintained preferred timezone, then rereads
current linkage/subject eligibility within those locked IDs. Existing import
helper/callers and notification/provider predicates are unchanged. The routine
reviewer inspected the exact source and found no concrete introduced defect.
Scoped rustfmt and diff-check pass; the canonical parent index is unchanged.
This correction subsequently passed the exact runtime test below; wider
declaration acceptance remains incomplete.

The remaining own-withdrawal security tests were prepared in
`tests/care_api/guardian_declarations/security.rs`; their later execution is
recorded below.
They first prove local validity of household sessions/app tokens and an issued
personal key, and use actual maintained OAuth code exchanges for integration
and foreign native credentials. They then test excluded credential denial,
foreign declaration404, terminal replay after exact grant revocation, Account
lockout/soft closure, and reached version/ledger failure rollback with recovery
retry. Nontransactional sequence markers distinguish reached audit failures from
an earlier denial. Existing authentication helper bodies are reused unchanged;
only two test-helper signatures gain sibling visibility. No production or RLS
change is inferred from these unrun candidates, and compiler allocation remains
with administration. Broader concurrency/context/age and browser locale evidence
are still outstanding.

The controller allocated the reviewed authority1/security4 batch as two exact
maintained Task selectors. Combined job `73385` terminated1, with both Tasks
returning201 at compilation: the new security helper called Reqwest `.form()`,
which is absent from this checkout's enabled features. ZERO tests executed;
there was no runtime authority or security result. Logs:
`/private/tmp/medtracker-guardian-authority-first-green.log` and
`/private/tmp/medtracker-guardian-withdrawal-security-first-run.log` (the former
filename records intent, not a passing result). The test helper now follows the
existing explicit `serde_urlencoded` body/content-type pattern without changing
dependencies or production code. Scoped formatting and diff-check pass. The
compiler lease was returned at terminal and the repaired batch is not rerun.

The next focused attempt `96614` compiled but failed in fixture setup before
authority assertions, Nextest `77450d21-026b-4a68-a645-bd3009162ba6`: the seed
encountered the obsolete required `people.household_id`. Security4 did not start.
Read-only diagnosis proved that the newly rebuilt care_api executable linked a
stale shared-target migration crate, not a reused PostgreSQL template. The
UUID-owned Task creates an empty reference database; the test directly invokes
`migration::Migrator::up` before seeding. Its linked migration dep-info omitted
canonical recovery SQL and guardian000013. Source was correct. The stale rlib
SHA was `592cfc829b4a52bcece6b18945f85c1fdc5d2536fc50b81e96c5bd6cf0042938`.

Controller-approved repair used the off-tree Taskfile
`/private/tmp/medtracker-guardian-local-package-repair.yml`. The first package
clean safely refused the missing Cargo cache marker and deleted nothing. After
controller verification of the exact owned target, the Task copied and compared
the real Cargo-generated registry marker, then ran only `cargo clean --package
med-tracker --package migration` against the shared target. Dependencies,
target/ci archives, source and website remained outside the cleanup scope.
Both local packages rebuilt. New migration dep-info includes recovery SQL and
guardian000013; new rlib SHA is
`34e317ce07d2fd92779a08ff28cf2a444d42aa75e71aaeef7817155b3f35346e`.
The rebuilt care_api SHA is
`3a3581c2c4311d5fbe50a81b6f86cad44f3168f6f0eaf832eda243865eb1dbcd`.
Package-only local-artifact invalidation is required when switching these
shared-target worktrees; a rebuilt test alone does not prove migration freshness.

Repaired job `65378` executed authority1 PASS, Nextest
`a7308a5f-c911-4211-991e-a76700d1e20a`,3.487seconds: aggregate Manage, genuine
schedule creation, unrelated-subject denial, withdrawal and denied post-withdrawal
direct/aggregate access with no clinical effects all reached. Security4 executed
3PASS/1 setupFAIL, Nextest `c9f6921d-b2f2-4f2b-92fd-ead53bfcb12e`,6.538seconds.
Foreign native exchange/profile200 followed by declaration404 passed. Terminal
replay after exact grant revocation, Account lockout and soft closure passed.
Injected version/ledger failures were reached, rolled back and recovered with
one terminal withdrawal plus idempotent replay. Credential exclusion stopped at
personal-key setup `INVALID_NAME_LENGTH` for an overlong synthetic test name;
no exclusion assertions or integration tail were credited. Logs:
`/private/tmp/medtracker-guardian-authority-package-repaired.log` and
`/private/tmp/medtracker-guardian-security-package-repaired.log`. The compiler
lease was returned at terminal and no production fix was inferred.

The sole exclusion-case retry is now PASS: job `90565`, terminal0, Nextest
`483b1f14-13c2-40e5-8f97-6dba1c5ee7df`,1/1 in2.898seconds. Only the synthetic
key name changed to the independently reviewed `Guardian exclusion`; validation
and assertions were retained. Each household session/app token/personal key
first reached local care200, and the integration token was issued through a
successful maintained code exchange before its global withdrawal denial.
Declaration/audit/clinical/identity no-effect assertions reached. The switch
back from administration again used only the two-package Task clean; rebuilt
migration dep-info includes canonical recovery SQL and guardian000013, with
expected rlib SHA34e317ce07d2fd92779a08ff28cf2a444d42aa75e71aaeef7817155b3f35346e.
Log: `/private/tmp/medtracker-guardian-credential-exclusion-retry.log`.
Compiler/cache was released at terminal. Authority1 and security4 have passing
focused evidence across the recorded batch and this exact retry, not a single
five-test green invocation or full declaration acceptance. Concurrency, broader
context/age/RLS/catalogue, selected-locale and whole-browser evidence remain
unfinished. No imports/provider scope or publication was added.

The next finite gate is TEST-ONLY source preparation in a new guardian module,
with five actual-App cases: (1) declared minor authority immediately before and
at the eighteenth birthday, using the maintained timezone/database date;
(2) guardian capacity loss removes only declaration-derived authority while
self/legacy alternatives remain; (3) a different authenticated household manager
removes the local care link through the maintained CSRF route, ending local
derived authority without withdrawing the personal declaration; (4) withdrawal
commits first and a subsequent real plan write is denied with zero clinical
effects; (5) a real plan request holds declaration SHARE while blocked at its
audit insert, and an independent withdrawal is witnessed waiting until plan and
audit commit. The last case uses a disposable audit gate, actual backend PIDs,
two-stage pg_blocking_pids witnesses and bounded cleanup. It does not simulate a
provider send. Existing self and legacy-grant positive controls stay explicit.
Only this record, the new guardian test module and its registration may change;
all production bytes and the staged canonical tree remain frozen. Tests are to
be independently reviewed before any runtime/compiler allocation. No schema,
import, notification/provider, browser suite or new protocol is included.

That five-case gate is now PASS: guarded job `32355`, terminal0, Nextest
`fb9d7d80-1035-45d3-88a4-35c3fb9c1fc4`, exactly5/5 in9.168seconds. It reached
the before/at-eighteenth-birthday boundary without identity retyping, guardian
capacity loss with self/legacy positives retained, a different manager's real
CSRF removal with the personal declaration still Recorded, committed withdrawal
before a denied effect-free plan write, and a real plan/declaration SHARE wait
before independent withdrawal. The concurrency case reached both PostgreSQL
blocking witnesses and declaration relation locking, then committed exactly one
plan/version/ledger and the real Account-FK request audit before the withdrawal
completed. Post-withdrawal plan denial also reached.

Raw log: `/private/tmp/medtracker-guardian-live-authority-five-first.log`.
The guarded off-tree Task used only the two-local-package clean and the exact
`guardian_declarations::live_authority::` selector. Pre/post checks confirmed
unchanged production/tests, parent tree34bf716 and full source fingerprint
b6fe3b9e373fdb1fa4afa6e0d8db200060df7121d1ba469f894f693939f6418b throughout
execution. Rebuilt migration dep-info includes guardian000013 and canonical
recovery SQL, with expected rlib SHA34e317ce07d2fd92779a08ff28cf2a444d42aa75e71aaeef7817155b3f35346e.
The compiler/runtime lease was returned immediately at terminal. The known
Darwin linker warning remained visible; this was not strict lint or full-suite
acceptance. This result adds no production changes, imports, provider work,
new protocol or publication. This evidence paragraph was added only afterward.

The subsequent approved gate was four test-only storage-boundary cases, reusing the
restricted App fixtures. The new `guardian_declarations/storage_boundary.rs`
will check actual table/column privileges, FORCE RLS, trigger ownership/search
path and immutable/terminal DML; another genuine household's shared-subject
visibility without declaration or direct/aggregate Manage authority, followed
by subject revocation; real password-confirmed account closure with retained
provenance and restrictive deletion; and reuse of one actual App backend across
household commit/rollback and global withdrawal success/reached audit rollback.
The pooled case observes all four actual transaction context settings both
inside the withdrawal audit and outside transactions, then proves a different
authenticated principal cannot inherit authority. It does not persistently
poison session settings, assert browser-pool coverage, or change production
context semantics. A test-only App pool-size option and visibility of the
existing closure helpers are the only harness changes. Production and the
frozen five-case module remain byte-identical; the staged canonical parent tree
remains `34bf716bc6d3fc4ffc5da4445160ea11a59ef7ae`. Independent read-only review
preceded runtime allocation.

Source preparation and retained Sol review are complete for this four-case
gate. The new module is 327 lines, SHA256
`feeb792c5c17049066ec8e0828ebb4e1aeb349fc098b9dbd2a17135d70d25e6f`.
Review corrected a test-only role assumption: the fixture pool logs in through
its restricted per-fixture login, and application transactions temporarily
switch to `med_tracker_app`. Catalogue DML now sets that local role explicitly;
outside transactions the pooled check verifies the actual login, application
role membership and non-superuser/non-bypass attributes. The same-PID and four
empty-GUC assertions remain. Narrow re-review found no remaining setup issue.
Scoped rustfmt and `git diff --check` passed before runtime. The frozen five-case SHA remains
`b23f91d0986d7874c038efe4a620876fd863353d74a38376e708c6e9d29a68f2`;
parent index equality was rechecked against tree34bf716.

The four-case storage gate then passed at job23414, terminal0, Nextest
`f223e470-944b-4c83-99d3-bc5898e5c95f`: exactly4/4 in5.428seconds. It reached
the restricted catalogue/DML assertions, foreign participation and revocation,
real password-confirmed account closure with retained provenance and restrictive
deletion, and the same App backend's four-GUC success and reached-audit-rollback
tails followed by a genuine foreign principal denial. Pre/post guards confirmed
source `b9c24b0acf5e1fd28c980064f474eb69cb46375780f181f94f27fbce834e5626`
and parent34bf716 throughout. The initial lease-environment guard failure executed
zero tests and no Cargo/database operations; its separate log remains preserved.
Raw runtime log: `/private/tmp/medtracker-guardian-storage-four-runtime.log`,
SHA256 `8879678d43cee99016fb994cd5a9635b5883499277ce24bdc880a0059fa22bc0`.
Migration dep-info again contains canonical recovery SQL and guardian000013;
rlib SHA34e317ce remains unchanged. The known Darwin linker warning is retained.

The bounded locale/OpenAPI follow-up observed a genuine request-contract RED
after declared dependent creation201: OpenAPI did not admit the required
`initial_guardian_declaration` field. The first locale invocation stopped on
an over-exact English document-language assertion; base `en-GB` is valid. After
the approved test-only en/en-GB correction, job13096, Nextest
`870b118e-f685-449c-97ad-35a24a756997`, reached English GET200 and invalid POST422
with unchanged state, then failed on the Spanish declaration statement at its
GET200. Spanish rerender and remaining locale tails were not reached.
Logs: `/private/tmp/medtracker-guardian-locale-contract-red.log` and
`/private/tmp/medtracker-guardian-locale-nonenglish-red.log`.

The source correction now passes the maintained Accept-Language selection into
the initial People form and validation rerender, using existing guardian keys
and leaving the base English fallback intact. It does not change overview;
future integration must preserve overview's explicit query/form locale precedence.
OpenAPI describes only the existing manual creation and own declaration list/
withdrawal operations. Creation requires an eligible principal's exact true
declaration for a new dependent; no independent grant/credential is promised.
Person update attributes remain separate. Active request/route inventories and
the SHA/count pin track129 operations; new declaration operations remain
unverified, and superseded implicit-grant creation evidence is replaced only
with the two actually passed manual declaration cases. Missing complete creation
API status/header/credential coverage remains explicit. Generic ApiClient POST
accepts serializable manual request bodies; no global withdrawal client adapter
or MCP write tool is claimed or added. MCP remains read-only, native trees are
excluded. Retained read-only review found no source blocker and corrected the
terminal-replay wording to resubmit revision1 and return revision2.

Focused locale/contract GREEN reached both cases, including all five languages
and invalid rerender tails plus actual creation/list/withdrawal schema checks.
After the reviewed extractor tuple lint correction, Nextest
`d27f4fa5-52eb-4cf2-a7ae-c6ce5b30ce91` passed2/2 in2.962seconds. The active
inventory gate exposed an inherited canonical-parent mismatch: closed
`createPersonAccessGrant` still had201 in its response list, schema map and
header map although the parent OpenAPI and controller already denied creation.
Only those three metadata entries were removed after controller/source review;
denial metadata and all equality assertions remain. Seven exact request
inventory/pin/schema/header/handler/route/evidence-tamper cases then passed1/1
each. This parent evidence reconciliation is explicitly included in the export.

Strict Clippy next found four allocating string-ID comparisons and one redundant
Ok/? in earlier guardian tests. The approved correction precomputes expected
strings and compares `as_str()` values, retaining the string wire contract;
the wait helper returns its existing result directly. Original test bytes and
hashes are retained in `/private/tmp/medtracker-guardian-before-testlint/`.
The affected selectors passed on runtime source
`b75873eb716c809d1a6645b4be0ac3a6f894efefe861f3e2a3360cb4d8cc3641`:
withdrawal2/2 (`692629cd-4bcb-4b0a-97b1-955c45ab762a`), live authority5/5
(`edb74415-e601-4464-9a2f-11dab9f9e232`), and storage4/4
(`f5f67978-1825-42cb-bcf7-8f310f560bcf`). These are three actual invocations,
not a single eleven-test invocation. Unchanged locale2 and metadata7 proofs
remain valid. Strict workspace/all-target/all-feature Clippy with `-D warnings`
passed. Maintained machete then failed only on unchanged root `csv` and migration
`serde_json`, already fixed on current main; no Cargo edits or suppressions were
added. The combined Task therefore exited201, not0. The raw log is
`/private/tmp/medtracker-guardian-testlint-focused-strict.log`.

The parent index stayed34bf716 throughout, and the compiler/runtime lease was
returned at terminal. This paragraph is an evidence-only record update after
that tested source; the final export distinguishes its hash from runtime-tested
bytes. Broader combined-current-main CI and actual guardian browser/visual
acceptance remain pending, as does complete creation API coverage. No reminder/
provider work, new import expansion, MCP writes, native implementation or
publication is credited by these focused gates.
