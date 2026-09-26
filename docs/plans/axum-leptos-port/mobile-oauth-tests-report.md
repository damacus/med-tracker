# Mobile OAuth medication test writer report

## Owned changes

- Added `medication_mobile_oauth_api.rs` to the medication read contract target.
- Seeded disposable mobile OAuth grants for valid and denied access, a delegated viewer, a revoked person grant, and current account states.
- Added assertions for medication list/show, current delegated visibility, invalid grants, account and membership denial, OAuth audit attribution, and grant activity refresh.

## Red evidence

At `7bec286ae524c00286d6486d825cb11d7eae936e`, before Rust API edits, `rtk task api:acceptance` reached the new valid grant case. `current_mobile_oauth_grant_reads_medication_list_and_show` failed at `tests/medication_mobile_oauth_api.rs:21`: actual HTTP 401, expected 200 for the owner household medication list. The new test had 0 passed and 1 failed. The fixture was ready after 122 seconds; the failing HTTP case took 0.01 seconds. Full run output: `/Users/damacus/Library/Application Support/rtk/tee/1790337852_task_api_acceptance.log`.

Before and after that run, the owned tracked diff SHA-256 was `7e79c383fb089ddda4e29104b45c6c78df9b7e21410e77c3b34b3b115531faaf`, and the new test file SHA-256 was `7b755807c826a1ce1830451b3d37bde44da618e5c1a033b39c497f1082d686b7`. No Rust API file had changed when the failure was recorded. The first sandbox attempt stopped at Docker socket access and did not count as red; the escalated retry produced this result.

## Subsequent checks

`task contract:fmt`, `task contract:clippy`, and `git diff --check` passed after extending the cases. The final `task rubocop` run inspected 1,892 files with no offences. The configured maximum-login-age case used `SESSION_MAX_AGE_DAYS=30` on the disposable Rust API service; its production default remains unlimited.

The final two disposable Compose acceptance runs each exited 0 with 19 passing medication API cases: 7 mobile OAuth, 9 medication read, and 3 forecast. These green runs cover the cases added after the first red. The runner's stable-input and overlapping-project evidence is recorded in [mobile-oauth-runner-report.md](mobile-oauth-runner-report.md); the test writer did not run either final project.
