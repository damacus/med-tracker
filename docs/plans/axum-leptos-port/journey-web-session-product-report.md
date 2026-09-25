# First-party web session product report

## Implemented boundary

- Direct `/login` now issues a signed, ten-minute standalone login intent with its own CSRF token. The existing registered OAuth authorization intent remains separate. Password verification, failure counting, lockout, account status and required-factor checks still govern both paths. A successful standalone sign-in creates a signed browser cookie backed by a hashed `account_active_session_keys` row and redirects to the first active membership's operational household dashboard, ordered by membership ID as Rails does. An account without an operational household receives an authenticated empty state.
- The minimal Leptos dashboard renders the current household name, a session-bound `meta[name="csrf-token"]` and a sign-out form. It rechecks the DB session, primary account actor and current household membership. It does not embed a mobile access or refresh token.
- Shared household medication and dose API handlers accept the browser session when no Authorization header is present. Any explicit Authorization header retains bearer-only handling, including malformed or invalid bearer credentials. Cookie requests use the same current household membership and person-grant checks as bearer requests; audit references contain a hash of the browser session key, never the raw key or cookie.
- An outer API middleware checks configured-public-origin Origin, or same-origin Referer only when Origin is absent, and the session CSRF token before unsafe cookie requests reach a handler. A recognized household dose rejection writes a redacted `api.request` failure audit with the same request ID returned in `X-Request-ID`. The shared medication GET audit request ID is also returned in that header.
- Successful cookie-authenticated API and dashboard responses renew the same signed cookie and CSRF while retaining DB inactivity and configured absolute age checks. Logout requires trusted origin and session CSRF, deletes the hashed DB session key in a transaction, and expires the cookie. A copied old cookie must fail on the next request.

## Checks and evidence

- Dedicated fixture-backed web-session HTTP tests were first run against checkpoint `39904af0`: all four selected cases failed at direct `/login` because the old information page had no CSRF form. The disposable test project was cleaned.
- Local source gates pass: `task api:fmt`, `task api:clippy`, `task api:test` (six API unit tests), the exact Rust web Taskfile `test` (one web unit test), and `git diff --check`.
- The independent reviewer found no remaining static security blocker in this bounded source. The frozen focused web-session run passed 8/8 HTTP cases; the canonical regression passed 36/36 HTTP and 7/7 existing browser cases, with no skips. Both runs exited 0 and completed owner-scoped cleanup. The [runner report](journey-web-session-runner-report.md) records the matching scoped runtime manifest, fixture and image evidence.

## Scope limit

This slice establishes first-party login, cookie API access, route-boundary CSRF and logout. The dashboard is a minimal authenticated landing. Person/source reads and the medication dose browser journey follow in the next bounded tranche. Required OTP or passkey accounts remain unable to complete password-only sign-in here; password recovery is still unavailable.
