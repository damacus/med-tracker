# Code Review - codex/refactor-schedules-card-1038

**Base Branch**: main
**Changed Files**: 8 files
**Review Date**: 2026-04-28

---

## Summary

Issue #1038 is implemented by turning the oversized schedule card into a thin container and three focused nested components:

- [Card container](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card.rb#L17) now delegates rendering to `HeaderComponent`, `DoseStatusComponent`, and `ActionsComponent`.
- [CardPresenter](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/presenters/schedules/card_presenter.rb#L14) owns schedule status, stock blocking, dose-count, and action-label decisions.
- [ActionsComponent](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card/actions_component.rb#L17) isolates take/edit/delete actions.
- [DoseStatusComponent](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card/dose_status_component.rb#L15) isolates date, notes, countdown, and take-history rendering.
- [HeaderComponent](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card/header_component.rb#L14) isolates medication metadata and stock/status badges.

## Critical Issues

No critical issues found.

## Design & Architecture

### OOP Violations

No blocking SOLID violations found in the changed behavior. The main card dropped from 343 lines to [47 lines](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card.rb#L1), and the extracted components each have a single rendering responsibility.

The [presenter](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/presenters/schedules/card_presenter.rb#L51) deliberately centralizes memoized take-history and availability queries so the components do not duplicate conditional decisions.

### Rails Patterns

The refactor follows existing repo patterns:

- Reuses `app/presenters/schedules` beside existing schedule form presenters.
- Keeps route and form rendering inside Phlex components.
- Preserves preloaded `todays_takes` behavior via [resolved_todays_takes](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/presenters/schedules/card_presenter.rb#L51).

No N+1 regression found; existing query-count spec still passes.

## Security Concerns

No new security concerns found. The delete form path remains scoped by person and schedule at [actions_component.rb#L90](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card/actions_component.rb#L90), and admin-only edit/delete rendering remains guarded at [actions_component.rb#L64](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/app/components/schedules/card/actions_component.rb#L64).

## Test Coverage

Added focused component specs:

- [Header component spec](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/spec/components/schedules/card/header_component_spec.rb#L21)
- [Dose status component spec](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/spec/components/schedules/card/dose_status_component_spec.rb#L24)
- [Actions component spec](file:///Users/damacus/.codex/worktrees/3d0a/med-tracker/spec/components/schedules/card/actions_component_spec.rb#L21)

Existing `Components::Schedules::Card` specs still cover integration behavior, invalid dose disabling, cooldown disabling, i18n, memoization, and preloaded take histories.

## Tool Reports

### RubyCritic Summary

RubyCritic is not installed or configured in this repo environment (`which rubycritic` returned no executable), so no RubyCritic metrics were available.

### SimpleCov Summary

SimpleCov is not configured in this repo (`rg 'SimpleCov|COVERAGE|simplecov'` found no matches), and the full suite did not create a `coverage/` directory. No coverage percentage is available.

### Verification

- `task rubocop`: passed with no offenses.
- `task test TEST_FILE=spec/components/schedules`: passed, 18 examples.
- `task test`: passed, 1998 examples, 0 failures, 2 expected pending.

## Recommendations

No required follow-up for issue #1038. A later cleanup could introduce a shared take-history renderer for schedule and person-medication cards, but that is outside this issue and the current code follows the established component style.

## Positive Observations

The refactor keeps the external `Components::Schedules::Card` interface stable while making each subcomponent directly testable. The presenter preserves existing memoization for stock and timing checks, which keeps the previous query-count behavior intact.
