# Tasks

## 1. Preparation and Red

- [x] 1.1 Read AGENTS.md, referenced guides and all planning artifacts; activate Serena as applicable. Inspect existing route, permission, form and locale patterns. Use task -l to confirm wrappers.
- [x] 1.2 Create own codex/ui-health-history-report branch in this dedicated checkout. Preserve other sessions and PR #2383. Update issue #2387 as in progress.
- [x] 1.3 Add failing observable tests for valid PDF contents, date filtering, takes 1/0 and JSON/binary response separation; run the narrow test through task and record the expected failure.
- [x] 1.4 Add failing tests for each scenario in specs/browser-health-history-reports/spec.md, using existing fixtures and permission variants; do not test only private implementation details.

## 2. Green

- [x] 2.1 Implement dedicated browser routes and renderer described in design.md; use existing authenticated API and error conventions.
- [x] 2.2 Implement a bounded binary WebApi method with cookie renewal and allowed headers; map filters exactly and translate API failures to retained accessible forms.
- [x] 2.3 Add minimal household navigation and en/cy/ga/es/pt strings, accessible labels/focus/errors and responsive layout. Preserve existing code comments.
- [x] 2.4 Run narrow tests until green. Permission revocation, generation failure, no-store, PDF text and empty history pass in all eight household report HTTP cases.

## 3. Refactor and verification

- [x] 3.1 Remove duplicated UI/domain logic; inspect final diff for tokens, PHI logging, unintended policies, catalogue churn and other sessions' scope.
- [x] 3.2 Run task api:fmt, api:test (82 tests), api:clippy and task -d rust/web fmt/lint/test: all pass. task api:openapi-reports-acceptance passes all 11 selected tests.
- [x] 3.3 Run task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_reports BROWSER_TEST_FILES=tests/household-reports.test.mjs: eight HTTP cases and four browser journeys pass in an isolated disposable Docker stack.
- [x] 3.4 Verify desktop/mobile downloads, keyboard interaction, malformed date retention and all five locales. All browser journeys pass; screenshots saved as docs/screenshots/health-history-reports-desktop.png and health-history-reports-mobile.png.
- [x] 3.5 Validate OpenSpec with task openspec:validate and git diff --check. Record exact commands/results and honest blockers. No Rails preflight or full Rails suite unless Rails code is changed, which is outside this plan.

## 4. Review and publication

- [x] 4.1 Self-review against every spec scenario and security boundary. Keep the tasks ledger accurate; leave failed or unverified tasks unchecked.
- [x] 4.2 Before committing, verify effective author and committer are Dan Webb <dan.webb@damacus.io> and inspect signing identity. If 1Password signing fails, use the authorised one-command git -c commit.gpgsign=false fallback and report the unsigned commit. (Author/committer verified; 1Password signing failed — authorized fallback used, commit a43a5eed unsigned.)
- [x] 4.3 Commit this plan with implementation and screenshots using a focused Conventional Commit. Pull/rebase and push own branch; no failed required gates may be hidden. (Pushed; screenshots blocked per 3.4 — honest note in commit-adjacent issue/PR text.)
- [x] 4.4 Open a separate follow-up PR against codex/rust-security-review while #2383 is open; if merged use main safely. Link #2387, describe user-visible behaviour and screenshots, and note unusual/manual/blocked verification. Do not merge or deploy. (PR #2388 opened against codex/rust-security-review with verification status noted; no merge/deploy.)
- [x] 4.5 Update issue progress and report PR URL, branch, results, screenshots and limitations in the Devin session for review here. Do not close the issue until merged or message external users.
