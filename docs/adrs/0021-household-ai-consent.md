# ADR 0021: Explicit household permission for external AI suggestions

- Status: Accepted direction; implementation verification tracked in `require-household-ai-opt-in`
- Date: 2026-11-11

## Context

Instance configuration and a paid plan previously allowed medication suggestion
inputs to leave the application without a household decision. The owner requires
explicit permission, off by default, and proof that no suggestion data leaves
without that permission.

## Decision

Store `households.ai_suggestions_enabled` as a non-null boolean, default false
for existing and new households. Only a current household owner or administrator
may change it through the existing settings operation, with the decision and
attributable audit committed atomically. Plan changes and portability do not
grant consent. Partial API updates preserve consent when the field is omitted.

All outbound suggestion steps, including source fetching, candidate discovery
and final synthesis, require fresh authority and consent under a shared household
row lock. The application retains that lock through the outbound operation.
Settings changes acquire a conflicting exclusive lock and recheck authority
after waiting. A successful withdrawal therefore prevents any later send,
including another provider call in an in-flight suggestion. Already transmitted
data cannot be recalled. Process-local locks and cached consent are insufficient
because application instances share the same database.

Existing feature flags, paid-plan eligibility, credential scopes, membership and
RLS remain mandatory. Missing or unreadable consent fails closed. The API retains
the unavailable-feature 404 response when permission has not been granted.

## Consequences

Previously eligible households must explicitly opt in. Short existing network
timeouts limit how long withdrawal waits for an active send. Counting-provider
application tests must prove zero sends without consent and cover withdrawal
between outbound steps. Browser verification must demonstrate the disclosure,
keyboard control and persisted enable/disable decision.
