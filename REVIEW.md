# Code Review - codex/restock-authorization-finder

**Base Branch**: origin/main
**Changed Files**: 30
**Changed Ruby Files**: 18
**Review Date**: 2026-05-13
**Review Skills**: review-ruby-code, rubycritic availability check, simplecov availability check

---

## Summary

This branch fixes the medication restock authorization path, removes the floating shortcut menu, and turns Medication Finder's existing-medication action into a restock modal. The core authorization shift is directionally correct: refill endpoints now authorize `refill?`, finder access allows add-or-restock users, and request/system specs cover the parent/carer restock paths.

I found no critical security issue. The main risk is a permissive fallback in the medication show component that can hide policy wiring mistakes by rendering privileged controls when policy lookup fails. There are also two smaller contract/test gaps around finder response consistency and the unknown package quantity modal path.

## Critical Issues

None found.

## Design & Architecture

### OOP Violations

Medium: [Components::Medications::ShowView#can_update? rescues `NoMethodError` and returns `true`](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/components/medications/show_view.rb#L202), and [#can_refill? does the same](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/components/medications/show_view.rb#L208). That means a missing policy helper, a missing policy method, or a policy wiring regression falls back to showing edit/restock controls. This weakens the branch goal that users should not see buttons they cannot perform. Prefer explicit permission inputs on the component, or require the view context policy and update component specs to stub it directly.

Warning: [MedicationsController is now 378 lines](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/controllers/medications_controller.rb#L1). The new restock/finder changes are small and follow existing controller style, but the controller already owns creation, finder search, refill, adjustment, reorder, stream rendering, and scan workflows. If more finder/restock behavior lands, extract a finder presenter/responder boundary rather than growing the controller further.

### Rails Patterns

The authorization pattern is sound in the controller: [refill uses `authorize @medication, :refill?`](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/controllers/medications_controller.rb#L112), and [scan restock authorizes the matched medication before restocking](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/controllers/medications_controller.rb#L136).

The finder match path remains scoped through `MedicationStockMatchResolver` initialized from [policy-scoped medications](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/controllers/medications_controller.rb#L272), so inaccessible stock is not exposed in normal search/match responses.

Low: [MedicationFinderSearchResponder#unavailable_response omits the `permissions` payload](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/services/medication_finder_search_responder.rb#L38), even though successful and blank responses include it. The current Stimulus controller ignores permissions when `error` is present, so this is not breaking today. Still, the branch contract says the finder response includes create/restock ability; passing `permissions` through error responses would keep the JSON schema stable for future clients.

## Security Concerns

No direct SQL injection, mass-assignment, or IDOR issue found in the changed Rails code.

The highest security-adjacent concern is the permissive component fallback described above. Authorization still happens server-side for refill/update/destroy, so the fallback does not grant the action by itself, but it can reintroduce unauthorized UI affordances and confusion.

Positive security observations:

- [MedicationPolicy#finder? allows finder access only through `create? || refill?`](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/policies/medication_policy.rb#L46).
- [Search results build existing medication metadata only from the scoped stock matcher](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/services/medication_finder_search_responder.rb#L45).
- [Request coverage asserts inaccessible matches are not exposed](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/requests/medications_refill_spec.rb#L196).

## Test Coverage

Strong coverage added:

- [Parent can restock through `PATCH /medications/:id/refill`](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/requests/medications_refill_spec.rb#L35).
- [Parent can scan-restock an accessible medication](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/requests/medications_refill_spec.rb#L94).
- [Carer finder access returns restock-only permissions](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/requests/medications_search_spec.rb#L251).
- [Medication Finder opens the known-quantity restock modal and submits it](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/system/medication_finder_spec.rb#L30).
- [Mobile navigation asserts the floating action menu is gone](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/system/mobile_navigation_spec.rb#L171).

Coverage gaps:

- Add a system or JS-level spec for the unknown package quantity path in [MedicationSearchController#renderRestockModal](file:///Users/damacus/.codex/worktrees/950d/med-tracker/app/javascript/controllers/medication_search_controller.js#L204). The current system spec always sends [package_quantity: 30](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/system/medication_finder_spec.rb#L78), so the required manual quantity field is not exercised.
- Add a request spec that service-unavailable finder responses preserve permissions if you decide to make the JSON schema consistent. Existing error specs assert only [status and generic error text](file:///Users/damacus/.codex/worktrees/950d/med-tracker/spec/requests/medications_search_spec.rb#L140).

Focused verification already run on this branch before review:

- `task rubocop` passed: 975 files, no offenses.
- `task test` passed after clean retry: 2383 examples, 0 failures, 2 pending.

## Tool Reports

### RubyCritic Summary

RubyCritic metrics are unavailable in this worktree:

- `rubycritic` is not installed on `PATH`.
- `task -l` exposes no RubyCritic task.
- The repo's `scripts/check_quality.sh` would mutate `Gemfile` to install RubyCritic and uses Bash, so I did not run it during this Fish-only review pass.

### SimpleCov Summary

SimpleCov metrics are unavailable:

- No `simplecov` or `coverage` configuration was found in `Gemfile`, `Gemfile.lock`, `Taskfile.yml`, or `spec`.
- No `coverage/` output directory exists in this worktree.

## Recommendations

1. Replace the permissive `NoMethodError => true` fallbacks in `Components::Medications::ShowView` with explicit permission injection or strict policy calls.
2. If finder JSON is a public-ish internal contract, include permissions in service-unavailable responses too.
3. Add coverage for the unknown package quantity modal flow before relying on scanned products without pack metadata.
4. Add repo-native `task` wrappers for RubyCritic/SimpleCov if those reports are mandatory review artifacts.

## Positive Observations

- The branch keeps restock authorization server-side and policy-scoped.
- UI permission splitting matches the intended `refill?`, `update?`, and `destroy?` boundaries on the main medication cards.
- The finder modal posts to the existing refill endpoint instead of inventing a parallel restock path.
- The removal of the floating shortcut menu is complete across layout, Stimulus, CSS, and specs.
