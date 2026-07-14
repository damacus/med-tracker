# Record lifecycle contract

This operational contract defines the retirement and reactivation lifecycle for
Medication, Person, and Location roots. It is the authoritative contract for
implementation and acceptance tests.

## Definitions and shared state machine

- Retirement is explicit and reversible.
- Retirement is an explicit, reversible transition to logical cold storage.
- Logical cold storage is a lifecycle and visibility state, not a separate
  database tier.
- Every root has the states `active`, `retired`, and `hard_deleted`.
- Allowed transitions are `active -> retired` (retire), `retired -> active`
  (reactivate), and `active -> hard_deleted` only when the hard-deletion gate
  below is satisfied. `hard_deleted` is terminal and is never recreated by
  reactivation, import, or restore.
- The selected root itself changes lifecycle state in every retirement or
  reactivation transition; changing a child source or relationship is not a
  substitute for changing the selected root.
- Repeating a completed same-state transition is idempotent. No transition
  silently changes another root.
- Reactivation changes only the selected root. It never silently restores
  associated schedules, assignments, stock placement, memberships, or care
  relationships.
- Retired roots remain retained and labelled for authorized history, admin,
  reports, API sync, export/import, and restoration. Active selectors and
  future activity exclude them.

## Medication state machine

Medication Catalogue owns the Medication root lifecycle (product identity and
its cold-storage state). Medication Administration owns only the child
administration sources (`Schedule` and `PersonMedication`) and their rules;
that child ownership is distinct from ownership of the Medication root.

| State | Allowed transition | Invariants |
| --- | --- | --- |
| `active` | `retire` -> `retired`; `hard_delete` -> `hard_deleted` only if the deletion gate passes | The selected Medication becomes retired or is permanently deleted; no Person changes. |
| `retired` | `reactivate` -> `active` | The selected Medication becomes active; previously retired child sources remain retired or absent. |
| `hard_deleted` | None | No historical identity, protected evidence, or dependent state existed. |

Retiring a Medication retires only that Medication's active `Schedule` and
`PersonMedication` rows. Every `MedicationTake`, its source, and its root
reference remain unchanged.

## Person state machine

People and Care Delegation owns the Person root lifecycle and outbound care
relationship changes. Household Access remains the authority for grants and
membership state; Identity owns linked-account authentication state.

| State | Allowed transition | Invariants |
| --- | --- | --- |
| `active` | `retire` -> `retired`; `hard_delete` -> `hard_deleted` only if the deletion gate passes | The selected Person becomes retired; only that person's future schedules and medication assignments are retired. |
| `retired` | `reactivate` -> `active` | The selected Person becomes active; schedules, assignments, memberships, and care relationships are not recreated. |
| `hard_deleted` | None | No dependent state, protected history, audit, retention obligation, or legal hold existed. |

Person retirement never retires, deactivates, deletes, or otherwise changes
another Person. Dependants remain active and retain history even when they need
a new carer assignment.

## Location state machine

Inventory owns the Location root lifecycle and stock-placement preconditions.
Location retirement is blocked while the location is primary or holds active
stock; reassign both before retiring it.

| State | Allowed transition | Invariants |
| --- | --- | --- |
| `active` | `retire` -> `retired` only when the location is not primary and holds no active stock; `hard_delete` -> `hard_deleted` only if the deletion gate passes | The selected Location becomes retired; reassign primary status and active stock before retirement. |
| `retired` | `reactivate` -> `active` | The selected Location becomes active; prior stock placement and memberships are not restored. |
| `hard_deleted` | None | No protected history, stock, audit, retention obligation, or legal hold existed. |

## Linked users and carer relationships

If the retiring Person or a deactivated linked user is a carer, People and Care
Delegation ends only that carer's outbound active care relationships. A
deactivation does not retire or deactivate the linked Person automatically and
never cascades to the dependant.

When a minor or dependent adult would lose their last active carer, before
confirmation show this privacy-safe warning (without names, diagnoses,
medication details, or other PHI):

> This action will remove the last active carer for one or more dependants.
> Confirm to continue and assign replacement supervision.

The warning is not an indefinite block: after explicit confirmation the
transition may proceed. The dependant remains active, is visible in the
needs-carer workflow, and requires a separate explicit assignment. This does
not silently deactivate the dependant or require cascading retirement.
Reactivation never recreates care relationships; relinking is explicit.

## Authorization, confirmation, and evidence

- The existing Household Access policy authorizes each root transition for the
  selected household. Cross-household requests remain hidden or fail closed.
- Retirement and reactivation use the existing policy authority for the
  corresponding destructive or update action.
- Retirement, reactivation, and hard deletion require an explicit operation;
  a warning or page visit is never confirmation. Sole-carer transitions require
  an explicit confirmation token/flag after the privacy-safe warning.
- API mutations use the existing `Idempotency-Key` contract. Replaying the same
  key and request returns the original outcome; repeating an already completed
  same-state transition is a no-op and does not duplicate audit evidence.
- Each actual transition writes one immutable PHI-safe audit event containing
  actor, household, root type and id, transition, and time only. It must not
  contain names, diagnoses, medication values, carer details, or warning text.
  A required audit write failure rolls back the transition.

An actual transition records one PHI-safe immutable audit event with actor,
household, root type and id, transition, and time. Required audit failure rolls
back the transition.

## Visibility, exchange, and API conflicts

| Surface | Active root | Retired root |
| --- | --- | --- |
| Future activity and active selectors | Included | Excluded |
| Historical and administrative views | Included | Included with a visible retired label |
| Reports, sync, export/import, and restoration | Included | Included with retired state preserved |
| Authorization and retrieval | Existing policy authority applies | Existing policy authority applies; historical identity remains resolvable |

API sync represents retirement without deleting historical identity. It also
represents retirement/cold storage without deleting historical identity: it
preserves the same portable identity (`portable_id`) and carries
the retired state through changes, snapshots, export/import, and restore.
Import and restore preserve retired state and never activate it implicitly.

Stale or concurrent lifecycle state, and a location precondition that is still
blocked, return the existing API conflict contract: **HTTP 409 Conflict** with
this stable JSON envelope (the request id is generated by the API):

```json
{
  "error": {
    "code": "conflict",
    "message": "Record has changed since it was last read",
    "request_id": "<request id>"
  }
}
```

No partial root, child, relationship, or audit writes are observable from a
conflicted request.

## Transaction, concurrency, and deletion gates

Lock and revalidate the selected root and affected associations in one
transaction. A stale or concurrent state returns the stable conflict envelope
above without partial writes. The lock covers the location's primary and stock
checks and the carer's active relationship count.

Hard deletion is permitted only for a never-used root with no protected
history, dependant state, audit evidence, retention obligation, export
obligation, or legal hold. A root with any such state must be retired instead;
retired roots are never hard-deleted.

## Acceptance examples

| Given | When | Then |
| --- | --- | --- |
| A medication with active administration sources | Medication retirement | The selected Medication becomes retired; only its active `Schedule` and `PersonMedication` rows retire and `MedicationTake` history is unchanged. |
| A Person with no dependants | Person retirement | Only the selected Person and that person's future sources retire; no other Person changes. |
| A Person with dependants | Person retirement | The selected Person retires, dependants stay active, and any lost-care assignment is surfaced without cascading retirement. |
| A sole carer linked to a user account | Linked-user deactivation | A privacy-safe last-carer warning is shown; explicit confirmation ends outbound relationships, leaves dependants active, and enters needs-carer discovery. |
| A Location that is primary or holds active stock | Location retirement | The request returns the stable 409 conflict until reassignment is complete. |
| A retired Person, Medication, or Location | Non-cascading reactivation | Only the selected root becomes active; prior schedules, assignments, placement, memberships, and care relationships remain retired or absent. |
| A never-used root with no protected state | Hard deletion | The deletion gate passes and the root may be permanently deleted; it cannot later be restored. |
| A repeated or stale retirement request | Concurrent application | A completed repeat is idempotent; stale state receives the stable 409 conflict with no partial writes. |
