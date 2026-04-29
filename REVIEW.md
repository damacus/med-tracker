# Code Review - codex/login-page-storyboard-redesign

**Base Branch**: origin/main
**Changed Ruby Files**: 24
**Review Date**: 2026-04-29
**Ruby LSP**: Not available in this Codex session; fallback `rg`, outlines, targeted reads, RubyCritic/SimpleCov attempts, and tests were used.

---

## Summary

This branch redesigns the Rodauth login page with a split auth layout, extracted auth artwork components, passkey/OIDC sign-in options, deferred themed illustration assets, and a login-specific logo/chevron treatment.

The production Ruby changes are generally well scoped and move the login visual complexity out of `LoginBrandSupport`. The shared `ChevronRight` behavior is restored correctly, login call sites opt into their custom chevron path/stroke, and the breadcrumb separator spec now protects the shared default path.

## Critical Issues

No critical issues remain from the focused review pass.

## Design & Architecture

### OOP Violations

No high-impact Sandi Metz/SOLID violations found in the current login extraction.

Positive observations:

- [LoginBrandSupport is now a focused composition module](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/app/views/rodauth/login_brand_support.rb#L5), down from the previous large visual-rendering module.
- [BenefitIconTile centralizes benefit icon dispatch](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/app/components/auth/benefit_icon_tile.rb#L5) without leaking icon switch logic back into the login view.
- [ChevronRight now preserves shared defaults and exposes opt-in overrides](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/app/components/icons/chevron_right.rb#L8), which addresses the PR review thread without creating a separate login-only icon class.

### Rails Patterns

No N+1 query, callback, scope, or service-object issues were found in the changed login component code. The changes are view/component oriented and do not introduce database reads beyond existing `oauth_enabled?` and `invite_only?` behavior.

### Maintainability Notes

[MtLogo#view_template](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/app/components/auth/mt_logo.rb#L11) is currently 27 lines because it contains the full inline SVG. This is acceptable for a small, isolated logo component, but if the logo is reused outside login or gains variants, prefer extracting repeated SVG primitives or loading a static asset to keep the component easy to scan.

[MedicationIllustration#illustration_attrs](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/app/components/auth/medication_illustration.rb#L31) cleanly centralizes the four themed asset paths. The explicit `image_path_resolver` dependency keeps the component testable and avoids reaching into Rails helpers implicitly.

## Security Concerns

No new security issues found in the reviewed Ruby changes.

Positive observations:

- [MedicationIllustration renders its inline activation script with a CSP nonce](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/app/components/auth/medication_illustration.rb#L59).
- The login illustration image is decorative (`alt: ''`, `aria_hidden: 'true'`) while the wrapper keeps the localized `role="img"` label.
- No raw SQL, mass assignment, unescaped user content, or authorization changes were introduced in the reviewed Ruby files.

## Test Coverage

### Passing Focused Tests

- `task test TEST_FILE=spec/components/icons/chevron_right_spec.rb`
- `task test TEST_FILE=spec/components/views/rodauth/login_spec.rb`
- `task test TEST_FILE=spec/components/ruby_ui/breadcrumb_separator_spec.rb`

### Full Suite / Coverage

`task test` passed after the breadcrumb spec fix during this final push pass:

- 1983 examples, 0 failures, 2 pending

### Missing or Stale Test Coverage

[ChevronRight spec covers both default and opt-in variants](file:///Users/damacus/.codex/worktrees/76a4/med-tracker/spec/components/icons/chevron_right_spec.rb#L6), and the breadcrumb separator spec now asserts the shared default path for non-login callers.

## Tool Reports

### RubyCritic Summary

RubyCritic could not be run:

- `rubycritic --format json --no-browser ...` failed because `rubycritic` is not installed as a direct executable.
- `bundle exec rubycritic --version` failed because the gem executable is not present in the bundle.

RubyCritic metrics are therefore unavailable.

### SimpleCov Summary

SimpleCov metrics are unavailable because the full suite was not run with coverage enabled.

## Recommendations

1. If RubyCritic is required as a quality gate, add it to the bundle or document that this review skill cannot run it in the current repo environment.

## Positive Observations

- The original review findings are addressed: `LoginBrandSupport` is small, the login spec is split into focused examples, and `secondary_sign_in_visible?` names the OIDC visibility policy.
- The chevron review thread is addressed in production code with the simpler approach: shared defaults are preserved, login call sites opt into the larger path/stroke.
- The new logo and illustration components isolate visual markup from Rodauth login flow logic.
