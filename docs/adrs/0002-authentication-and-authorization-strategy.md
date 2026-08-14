# ADR 0002: Authentication and Authorization Strategy

- Status: accepted and partly superseded by [ADR 0009](0009-bounded-context-map.md)
- Date: 2025-11-27

ADR 0009 replaced the original global role hierarchy with household membership
and person access boundaries. This record keeps the accepted identity,
authorization, and audit decisions.

## Context

MedTracker must protect medication and health data without treating an account,
a person, and a household role as the same concept.

Authentication must support account verification, recovery, multi-factor
methods, and optional external identity providers. Authorization must apply the
active household and person access rules to every request.

## Decision

### Rodauth manages account authentication

Rodauth is the primary web authentication system. `Account` stores the
authentication identity. `Person` stores demographic information, and `User`
connects the person to application access.

Rodauth provides:

- verified email and password sign-in;
- password reset and account recovery;
- remembered and active session management;
- TOTP, recovery codes, and passkeys;
- optional OpenID Connect sign-in through OmniAuth.

The supported flows are documented in
[Two-factor authentication](../two-factor-authentication.md),
[Passkey setup](../passkey-setup.md), and
[OpenID Connect setup](../oidc-setup.md).

### Pundit enforces household and person access

Pundit policies deny access by default. Controllers use policy checks and policy
scopes for records that belong to a household.

Authorization uses:

- one active `HouseholdMembership` with the role `owner`, `administrator`, or
  `member`;
- household ownership on tenant records;
- active person access grants for delegated care;
- separate record and manage permissions where the workflow needs them.

Professional titles such as doctor or nurse describe a person. They do not
grant a household role or access to another person's records.

The current model is documented in
[Manage people and household access](../user-management.md) and
[ADR 0009](0009-bounded-context-map.md).

### Audit records remain separate from access checks

PaperTrail records version history for audited model changes. MedTracker also
stores append-only audit evidence for security and health-data workflows.
Neither audit system grants access. Pundit remains the authorization boundary.

See [Audit trail](../audit-trail.md) for the current evidence model.

## Consequences

- Authentication identity stays separate from demographic and household data.
- Household and person access can change without changing the account identity.
- Policy scopes must be applied consistently to list and record endpoints.
- External identity providers cannot grant MedTracker data access by themselves.
- Authentication and data changes produce reviewable audit evidence.

## Superseded content

The original six global roles, legacy authentication migration phases, and
links to delivery plans no longer describe MedTracker. Git history keeps
that earlier decision text.

## Related decisions

- [External identity provider](0005-external-identity-provider.md)
- [Bounded context map](0009-bounded-context-map.md)
