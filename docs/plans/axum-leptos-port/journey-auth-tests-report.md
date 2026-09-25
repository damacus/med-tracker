# Secure journey entry: test writer report

## Red evidence

At baseline `0b6fa2a8`, I added the existing six `oauth.rs` cases to the canonical medication acceptance selection and ran `task api:acceptance CONTRACT_TEST_SUBNET=192.168.240.0/28`. The subnet validator passed before the run. The disposable fixture was written under `tmp/contract-tests/run.Xs5Lg2/fixture.json`; the run cleaned its Compose project and storage afterward.

The Rust OAuth target compiled and ran. All six cases failed against the unchanged API because discovery, capabilities, `/authorize`, and `/token` were absent (HTTP 404). The three forecast, seven existing mobile OAuth, and nine medication read cases passed. The task exited 201. This proves a behavioural red for the newly selected OAuth surface; it does not establish a full runtime input digest or green parity.

## Added focused coverage

- A newly issued token must list the primary operational household, exclude a foreign household, read the primary household's medication list, and receive 403 for foreign medication access.
- Password and consent POSTs without their CSRF token cannot continue to authorization or issue a native callback code.
- A separate fixture account with enrolled OTP cannot reach consent or a native callback after password alone. A safe denial of unsupported second-factor completion satisfies the case.
- The browser smoke test, when supplied `CONTRACT_FIXTURE_PATH`, starts a mobile authorization request, submits a wrong password by keyboard, and checks that a visible error and focused usable password field remain.
- Authenticated consent POSTs reject an unknown client, unregistered callback, unsupported scope, missing PKCE challenge, and plain PKCE. Token redemption rejects another client, a mismatched callback, and reuse of a spent code without returning credentials.
- The fixture-backed browser smoke test also completes login and renders labelled consent on desktop and mobile. It submits consent by keyboard while intercepting the native callback, checks state and an issued code, and writes consent screenshots when `SCREENSHOT_DIR` is set.
- A read-only audit database assertion checks that refresh rotation leaves the issued grant's `last_used_at` and `authenticated_at` unchanged.
- Public `/login` must explain that sign-in starts in the registered mobile app and show no unusable dashboard submit. The active OAuth login must lay labels above usable-width inputs within desktop and mobile viewports, without horizontal overflow. Browser focus checks read `document.activeElement` through the locator.

## Checks and handoff

`task contract:fmt`, `task contract:clippy`, `task rubocop` (1,892 files, zero offences), and `git diff --check` passed after the test and fixture edits. The new cases have not been run against the product implementation. Browser execution and screenshots also remain pending. Runner owns the next stable combined acceptance and full input digest.

Frozen test-owned paths: `rust/contract-tests/run_medication_api_tests.fish`, `rust/contract-tests/src/lib.rs`, `rust/contract-tests/tests/oauth.rs`, `rust/web/tests/login.smoke.test.mjs`, and `scripts/contract_provision.rb`. No other test-owned paths changed. No Git publication or live operation occurred in this lane.
