# Glossary

This glossary defines MedTracker's core domain language so model names, UI
copy, documentation, and business rules stay consistent.

UI components and presenters use these terms and consume server-side domain
objects rather than inventing parallel terminology. The
[bounded-context map](adrs/0009-bounded-context-map.md) defines which context
owns each concept when Rails persistence records are shared.

Prefer these names in new code, UI copy, documentation, and tests.
Avoid introducing new patient, individual, plan, routine, stock, or dose-record
names unless the surrounding bounded context requires them.

## People and Care Terms

### Person

The individual receiving medication or being represented in MedTracker.

- `Person` is the canonical model and code term for the tracked individual.
- User-facing copy may say "person" or a specific relationship label when that
  is clearer.
- Avoid introducing parallel `Patient`, `Individual`, or `Subject`
  abstractions for this concept.

### Carer

A person with responsibility for supporting another person's care.

- Being described as a carer does not itself grant access.
- Use "carer", "parent", or another explicit relationship label in UI copy
  when that is clearer than a generic actor label.

### CarerRelationship

The descriptive record of care responsibility between a carer and a person
receiving care.

- `CarerRelationship` records the two people, relationship type, household,
  and whether that responsibility is active.
- It answers **who is responsible for whom**, not **what may this account do**.
- The current model names the care-recipient association `patient`, but
  `Person` remains the canonical domain term.
- Code must not infer authority from this record alone.

### Care Delegation

The transactional workflow that coordinates responsibility and authority.

- `CareDelegation::Assign` always creates or reactivates a
  `CarerRelationship`.
- When the carer has an account, assignment ensures household participation and
  a standalone self grant.
- Account-backed non-self delegation creates or validates the
  relationship-owned `PersonAccessGrant`; revocation deactivates the
  relationship and revokes only the grants that relationship owns.
- An account-backed self relationship uses a standalone self grant whose
  `carer_relationship` is absent. Deactivating the descriptive self
  relationship does not revoke that grant.
- Accountless delegation, including self delegation, creates no membership or
  grant. Revocation only deactivates the descriptive relationship.
- An independent manual grant is preserved and must be resolved explicitly
  when it conflicts with delegation changes.

## Household Access Terms

### HouseholdMembership

An account's participation in one household.

- Membership roles are `owner`, `administrator`, and `member`.
- Membership status is independent of a person's care relationship.
- Identity establishes the account; an active membership establishes the
  household in which that account is acting.

### PersonAccessGrant

The authority for one household membership to access one person.

- Access levels are `view`, `record`, and `manage`.
- A grant may expire or be revoked.
- A grant may be owned by a `CarerRelationship` or exist independently.
- Person-scoped Pundit decisions authorize from active grants, not from
  relationship labels.
- Medication and inventory policies also recognize household `owner` and
  `administrator` governance and the narrow creator-owned, unlinked medication
  exception. Household governance roles are not the obsolete application-wide
  roles.

### Care Relationship Type

The descriptive responsibility recorded by `CarerRelationship`.

- Exact values are `parent`, `family_member`, `professional_carer`, and `self`.
- This value is separate from household membership role, person access level,
  and grant relationship metadata.

### Grant Relationship Metadata

The descriptive reason stored on a `PersonAccessGrant`.

- Exact values are `self`, `parent`, `family_member`, `carer`, and
  `professional`.
- The default access-level/metadata mappings are `parent` to `manage`/`parent`,
  `family_member` to `manage`/`family_member`, and `professional_carer` to
  `record`/`professional`.
- A valid explicit non-self access level overrides only the default access
  level; the mapped grant relationship metadata does not change.
- Account-backed self delegation always ensures a standalone
  `manage`/`self` grant rather than a relationship-owned grant. Care delegation
  does not currently create the `carer` grant metadata value.

## Medication Inventory Terms

### Supply

The inventory quantity for a medication.

- Use `Supply` as the umbrella domain term for inventory quantity.
- Use the more specific terms below for current quantity, last-restock
  quantity, or reorder thresholds.
- Prefer "Remaining Supply" over "Stock" in person- and carer-facing copy when
  the meaning is the amount left now.

### Remaining Supply (`medications.current_supply`)

The number of dispensable units left **right now**.

- Decrements when a dose is recorded against that selected, tracked inventory
  source. A valid dose against untracked inventory does not change a quantity.
- Drives low- and out-of-stock logic.
- Should be shown in person- and carer-facing quantity displays.
- Prefer **Remaining Supply** or **units remaining** over generic **Stock**
  where the meaning is "what is left now".

### Supply at Last Restock (`medications.supply_at_last_restock`)

The value of `current_supply` immediately after the most recent restock.

- Set by `Medication#restock!`.
- Used as the denominator for progress bars so the bar drains proportionally
  from 100% to 0%.
- Falls back to `reorder_threshold` when absent on older data.

### Reorder Threshold (`medications.reorder_threshold`)

The level at or below which a medication is considered low stock.

- `low_stock?` is true when remaining supply is at or below the threshold.

## Medication Catalogue Terms

### Medication

The canonical record for one medicine or product held by a household.

- Catalogue semantics include product names, dm+d identity, barcode, category,
  and selectable dose identity.
- Administration semantics include default schedule configuration, frequency,
  dose limits, minimum timing, and dose cycle.
- Inventory semantics include location, supply, restock, and reorder state.
- `Medication` is currently a shared persistence adapter for these contexts; it
  does not collapse Catalogue, Medication Administration, and Inventory into
  one bounded context.

### MedicationDosageOption

A selectable dose identity, regimen defaults, and optional inventory for a
medication.

- Dose amount, unit, and description belong to Catalogue semantics.
- Frequency, default maximum daily doses, default minimum hours between doses,
  and default dose cycle belong to Medication Administration semantics.
- `default_for_adults` and `default_for_children` belong to Medication
  Administration's age-based default-selection semantics.
- Optional dose-specific supply and threshold values belong to Inventory
  semantics.
- The model persists in the `dosages` table for compatibility.

## Medication Administration Terms

### Schedule

A date-bounded medication administration source tied to a person.

- A `Schedule` carries dose and timing snapshots for its active period and
  supports scheduled types plus the retained `prn` type.
- It is one source of medication administration.

### PersonMedication

A direct routine or as-needed medication assignment without a schedule.

- `PersonMedication` remains distinct from `Schedule`.
- It is the second source of medication administration.

### MedicationTake

The persisted record of one completed administration event.

- A take has exactly one source: `Schedule` or `PersonMedication`.
- It stores the administered dose and selected inventory source as history.
- Persisted takes are immutable.
- UI copy may use "dose" or "administration" where clearer, but code should
  not introduce a second model name.
- Avoid introducing a separate `DoseRecord` model name unless an ADR changes
  the persistence boundary.

## Record Lifecycle Terms

### Retirement

An explicit, reversible lifecycle transition for a Medication, Person, or
Location root.

- Retirement moves a root to logical cold storage and excludes it from active
  selectors and future activity while preserving authorized historical access.
- Retirement must not cascade to dependent records beyond the root-specific
  lifecycle rules in the [record lifecycle contract](operations/record-lifecycle.md).

### Logical cold storage

A lifecycle and visibility state for a retired root, not a separate database
tier.

- Retired roots remain resolvable to authorized historical, administrative,
  reporting, sync, export/import, and restoration workflows with a retired
  label.

### Reactivation

The explicit transition that returns one selected retired root to active state.

- Reactivation changes only that root and must not cascade to dependent records.
- It never silently restores schedules, assignments, stock placement,
  memberships, or care relationships.

## Health and Medication Safety Terms

### HealthEvent

A recorded illness or suspected side effect for a person.

- Event kinds are `illness` and `suspected_side_effect`.
- An event may identify associated medications without becoming medication
  administration history.

### MedicationReviewPrompt

The practitioner-review state created for a detected medication interaction.

- A prompt retains an immutable snapshot of the evidence that produced it.
- Review status and practitioner acknowledgement belong to Health and
  Medication Safety, not Medication Catalogue.

## Supporting Capability Terms

### Report and Insight

A read-only projection or derived interpretation of records owned by domain
contexts.

- `Reports::*` owns report-specific calculations and presentation records.
- `SmartInsights::*` owns detector results derived from administration,
  inventory, health, and safety records.
- Neither capability owns or writes back the source facts it reads.

### Notification Preference

A person's choices for reminder and stock-notification delivery.

- Notifications own preferences, subscription and delivery mechanics,
  delivery deduplication, and push transport.
- The context that produces an administration or inventory outcome retains
  ownership of the fact that triggered a notification.

## Usage Guidance

- Use **Remaining Supply** where users need to know what is left now.
- Use **Reorder Threshold** for the level that triggers low-stock behavior.
- Use **Schedule** for date-bounded administration sources, including the
  retained `prn` type, not **Plan** or **Routine** in new code.
- Use **MedicationTake** for persisted administration records, not
  **DoseRecord** in new code.
- Use **CarerRelationship** for responsibility and **PersonAccessGrant** for
  authority; use **Care Delegation** for the workflow that coordinates them.
