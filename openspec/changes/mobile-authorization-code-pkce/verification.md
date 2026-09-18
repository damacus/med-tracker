# Acceptance evidence

## Baseline

Main was refreshed to `62db0c58` on 18 September 2026. The implementation target
is the merged Rodauth flow from #2232, not the deleted ID-token exchange or #2222.

| Requirement | Existing evidence location | Remaining evidence |
| --- | --- | --- |
| Bound/single-use code | `spec/requests/mobile_oauth_authorization_spec.rb` | Missing/plain challenge, wrong registered client, expiry; rerun existing verifier/callback/replay cases |
| Atomic issuance/retry | Same request boundary; library grant row lock | Concurrent requests, persistence rollback, lost-response retry |
| Identity/callback rejection | `spec/security/oidc_security_spec.rb`, Android `MobileAuthDiscoveryTest.kt` | Existing OIDC tests largely inspect source; real invalid provider results and callback state/cancellation need behaviour tests |
| Household authority | Mobile OAuth request specs; `spec/models/oauth_grant_mobile_spec.rb`; SMART OAuth request specs | Membership removal, concurrent context, failed writes and idempotency isolation |
| Private diagnostics/discovery | Parameter filters, mobile OAuth discovery request examples, Android discovery tests | Sentinel checks across affected diagnostics and current reruns |

Parent task mapping is recorded in design.md. Prior verification in
`unify-mobile-login-with-rodauth/verification.md` is historical, not a pass for
this change. No live-provider acceptance is claimed.

## Environment

The initial sandboxed preflight could not access Docker. The elevated retry
installed the wrapper's required host Ruby 4.0.7 and passed dependency/browser
checks and all 15 preflight examples. Rails tests run through the repository's
Docker-backed task commands.

Android initially had a stale `JAVA_HOME` and no SDK setting. The installed
Gradle-managed JDK 17 and `~/Library/Android/sdk` were selected only for the test
command; repository configuration was not changed. The existing discovery and
dashboard-session tests pass: 20 tests, no failures. `task android:api:check`
passes without generated-contract changes.

## Library boundary

The locked `rodauth-oauth` 1.6.7 implementation uses a locked grant lookup before
challenge verification and token persistence. It defaults
`oauth_pkce_allow_plain_method` to true. A new request example failed because
mobile authorization with `plain` created a grant. The configuration now disables
`plain` only for mobile applications; restricted integrations retain their policy.
The first combined mobile/SMART run passed all 28 examples after the fix.

Further focused checks passed 37 examples across mobile OAuth, SMART OAuth,
concurrent redemption and OIDC middleware validation. The race proves two
independent database sessions wait on the same grant lock before release, and
only one redemption succeeds. A database constraint injects a token-write failure:
the grant and audit state roll back, then a valid retry succeeds once. Replaying
a committed code does not replace the issued token hashes.

The upstream checks use the installed OmniAuth OIDC middleware with real RS256
signatures and synthetic HTTP responses. A valid control reaches its application
handoff; invalid issuer, audience, expiry, nonce and signature do not. These are
library-boundary checks, not a claim of real ZITADEL login or deployed Rails wiring.

## Integrated checks

The final focused run passed all 39 examples across mobile authorization,
concurrent redemption, OIDC middleware and restricted SMART authorization.
The authorised cross-household control initially lacked a person-access grant;
the fixture was corrected without weakening the application policy.

`task test` passed 6,166 examples with no failures and one existing pending
OIDC-configuration example. Subsequent test-only additions were verified in the
final focused run; production code did not change after the full run.
`task rubocop` passed all 1,961 files. `task brakeman` reported no warnings or
errors. Documentation build, Android contract check, strict OpenSpec validation
and whitespace checks passed. Android implementation was not changed, so the
full Android build and device UI gates do not apply to this slice.

## Remaining scope decision and live evidence

The current Android activity binds callback state, redirect and session revision,
but there is no account-bound reauthentication/pending-action coordinator.
Task 2.3 therefore needs a scope decision: validate existing login and track
account-bound action resumption separately, or extend this delivery to implement
it. The user has been asked; this portion is paused rather than declared complete.

Concurrent household-write/idempotency acceptance, complete diagnostic/trace
coverage, actual provider/device login and the representative rollback rehearsal
remain open. Do not close #1889 or archive either change on these partial results.
