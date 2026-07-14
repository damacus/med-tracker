# Record lifecycle contract

This operational contract defines the retirement and reactivation lifecycle for
Medication, Person, and Location roots. It is the authoritative contract for
implementation and acceptance tests.

## Definitions

- Retirement is explicit and reversible.
- Logical cold storage is a lifecycle and visibility state, not a separate
  database tier.
- Retired roots remain authorized and resolvable with a visible retired label
  in historical and administrative views, reports, sync, export/import, and
  restoration. Active selectors and future activity exclude them.
- Reactivation changes only the selected root. It never silently restores
  associated schedules, assignments, stock placement, memberships, or care
  relationships.

## Root transition rules

| Root | Retirement | Reactivation |
| --- | --- | --- |
| Medication | Retires only that medication's active `Schedule` and `PersonMedication` rows. It never changes a `Person` lifecycle state. | Does not reactivate those sources. |
| Person | Retires only that person's own future schedules and medication assignments. It never retires, deactivates, deletes, or otherwise changes another `Person`. | Does not recreate care relationships or medication assignments. |
| Location | Location retirement is blocked while it is primary or holds active stock; reassign first. | Does not restore prior placement or memberships. |

If the retiring Person or a deactivated linked user is a carer, end only that
carer's outbound active care relationships. Before confirmation, warn when a
minor or dependent adult will lose the last active carer and requires
supervision assignment. The dependant remains active, retains history, and is
discoverable through the needs-carer workflow. Reactivation never recreates
relationships; relinking is explicit.

## Visibility matrix

| Surface | Active root | Retired root |
| --- | --- | --- |
| Future activity and active selectors | Included | Excluded |
| Historical and administrative views | Included | Included with a visible retired label |
| Reports, sync, export/import, and restoration | Included | Included with retired state preserved |
| Authorization and retrieval | Existing policy authority applies | Existing policy authority applies; historical identity remains resolvable |

Retirement and reactivation use the existing policy authority for the
corresponding destructive or update action. Cross-household requests remain
hidden or fail closed.

## History, deletion, and exchange

Every `MedicationTake` and its source and root references remain unchanged.
An actual transition records one PHI-safe immutable audit event with actor,
household, root type and id, transition, and time. Required audit failure rolls
back the transition.

Hard deletion is allowed only for never-used roots with no protected history,
dependent state, audit, retention, export obligation, or legal hold. API sync
represents retirement without deleting historical identity. Import and restore
preserve retired state and never activate it implicitly.

## Transaction and concurrency rules

Lock and revalidate the root and affected associations in one transaction.
Repeating the same completed transition is idempotent and does not duplicate
audit evidence. Stale or concurrent state returns a stable conflict without
partial writes.

## Acceptance examples

| Given | When | Then |
| --- | --- | --- |
| A medication with active administration sources | It is retired | Only its active `Schedule` and `PersonMedication` rows retire; `MedicationTake` history is unchanged. |
| A person who is a sole carer for a dependent adult | Retirement is requested | Confirmation warns of the last-carer loss and requires supervision assignment; the dependant stays active and enters needs-carer discovery. |
| A retired person, medication, or location | It is reactivated | Only the selected root becomes active; prior schedules, assignments, placement, memberships, and care relationships remain retired or absent. |
| A location that is primary or holds active stock | Retirement is requested | The request is blocked until reassignment is complete. |
| A repeated or stale retirement request | The transition is applied concurrently | A completed repeat is idempotent; stale state receives a stable conflict with no partial writes. |
