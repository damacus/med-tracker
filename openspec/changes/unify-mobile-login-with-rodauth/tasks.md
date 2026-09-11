## 1. Confirm integration boundaries

- [ ] 1.1 Trace current Rails household resolution, Rodauth OAuth grant/refresh hooks and API session consumers; record a concrete change map in design.md and verify it covers both new mobile and existing restricted credentials.
- [ ] 1.2 Confirm native callback registrations and canary/demo preset URLs; reconcile mobile-monorepo-remediation's fixed-server restriction with instance selection and verify the revised Android contract preserves existing MFA policy.
- [ ] 1.3 Run `task test:preflight` before Rails implementation and verify the test environment is usable; use repository task definitions for all later focused checks.

## 2. Account-level OAuth credentials

- [ ] 2.1 Add failing model/database tests for explicit first-party account grants and still-required membership/person fields on restricted grants; implement minimal persistence invariants and verify focused model specs.
- [ ] 2.2 Add failing authorization request tests for local-only and mixed local/delegated sign-in with no household selection, including zero memberships; integrate first-party Rodauth authorization and verify focused request specs.
- [ ] 2.3 Add failing request tests for valid S256 redemption, missing/wrong verifier, client/redirect mismatch, expiry, replay, concurrent redemption and persistence failure; configure library-managed redemption and verify those cases plus existing SMART/FHIR authorization specs.
- [ ] 2.4 Add failing tests for preserved local/delegated authentication context, original authentication time, malformed provider claims and unchanged optional MFA/admin policy; implement evidence propagation and verify focused authentication and privileged-action specs.
- [ ] 2.5 Add failing tests for account lockout, refresh rotation/replay, per-device session listing and individual revocation; adapt the API credential/session boundary and verify both mobile OAuth and legacy session cases.

## 3. Household authorisation on every request

- [ ] 3.1 Add failing request tests for one credential accessing two households with different roles/person grants; reuse Rails membership resolution in the API context boundary and verify permitted and forbidden actions.
- [ ] 3.2 Add failing tests for account-level household listing and session management with zero, one and multiple memberships; implement these endpoints without implicit household selection and verify response contracts.
- [ ] 3.3 Add failing tests for revoked membership, changed permissions, non-operational households and cross-household record identifiers; enforce current authority and verify unrelated authorised household access continues.
- [ ] 3.4 Add failing tests for concurrent household contexts, correct audit membership, transactional write rollback and idempotency isolation using the same credential/key across households; implement minimal fixes and verify focused request/service specs.
- [ ] 3.5 Verify regression cases retain existing SMART/FHIR/application-token scope and household ceilings; add missing failing boundary tests before modifying shared credential logic.

## 4. Mobile contract and Android flow

- [ ] 4.1 Add failing discovery/capability tests for OAuth server metadata, public platform client configuration, account-level login and household listing; update root OpenAPI and verify schema/generated-contract checks through task wrappers, including Android's pinned copy.
- [ ] 4.2 Add failing Android tests for editable instance URL, labelled canary/demo presets, validated discovery, cross-instance credential isolation, state/callback handling and browser login without a household; implement the flow and verify Android unit/contract checks.
- [ ] 4.3 Add failing Android tests for zero/one/multiple household navigation and switching with the same credential; implement the UI and verify visible behaviour in the actual Android app.
- [ ] 4.4 Add failing Android tests for late responses, account/instance changes, cache separation, original-household queued mutations and lost access; implement isolation and verify the relevant session, repository and sync tests.

## 5. Privacy, documentation and integrated verification

- [ ] 5.1 Add failing redaction tests for authorization codes, verifiers and tokens in request/provider diagnostics and audits; implement filtering and verify authentication failures expose no credential material.
- [ ] 5.2 Update docs/design.md, authentication ADRs and setup guides to match discovery and direct replacement with no compatibility window; verify `task docs:build`, OpenSpec strict validation and `git diff --check`.
- [ ] 5.3 Exercise browser/mobile sign-in against local-only and mixed local/Zitadel installations, including SSO reuse and configured password, passkey and MFA choices; record actual results and capture screenshots for visible UI changes without treating unavailable provider checks as passed.
- [ ] 5.4 Run applicable full Rails, RuboCop, security and Android task checks after focused tests; record failures or environmental limits and require applicable gates to pass before pushing implementation.

## 6. Rollout and retirement

- [ ] 6.1 Verify coherent server/client rollout and rollback on a representative environment, including existing restricted grants and new account grants; document evidence before changing deployed flow availability.
- [ ] 6.2 Remove the legacy ID-token exchange and obsolete first-party selection path in this delivery with failing retirement tests first; verify new clients, unrelated integrations and truthful capabilities without a legacy compatibility window.
- [ ] 6.3 Update issue #1889 with delivered protocol and verification evidence, reconcile its original household-independent acceptance criteria with this proposal, and close only when required delivery is complete; verify issue state and remaining platform follow-up links.
