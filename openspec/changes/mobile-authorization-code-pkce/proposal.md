## Why

[Issue #1889](https://github.com/damacus/med-tracker/issues/1889) remains open after
[PR #2232](https://github.com/damacus/med-tracker/pull/2232) replaced the custom
ID-token exchange with Rodauth authorization-code/S256 PKCE sign-in. The remaining
gap is evidence that failed, replayed and concurrent exchanges cannot issue unsafe
credentials, and that the real delegated browser flow works. Current ADR 0005
still describes the removed exchange.

## What Changes

- Complete the outstanding security acceptance work from
  `unify-mobile-login-with-rodauth`, rather than introduce another login flow.
- Exercise the real Rodauth token endpoint for invalid client/challenge, expired
  code, concurrent redemption and persistence failure. Make only fixes demonstrated
  by failing behavioural tests; reuse the installed OAuth library.
- Verify upstream OIDC rejection, mobile callback/state binding, refresh and
  current household access without collecting credentials or health data in logs.
- Exercise configured ZITADEL SSO and login methods through the actual Android
  system-browser journey, keeping local login and optional MFA policy unchanged.
- Reconcile authentication ADRs, setup guidance, capabilities and the original
  issue's acceptance criteria with the already-merged account-level OAuth flow.
- Attach versioned, redacted rollout and rollback evidence from the deployment
  owner before closing #1889. A passed unit test or Ready Secret is not live-login
  evidence.

Non-goals: rebuilding PKCE or JWT cryptography; restoring retired mobile routes;
deploying obsolete PR #2222; new session-lifetime or MFA policy; new iOS features;
an offline replay engine; expanding SMART/FHIR or application-token permissions;
changing ZITADEL clients or deploying from this planning change.

## Capabilities

### New Capabilities

- `mobile-authorization-failure-safety`: Observable rejection, concurrency,
  persistence and identity-binding guarantees for the existing mobile OAuth flow.
  This focused acceptance supplement does not replace the two broader capabilities
  in the unarchived `unify-mobile-login-with-rodauth` change.

### Modified Capabilities

None. The parent authentication capabilities have not been archived into
`openspec/specs/`; preserve that ownership and reconcile evidence with its tasks.

## Impact

Primary boundaries are `RodauthMobileOauth`, Rodauth/OIDC configuration,
`OauthGrant`, the API authentication/household context and Android AppAuth/discovery.
Extend existing request, model and Android behaviour tests, not source-text or
exact-YAML contract tests. Documentation work includes ADR 0005, `docs/design.md`,
root OpenAPI and any affected pinned client contract. No new dependency, schema
migration or production configuration is planned unless a reproduced defect
requires one and its scope is reviewed.

Related work: #2055 consumes the same contract for iOS; #2048 concerns the retired
password-login path and must be revalidated, not treated as proof of this flow.
The separate deployment thread owns live activation. Existing parent tasks about
unimplemented offline replay remain separate and must not be marked complete.
