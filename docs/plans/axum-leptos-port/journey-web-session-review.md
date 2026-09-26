# First-party web session: independent review

Review scope: standalone browser login intent, signed DB-backed session,
cookie authentication at the shared API boundary, CSRF/origin enforcement,
current household and person authorization, and logout. This is an API
boundary review, not acceptance of the later medication browser UI.

## Design boundary

The existing OAuth form contract posts `/login` without Origin or Referer and
requires an exact CSRF token tied to a signed pending authorization claim.
Standalone login may retain that pattern with a separate, short-lived signed
intent. A supplied cross-origin Origin or Referer must be rejected. Unsafe
authenticated cookie requests to `/api/v1` and `/logout` require a trusted
Origin or Referer matching the configured public origin and a session-bound
CSRF token; absence of both provenance headers must fail closed. Explicit
Authorization, even if invalid, must take precedence over a browser cookie.

The browser session must be checked against its active DB key, account and
factor state, current household membership, and person grant on each use.
Logging out invalidates the DB key and clears the cookie. No bearer or refresh
token belongs in rendered HTML, JavaScript, URL or browser storage.

## Independent review status

The dedicated `web_session_api.rs` contract began with the existing Rust
`/login` limitation as its expected RED. Runner wiring uses the existing
disposable project and owner-scoped cleanup, with an isolated Task target;
the fake-runner Rails failure branch now asserts image cleanup. After the
first RED, the test writer added absent Origin/Referer, hostile Origin with
trusted Referer, trusted Referer fallback, foreign household and hidden-person
denial, and a 31-day aged DB session key. Source review finds these meaningful
boundary checks. The final runtime results are recorded below.

The first source pass raised two issues before freeze. Cookie CSRF middleware
returned 403 for authenticated bad-CSRF or hostile-origin writes before the
dose handler's `api.request` failure audit; the direct-dose contract records
authenticated rejected POSTs. Also, session availability initially accepted
any active account-linked user. Rails selects the account's lowest-ID primary
person/user as the global actor across household memberships, so an inactive
primary user must not remain signed in merely because another linked user is
active. The target household still needs its own current membership binding.
The product writer added redacted denial audit with a shared response request
ID, primary-user checks, and current target membership binding. The next
source pass found that DB `last_use` slid while the browser cookie's 30-day
`Max-Age` initially remained fixed at login. The revised source renews the
cookie on authenticated API and dashboard responses, bounded by the
configured absolute age, while explicit bearer requests do not renew it.
No static boundary blocker remains in this draft. These source observations
were checked by the dedicated run below.

The settled source pass confirms signed standalone login intent alongside
the existing OAuth form, password/lockout and enrolled-factor checks,
DB-backed signed session lookup, lowest-ID primary actor with the current
target household membership, explicit Authorization precedence, configured
origin and session-CSRF enforcement on cookie API writes, redacted failed
dose-write audit with a matching response request ID, sliding cookie renewal,
and logout key deletion. The product lane reports API format and Clippy,
six API unit tests, one web unit test, and diff checks passing.

## Acceptance evidence and verdict

The frozen focused `task api:web-session-acceptance` passed all 8/8 cases,
covering standalone login, cookie and bearer precedence, current household
and primary-user binding, CSRF and configured origin checks, denial auditing,
sliding session cookie, revocation, expiry, and logout. The separate canonical
run passed 36/36 existing HTTP contracts and 7/7 auth browser smoke checks,
with no skips. Both disposable projects completed owner-scoped cleanup.
The 129-file scoped Rust/runtime and fixture-provisioning manifest matched
before and after at SHA-256
`5950c95fb5df771c0224f0d2896621fa8cb7428c42bbd63d6ba5a38ad7e8ee8a`.
Focused and canonical fixture hashes were respectively
`ad294ace448c925e4787bed96fe8abcc56aeb325fd720ff088d01043037d81fb`
and `3ba47d1b33dbe6063177d5a8f469e82afc8ec4b85b7edf296d47859ceaa2db8c`.
The manifest excludes the full Rails app/schema/config; the runner records
image and fixture identities separately. The current standalone-login desktop
and mobile screenshots show legible forms, viewport fit and visible keyboard
focus. Historical `login-public-*` images show the earlier placeholder and
are not evidence for this implementation.

**Requirements verdict: passed for the bounded first-party session and shared
API cookie boundary.** The existing native bearer contract remains green.
This does not establish the later first-party medication picker, dose form,
stock/history browser journey or full web parity. The shared people, schedule
and person-medication read migration has its own Rails baseline and acceptance
track.

**Code-quality verdict: acceptable for this slice.** Independent source
review found no remaining blocking authentication, tenant, CSRF, audit or
session-lifetime issue after the recorded corrections. The focused runtime
cases and canonical regression run back that source assessment. Further UI
work must carry the established Rails Escape-focus defect as an accessibility
requirement and prove the complete browser medication journey separately.
