# Implementation checkpoint

This change is not complete and has not been deployed. The new Rodauth mobile
flow, central lifetime settings and removal of action-specific MFA freshness
gates are implemented. Legacy mobile endpoint retirement is blocked by automatic
approval review; see [the precise removal scope](retirement-approval.md).

## Configuration

| Environment variable | Default | Meaning |
| --- | --- | --- |
| `SESSION_INACTIVITY_TIMEOUT_DAYS` | `30` | Interactive inactivity window for web and mobile |
| `SESSION_MAX_AGE_DAYS` | `0` | Optional absolute interactive lifetime; zero disables it |
| `API_APP_TOKEN_MAX_AGE_MONTHS` | `12` | Fixed maximum API/MCP app-token age from issuance |

Mobile background token renewal preserves the original authentication and last
activity times. Token updates preserve the Android session revision and do not
notify screens that the account or household changed. Household navigation uses
the same account credential and resolves current permissions on each request.

## Verified locally

- Focused Rodauth browser, PKCE, refresh, device revocation, household and MFA
  request tests pass: 15 examples. These include an unauthenticated browser
  returning to authorization after local login and completing enrolled OTP.
- The affected Rails regression files pass: 185 examples.
- API app-token model tests pass, including calendar expiry, shortened limits
  and no revival after increasing a limit. Migration backfill tests pass.
- Android phone unit tests pass, including the regression that previously
  treated silent refresh as an account change.
- Android phone and Wear lint pass.
- The Android API pin check and root API-client deterministic generation pass.
- Android authentication instrumentation passed on a Pixel 9 Pro emulator,
  including the final household retry/sign-out controls.
- All Android debug, staging and release builds pass. The packaged release
  passes the check excluding password-login transport.
- The actual Android instance-selection screen was visually checked and
  captured in `docs/screenshots/medtracker-login-android.png` using the emulator
  directly. The editable URL, canary/demo preset and sign-in button fit the
  phone viewport.
- Brakeman reports zero errors and zero security warnings.
- Documentation builds and OpenSpec strict validation pass.

The final full `task test` run passed: 6,275 examples, zero failures and six
pending, in 12 minutes 33 seconds. Five pending examples are the blocked
retirement contract below; one was already pending. The previous full run had
one inventory scanner visibility failure; all eight examples in that file and
the final full run passed without changing that test or its implementation.

`task rubocop AUTOCORRECT=true` passed after correcting two formatting offences:
1,971 files inspected. Android unit, emulator UI, lint, all build variants and
release transport checks passed. The root generated-client determinism check,
Android contract pin check, documentation build and OpenSpec strict validation
also passed.

## Outstanding evidence and delivery

- Five route/capability retirement examples are explicitly pending because the
  removal was rejected by automatic approval review. They failed against the
  retained implementation before being marked pending. They are not evidence
  that retirement works.
- Provider-backed Zitadel/SSO, passkey and representative rollout/rollback
  checks have not run. No deployment or merge has occurred.
- Desktop browser screenshots remain unavailable while the Mac is locked;
  Android visual verification was completed through the running emulator.
- The phone currently has an in-memory offline mutation model with household
  identifiers, but no implemented producer/replay engine. Queued replay under
  a changed account or instance is not verified by this change.
- Additional redemption race/failure and account/household boundary cases in
  `tasks.md` remain open. Do not archive the change or close issue #1889.
