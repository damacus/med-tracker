## Context

`record-scheduled-dose-outcomes` establishes a two-source occurrence schema, resolver, consumer projection, and external contract but initially permits only formal `Schedule` occurrences. A routine `PersonMedication` has no configured date range or administration time. Its existing `DoseCycle`, dose limits, creation/retirement lifecycle, pause periods, and take history define when a routine dose remains expected.

## Goals / Non-Goals

**Goals:**

- Enable the existing occurrence contract for direct routine assignments.
- Preserve current dose-cycle and administration rules.
- Keep formal schedules and direct assignments distinguishable through concrete source references.
- Complete interface and report parity without duplicating resolver logic.

**Non-Goals:**

- Create expectations, reminders, or not-taken actions for as-needed assignments.
- Add configured administration times to direct assignments.
- Change maximum-dose or cooldown semantics.
- Add a second occurrence table or outcome API.

## Decisions

### Use the existing occurrence record and resolver

Enable the `person_medication_id` branch already constrained by the lower change. Do not add a new table, outcome enum, reason enum, policy family, or API shape. `MedicationDoseSource` remains the value-object boundary for resolving either concrete source.

The stable direct-assignment identity is source plus the local start date of the existing `DoseCycle` window plus one-based position. The expected position count uses the assignment's existing routine limit, defaulting exactly as the current dashboard/reminder logic does. `scheduled_at` remains null because the assignment contains no formal time.

The assignment's `created_at` date and optional `retired_at` bound valid windows. Recorded pause periods exclude overlapping windows under the same interval rule used for schedules. No occurrence can be backdated before creation, after retirement, during a full pause window, or into the future.

Rejected alternatives:

- Inventing a midnight due time would misrepresent the regimen.
- Converting direct assignments into schedules would change existing product meaning and history.
- A separate routine outcome model would duplicate authorization, audit, reporting, and sync behaviour.

### Extend one projection across both source types

Add the direct routine source adapter to the shared occurrence query. Dashboard, reminder, history, reporting, and insight consumers continue to receive the same occurrence value object and outcome categories. They branch only where source rules derive the window or display label.

Existing unlinked direct-assignment takes are allocated deterministically inside their dose-cycle window, using the same non-persistent legacy rule as formal schedules. A new linked take continues through `MedicationAdministration::RecordDose` and the shared resolver, so stock and immutable history remain unchanged.

### Reuse authorization and interoperability shapes

Resolve authorization through the `PersonMedication` person's existing view, record, and manage policy checks. The API occurrence response uses the existing source discriminator with `person_medication`; all operation IDs and outcome schemas stay stable.

Sync, portable v2, audit, and generated clients already understand the shared occurrence record. Extend validation and import preflight to allow direct routine source references. Capability metadata needs no second client feature flag if the scheduled-outcome capability explicitly advertises supported source types; add `person_medication` to that advertised list.

### Keep UI changes local to existing routine rows

Add the not-taken action and correction state to the existing direct routine dashboard row. Reuse the lower change's dialog component and exact reason copy. Do not render the action for `as_needed`, paused, future, retired, or already resolved assignments.

## Risks / Trade-offs

- **Weekly or monthly direct assignments have no precise due instant** → Treat the cycle window as due from its start and display it as untimed; never invent a clock time.
- **Current direct-routine defaults are surprising** → Reuse them without alteration and keep any dose-cycle redesign in a separate issue.
- **A source-kind branch can drift across consumers** → Centralize it in the shared occurrence query and cover each materially different derivation there, not with duplicate wiring tests.
- **As-needed rows could accidentally enter adherence data** → Reject them at the source adapter and assert their absence at the shared projection boundary.

## Migration Plan

1. Require the complete `record-scheduled-dose-outcomes` change.
2. Enable direct routine source validation and occurrence derivation without changing the database schema.
3. Extend dashboard, reminder, report, history, API, sync, and portable adapters through the shared projection.
4. Advertise `person_medication` as a supported occurrence source and regenerate clients.

Rollback disables the direct-assignment adapter and UI while retaining any recorded rows and audit evidence. Formal schedule outcomes continue unchanged. A forward fix must restore reading of retained direct outcomes before users can mutate those same cycle positions again.
