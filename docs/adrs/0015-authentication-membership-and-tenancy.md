# ADR 0015: Authentication, membership and tenancy responsibilities

- Status
  Proposed (organisation-plugin replacement boundary)
- Date
  2026-10-15

## Context

Better Auth RS is pinned to `9f91cd7fdb8a5f73a9c69d5c56d363468a7dc281`.
It has not been proved faulty. The root integration uses its organisation plugin
and adapters under `src/models/identity/better_auth/`. The pinned invitation
acceptance path makes separate writes for membership and invitation status;
those writes alone cannot guarantee an atomic domain acceptance operation.

## Decision

Accepted constraints: reuse maintained authentication/protocol libraries. Household
membership, care disclosure, live revocation checks and RLS have one explicit
application authority. Organisation adapters must not create a second membership
source. Keep atomic domain boundaries wherever the pinned plugin cannot provide
them. Preserve the current secure canonical adapters and atomic invitation domain
operation rather than replacing them with plugin separate writes. In particular,
`src/models/identity/better_auth/store/members.rs::create_member` rejects direct
membership creation and requires atomic invitation acceptance or registration.

The exact organisation-plugin removal or replacement boundary remains **Proposed**.
Do not interpret this record as an approved instruction to remove the plugin or
restrict it to authentication only. Verify library capabilities and negative
security tests before choosing a replacement.

Authentication proves account identity. Application membership decides household
participation; disclosure authority decides cross-household sharing. Session active
household selection cannot grant membership. Every request and mutation rechecks
current authority; RLS, audit attribution, replay protection, stock atomicity and
mail outbox remain mandatory.

## Alternatives

Blind plugin removal discards maintained capabilities without evidence. Separate
organisation membership tables create conflicting authorities. Separate acceptance
writes can leave partial membership. None is accepted as the replacement design.

## Consequences

Retain the current secure adapter while investigating the narrow limitation.
Future approval needs transaction/rollback, replay, revocation and cross-tenant
tests plus a documented supported library boundary. This ADR neither certifies
new behaviour nor claims Better Auth is defective.
