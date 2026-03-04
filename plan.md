# Implementation Plan

## N+1 Query Optimization
1. **Understand the N+1 issue**:
   - The view `app/components/medications/show_view.rb` calls `dosages = medication.dosages.order(:amount)` on line 306.
   - When iterating through `dosages` later in the method (`dosages.each do |dosage|`), Rails triggers an N+1 issue.
   - Even if we preload `dosages` in the controller (`includes(:dosages)`), calling `.order` on an association triggers a *new* database query because `.order` is executed at the database level rather than entirely in Ruby.

2. **Implement Fix**:
   - Following `.jules/bolt.md` guidelines for preloaded associations, I need to replace database-level methods with Ruby `Enumerable` equivalents when dealing with `dosages`.
   - In `app/components/medications/show_view.rb`, I'll change:
     ```ruby
     dosages = medication.dosages.order(:amount)
     ```
     to:
     ```ruby
     dosages = medication.dosages.sort_by(&:amount)
     ```
     This filters and sorts using Ruby entirely in memory without hitting the database, allowing preloading to work effectively.

3. **Verify Fix**:
   - I will run `task test` to ensure functionality is intact and confirm no regressions in rendering.

## Fix Schedule Dosage Selection + Flexible Dosing + Inventory Labels/Assignees

### Summary
Implement a combined bugfix and feature update on branch `codex/fix-schedule-dosage-and-inventory-labels` to:
1. Fix non-selectable dosage in new schedule flow.
2. Support flexible schedule doses (`0.5`, `2`, `3`, etc.) with custom amount + unit.
3. Make inventory quantity copy unit-aware (`16 sachets`, fallback to `units`).
4. Add assignee badges on inventory cards (first names, multiple people).
5. Redesign medication setup to capture dose options with mobile-friendly Yes/No + chips + CSV input, and persist those as dosage presets.
6. Require medication unit on create/edit validation.

### Implementation Details
- DB schema updated to support `custom_dose_amount` and `custom_dose_unit` on `Schedule`.
- `Schedule` model updated with `effective_dose_amount` and `effective_dose_unit` helpers.
- `Medication` model requires valid `dosage_unit`.
- Inventory cards updated with assignee badges and unit-aware labels.
