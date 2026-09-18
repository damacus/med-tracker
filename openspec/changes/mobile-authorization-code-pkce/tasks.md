# Tasks

## 1. Establish the remaining acceptance baseline

- [ ] 1.1 Refresh main and map each requirement in `specs/mobile-authorization-failure-safety/spec.md` to existing request/model/Android tests and parent-change tasks. Record existing passes separately from missing evidence; verify no removed endpoint or obsolete PR is used as the implementation target.
- [ ] 1.2 Inspect the locked OAuth/OIDC library hooks and transaction boundaries; run `task test:preflight` before any Rails implementation and `task test TEST_FILE=spec/requests/mobile_oauth_authorization_spec.rb` for the starting behaviour. Record environment failures without marking them passed.

## 2. Red: prove security failures at public boundaries

- [ ] 2.1 Extend existing mobile request tests for bound/single-use redemption: missing or non-S256 challenge, wrong client, expired code and the already-covered verifier/callback/replay cases. Run the focused request file and record which new cases fail; retain passing coverage without inventing a code change.
- [ ] 2.2 Add synchronised independent-connection redemption race and grant-write failure cases for atomic issuance/safe retry. Verify actual API usability, grant state and issuance audit after concurrent requests, rollback and committed-response loss; run the focused request file without sleeps or mocked PKCE.
- [ ] 2.3 Add deterministic delegated-login rejection cases for issuer, audience, signature, expiry and nonce, plus Android callback state/instance/account mismatch and cancellation cases. Verify no grant or pending write is authorised using focused Rails files through `task test TEST_FILE=...` and `task android:phone:test`.
- [ ] 2.4 Extend household/API tests for membership removal, concurrent contexts, cross-household record IDs, failed-write rollback and idempotency isolation. Verify returned records and audit context, plus unchanged restricted credentials with `task test TEST_FILE=spec/requests/smart_oauth_authorization_spec.rb` and the affected household request files.
- [ ] 2.5 Extend private-diagnostics/discovery tests using synthetic sentinel credentials. Verify responses, logs, traces and audits omit secrets and discovery describes the replacement flow; run affected Rails request/privacy tests and `task android:phone:test`.

## 3. Green and refactor: fix only demonstrated gaps

- [ ] 3.1 For failures from section 2, make the smallest change in existing authentication/library integration boundaries. Preserve account eligibility, optional login MFA, refresh rotation and restricted grant ceilings; rerun each failing case until it passes. If all new behaviour tests pass, record that no production fix was needed.
- [ ] 3.2 Refactor only affected test setup or integration code after green, preserving independent race requests and real library verification. Rerun focused mobile OAuth, affected model/request and SMART OAuth files; verify no duplicate protocol implementation or new authentication endpoint was introduced.

## 4. Documentation and integrated checks

- [ ] 4.1 Reconcile ADR 0005, `docs/design.md`, setup guidance and parent acceptance records with the merged Rodauth flow. Check root OpenAPI/capabilities and update pinned contracts only where actually inconsistent; verify `task docs:build`, `task android:api:check`, strict OpenSpec validation and `git diff --check`.
- [ ] 4.2 Run changed-area gates after focused checks: `task test`, `task rubocop` and `task brakeman` for Rails/Ruby changes; `task android:ci` and `task android:phone:ui-test` for Android changes. Record current results and preserve unrelated pending tests; do not claim prior runs verified new changes.

## 5. Live evidence and issue completion

- [ ] 5.1 Obtain the deployment owner's exact server digest, Android build and public discovery/callback configuration. Exercise local-only login and configured ZITADEL login/SSO/passkey/MFA through the real system browser with a test account; verify cancellation, authenticated household/dashboard reads, refresh, sign-out and device revocation. Record redacted results; unavailable provider/device checks remain blocked.
- [ ] 5.2 Coordinate a separately authorised representative rollout/rollback rehearsal with the deployment owner. Verify schema-compatible rollback or issuance-disable recovery, account grants and existing restricted integrations; record versions and outcomes without production patient data or unapproved live mutations.
- [ ] 5.3 Map completed evidence back to the existing parent tasks and #1889 acceptance criteria, leaving unrelated unfinished work open. Publish a precise issue update and close #1889 only after required security and live-delivery checks pass; verify the issue state and retain #2055 as separate iOS delivery.
