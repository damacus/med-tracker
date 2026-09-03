## Why

Medication pauses currently suppress future doses without recording why the treatment was suspended or preserving a pause timeline after it resumes. [GitHub issue #1981](https://github.com/damacus/med-tracker/issues/1981) requires an attributable reason and optional context for every current pause workflow.

## What Changes

- Record each pause as a household-scoped period with a reason, optional note, start, end, and actors.
- Apply the same pause history to `Schedule` and `PersonMedication`, including routine and as-needed assignments.
- Require the web workflow and new API operation to select `out_of_supply`, `temporarily_not_needed`, `clinician_advice`, `side_effects`, or `other`.
- Show the active pause context and retain completed periods in medication history, reports, audit evidence, sync, and portable data.
- Exclude paused periods from expected-dose and adherence calculations.
- Add an additive, capability-advertised API operation that requires pause context. Keep the existing `/api/v1` pause operation compatible during its deprecation period and record omitted context as `reason_not_recorded`.
- Preserve existing paused records without inventing a historical start date or reason.
- Do not add stock/reorder automation, clinical advice, or one-off dose outcomes.

## Capabilities

### New Capabilities

- `medication-pause-periods`: Records, displays, exchanges, and reports attributable pause periods for every pausable medication source.

### Modified Capabilities

None.

## Delivery Shape

**Mode**: Small dependent PR stack

**Stack scope**: Cross-slice

**Reason**: The three product outcomes remain coherent specifications, but each implementation boundary must fit one focused test cycle, remain well below 2,000 changed lines, and target less than one hour of active work. The fixed lower-first order keeps each review focused without separating tests from behaviour.

**Done when**: Pauses retain attributable periods, formal scheduled doses support explicit outcomes, and the same outcome contract covers direct routine assignments while PRN medication and separate inventory work remain excluded.

| # | Product slice | Tranches | Depends on | Release state |
| --- | --- | --- | --- | --- |
| 1 | `record-medication-pause-periods` | 16 | None | Each accepted tranche is dormant, backward compatible, or independently useful |
| 2 | `record-scheduled-dose-outcomes` | 19 | Pause interval projection | Each accepted tranche preserves current behaviour until its user surface is enabled |
| 3 | `extend-dose-outcomes-to-routine-assignments` | 11 | Shared occurrence model and resolver | Completes routine-dose coverage without changing PRN behaviour |

The durable team plan records every tranche's owned paths, evidence, review gate, and branch order. Any tranche that approaches 1,200 changed lines or 45 minutes of active work must stop at a safe boundary and be split before delivery.

## Impact

This change affects medication administration lifecycle services, schedule and person-medication web flows, product API and capability metadata, portable/sync records, history and report projections, PaperTrail evidence, PostgreSQL constraints, OpenAPI, and generated clients. It is the lower stack dependency for explicit routine-dose outcomes because occurrence generation must respect pause periods.
