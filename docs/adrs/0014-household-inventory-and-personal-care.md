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
guardian is normally authorised by an existing guardian. For a minor with no
currently effective guardian in any household, a current owner or administrator
of a household in which the minor actively participates may explicitly restore
an eligible open local declaration link or appoint an eligible adult participant.
The manager's appointment is a new declaration with its own source and actor;
it does not assert the appointee's consent or verified legal status. Withdrawn
and superseded declarations remain terminal and attributable. Recovery requires
a reason, deliberate confirmation, a fresh global authority check and audit in
one transaction. It does not create an account or membership, and it does not
authorise dependent-adult recovery or generic care delegation.
The confirmation binds to an opaque latest declaration-history token and a
separate opaque local-link token that rotates on activation or deactivation;
neither token grants authority or reveals a foreign guardian.

Around an eighteenth birthday, recovery uses database-clock bounds covering
UTC−12 through UTC+14. Possible valid guardian authority anywhere blocks
recovery, while a subject or appointee must be unambiguously eligible. This may
delay recovery for about 26 hours around the boundary; it prevents a timezone
mismatch from authorising takeover.
An existing guardian's temporary login lockout does not by itself permit a
replacement appointment; the acting manager and new appointee must still have
usable accounts. Ordinary initial declaration birthday rules are unchanged.
Recovery takes actor and appointee account locks before the declaration lock to
match personal withdrawal. It retains the declaration barrier before subject
person locks and freshly checks candidate lockout, local eligibility, the
confirmation snapshot and global orphanhood after all lock waits.

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
