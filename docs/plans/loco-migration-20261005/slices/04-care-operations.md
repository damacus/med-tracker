# Shared household and medication operations Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve complete care operations with one permission and transaction boundary for every caller.
**Architecture:** Focused SeaORM model modules own domain operations and their transactions.
Loco controllers expose API routes without embedding business logic or calling an in-process legacy router.
**Tech Stack:** Loco, SeaORM, PostgreSQL 18; maintained decimal/date/time libraries.
**Spec:** [implementation-spec.md](../implementation-spec.md), [capability-inventory.md](../capability-inventory.md).

## Global constraints

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
