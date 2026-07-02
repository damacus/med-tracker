# ADR 0006: MedicationTake Aggregate Source Boundary

- Status: Accepted
- Date: 2026-07-02

## Context

`MedicationTake` records a dose administered from either a scheduled plan or an ad hoc person-medication assignment. The table historically represented that with two optional foreign keys, `schedule_id` and `person_medication_id`, plus a Rails validation requiring exactly one of them.

That validation expressed the domain invariant, but it left the database unable to reject invalid rows written outside model validations. It also spread source discriminator logic across helpers, policies, serializers, and reporting code.

## Decision

Keep `MedicationTake` as the single dose administration record and make the source boundary explicit in two places:

- Add a PostgreSQL check constraint requiring exactly one of `schedule_id` or `person_medication_id`.
- Introduce `MedicationDoseSource` as the value object used by `MedicationTake#dose_source`, `#source`, `#source_type`, `#source_record_id`, `#person`, and `#medication`.

The aggregate ownership remains:

- `Schedule` owns scheduled dose records.
- `PersonMedication` owns ad hoc dose records.

## Rationale

### Why keep one table

Dose records share auditing, stock mutation, offline sync, reporting, authorization, and notification behavior. Splitting into `ScheduledDose` and `AdHocDose` would duplicate that surface or introduce a new query abstraction across most medication history paths.

### Why not polymorphic `sourceable`

A polymorphic association would make the discriminator explicit, but Rails cannot create normal foreign keys for polymorphic targets. This application relies heavily on household-scoped composite foreign keys, so losing concrete references would weaken tenant integrity.

### Why a value object plus constraint

The check constraint enforces the invariant for every writer. The value object gives application code a single source boundary without changing existing foreign keys, policies, or reporting queries.

## Consequences

### Positive

- Invalid dual-source or missing-source rows are rejected by PostgreSQL.
- Domain code can depend on one explicit source object.
- Existing reporting and API queries keep their concrete joins.
- Household composite foreign keys remain intact.

### Negative

- Callers still need two concrete joins when querying across both source tables.
- The application still carries two nullable foreign keys, but the nullability is now constrained as a pair.

## Follow-up

- Prefer `MedicationTake#dose_source` for new domain logic.
- Revisit a table split only if scheduled and ad hoc doses develop materially different lifecycle behavior.
