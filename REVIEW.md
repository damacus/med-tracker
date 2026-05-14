# Code Review - codex/pluralize-gummy-dosage

**Base Branch**: origin/main
**Reviewed File**: app/domain/dose_amount.rb
**Review Date**: 2026-05-14

---

## Summary

Reviewed the single requested file after the refactor from implicit stringification to an explicit dose label API. The class is small, cohesive, and now avoids overriding `Object#to_s` for user-facing formatting. Integer decimal values such as `BigDecimal('1.0')` render as `1 tablet`, while meaningful decimals still render as decimals.

## Critical Issues

None found.

## Design & Architecture

### OOP Violations

No blocking OOP issues found. [DoseAmount#label](file:///Users/damacus/.codex/worktrees/4e03/med-tracker/app/domain/dose_amount.rb#L17) is explicit about presenting a user-facing label rather than redefining generic object stringification.

RubyCritic reports `FeatureEnvy` for [DoseAmount#formatted_amount](file:///Users/damacus/.codex/worktrees/4e03/med-tracker/app/domain/dose_amount.rb#L25), but this is a false positive for a small value-formatting method whose local `BigDecimal` variable is intentionally the object being formatted.

### Rails Patterns

No Rails query, callback, or controller concerns apply to this plain domain value object.

One maintainability risk remains: [PLURALIZABLE_UNITS](file:///Users/damacus/.codex/worktrees/4e03/med-tracker/app/domain/dose_amount.rb#L4) is a separate countable-unit list from `MedicationStockConsumption::COUNTABLE_UNITS`. The previous missing `gummy` case came from that kind of list drift. Consider centralizing countable medication units if another unit list changes.

## Security Concerns

None found. The file does no SQL, HTML rendering, authorization, or parameter handling.

## Test Coverage

Targeted SimpleCov run covered all executable lines in `app/domain/dose_amount.rb`: 16 / 16 lines covered. Branch coverage for this file was also exercised by the focused `DoseAmount` specs.

The focused specs cover integer formatting, float formatting, decimal integer formatting, meaningful decimals, blank amount/unit handling, countable unit pluralization, irregular `gummy` pluralization, and measurement-unit non-pluralization.

## Tool Reports

### RubyCritic Summary

- **Rating**: A
- **Score**: 94.18
- **Complexity**: 23.27
- **Duplication**: 0
- **Churn**: 4
- **Smells**: 2

RubyCritic smells:
- `FeatureEnvy` on [DoseAmount#formatted_amount](file:///Users/damacus/.codex/worktrees/4e03/med-tracker/app/domain/dose_amount.rb#L25), assessed above as acceptable for this value formatter.
- `IrresponsibleModule` on [DoseAmount](file:///Users/damacus/.codex/worktrees/4e03/med-tracker/app/domain/dose_amount.rb#L3). No class comment was added because the repository instructions say not to add comments unless explicitly asked.

### SimpleCov Summary

- **Targeted suite**: `COVERAGE=true task test TEST_FILE=spec/domain/dose_amount_spec.rb`
- **Overall targeted run line coverage**: 35.32%
- **Reviewed file line coverage**: 100% (16 / 16)

## Recommendations

1. Keep the explicit `#label` API and avoid reintroducing `#to_s` for product copy.
2. Consider centralizing countable dosage units if more unit-related behavior is added.
3. Keep the `BigDecimal` formatting path; it avoids the precision loss and `.0` regression risk from `to_f`.

## Positive Observations

- The class stays under Sandi Metz size guidance.
- Methods are short and focused.
- The public API now communicates presentation intent directly.
- The focused spec suite covers the reported `1.0 tablet` regression and the prior `2 gummies` regression.
