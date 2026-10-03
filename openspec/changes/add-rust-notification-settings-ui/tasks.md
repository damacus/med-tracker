# Tasks

## 1. Preparation and Red

- [x] 1.1 Read AGENTS.md, referenced guides and all planning artifacts; activate Serena as applicable. Inspect existing route, permission, form and locale patterns. Use task -l to confirm wrappers.
- [x] 1.2 Create own codex/ui-notification-settings branch in this dedicated checkout. Preserve other sessions and PR #2383. Update issue #2386 as in progress.
- [x] 1.3 Add failing observable tests for saved flags, explicit false, reminder-time preservation and reload; run the narrow test through task and record the expected failure.
- [x] 1.4 Add failing tests for each scenario in specs/browser-notification-preferences/spec.md, using existing fixtures and permission variants; do not test only private implementation details.

## 2. Green

- [x] 2.1 Implement dedicated browser routes and renderer described in design.md; use existing authenticated API and error conventions.
- [x] 2.2 Submit only the five boolean attributes in the singular API wrapper, verify CSRF before mutation, preserve attempted values on error, and show neutral masked-404 state.
- [x] 2.3 Add minimal household navigation and en/cy/ga/es/pt strings, accessible labels/focus/errors and responsive layout. Preserve existing code comments.
- [x] 2.4 Run narrow tests until green. Prove private content semantics, master-off selection preservation and failed-save behaviour.

## 3. Refactor and verification

- [x] 3.1 Remove duplicated UI/domain logic; inspect final diff for tokens, PHI logging, unintended policies, catalogue churn and other sessions' scope.
- [x] 3.2 Run task api:fmt, task api:test, task api:clippy and relevant rust/web wrappers via task -d rust/web. Run task api:openapi-notifications-acceptance.
- [x] 3.3 Add focused browser coverage to the existing harness; run task api:browser-rust BROWSER_TEST_FILES=<actual-new-test-path>, using HOUSEHOLD_TEST_FILE selectors where required. Give each disposable stack a distinct identity; never stop another session's stack.
- [x] 3.4 Verify actual desktop/mobile browser behaviour and keyboard interaction; save the two screenshots named in design.md. Check all five locales.
- [x] 3.5 Validate OpenSpec with task openspec:validate and git diff --check. Record exact commands/results and honest blockers. No Rails preflight or full Rails suite unless Rails code is changed, which is outside this plan.

## 4. Review and publication

- [x] 4.1 Self-review against every spec scenario and security boundary. Keep the tasks ledger accurate; leave failed or unverified tasks unchecked.
- [ ] 4.2 Before committing, verify effective author and committer are Dan Webb <dan.webb@damacus.io> and inspect signing identity. If 1Password signing fails, use the authorised one-command git -c commit.gpgsign=false fallback and report the unsigned commit.
- [ ] 4.3 Commit this plan with implementation and screenshots using a focused Conventional Commit. Pull/rebase and push own branch; no failed required gates may be hidden.
- [ ] 4.4 Open a separate follow-up PR against codex/rust-security-review while #2383 is open; if merged use main safely. Link #2386, describe user-visible behaviour and screenshots, and note unusual/manual/blocked verification. Do not merge or deploy.
- [ ] 4.5 Update issue progress and report PR URL, branch, results, screenshots and limitations in the current Codex task for review here. Do not close the issue until merged or message external users.
