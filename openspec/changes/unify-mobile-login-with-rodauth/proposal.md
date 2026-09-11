## Why

The mobile exchange in [issue #1889](https://github.com/damacus/med-tracker/issues/1889) advertises PKCE without verifying it and duplicates provider-token handling outside Rodauth. Phones should sign in through the installation's existing authentication system and navigate between authorised households without another login.

## What Changes

- First-party phones use MedTracker's Rodauth authorization-code flow with S256 PKCE in a platform browser authentication session. Rodauth delegates to the configured OIDC provider where appropriate, including Zitadel, or uses existing local authentication.
- Preserve installations offering both local and delegated login, optional MFA enrolment, available MFA methods, admin tagging and existing access policy.
- Issue account-level first-party mobile credentials. Resolve current household membership and action permissions on every household request; do not grant access merely because a household shares the instance.
- Complete login without household selection. Let users list authorised households and switch within the app using the same credential; isolate cached records and queued actions by instance, account and household.
- Preserve refresh rotation, device session visibility and revocation, account lockout, and authentication assurance through the new flow.
- Phones accept an instance URL and discover its authorization configuration. Canary/demo presets provide instance URLs, not hard-coded provider endpoints.
- **BREAKING**: Replace the custom ID-token exchange and first-party household-bound login responses directly. There are no active mobile clients to migrate, so no compatibility window or dual-flow support is required. Remove obsolete behaviour and misleading capabilities in this delivery.
- Update API documentation, authentication ADRs and native client contracts to describe implemented behaviour only.

Non-goals: changing login or MFA policy; granting every account access to every household; broadening existing SMART/FHIR or application-token permissions; implementing native provider-specific passkey screens; migrating identity-provider credentials; changing provider-wide logout; importing the iOS project.

## Capabilities

### New Capabilities

- `mobile-rodauth-authentication`: Standard first-party mobile authorization through Rodauth with instance discovery, local or delegated sign-in and preserved session protections.
- `mobile-account-household-access`: Account-level mobile login with per-request household authorisation and safe household navigation.

### Modified Capabilities

None. These authentication requirements have no existing canonical capability under `openspec/specs/`. Coordinate the Android changes with the active `mobile-monorepo-remediation` change so its direct-provider configuration is superseded explicitly.

## Impact

Rails Rodauth configuration, OAuth grant/session models, API authentication and tenant context, household and session endpoints, audit handling, request filtering and authentication tests. Android AppAuth configuration, credential storage, household navigation, caches, queued work and generated API contracts. The root OpenAPI document remains authoritative; update Android's pinned copy through its generation workflow. Specify the same contract for future iOS adoption without depending on an absent iOS workspace. Reuse installed Rodauth/OAuth/OIDC libraries rather than implement PKCE or provider cryptography in application code.
