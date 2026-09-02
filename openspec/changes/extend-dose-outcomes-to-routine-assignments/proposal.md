## Why

Direct routine `PersonMedication` assignments generate expected doses without a formal `Schedule`. After formal scheduled outcomes ship, leaving these routine assignments on inferred take counts would give users two different ways to resolve the same kind of medication task.

## What Changes

- Extend occurrence identity and explicit outcomes to routine `PersonMedication` assignments.
- Derive each assignment occurrence from its dose-cycle window and ordinal position because direct assignments have no formal scheduled timestamp.
- Reuse the scheduled-outcome reason, audit, correction, authorization, reporting, notification, API, sync, portable-data, and stock rules.
- Show due or overdue direct-routine outcomes in the existing dashboard workflow.
- Keep `as_needed` assignments and retained PRN schedules outside occurrence generation.
- Do not change dose-cycle limits, invent administration times, or add reminders for as-needed medication.

## Capabilities

### New Capabilities

- `routine-dose-outcomes`: Resolves expected occurrences from direct routine medication assignments without treating as-needed use as scheduled work.

### Modified Capabilities

None.

## Impact

This change extends medication administration occurrence queries, dashboard and reminder projections, reports, audit, API/sync/portable adapters, OpenAPI, and generated clients. It depends on `record-scheduled-dose-outcomes` and should be the top stack slice.
