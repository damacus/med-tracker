# Proposed Root Record Lifecycle

> [!WARNING]
> This page records a future design. MedTracker does not currently support
> retirement or reactivation for Medication, Person, or Location records.

## Current behaviour

`Schedule` and `PersonMedication` can be retired through their existing
`retired_at` fields. Their dose history remains available.

Medication, Person, and Location do not have the root lifecycle states or
routes described below. Do not promise users that these records can be retired,
reactivated, or moved to logical cold storage.

Household offboarding has its own lifecycle and retention process. It does not
deliver the root lifecycle proposed here.

## Proposed goal

The proposed design would let an authorized user remove a root record from
future activity without erasing its protected history. It would apply the same
high-level rules to Medication, Person, and Location:

- retirement is explicit and reversible;
- retired roots stay available to authorized historical views and exports;
- active selectors and future activity exclude retired roots;
- reactivation changes only the selected root; and
- hard deletion is limited to never-used records with no protected state.

The design needs code changes, public behavior tests, API contract changes,
audit evidence, and migration planning before it becomes a product contract.

## Proposed states

| State | Meaning |
| --- | --- |
| `active` | The root is available for current activity. |
| `retired` | The root is hidden from future activity but retained for authorized history. |
| `hard_deleted` | The root no longer exists and cannot be restored. |

Proposed transitions are `active` to `retired`, `retired` to `active`, and a
guarded `active` to `hard_deleted`. Repeated requests should be idempotent.

## Medication proposal

Retiring a Medication would retire that medication's active schedules and
person-medication records. It would preserve every `MedicationTake`, dose
snapshot, source identity, and stock history.

Reactivating the Medication would not restore its child plans. Each plan would
need a separate, explicit decision.

## Person proposal

Retiring a Person would stop only that person's future medication activity. It
would preserve their history and would not retire another person.

When the person is a carer, the workflow would need a privacy-safe warning if a
dependent person would lose their last active carer. The dependent person would
stay active and enter a needs-carer workflow.

Reactivation would not recreate memberships, grants, care relationships, or
medication plans.

## Location proposal

Location retirement would be blocked while the location is a required primary
location or holds active stock. Operators would first move the primary status
and stock to another active location.

Reactivation would not restore prior stock placement or person memberships.

## Authorization and evidence

Every transition would use the existing household authorization boundary.
Cross-household records would remain hidden or fail closed.

An actual state change would write one immutable audit event with the actor,
household, root type, root identifier, transition, and time. It would not put
names, diagnoses, medication values, or care details in audit metadata. A
failed audit write would roll back the state change.

Account deactivation is a separate global identity action. Household authority
must not gain the ability to deactivate an account across other households.

## API and concurrency proposal

API mutations would use the existing `Idempotency-Key` contract. A stale state
or blocked location precondition would return HTTP 409 with the standard
conflict envelope. No partial root, child, relationship, or audit writes would
be visible.

The transaction would lock and recheck the selected root together with every
association used by its safety decision.

## Hard-deletion gate

Hard deletion would be allowed only when the root has never been used. The root
could have no protected or dependent state, including audit evidence,
retention duties, export duties, or legal holds. Every other root would require
retirement.

## Work needed before adoption

1. Add root lifecycle columns and database constraints.
2. Define authorization and confirmation for each root.
3. Add user-facing retired labels and active filters.
4. Preserve lifecycle state in sync, export, import, and restore.
5. Add audit events and rollback behavior.
6. Add API operations, conflict behavior, and idempotency coverage.
7. Prove non-cascading retirement and reactivation in public browser tests.
8. Replace this proposal with an operator contract only after those checks pass.
