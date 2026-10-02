# Tasks

## 1. Compatibility evidence and bounded OTP proof

- [x] 1.1 Record current Rust/Rails behaviour, maintained protocol crates and exact credential-format gaps in the auth research report; verify repository paths and official dependency manifests.
- [x] 1.2 Create proposal, design and capability scenarios preserving accepted native/session policy; verify OpenSpec change validation.
- [x] 1.3 Capture deterministic synthetic Rails/ROTP vectors for 16/32-character seeds, current/old HMAC secret and fixed-time OTP; document the oracle command and values in the auth report.
- [x] 1.4 RED: add failing Rust tests for exact legacy derivation, malformed seeds, time drift and replay boundaries; capture the focused task failure before production code.
- [x] 1.5 GREEN: implement only a pure explicit-input OTP compatibility helper using reviewed primitives; verify all focused tests pass without reading live secrets or enabling routes.
- [x] 1.6 Refactor within the owned helper while keeping focused tests green; run Rust format/lint and record completed scope and remaining gates.

## 2. Existing passkey import proof

- [ ] 2.1 RED: add synthetic Rails-created credential/user-handle vectors and passwordless/autofill assertion cases covering wrong origin/RP, missing UV, replay and counter regressions; capture failures against Rust.
- [ ] 2.2 Implement a reviewed credential adapter retaining 64-byte user handles and existing IDs/keys without invented metadata; verify valid imported credentials pass and negatives fail.
- [ ] 2.3 Document chosen high-level/core boundary, dependency/licence/build results and unsupported formats; verify review resolves the import gate before enabling factors.

## 3. Factor completion and credential management

- [ ] 3.1 RED: add observable password-plus-OTP/passkey/recovery and native consent-resumption cases; verify no session or code is granted before factor completion.
- [ ] 3.2 Implement trusted session assurance and transactional OTP failure/last-use updates and one-use recovery consumption; verify concurrent submissions yield at most one success.
- [ ] 3.3 Implement enrolment/removal/viewing with password and existing-factor management protections; verify no primary method means recovery-only rows do not force login MFA.
- [ ] 3.4 Document factor behaviour and capture browser desktop/mobile evidence; verify session expiry/revocation and action-specific fresh-MFA remains absent.

## 4. Generic OIDC login

- [ ] 4.1 RED: add provider callback, existing identity, mixed-case email, invite-only signup and invalid claim/state/nonce tests; capture current missing-flow failures.
- [ ] 4.2 Integrate reviewed discovery/code/claim validation with bounded HTTP transport and safe pending state; verify protocol negatives and existing account-ID retention.
- [ ] 4.3 Integrate validated provider assurance, local-factor fallback and existing logout behaviour; verify configured-provider browser/native acceptance and document deployment-specific checks.

## 5. Account lifecycle

- [ ] 5.1 RED: add registration/verification/invitation scenarios, adult eligibility and rollback cases; capture current missing-flow failures.
- [ ] 5.2 Implement transactional registration/verification and after-commit mail; verify realistic fixture and rollback tests plus browser behaviour.
- [ ] 5.3 RED/GREEN: implement reset/change password, verified login change and unlock with expiry/resend/concurrent one-use tests; document observable behaviour and verify no secrets in failures/audits.
- [ ] 5.4 RED/GREEN: implement account closure and credential revocation; verify retained person/medical history, cleared account links and denied subsequent authentication.

## 6. Integrated acceptance and cutover readiness

- [ ] 6.1 Run relevant native S256 PKCE, consent, refresh/revoke, browser CSRF/origin and household/person isolation contracts after integration; all required checks must pass.
- [ ] 6.2 Independently review account linking, secret compatibility, passkey invariants and transactional factor/session state; resolve findings and record remaining provider/device acceptance.
- [ ] 6.3 Produce reviewed cutover/rollback evidence retaining accounts and credentials; leave deployment and performance claims gated on separately authorised measured work.
