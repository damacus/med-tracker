# Household test lane

Owner: test writer. Deadline: 2026-10-01 10:27 UTC.

Owned files: new `rust/web/tests/household-*.test.mjs`,
`rust/web/tests/household_rendering.rs`,
`rust/contract-tests/tests/household_web.rs`, and this lane's brief/report.
Coordinator subsequently added separate web/contract `household_navigation.rs`
checks and contract `household_lifecycle.rs` checks. These remain separate from
the initial live-mounted acceptance file while its snapshot runs.

Start with authenticated missing-route tests for people, locations and manual
medication creation. Record real HTTP/browser failures before implementation.
Then cover persisted new/edit workflows, validation draft retention, immediate
location usability, foreign record denial, CSRF and private response headers.
Keep black-box browser/session checks separate from rendering unit tests.

Use the existing disposable contract fixture and browser runner. Coordinator
owns fixture extensions, task wiring and runtime startup; request changes there.
No production source changes, comments, commits or publication from this seat.

Rails views and OpenSpec observable behaviour are the reference. Tests must not
relax expectations to accommodate implementation. Record exact commands,
results and remaining scope in `run-20261001-tests-report.md`.
