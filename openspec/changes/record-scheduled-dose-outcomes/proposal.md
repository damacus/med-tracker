## Why

MedTracker treats any scheduled dose without a `MedicationTake` as outstanding or missed, even when somebody considered the occurrence and deliberately did not administer it. [GitHub issue #1980](https://github.com/damacus/med-tracker/issues/1980) requires explicit, auditable outcomes without creating fake takes or changing stock.

## What Changes

- Give each non-PRN `Schedule` occurrence a stable identity derived from its source, expected date or dose window, and ordinal position.
- Record `taken` or `not_taken` as the occurrence outcome while keeping `MedicationTake` as the immutable administration record.
- Let an authorised recorder mark a due or overdue occurrence not taken with an optional reason and note; reject future or out-of-schedule outcomes.
- Support `refused`, `unwell`, `asleep`, `medicine_unavailable`, `clinician_advice`, and `other` reasons without providing clinical advice.
- Resolve dashboard work and missed-dose escalation for not-taken occurrences without reducing stock.
- Report taken, not-taken, and unexplained misses separately; only unexplained misses feed missed-dose patterns.
- Allow an auditable correction that reopens a not-taken occurrence or replaces it through the canonical dose-recording service.
- Add compatible web, API, offline sync, portable-data, audit, history, reporting, and generated-client support.
- Keep legacy takes valid without inventing historical occurrence links.
- Exclude direct `PersonMedication` assignments and all PRN/as-needed doses; a dependent change handles direct routine assignments.

## Capabilities

### New Capabilities

- `scheduled-dose-outcomes`: Identifies and resolves formal scheduled occurrences across user, notification, reporting, audit, and interoperability surfaces.

### Modified Capabilities

- `medication-take-sync`: Accepts an optional occurrence identity when a queued take resolves a formal scheduled occurrence while preserving existing idempotency and immutable-take behaviour.

## Impact

This change affects medication administration services, schedule projections, reminder eligibility and jobs, dashboard actions, history and adherence reports, audit evidence, PostgreSQL constraints, API/sync/portable contracts, OpenAPI, and generated clients. It depends on `record-medication-pause-periods` so expected occurrences are not generated inside paused periods. [Issue #1979](https://github.com/damacus/med-tracker/issues/1979) remains the separate automatic-inventory follow-up, and [issue #1982](https://github.com/damacus/med-tracker/issues/1982) remains the separate stock-removal follow-up; neither belongs in this stack because a not-taken outcome must not change stock.
