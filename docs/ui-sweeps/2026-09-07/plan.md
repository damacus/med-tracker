# Web UI sweep

Baseline: `45db2112` on `codex/fix-web-ui-sweep`. Test and local development are authoritative;
Canary is excluded because it has not been updated to the latest release.

## Team contract

The coordinator owns acceptance and publication. Luna scouts inspect bounded surfaces read-only.
One Luna writer owns product/test changes for each active tranche, including review corrections.
Terra independently reviews requirements and code quality; consequential ambiguity escalates to Sol.
No separate project charter was found; repository instructions and team skill boundaries apply.

## Coverage and acceptance

Cover household, admin and signed-out routes: dashboard, medicines, schedules, people, inventory,
locations, finder, reviews, reports and settings. Inspect light/dark at 390 and 1280 pixels; stress
shared components at 320 pixels, tablet widths and enlarged text. Exercise empty/populated states,
long names, singular/plural counts, errors, disabled actions, focus and dialogs. Use isolated fixture
data for role checks and state-changing scenarios. Record unavailable surfaces explicitly.

Findings need route, role, viewport/state, expected behaviour, reproducible evidence, source location,
severity and independent review. Source-only suspicions are not confirmed visual defects.

## Tranches

1. Reproduce and fix card action clipping; strengthen browser geometry coverage.
2. Fix reviewed wording defects with synchronised translations and singular/plural tests.
3. Fix additional accepted sweep findings grouped by shared component and verification boundary.

No changes to dosing rules, permissions, database/public API contracts or native apps.
Keep comments intact. Write a failing regression first. One writer completes a tranche through review
before the next begins. Create/reuse tracking issues before implementation.

## Verification

Use `task test:preflight` and focused `task test TEST_FILE=...`, including Playwright-backed system
specs in the isolated test project. The repository's `task playwright` uses a shared host database,
so it is excluded from this sweep in favour of isolated browser execution through `task test`.
Before publication run `task rubocop`, `task test`, `git diff --check` and appropriate documentation
and locale checks. Capture local before/after screenshots. Independent per-tranche review and a final
broad review must pass. Coordinator commits/pushes using verified author/committer and signing.

## Records

Each numbered brief defines owned paths and read-only context. Matching reports/reviews retain exact
commands and outcomes. The ledger records ID, state, issue, writer, evidence, reviewer verdict and next
action. Only mark accepted after verification and independent review.
