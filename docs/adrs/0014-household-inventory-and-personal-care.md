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

Deleting a storage location never deletes medication, stock, schedules,
assignments or administration history. Reject deletion while any medication is
stored there or an administration retains that taken-from location, including
after current stock has moved. An empty location without historical references
may be deleted. Making medication locations optional and detaching stock are
separate work; deletion does not implement either operation.

Portable v3 omits person-to-location membership links. Imports reject v1/v2
bundles with non-empty `people.location_portable_ids` with an actionable error;
obsolete membership metadata is not retained or silently discarded. Inventory
location labels and immutable administration taken-from locations remain part of
the portable records. Selecting v3 is explicit; existing export selectors retain
their documented versions.
V3 validates an administration's historical location independently of the
medicine's current location. Household reference checks and immutable-history
conflict checks remain mandatory.

Personal care follows the person. Current households receive current treatments,
schedules, active pauses and latest administration status. They do not automatically
receive another household's detailed history or narrative notes. The person keeps
full personal-history access and controls additional disclosure. After departure,
the old household retains events recorded there and immutable attribution, but
receives no future updates. Guardian authority must be explicit, not inferred
from household management.

For non-managers, existing household-scoped portability readers select the
account's own participating person and dependants covered by current eligible
guardian declarations. Legacy per-person Manage grants do not establish this
authority. This rule does not turn a household reader into a cross-household
personal-history reader or establish global disclosure authority for managers.

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

Canonical schedules have a person-owned UUID, typed clinical columns, a revision,
and a prescribed product/stock-unit snapshot independent of mutable inventory.
The originating household keeps its original schedule ID, portable ID and history
foreign keys as a local execution link. Revocation terminally closes that link
before participation visibility is removed; restoration never reopens it.
Suspension denies household access and retains the link for restoration.
Account-bound personal browser/mobile access retains self read/edit/pause/resume/
retire continuity after the final departure, without exposing household inventory.
Personal edits cannot select household inventory; medicine selection uses an
authorised household treatment operation. Second-household approval and binding
are separate delivery work.

New schedules capture the creating sharer's validated named timezone. Calendar,
occurrence and dose-safety calculations use that stored zone; viewer zones only
affect display. Existing schedules did not store timezone provenance and their
readers mixed application and viewer zones. At first authorised adoption, preserve
the configured application zone with `adopted_application_zone` provenance.
This is deterministic preservation, not recovery of historical origin. Store the
zone once and allow an explicit authorised correction. Users who relied on the
old viewer-dependent interpretation may need to correct an adopted zone.

Legacy per-person grants remain historical records and confer no runtime Manage
authority, regardless of their level, expiry or relationship label. Direct writes,
aggregate capabilities and treatment edit controls use the same current self or
eligible declaration-backed guardian authority. Creating a dependent follows the
household management rule; a legacy grant cannot permit creation.

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
