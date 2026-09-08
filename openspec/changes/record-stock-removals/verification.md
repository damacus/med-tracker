## Verification

Verified on 8 September 2026 against main commit `80d140bcfe84e4df9512d46743d0f55188c6b4db`.

- Rails test preflight: 31 examples passed.
- Focused stock-removal, concurrency, audit-chain, enlarged-text and mobile-navigation checks: 56 examples passed.
- Final desktop/mobile screenshot run: 2 browser examples passed. Evidence is under `docs/screenshots/issue-1982/`.
- RuboCop: 1,838 files checked, no offences.
- All five locale trees match. Strict OpenSpec validation and the documentation build passed.
- Full RSpec suite: 5,585 examples, zero failures and one pending test, completed in 13 minutes 7 seconds.

## Test environment

The first full run was stopped after failures exposed committed audit state from the original concurrency fixture and stale compiled layout assets. The concurrency specs now use a separate household and clean up its audit export deliveries, ledger, source events and stock records. No shared fixture audit chain is modified.

The final verification uses an isolated Compose test project through the repository's `internal:run` Task command. A temporary Taskfile supplies the project name directly to that task; a top-level `COMPOSE_PROJECT` argument does not override its locally defined value. The RSpec command and complete `spec` target are the same as `task test`.

Fresh image verification reproduced the browser revision mismatch tracked in #2129. Playwright 1.62.0 Chromium headless shell was installed in the ignored `tmp/playwright-stock-removal` cache and selected with `PLAYWRIGHT_BROWSERS_PATH`. Tailwind assets were rebuilt before the final checks. Dependency manifests and the browser image configuration were not changed.

The OpenSpec change remains available for review and later archive. This work does not deploy or merge the feature.
