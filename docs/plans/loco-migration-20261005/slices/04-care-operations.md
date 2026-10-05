# Shared household and medication operations Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve complete care operations with one permission and transaction boundary for every caller.
**Architecture:** Focused SeaORM model modules own domain operations and their transactions.
Loco controllers expose API routes without embedding business logic or calling an in-process legacy router.
**Tech Stack:** Loco, SeaORM, PostgreSQL 18; maintained decimal/date/time libraries.
**Spec:** [implementation-spec.md](../implementation-spec.md), [capability-inventory.md](../capability-inventory.md).

## Global constraints

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
Each operation module exposes specification `execute(db, scope, input) -> Result<Output, OperationError>`.
Its `Command` enum has the actions listed below; typed payloads use existing OpenAPI field names and
validation rules. `Output` contains model/projection results without HTTP status or headers.
Controllers deserialize the same typed input that browser controllers will use and map errors at the edge.

### C1: Household membership, invitations, people and grants

**Files:** Household model/controller above; `tests/care_households.rs`.
**Interfaces:** `households::Command` actions `Create`, `Update`, `Invite`, `AcceptInvitation`,
`ChangeMembership`, `CreatePerson`, `UpdatePerson`, `GrantAccess`, `RevokeAccess`, `Retire`.
Read Rails household/person policies and contract `care.rs`, `invitations.rs`, `household_minor_readiness.rs`.

Treat each named action as a separate task with the test cycle below. Test names start
`households_<snake_case_action>`; for example `households_accept_invitation_*`.
Run `TARGET=care_households FILTER=households_<snake_case_action>` for focused RED/GREEN,
then the whole binary at slice acceptance. Commit a complete action rather than waiting
for the entire household family. Review related action diffs together with separate verdicts.

- [ ] Test each action for admin, clinician, self, carer, parent and unauthorised actors. Include last-owner
  removal, cross-household IDs, expired invitations and capacity manipulation.
  Assert `minor.has_capacity == false`, `dependent.has_capacity == false`,
  `cross_household_write_count == 0`, `household_has_owner == true` after rejected owner removal.
- [ ] Run `rtk task slice:test TARGET=care_households`; absent operations must fail.
- [ ] Implement typed actions with current grants/RLS and transactional audit through P3.
- [ ] Run all action/role cases and `rtk task ci`; verify results through real Loco routes.
- [ ] Review and commit `feat(care): preserve household roles and person grants`.

### C2: Medicines, inventory, dosage, schedules and assignments

**Files:** Medication/treatment modules/controllers above; `tests/care_medications.rs`,
`tests/care_treatments.rs`. Read existing stock adjustment service and Rust medication/schedule modules.
**Interfaces:** `medications::Command`: `Create`, `Update`, `Retire`, `AdjustStock`, `RemoveStock`,
`Order`, `Receive`, `CreateDosage`, `UpdateDosage`, `RetireDosage`;
`treatments::Command`: `CreateSchedule`, `UpdateSchedule`, `RetireSchedule`, `Assign`, `Unassign`,
`Pause`, `Resume`. Preserve original conflict/precondition semantics and date boundaries.

Split tasks into medication CRUD, stock adjustment/removal, order/receipt, dosage CRUD,
schedule CRUD, assignment/unassignment and pause/resume. Each unit uses the five steps
below, with filter prefixes `medication_crud`, `stock_adjustment`, `stock_orders`,
`dosage_crud`, `schedule_crud`, `assignments` and `pause_resume` respectively.
Use `care_medications` for the first four units and `care_treatments` for the final three.

- [ ] Test full successful/invalid/forbidden actions, duplicate receipt, two competing stock writes,
  active dosage references and pause/resume boundaries. Assert one receipt effect for replay,
  exact decimal stock, no unauthorised mutation and stable occurrence identities.
- [ ] Run `rtk task slice:test TARGET=care_medications` and `TARGET=care_treatments`; record RED.
- [ ] Implement the named actions with locking, current access and existing transactional audit requirements.
- [ ] Run both focused binaries and existing stock/treatment contract expectations against Loco, then root CI.
- [ ] Review and commit complete medication and treatment operations in independently reviewable commits.

### C3: Dose capture, history, correction and replay/sync

**Files:** Dose/sync modules/controllers above; `tests/care_doses.rs`, `tests/care_sync.rs`.
Read `rails/app/services/medication_administration/`, `offline_dose_eligibility.rb`,
`rust/api/src/dose/`, `mutation_idempotency.rs` and contract `doses.rs`, `replay.rs`, `sync.rs`.
**Interfaces:** `doses::Command`: `Take`, `Correct`, `Delete`, `RecordMissed`;
`sync::Command`: `ReadChanges`, `ReplayBatch`. Keep client UUID and sync cursor formats.

Separate tasks: `dose_take`, `dose_correction`, `dose_delete`, `dose_missed` in `care_doses`;
`sync_read` and `sync_replay` in `care_sync`. Apply the following cycle to each filter;
the Take and Replay tasks own the concurrency/idempotency assertions.

- [ ] Test simultaneous same-UUID doses, same UUID/different payload, insufficient stock,
  partial batch conflicts, revoked permission before replay and forced audit failure.
  Assert `take_count_after_duplicate == 1`, `stock_decrements == 1`, `successful_audits == 1`,
  `payload_mismatch_is_conflict == true`, `writes_after_audit_failure == 0`.
- [ ] Run `rtk task slice:test TARGET=care_doses` and `TARGET=care_sync`; record actual invariant failures.
- [ ] Implement existing idempotency/conflict rules and atomic dose/stock/audit writes; include explicit
  historical timezone/occurrence handling. Preserve unsuccessful-access audit rules separately.
- [ ] Run concurrent/replay tests, route-level contracts and root CI. Verify database state, not only responses.
- [ ] Obtain concurrency/permission review, integrate and publish; update the slice report.

**Done:** Every named operation works through Loco and has positive, validation, role, concurrency and
replay evidence. API and browser callers reuse these operations; no read-only substitute counts as migration.
