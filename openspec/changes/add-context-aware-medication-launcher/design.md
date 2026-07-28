## Context

MedTracker currently has a global Add Medication route that selects a person before opening the unified `MedicationAssignmentsController` workflow. Person pages instead open an older landing page that asks the user to choose a medication-plan type. Persisted medication links already pass `medication_id`, and the unified assignment controller already scopes people and medications, rejects forged create requests, degrades stale preselection to an empty selection, sanitizes `return_to`, and surfaces duplicate-assignment validation.

The change crosses profile experiments and medication launch controllers, but it must not duplicate assignment, authorization, or medication-plan classification logic.

## Goals / Non-Goals

**Goals:**

- Carry optional person and medication context into the unified assignment workflow.
- Avoid asking for a known authorized person twice while keeping person and medication visibly changeable.
- Preserve the current entry behavior unless an account enables the experiment.
- Keep absent, stale, unauthorized, and empty context safe and predictable.

**Non-Goals:**

- Replace or extend the unified assignment workflow.
- Change medication discovery, assignment persistence, duplicate rules, or success redirects.
- Add routes, tables, dependencies, or an open-ended intent dispatcher.

## Decisions

### Store one account-level launcher variant

Add `medication_launcher_variant` to the existing JSON preferences store with `current` and `context_aware` values. The profile Experiments card will expose the choice, and invalid values will normalize to `current`.

This reuses the established experiment mechanism and makes rollback immediate. Reusing the wizard-style preference was rejected because presentation style and launch routing are independent choices.

### Treat the existing route as the launch-context boundary

`add_medication_path` will accept optional `person_id`, `medication_id`, `intent`, and `return_to` parameters. Blank intent and `assign_medication` mean the route's existing assignment intent; unsupported intent falls back to person selection.

When the experiment is enabled, the controller will only use a requested person found by `MedicationWorkflowPeopleQuery`, which already applies the policy scope and `add_medication?` check. A valid person proceeds to the existing `new_person_medication_assignment_path` with `source: :workflow`. Missing, stale, or unauthorized person context renders the existing selector without revealing why the context was rejected.

Direct person-page launches will redirect into this same boundary with the person ID and person page as the default return destination. The current landing page remains unchanged as the experiment-off fallback.

Adding a separate launcher service was rejected because the behavior is a small HTTP routing decision with no domain mutation.

### Delegate medication and navigation safety to existing boundaries

The launcher passes `medication_id` through without querying it. `MedicationAssignmentsController` already preselects only an allowed household medication, leaves stale or foreign medication unselected, and rejects forged submissions. Its modal title exposes the person, its medication field exposes and permits changing the medication, and its existing back link returns to person selection.

`return_to` is sanitized with Rails' `url_from` before it is forwarded. Creation keeps the existing redirect to the selected person's page. Duplicate assignment outcomes keep the existing validation behavior.

## Risks / Trade-offs

- [A rejected person context looks the same as absent context] → This is intentional to avoid cross-household or person-existence disclosure.
- [The old person landing and new unified workflow coexist] → The experiment defaults to `current` and can be disabled without deployment.
- [The first version recognizes only medication assignment intent] → Unsupported values use the safe current selector rather than introducing speculative flows.

## Migration Plan

No database migration is required because the preference uses the existing JSON store. Deploy with `current` as the default, enable `context_aware` per account through Experiments, and roll back by selecting `current` or reverting the code.

## Open Questions

None.
