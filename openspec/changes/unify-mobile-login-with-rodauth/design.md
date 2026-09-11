## Context

See proposal.md for motivation and issue #1889. Browser authentication already uses Rodauth with local methods and OmniAuth OIDC. Android currently redeems provider codes itself, then calls a custom Rails ID-token exchange. That exchange checks only verifier presence and creates household-bound ApiSession records. The existing Rodauth OAuth path serves SMART/FHIR; resource_owner_params selects the first membership. Neither shortcut is the new first-party mobile contract.

## Goals / Non-Goals

Provide one first-party mobile authorization boundary, account-level sessions and request-level household authorisation. Preserve the policy and non-goals in proposal.md. This change plans Android delivery and a platform-neutral contract; an iOS implementation requires its own available workspace and delivery tasks.

## Decisions

### Rodauth owns mobile authorization

Use installed rodauth-oauth authorization-code and S256 PKCE features. Register first-party native applications separately from existing SMART/FHIR applications, with exact registered callbacks and public-client authentication; never embed a client secret in a phone. AppAuth opens MedTracker authorization in the platform browser. Rodauth uses its existing local or delegated sign-in, including both being offered together. The upstream provider exchange stays in the existing OIDC library boundary. Do not expose a mobile-specific provider SDK or native passkey UI.

Reject retaining direct mobile-to-provider token redemption as the final architecture: it duplicates identity integration and prevents phones from using one installation-owned login journey. Browser SSO reuse remains subject to platform and provider session policy.

### First-party OAuth grants identify accounts

Token/session integration is queued for detailed verification after the authentication-context discussion. The library-managed approach below is the working design; do not mark its device-session and refresh integration as verified merely because planning artifacts exist.

Use Rodauth-managed OAuth credentials for the new flow, with explicit first-party grant/application classification and mobile API scopes. Extend grant persistence and the API credential adapter to represent account-level mobile grants, including device name, last use, authentication context and revocation. Household/person binding remains mandatory for existing restricted grant kinds; do not interpret a missing membership as permission to access everything. Add conditional model and database invariants for each kind.

Keep refresh rotation and replay protection in the OAuth library. Adapt existing device-session listing/revocation to the new credential type without issuing a second set of custom tokens. Separate OAuth client registration from each user's device session so revoking one device does not revoke the entire registered mobile application.

### Resolve household authority per request

Authenticate and validate the account first. Account endpoints such as household listing and session management operate without choosing a household. For household endpoints, resolve an active membership belonging to that account and the explicitly requested operational household; establish TenantContext and AuthorizationContext from that membership before reading records or applying Pundit person/action checks. Reuse the Rails household policy boundaries after tracing their current implementation. Never fall back to first_active_household_membership or mutable account-wide current-household state.

Membership and permission changes take effect on subsequent requests without revoking unrelated household access. Existing restricted credentials retain their household, person and scope ceilings. Preserve transactional authorisation and locking rules for writes; no cross-household record lookup before context binding. Audit each action with the actual request membership, account, household and credential identity.

### Household choice is application navigation

Login returns an account credential without household selection, including for an otherwise eligible account with zero memberships. The phone loads authorised households, shows an empty state for none, opens the sole household automatically, or lets the user choose among several. A remembered choice is a UI preference, revalidated against current access.

Switching changes the active household and visible data together without changing the credential. Partition storage, pagination/sync cursors and pending mutations by instance, account and household. Ignore late responses from the previous view. Queued operations retain their original household; lost access stops them rather than moving them to the newly selected household. Preserve idempotency isolation across households.

Reject a browser household picker or a post-login selection grant for this new flow: both unnecessarily attach login to navigation. Reject broadening every existing token: third-party integrations have distinct restrictions.

### Preserve authentication policy and evidence

Reuse local and delegated authentication, optional MFA enrolment and admin tagging. Remove bespoke privileged-action freshness rules from web and API administration, including invitation resend. Keep configured login MFA, credential-management protections, role/person permissions, scopes, revocation and lockout. Preserve verified authentication evidence for session policy and audit; refresh must not manufacture a new authentication time. Validate upstream identity and browser state/nonce through the OIDC library.

All login and reauthentication after session expiry go through Rodauth. While the session remains valid, an authorised administration action does not trigger separate fresh MFA. Bind reauthentication to the initiating account and mobile session, state and PKCE. Cancellation leaves pending work unperformed; after success reauthorise its original household/action and preserve idempotency. A callback alone never executes a write.

Removal inventory: API invitation create/revoke/resend, membership update/revoke, person-grant and app-token create/revoke, and settings updates; web platform writes, household-admin gating behind HOSTED_ADMIN_MFA_REQUIRED and ambiguous-grant review. Retire these bespoke freshness checks/flag and corresponding capabilities without removing permission checks.

Current code configures web sessions for 30 minutes idle and 24 hours absolute, mobile access for 15 minutes with a 30-day refresh lifetime, and app tokens without time-based expiry. These observations do not prove the reported logout cause: trace remember-cookie and refresh/error handling. Separate interactive login lifetime from access-token lifetime: mobile access refresh can be silent while login remains valid. Refresh must respect any configured absolute login deadline. Define whether background refresh counts as activity for idle expiry. MCP app tokens represent delegated integration access rather than an interactive session and retain their existing restrictions; their expiry policy is a separate decision.

### Privacy and transaction boundaries

Authorization codes are short-lived, single-use and bound to client, callback and S256 challenge through the library. Concurrent redemption must not create multiple usable sessions. Define rollback/retry behaviour using the library's grant transaction so persistence failures cannot leave partially issued credentials. Filter codes, verifiers and tokens from parameters, HTTP logs, traces and audit metadata. Store credential digests as already configured; emit generic authentication errors without provider response bodies or household record details.

## Risks / Trade-offs

- Account tokens have broader reach if stolen → preserve secure device storage, expiry, refresh replay protection and per-device revocation; enforce membership and person policy on every request.
- Shared OAuth code may accidentally broaden SMART/FHIR access → explicit credential kinds, database invariants and regression tests for existing integration scopes and household limits.
- Membership changes during queued work or concurrent navigation → reauthorise server requests, preserve write transaction rules and keep client work tied to its original household.
- Existing capability and native pinning work overlaps → reconcile mobile-monorepo-remediation requirements/configuration during implementation without marking its unrelated work complete.
- Upstream MFA claims differ → provider-specific claim mapping behind the OIDC boundary with fixtures and actual configured-provider verification.

## Persistent access and integration-token expiry

The Rails PWA uses browser sessions and Rodauth remember cookies, not the native API refresh-token path. RodauthApp calls load_memory and the configuration extends remember deadlines. A PWA is not inherently unable to renew access; diagnose the existing remembered-login, cookie and session-expiry handling before deciding it requires frequent sign-in. Native apps use silent access-token refresh while the interactive session remains valid.

API application tokens, including MCP tokens, gain a centrally configured positive maximum age. Thirty and ninety days are examples; the default remains to be agreed. Store an expiry for each issued token no later than issuance plus the configured maximum, expose expiry in token-management responses/UI, and reject expired tokens across all accepting entry points. Usage must not slide or refresh expiry. Shorter requested expiry is allowed; longer or unlimited values are rejected. Apply any reduced instance maximum as an upper bound based on issuance, and never revive an expired token by increasing the setting. Keep revocation and membership restrictions independent of expiry.

Persist shortened deadlines when reducing the maximum so a later configuration increase cannot revive credentials invalidated by the earlier reduction. Migration must also cover pre-existing app tokens with no expiry: backfill a bounded deadline from original issuance and the selected maximum, identify already-expired credentials and document replacement. The absence of active mobile clients is not evidence that no integration tokens exist. Replacement uses the existing authenticated token-management workflow; old app tokens cannot self-renew indefinitely. Token expiry is not a human MFA challenge.

## Instance discovery

The phone needs only the chosen MedTracker instance URL. Provide an editable instance URL plus labelled canary/demo presets in distributed apps. This explicitly supersedes the active Android plan's fixed-server-only release constraint; it does not add native password forms. Changing instance clears active authentication state and keeps credentials and cached records isolated.

Expose OAuth authorization-server metadata at `/.well-known/oauth-authorization-server` through the Rodauth library. Discover issuer, authorization/token/revocation endpoints, supported grants, scopes and S256. Do not claim to be an OpenID Provider solely to provide discovery; upstream OIDC delegation is a separate role. Use the existing public API capabilities endpoint for MedTracker-specific information, including the public first-party client ID for each supported platform. Pre-provision registered clients with exact allowed callbacks; metadata discovery does not itself register an app and no client secret is delivered to phones.

Validate HTTPS, the expected issuer and the instance-owned endpoint contract before authorization. Bind callbacks, state, PKCE and credentials to the selected instance; reject inconsistent metadata without forwarding existing credentials. Show a useful error for an unsupported instance. Presets contain instance URLs only; exact preset URLs and callback identifiers remain deployment configuration.

## Migration Plan

The user confirmed there are no active mobile clients. Replace the old flow directly without a compatibility window, version inventory or dual-protocol period.

1. Implement new grant invariants, authorization discovery and account/household API handling while preserving existing restricted integration credentials.
2. Update Android, the root OpenAPI schema, pinned contract and authentication documentation together. Remove the old ID-token exchange and obsolete first-party selection path; do not retain dead mobile compatibility code. Preserve local browser login and unrelated integrations.
3. Verify local-only, mixed local/delegated and Zitadel SSO flows, instance selection/presets and existing MFA choices. Define the future iOS contract without claiming iOS implementation.
4. Roll out the coherent server/client change. A rollback restores the previous release/schema as appropriate or disables new issuance; never reinterpret account grants as restricted grants or broaden legacy grants. Revoke incompatible test/new credentials when necessary and require sign-in. Do not close #1889 before delivery and verification.

## Open Questions

Before implementation, resolve global interactive-session timeout semantics/defaults, remember/background refresh behaviour and integration-token expiry policy. Twelve hours was an example, not an agreed default. These are blocking design decisions despite CLI artifact completeness. Exact callbacks and preset URLs remain deployment values. Optional MFA enrolment and admin tagging stay fixed; action-specific freshness is removed.
