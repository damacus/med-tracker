# Shared household and medication operations Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve complete care operations with one permission and transaction boundary for every caller.
**Architecture:** Focused SeaORM model modules own domain operations and their transactions.
Loco controllers expose API routes without embedding business logic or calling an in-process legacy router.
**Tech Stack:** Loco, SeaORM, PostgreSQL 18; maintained decimal/date/time libraries.
**Spec:** [implementation-spec.md](../implementation-spec.md), [capability-inventory.md](../capability-inventory.md).

## Global constraints

### Current bounded delivery: legacy Manage removal (#2682, 22 October 2026)

Reference: main `e624f628f`, ADRs 0013–0016 and the issue's explicit remaining
runtime callers. Active household participants retain View/Record. Manage belongs
to canonical self or a current eligible declared guardian; owner, administrator
and member roles with legacy-only grants cannot manage someone else's care.
Grant rows and historical attribution remain unchanged. Person creation remains
a household-manager action. This does not implement shared-plan disclosure,
departure, optional inventory locations, new exports or production deployment.

One writer owns access/Cedar/care projections and their tests. OAuth consent
work is independently owned in identity/controller files. Reuse the current
guardian declaration locking, eligibility and RLS checks rather than introduce
another authority. Source review uses Astra because this changes authorisation;
normal model routing applies (93% allowance remaining at selection).

Observed RED: `task test:unit FILTER=legacy_manage_grants_never_authorise_another_person`
failed on the old Cedar permit, Nextest `f140f12e-6d90-40f7-90d3-d8588561accf`.
API regressions cover legacy-only role capabilities, treatment reads/writes and
unchanged clinical/sync/outbox state; declaration-only schedule and assignment
flags must match direct write authority. Retain revocation, expiry, lockout,
withdrawal, supersession, replay and rollback cases. Browser coverage checks
the denied treatment edit controls at 1440×1000 and 390×844 against the existing
treatment layout and approved permission difference.

The Cedar matrix passed 11/11 after removal (Nextest
`61526b0c-0a5a-4b86-806e-931ceca00799`), with JavaScript unit checks and doctests
passing through the same Task. Documentation builds. Independent Astra security
review and affected re-review cleared the source after correcting obsolete grant
expectations, public creation wording, denial-audit coverage and withdrawn-guardian
projection coverage. `task check` passed all targets. Focused full-App authority
checks passed 2/2 (`e1b558f6-7ab8-42b0-b56b-eab62123e85b`), including attributable
denial audit, stock preservation, creation denial and withdrawal of treatment flags.
The desktop/mobile browser journey passed 1/1
(`a685d992-115b-49ff-a96a-4473529dbcf1`), including keyboard focus and no horizontal
overflow. Captures are `docs/screenshots/browser-journeys/legacy-manage-denied-*.png`.
Vanilla Astra, UI Professional and Accessibility independently passed the scoped
visible change against retained main treatment captures and the unchanged Tera
template. Reference people/doses differ, so this is not exact pixel parity. The
pre-existing stacked mobile navigation is unchanged. Accessibility review used
template, screenshots and keyboard evidence; no live axe/Lighthouse or screen-reader
conformance is claimed. Final `task ci` passed: 153 Rust unit cases, 19 JavaScript
checks, 33 browser journeys, 1,159 integration cases and template concurrency,
alongside formatting, strict Clippy and dependency checks. Disposable acceptance
resources were removed. Publication is next; no merge or deployment is claimed.
An additional source review found no introduced lock cycle between treatment
projection, guardian withdrawal and recovery; this was a lock-order assessment,
not a new concurrency test.

First production must include complete platform administration, time-limited support
access and household export/closure/retention holds/permanent deletion. These are
core security outcomes, not optional follow-ups. Automated NHS dm+d import/
reconciliation, scanner, review generation/background refresh and automatic live
dose/stock updates are also core. Optional AI/provider lookup deferral does not
exclude them. No live destructive action is authorised by these scope decisions.

Domain implementation depends on locally verified tenant/actor/error interfaces.
Owned domain tests may establish actors through synthetic fixture setup; do not
add a second production authentication path. Production HTTP integration and care
acceptance require validated identity. Full identity acceptance does not block
porting and testing independent transactions. Use exact authoritative wire contracts; integer database IDs
remain `i64`, portable identifiers remain preserved strings. Never use floating point for dose quantities.
Writer owns model/controller/tests; existing Rails services and Rust modules are read-only inputs.

## Review focus

Role/capacity boundaries (C1), household changes during requests (C1), competing stock writes (C2),
duplicate/concurrent replay (C3), and audit failure after clinical writes (C3) need explicit tests.

## File and interface map

Create `src/models/care/{households,medications,treatments,doses,sync}.rs`,
`src/models/care.rs`, `src/controllers/api/{households,medications,treatments,doses,sync}.rs`,
`src/controllers/api.rs`; modify module registration and `src/app.rs`.
Use named model methods with the existing `TenantTransaction` and `OperationError`
interfaces. Preserve OpenAPI field names and validation rules. Return model or
projection results without HTTP status or headers; controllers map errors at the
edge. API and browser adapters call the same methods. No migration-only command
dispatcher or forwarding wrapper is required.

### C1: Household membership, invitations, people and grants

**Files:** Focused household, People and administration model/controller modules;
tests in the existing `tests/care_api.rs` binary and its `tests/care_api/` modules.
**Operations:** `Create`, `Update`, `Invite`, `AcceptInvitation`,
`ChangeMembership`, `CreatePerson`, `UpdatePerson`, `GrantAccess`, `RevokeAccess`, `Retire`.
Read Rails household/person policies and contract `care.rs`, `invitations.rs`, `household_minor_readiness.rs`.

Retain the actual entry points. Household creation belongs to account
registration/bootstrap; retirement belongs to authorised operator offboarding.
Do not invent public household POST/DELETE routes for those operations. Invitations
use the existing administration list/issue/resend/revoke routes and verified-session
acceptance at `/api/v1/invitations/accept`, plus the retained browser acceptance
flow. Cover seven-day expiry, resend token rotation, atomic membership/grant creation
and replay after access is revoked. Reuse Loco mail delivery with synthetic capture.
Acceptance follows the authoritative root OpenAPI user API-session requirement.
The retained Rails controller also admits mobile OAuth credentials, which diverges
from that contract; do not broaden Loco acceptance to reproduce the discrepancy.
Resend keeps its separately documented mobile credential support.

Test every named action with the cycle below, using descriptive tests in its
owning module. Run `TARGET=care_api FILTER=<module-or-test>` for focused RED/GREEN,
then the whole binary at slice acceptance. Reuse the registered fixture rather
than creating a parallel household test runner. Group related actions into usable
deliveries, such as household settings and membership/access management. Review
the related action diffs together with separate verdicts. Run final full CI and
publish once per reviewed delivery, rather than once per action; do not wait for
the entire care migration before publishing working capabilities.

- [ ] Test each action for admin, clinician, self, carer, parent and unauthorised actors. Include last-owner
  removal, cross-household IDs, expired invitations and capacity manipulation.
  Assert `minor.has_capacity == false`, `dependent.has_capacity == false`,
  `cross_household_write_count == 0`, `household_has_owner == true` after rejected owner removal.
- [ ] Run `rtk task slice:test TARGET=care_api FILTER=<owning-module>`; absent operations must fail.
- [ ] Implement typed actions with current grants/RLS and transactional audit through P3.
- [ ] Run all action/role cases and `rtk task ci`; verify results through real Loco routes.
- [ ] Review and commit `feat(care): preserve household roles and person grants`.

### C2: Medicines, inventory, dosage, schedules and assignments

**Files:** Medication/treatment modules/controllers above; focused leaves under
`tests/care_api/`, registered in the existing `tests/care_api.rs` binary. Read existing
stock adjustment service and Rust medication/schedule modules. Preserve the retained
entry points: browser-only mutations do not justify new public API routes.
**Medication operations:** `Create`, `Update`, `Retire`, `AdjustStock`, `RemoveStock`,
`Order`, `Receive`, `CreateDosage`, `UpdateDosage`, `RetireDosage`;
**Treatment operations:** `CreateSchedule`, `UpdateSchedule`, `RetireSchedule`, `Assign`, `Unassign`,
`Pause`, `Resume`. Preserve original conflict/precondition semantics and date boundaries.

Split tasks into medication CRUD, stock adjustment/removal, order/receipt, dosage CRUD,
schedule CRUD, assignment/unassignment and pause/resume. Each unit uses the five steps
below, with filter prefixes `medication_crud`, `stock_adjustment`, `stock_orders`,
`dosage_crud`, `schedule_crud`, `assignments` and `pause_resume` respectively.
Use the existing `care_api` binary with the owning module or descriptive test filter.
Browser adapters call the same tested model methods; no extra runner is required.

- [ ] Test full successful/invalid/forbidden actions, duplicate receipt, two competing stock writes,
  active dosage references and pause/resume boundaries. Assert one receipt effect for replay,
  exact decimal stock, no unauthorised mutation and stable occurrence identities.
- [ ] Run `rtk task slice:test TARGET=care_api FILTER=<owning-module>`; record RED.
- [ ] Implement the named actions with locking, current access and existing transactional audit requirements.
- [ ] Run both focused binaries and existing stock/treatment contract expectations against Loco, then root CI.
- [ ] Review and commit complete medication and treatment operations in independently reviewable commits.

### C3: Dose capture, history, correction and replay/sync

**Files:** Dose/sync modules/controllers above; focused leaves under `tests/care_api/`.
Read `rails/app/services/medication_administration/`, `offline_dose_eligibility.rb`,
`rust/api/src/dose/`, `mutation_idempotency.rs` and contract `doses.rs`, `replay.rs`, `sync.rs`.
**Dose operations:** `Take`, `ReopenMissed`, `RecordMissed`;
**Sync operations:** `ReadChanges`, `ReplayBatch`. Keep client UUID and sync cursor formats.

The authoritative API and retained scheduled-dose contract correct a missed
occurrence by reopening it. They prohibit rewriting or deleting a recorded take.
Preserve immutable take history and stock; do not invent take PATCH/DELETE routes.
Guarded medication deletion remains part of medication CRUD above.

Separate tasks: `dose_take`, `dose_reopen`, `dose_missed`,
`sync_read` and `sync_replay` in the existing `care_api` binary. Apply the following cycle to each filter;
the Take and Replay tasks own the concurrency/idempotency assertions.

- [ ] Test simultaneous same-UUID doses, same UUID/different payload, insufficient stock,
  partial batch conflicts, revoked permission before replay and forced audit failure.
  Assert `take_count_after_duplicate == 1`, `stock_decrements == 1`, `successful_audits == 1`,
  `payload_mismatch_is_conflict == true`, `writes_after_audit_failure == 0`.
- [ ] Run `rtk task slice:test TARGET=care_api FILTER=<owning-module>`; record actual invariant failures.
- [ ] Implement existing idempotency/conflict rules and atomic dose/stock/audit writes; include explicit
  historical timezone/occurrence handling. Preserve unsuccessful-access audit rules separately.
- [ ] Run concurrent/replay tests, route-level contracts and root CI. Verify database state, not only responses.
- [ ] Obtain concurrency/permission review, integrate and publish; update the slice report.

**Done:** Every named operation works through Loco and has positive, validation, role, concurrency and
replay evidence. API and browser callers reuse these operations; no read-only substitute counts as migration.

### Typed Take refactor (#2681, verified local candidate on 22 October 2026)

Reference: `main` at `601f8f354ee445aa9e0cec950ec395bd62f0aa21`; no visible UI
difference is approved or intended. API, browser, sync and occurrence inputs now
validate into a typed Take command before preparation, replay and writing. The
local focused `care_doses` integration binary passed 32 tests, including portable
source IDs, microsecond and timezone preservation, exact Decimal replay, duplicate
UUIDs, permission withdrawal, stock and audit rollback. Sol owned implementation
and independent review because this bounded refactor required judgement about
replay and validation ordering. Review found no remaining actionable finding.
The final root `task ci` passed formatting, Clippy, dependency checks, unit tests,
doctests, 1,146 integration tests, browser journeys and template concurrency.
The documentation build also passed. CI used a temporary internal-disk build
directory after the external Cargo cache filled; source and repository settings
were unchanged. Publication is the next step; no merge or deployment is claimed.

The owner confirmed that malformed timestamp or amount strings sent with an
already used, accessible dose UUID must retain conflict (409). The follow-up uses
a non-executable typed attempt to check replay authority before returning that
conflict; the same invalid input with a new UUID returns validation (422).
Permission checks precede lookup, inaccessible stored doses retain the validation
error, and database failures propagate. Valid replay and differing valid payload
conflicts retain their existing behaviour. The focused HTTP regression passed
with unchanged dose, stock and audit effects, uppercase UUID reuse and permission
withdrawal; the browser malformed-confirmation unit test also passed. Final root
`task ci` passed: 150 unit tests, 19 JavaScript tests, 1,149 integration cases,
28 browser journeys, template concurrency, formatting, Clippy and dependency
checks. The documentation build passed. Publication is the next step.

### Typed treatment forms (#2870, local candidate on 22 October 2026)

Reference: `main` at `c4c5fc88bc41a09d5c0119fe260ed91f22915b60`. Treatment
create/edit forms and the medication wizard construct validated schedule or
assignment inputs with distinct person, medication and dosage-option selectors,
exact Decimal amounts, dates and enum kinds. API/sync adapters parse into the
same typed resolver at their existing authorization/replay/ETag boundaries.
Persisted configuration, audit snapshots, wire projections and templates retain
their legitimate JSON. Pause/resume and unrelated JSON hand-offs remain outside
this issue; no UI or wire-field change is intended.

Sol owned implementation and independent review because omitted/cleared fields
and validation ordering required judgement. Review found no remaining actionable
finding. Focused browser schedule create/edit, assignment edit and wizard-person
linkage passed; desktop/mobile screenshots were inspected. Focused schedule
lifecycle (including revoked access before malformed input), treatment creation,
assignment and sync checks passed. Type-contract and strict-date regressions were
observed failing before their fixes. Root `task ci` passed on this frozen source:
151 unit tests, 1,148 integration-suite cases, 29 browser journeys and template
concurrency, alongside formatting, Clippy and dependency checks. The documentation
build passed. Publication remains next; no merge or deployment is claimed.

Syntactic parsing now precedes database resolution within authorized schedule
requests. When several fields are invalid, the reported error may therefore
change; authorization, replay, ETag ordering and single-field validation remain.

## Approved completion plan — 7 October 2026

This section supersedes conflicting older behaviour in this plan. The owner confirmed the complete plan after questions Q1–Q17. Finish the existing auth PR first; invitations remain there. Only after accepted auth is on main, deliver two independent PRs based on main: platform administration/support, then household lifecycle. Other migration work stays paused. No deployment or live-data action is authorised.

### Platform administration and temporary support

The 9 October UI correction uses the existing Rails administration components as its reference: a desktop users table, separate mobile records, user names and email addresses, distinct account/sign-in/platform access states, labelled search and result counts. Reuse the Rails heading, navigation and contained form patterns across settings, owner recovery and support. Do not add controls for capabilities the migrated application cannot perform. Verify desktop, 320px mobile navigation and dark appearance through the actual UI.

The 9 October review correction aligns platform grant eligibility, remaining-administrator checks and directory sign-in state with authentication's first linked person. User page numbers clamp to the final result page before offset calculation. Support session history has bounded pages so older approved and active sessions retain their controls. Provider registration initialises its credential fields from the current selection even when the deferred script loads after that selection; CI retains browser failure diagnostics.

- Browser pages cover platform users/admin rights, existing non-secret settings, owner recovery and support requests. INVITE_ONLY environment precedence remains visible/locked. Provider credentials stay externally configured.
- Reuse the auth five-minute proof bound to action/target for privileged operations. Guard last-active-platform-admin removal/disable under a lock, including races.
- Recover a household without an active eligible owner by promoting an existing eligible member, with fresh proof/reason/audit. A platform admin cannot grant themselves membership/ownership.
- Support requests require an administrator reason and expire after24 hours. A current owner approves with fresh proof; administrator activation also requires fresh proof and starts30 minutes. Either party can end access.
- Support retains the real administrator actor and grants read-only household configuration/clinical access. No care writes, permission changes, exports or file downloads. Recheck operator status, approving-owner authority, household state and expiry on every request. Never create a fake membership/impersonation identity.
- Reuse adopted support records, RLS and explicit validated support context. Audit request, approval, activation, termination and expiry; keep protected reasons out of general logs.

### Household export, closure, holds and deletion

- Browser exports are available to owners/admins. Owners/platform admins can close and reopen; platform admins alone manage holds/purge. Fresh proof binds closure, reopening, hold changes and purge to the target/action.
- Closure immediately blocks clinical access and revokes affected tokens/support, preserving access to other households. Keep membership/grant states for conditional reopening; do not restore old tokens, expired grants or previously revoked access.
- Add durable closure generation/deadline; deletion becomes eligible after30 days. Any still-valid owner or platform admin may reopen before purge starts; reopening invalidates the closure's deletion eligibility.
- Holds block deletion and cleanup, not normal care. Keep immutable protected reason/review/release evidence; review dates never auto-release.
- Queue a private versioned JSON+files ZIP with checksum manifest. Include clinical/membership records and owned files, never authentication secrets. Reuse maintained S3 SDK, chosen S3-compatible configuration, canonical blob keys and shared-reference checks. No live bucket provisioning.
- Reuse adopted export states. Downloads recheck authority; exports expire after30 days unless held. Add only the necessary generation/expiry jobs, not unrelated workers.
- Purge is an explicit platform-admin confirmation after closure+30 days, no active hold and a valid verified export bound to the current closure generation. Regenerate expired exports. No automatic purge.
- Reuse resumable purge ledger; protect shared identities/other-household files, immutable audit history and minimal completion tombstone. Delete blobs only after their final attachment reference is removed.

### Acceptance and delivery

Use TDD, PostgreSQL18, canonical fixtures and existing task runners. One source writer and one costly verification lane. Cover cross-household/role withdrawal, support approval/expiry/termination/forbidden writes/downloads, last-admin races, owner-recovery abuse, closure/reopen/stale tokens/other households, holds/purge races, exact deadlines, export checksums/expiry/generation binding, shared files, interrupted retry and audit rollback/preservation. All deletion checks use disposable owned fixtures.

Verify actual desktop/mobile journeys, save safe screenshots, use Devin SWE-2 High for bounded implementation and SWE-2 Max for coherent security review. Run required complete checks, publish each independent PR and verify exact-head hosted checks. Update the existing progress report at meaningful checkpoints. Merge/deployment/live support/closure/deletion need their own authority; this plan does not authorise them.
