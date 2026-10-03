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
- [ ] 2.4 Run narrow tests until green. Prove permission revocation, generation errors, no-store headers, actual PDF text and empty-history behaviour. (Unit/renderer tests green: 6 reports_rendering + 5 web_pages filters/download tests pass. Runtime proofs exist in rust/contract-tests/tests/household_reports.rs — revocation, generation failure via rust-api-report-fail, no-store, pdftotext content and empty-range download — but the disposable Docker stack could not start: daemon reported read-only filesystem on network store with ~1GB host space free. NOT RUN — blocker recorded, see 3.3.)

## 3. Refactor and verification

- [x] 3.1 Remove duplicated UI/domain logic; inspect final diff for tokens, PHI logging, unintended policies, catalogue churn and other sessions' scope.
- [ ] 3.2 Run task api:fmt, task api:test, task api:clippy and relevant rust/web wrappers via task -d rust/web. Run task api:openapi-reports-acceptance. (api:fmt clean, api:test all green, api:clippy clean, task -d rust/web fmt/lint/test all green. contracts:clippy fails on pre-existing rust-1.99 chunks_exact_to_as_chunks lint in untouched openapi_external_integrations.rs — toolchain drift, not this change; the new household_reports.rs is clippy-clean under -D warnings. openapi-reports-acceptance BLOCKED by the same Docker daemon read-only failure — NOT RUN.)
- [ ] 3.3 Add focused browser coverage to the existing harness; run task api:browser-rust BROWSER_TEST_FILES=<actual-new-test-path>, using HOUSEHOLD_TEST_FILE selectors where required. Give each disposable stack a distinct identity; never stop another session's stack. (BLOCKED: `task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_reports BROWSER_TEST_FILES="tests/household-reports.test.mjs"` fails at stack bring-up — Docker daemon cannot create the disposable network: "write /var/lib/docker/network/files/local-kv.db: read-only file system". Two attempts, identical failure. Test files exist and compile/syntax-check clean locally: rust/contract-tests/tests/household_reports.rs and rust/web/tests/household-reports.test.mjs. No other session's stack touched.)
- [ ] 3.4 Verify actual desktop/mobile browser behaviour and keyboard interaction; save the two screenshots named in design.md. Check all five locales. (BLOCKED with 3.3 — the Playwright lane needs the disposable stack. Screenshot wiring is in place: the test writes docs/screenshots/journey-medication-rust/health-history-reports-{desktop,mobile}[-error].png plus a locales capture, ready to rename to the design names once the run can complete. Locale rendering itself is proven green by rust/web/tests/reports_rendering.rs across all five locales.)
- [x] 3.5 Validate OpenSpec with task openspec:validate and git diff --check. Record exact commands/results and honest blockers. No Rails preflight or full Rails suite unless Rails code is changed, which is outside this plan.

## 4. Review and publication

- [x] 4.1 Self-review against every spec scenario and security boundary. Keep the tasks ledger accurate; leave failed or unverified tasks unchecked.
- [x] 4.2 Before committing, verify effective author and committer are Dan Webb <dan.webb@damacus.io> and inspect signing identity. If 1Password signing fails, use the authorised one-command git -c commit.gpgsign=false fallback and report the unsigned commit. (Author/committer verified; 1Password signing failed — authorized fallback used, commit a43a5eed unsigned.)
- [x] 4.3 Commit this plan with implementation and screenshots using a focused Conventional Commit. Pull/rebase and push own branch; no failed required gates may be hidden. (Pushed; screenshots blocked per 3.4 — honest note in commit-adjacent issue/PR text.)
- [x] 4.4 Open a separate follow-up PR against codex/rust-security-review while #2383 is open; if merged use main safely. Link #2387, describe user-visible behaviour and screenshots, and note unusual/manual/blocked verification. Do not merge or deploy. (PR #2388 opened against codex/rust-security-review with verification status noted; no merge/deploy.)
- [x] 4.5 Update issue progress and report PR URL, branch, results, screenshots and limitations in the Devin session for review here. Do not close the issue until merged or message external users.
