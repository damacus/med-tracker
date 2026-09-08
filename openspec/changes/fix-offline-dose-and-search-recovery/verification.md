# Verification

- Full Rails suite after review fixes: 5,599 examples, zero failures, one existing pending OIDC auto-link example because OIDC is not configured.
- Focused offline browser checks: 29 examples passed together, including temporary HTTP failures, transport/malformed responses, authentication, tenant isolation, atomic rollback, connection closure, recovery UI, stock reservations and cached eligibility.
- Offline snapshot, eligibility and stock browser checks passed, including restricted sources, stale metadata, pending overlaps, remaining stock and concurrent clicks.
- Search browser checks: eight examples passed, including all existing search scenarios and HTTP/network/malformed-response recovery.
- Repository RuboCop after review fixes: 1,839 files, no offences.
- Documentation build and strict OpenSpec validation passed. Git diff whitespace checks passed.
- Desktop/mobile screenshots under docs/screenshots use application test fixtures and simulated failed HTTP responses. They are runtime captures, not design mockups.

## Environment qualifications

Browser checks used the repository Task wrapper with `DOCKER_RUN_ARGS=-e PLAYWRIGHT_BROWSERS_PATH=/app/tmp/playwright` after installing the matching Chromium runtime in the checkout temporary cache. The underlying image/client revision mismatch is tracked in https://github.com/damacus/med-tracker/issues/2129.

The whole-locale tree checker reports three pre-existing English pagination key omissions. The same mismatch was reproduced from untouched origin/main files; all keys introduced here are present in all five languages. Follow-up: https://github.com/damacus/med-tracker/issues/2128.

The first full-suite run was stopped after detecting the new split-locale layout conflicted with existing locale checks. The translations were moved into the existing language files before the successful full run. Final snapshot checks were rerun after refining the timing/stock text.

This verification does not claim a deployed application or configured OIDC acceptance run.

## Review verification

The first review verification run exposed two stock fixture failures: the tests seeded the active cache while startup refreshes could still replace it. Stock and eligibility fixtures now run with a simulated offline connection. All 29 offline browser checks then passed together, followed by the full 5,599-example suite in 12 minutes 55 seconds.

The snapshot query-growth regression passes when adding four schedules and four direct medicines. Stock resolver parity checks cover delegated isolation, same-person alternate stock and manager access; dose decision checks cover overlapping sources, taper limits, expired schedules and weekly cycles.

Dedicated desktop and mobile unavailable-dose captures are committed as `docs/screenshots/offline-eligibility-desktop.png` and `docs/screenshots/offline-eligibility-mobile.png`. The disabled action and its explanation were visually inspected at both sizes.

## Published stack

The original four PRs were merged while the review corrections were being verified:

1. https://github.com/damacus/med-tracker/pull/2130 — sync recovery, based on main.
2. https://github.com/damacus/med-tracker/pull/2131 — pending stock, based on the sync fix.
3. https://github.com/damacus/med-tracker/pull/2132 — cached eligibility, based on the stock fix.
4. https://github.com/damacus/med-tracker/pull/2133 — search recovery, based on eligibility.

Their merged main tree (`0eda8a7c`) exactly matched the original published stack tip (`fa31b7a7`). The review corrections were therefore carried onto that main tree as three follow-up layers: sync recovery, atomic stock reservations, and eligibility/batching. Before publication-only documentation edits, the complete follow-up tree was verified identical to the tested review candidate (`5b29259b`).

The original stack identifier is 2134. This session did not perform those merges or a deployment.

The published follow-up stack is:

1. https://github.com/damacus/med-tracker/pull/2136 — read-connection closure and effective retry controls; based on main.
2. https://github.com/damacus/med-tracker/pull/2137 — atomic stock reservations and real browser coverage; based on #2136.
3. https://github.com/damacus/med-tracker/pull/2138 — full stock quantities, batched eligibility and unavailable-dose screenshots; based on #2137.

All nine original review threads received individual replies linking their follow-up PR, followed by resolution. A separate GraphQL read verified all nine resolved states and the created reply IDs. Remote follow-up CI runs independently from the completed local gates.

## Follow-up PR review

PR #2136 now checks that the offline database can be deleted after all three public read operations. This observes whether connections prevent deletion instead of counting calls to `close`. The recovery browser suite passes all 11 examples.

PR #2137 adds a tenant-specific BroadcastChannel after the controller connects and closes it on disconnect. A committed reservation notifies other connected pages without sending dose details. The existing DOM event still refreshes the originating page. A rendered second-document regression first reproduced a stale pending count of zero, then passed with the notification: both pages show one pending take and the remaining dose action becomes unavailable. All four stock browser examples pass.

The updated complete stack passed the full Rails suite: 5,600 examples, zero failures, one existing pending OIDC example, in 16 minutes 9 seconds. Both changed browser suites also passed within that full run. Test preflight passed 31 examples. Documentation build and strict OpenSpec validation passed.
