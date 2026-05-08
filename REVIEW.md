# Code Review - codex/medication-scan-stock-merge

**Base Branch**: origin/main
**Changed Files**: 5
**Changed Ruby Files**: 5
**Review Date**: 2026-05-08
**Review Skills**: review-ruby-code, RubyCritic availability check, SimpleCov availability check

---

## Summary

This branch updates medication creation so scanned stock can merge into an existing inventory item instead of creating a duplicate, while preserving separate inventory rows for different strengths such as 200mg and 400mg ibuprofen.

The authorization boundary is sound: matching is scoped through `policy_scope(Medication)`, and the new request specs cover the primary same-strength merge and different-strength split. I found one behavioral bug in the failure path for merged wizard submissions, plus design and coverage risks around the new matching service.

## Critical Issues

High: [MedicationOnboardingCreateService#restock_existing_medication returns the existing medication even when schedule creation fails](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/services/medication_onboarding_create_service.rb#L144), while [schedule errors are copied onto the unsaved candidate medication](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/services/medication_onboarding_create_service.rb#L193). The controller then [replaces `@medication` with `result.medication`](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/controllers/medications_controller.rb#L79) before rendering failure, so a failed merged wizard submission can render the existing persisted medication without the validation errors that explain why the schedule failed. Keep the errored candidate as the failure result, copy errors to the returned object, or make `Result` carry a separate `form_record`.

## Design & Architecture

### OOP Violations

Warning: [MedicationInventoryMatcher is 167 lines and owns barcode matching, dm+d matching, name normalization, strength parsing, form parsing, and decimal normalization](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/services/medication_inventory_matcher.rb#L3). This exceeds the Sandi Metz 100-line class rule. The class is cohesive enough for the first version, but adding more medicine-name heuristics should be done by extracting small collaborators such as a `MedicationNameFingerprint` or `MedicationStrengthParser`.

Warning: [MedicationOnboardingCreateService now handles normal creation, schedule creation, matching, aggregate restock, per-dose restock, and merge failure behavior](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/services/medication_onboarding_create_service.rb#L19). It was already a workflow service, but the new merge branch has pushed it toward multiple reasons to change. Consider extracting the merge operation if more scan/import behavior is added.

### Rails Patterns

No N+1 query found in the reviewed change. [MedicationInventoryMatcher preloads dosage records before scanning authorized medications in memory](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/services/medication_inventory_matcher.rb#L72).

Performance note: the matcher uses Ruby-side `detect` across the full authorized medication scope for fallback name matching. That is acceptable for household inventory sizes, but it will not scale well if clinicians/admins scan against a large shared inventory. If that becomes a real path, prefilter by normalized terms or add a persisted/searchable fingerprint.

The controller integration follows existing patterns: [MedicationsController passes policy-scoped medications into the service](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/controllers/medications_controller.rb#L258), and [MedicationWizardSupport accepts a caller-supplied success notice without changing the response shape](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/controllers/concerns/medication_wizard_support.rb#L8).

## Security Concerns

No new security issue found.

Positive observations:

- [Matching is bounded by `policy_scope(Medication)`](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/app/controllers/medications_controller.rb#L263), so inaccessible inventory is not silently merged.
- The new matcher does not build SQL from user-controlled input in the fallback path; normalization and matching are Ruby-side.
- Existing duplicate-barcode validation remains in place for inaccessible inventory collisions.

## Test Coverage

The new request coverage is valuable and behavior-oriented:

- [Same-strength scan restocks the existing ibuprofen item without increasing `Medication.count`](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/spec/requests/medications_create_scope_spec.rb#L264).
- [Different-strength scan creates a separate ibuprofen inventory item](file:///Users/damacus/.codex/worktrees/8c9c/med-tracker/spec/requests/medications_create_scope_spec.rb#L294).

Coverage gaps:

- Add a request or service spec for the merged wizard failure path described in Critical Issues.
- Add focused `MedicationInventoryMatcher` specs for name/strength parsing edge cases: `micrograms`, `250mg/5ml`, pack counts in names, form mismatch, and same dm+d code precedence. Request specs cover the main user outcome, but the matcher now contains enough parsing logic to deserve unit-level examples.

Focused verification run during this review:

- `env COVERAGE=true task test TEST_FILE=spec/requests/medications_create_scope_spec.rb`
- 21 examples, 0 failures

## Tool Reports

### RubyCritic Summary

RubyCritic metrics are unavailable in this worktree:

- `rubycritic --format json --no-browser ...` failed because `rubycritic` is not installed on `PATH`.
- `task -l` does not expose a RubyCritic task.

### SimpleCov Summary

SimpleCov metrics are unavailable:

- No SimpleCov configuration or gem entry was found in `Gemfile`, `Gemfile.lock`, `spec`, or `config`.
- Running the focused request spec with `COVERAGE=true` produced no `coverage/` directory.

## Recommendations

1. Fix the merged wizard failure path so validation errors are rendered on the object returned to the controller.
2. Add unit specs for `MedicationInventoryMatcher` before extending matching heuristics.
3. Extract name/strength parsing from `MedicationInventoryMatcher` if another medication-form or strength rule is added.
4. If RubyCritic/SimpleCov reports are required for review workflows, add repo-native `task` wrappers so analysis can run consistently in the project container.

## Positive Observations

- The change preserves the safety requirement that different strengths remain separate.
- Matching is kept out of the controller and follows the repo's service-object style.
- The request specs cover both sides of the highest-risk behavioral boundary.
- The branch kept quality gates green before review: RuboCop and the full suite passed post-rebase.
