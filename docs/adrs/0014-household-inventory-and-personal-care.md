# ADR 0014: Household inventory and personal care continuity

- Status
  Accepted (architecture; implementation acceptance pending)
- Date
  2026-10-15

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

The moving person or authorised guardian may explicitly transfer stock allocated
to that person. Communal stock stays put. Transfer exact selected quantities
atomically and idempotently between valid source and destination household
participations. Stock never moves automatically.

## Alternatives

Moving all inventory with a person changes communal ownership. Copying all history
to every household discloses private notes unnecessarily. Treating locations as
tenants forces labels before useful inventory work. These alternatives are rejected.

## Consequences

Delivery must test disclosure, departure cutoffs, immutable attribution, optional
locations, exact quantities, rollback and replay without double deduction. Preserve
current-person safety status without leaking historical narrative. API location
and access semantics deliberately change; update OpenAPI and clients together.
No application acceptance is asserted by this ADR.
