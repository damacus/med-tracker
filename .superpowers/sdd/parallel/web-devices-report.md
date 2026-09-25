# Web session device contract report

## Scope

This isolated branch adds black-box contracts for the Rails web-session native device token, push subscription and notification preference routes. It changes contract tests, a disposable fixture, test-only CSRF preload, the Task runner and parity matrix. It does not change Rails product code, schema, native clients or OpenAPI. The combined suite belongs to the integration owner.

## Observed HTTP contract

- A dedicated disposable PostgreSQL 18 household supplies a web login and API bearer for preference read-back. The contract logs in through `/login`, extracts a session CSRF token from the rendered profile, then sends form requests with the cookie and token. An anonymous request with a valid CSRF token redirects to `/login`; an invalid CSRF token returns JSON 401 under the test-only CSRF preload.
- Native token POST returns 201 with an empty body, including an update for the same account. DELETE returns 204. A second account cannot claim an existing token (422), cannot remove its owner's token through its own route (204 without effect), and can claim it only after the owner deletes it. The first account is then rejected with 422. No public web token listing exists, so ownership is verified through these observable transitions.
- Push subscription POST returns 201 with an empty body; invalid endpoint and missing keys return 422 with structured validation errors. DELETE returns 204. A second account cannot claim the endpoint until the owner removes it. The test-push route returns 204 for the dedicated account before any subscription exists. That safely exercises the HTTP dispatch without contacting an external push provider; actual provider delivery and error behavior need a deterministic sender boundary.
- Notification preference PATCH with Turbo accept returns 200 `text/vnd.turbo-stream.html` and replaces the notifications card and flash. HTML PATCH redirects to the profile. HTML PUT accepts the same route and redirects to the notifications section when selected. Public API GET verifies persisted flags and times after each update and a stable preference ID.

## Test-first observations and checks

- Initial compile was red for the new fixture fields and guarded web form helper. Initial Rails runs established the anonymous redirect, then exposed that `config/environments/test.rb` disables forgery protection. The branch-local preload enables Rails' request-level forgery guard only while this contract target runs. A run with the guard active established invalid-CSRF 401 and the absolute profile redirect. A later run encountered login rate limiting from separate account logins in three cases; the device and push assertions now reuse the same authenticated sessions. The final focused Rails run passed 2/2.
- `rtk task contract:web-devices-rust` compiled and remained the expected absent-server red: 0/2 connection failures to `127.0.0.1:39999`.
- Final gates: `rtk task contract:fmt`, `rtk task contract:clippy`, `rtk task rubocop` (1,888 files, no offenses), `rtk proxy fish -n rust/contract-tests/run.fish`, and `rtk git diff --check` passed. `task docs:build` could not fetch the locked `markdown` wheel because the Python package host did not resolve; the installed read-only Zensical executable built this branch's Markdown with `No issues found`.

## Limits

The test-push 204 response does not prove an external provider accepted a message. Token and subscription records have no public web list endpoint for direct read-back. The Rails request spec's Turbo validation-failure branch stubs the updater; no deterministic HTTP input in this fixture produces that failure. The test-only CSRF preload restores production-style request checking that Rails test mode disables; it is scoped to this contract runner target and is removed before the next target in a full run.
