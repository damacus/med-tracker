# ADR 0014: Household inventory and personal care continuity

- Status
  Accepted (architecture; implementation acceptance pending)
- Date
  2026-10-15; owner refinements recorded 2026-10-16

## Context

People can participate in more than one household. Medication stock has physical
ownership, while care continuity and disclosure belong to the person. This
supersedes old location permissions and automatic shared-history assumptions.

## Decision

Households own medication inventory and stock. Locations are optional physical
storage labels within a household, with no memberships or permissions. Inventory
must work without creating any location.

Personal care follows the person. Current households receive current treatments,
schedules, active pauses and latest administration status. They do not automatically
receive another household's detailed history or narrative notes. The person keeps
full personal-history access and controls additional disclosure. After departure,
the old household retains events recorded there and immutable attribution, but
receives no future updates. Guardian authority must be explicit, not inferred
from household management.

When an adult creates a new accountless dependent, that adult declares themselves
the initial guardian. Record a declaration, not verified legal status. A successor
guardian must be authorised by an existing guardian. Disputed guardian recovery
is deferred and fails closed. This declaration does not create an account for the
dependent.

A person or an explicitly authorised guardian must approve sharing that person's
care with another household. Household role and manager status do not establish
guardian authority. Only the person or an explicitly authorised guardian may
change a shared treatment plan, including its medicine, dose, schedule or pause.

Preserve automatic notifications for dependants authorised through an explicit
guardian relationship, and retain each account's own preferences. Notifications
about another non-dependent person require an explicit per-person opt-in from the
receiving account. Household manager status does not authorise notifications. Do
not apply a blanket opt-in to every household member.

A household manager may explicitly match local stock to the medicine prescribed
by the shared plan. Record that decision in the audit trail. A match requires the
exact medicine, formulation, strength and stock unit. Never infer a match from
names, dose text or descriptive similarities, and never convert between units.
Matching stock does not authorise changing the shared treatment plan.

The moving person or authorised guardian may explicitly transfer stock allocated
to that person. Communal stock stays put. Transfer the exact selected quantity
and stock unit atomically and idempotently between valid source and destination
household participations. Stock never moves automatically.

## Alternatives

Moving all inventory with a person changes communal ownership. Copying all history
to every household discloses private notes unnecessarily. Treating locations as
tenants forces labels before useful inventory work. Treating household management
as guardianship confuses two different authorities. These alternatives are rejected.

## Consequences

Delivery must test disclosure, departure cutoffs, immutable attribution, optional
locations, exact medicine and unit matching, exact quantities, rollback and replay
without double deduction. Preserve current-person safety status without leaking
historical narrative. API location and access semantics deliberately change;
update OpenAPI and clients together.

No application acceptance is asserted by this ADR.
