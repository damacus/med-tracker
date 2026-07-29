## Why

[Issue #1714](https://github.com/damacus/med-tracker/issues/1714) identifies that adding medication starts from several surfaces but repeatedly asks for person or medication context the product already knows. A context-aware launcher can carry that context into the existing unified assignment workflow while preserving authorization and a safe fallback.

## What Changes

- Define a launch-context contract for an optional person, medication, intent, and safe return destination.
- Start with the person whose Add Medication action was selected; a global Add Medication action still asks for a person.
- Preserve known medication context and allow the existing assignment form to validate or replace it.
- Fall back predictably when context is absent, stale, or unauthorized.
- Gate the launcher through account experiments, with the current entry behavior as the default.

Explicit non-goals are a new medication-assignment workflow, new medication discovery behavior, new persistence models, and changes to assignment creation semantics.

## Capabilities

### New Capabilities

- `context-aware-medication-launcher`: Carries safe launch context into the canonical medication workflow without requesting known information twice.

### Modified Capabilities

None.

## Impact

The change affects account experiment preferences, profile experiment controls, medication launch controllers, and their request, model, component, and system coverage. It adds no dependency, route, database migration, or public API.
