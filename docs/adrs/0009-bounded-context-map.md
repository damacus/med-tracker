# ADR 0009: Bounded Context Map

- Status: Accepted
- Date: 2026-07-13

## Context

MedTracker has domain rules for medication identity, stock, administration,
care responsibility, authorization, authentication, data exchange, and audit.
Those rules currently live in one Rails application and sometimes share the
same Active Record models and tables. Without an explicit context map, a model
name can be mistaken for ownership of every concern stored on that record, and
descriptive care relationships can be mistaken for authorization.

The application is a modular monolith. Its bounded contexts are semantic and
transactional boundaries today. They are not separate Rails engines, services,
deployment units, or databases.

## Decision

Use the following bounded contexts and supporting capabilities to describe
present ownership and dependency direction. This map covers the current
medication, care, safety, access, integration, reporting, notification, and
compliance capabilities named below; it is not an exhaustive classification of
every Rails or operational component. Existing Rails models and tables may act
as shared persistence adapters. A shared adapter may expose fields used by more
than one context without making those contexts the same domain.

| Context | Present responsibility and ownership | Current code landmarks |
| --- | --- | --- |
| Medication Administration | Defines scheduled and direct administration sources, applies dose and timing rules, owns regimen defaults, and records immutable dose history. It owns `Schedule`, `PersonMedication`, and `MedicationTake` semantics. | `MedicationAdministration::RecordDose`, `MedicationAdministration::RestoreHistory`, `MedicationAdministration::HistoricalDataMigration`, `MedicationDoseSource`, `DoseTimingPolicy`, `DoseCycle`, administration defaults on `Medication` and `MedicationDosageOption`, medication-take controllers |
| Medication Catalogue | Defines medicine/product identity, catalogue codes, display data, and selectable dose identity. It does not own household stock, regimen defaults, or administration history. | Catalogue attributes on `Medication`, dose identity on `MedicationDosageOption`, `BarcodeCatalogEntry`, `NhsDmdBarcode`, `NhsDmd::*`, `BarcodeCatalog::*` |
| Inventory | Tracks location-bound quantities, restocks, reorder state, thresholds, and the tracked stock source consumed by an administration. | Inventory attributes on `Medication` and `MedicationDosageOption`, `SupplyLevel`, `RestockMedicationService`, `AdjustMedicationInventoryService`, `MedicationStockSourceResolver`, `MedicationTakeStockMutation` |
| Health and Medication Safety | Owns recorded illnesses and suspected side effects, medication-interaction evidence, and the practitioner-review state and immutable evidence snapshots created from that evidence. It does not own administration history or medicine identity. | `HealthEvent`, `HealthEventMedication`, `HealthEvents::*`, `MedicationReviewEvidenceRecord`, `MedicationReviewPrompt`, `MedicationReviewPromptSync` |
| People and Care Delegation | Defines the tracked person, care capacity, and descriptive responsibility between people. It coordinates creation and revocation of the authority needed to act on that responsibility. | `Person`, `CarerRelationship`, `CareDelegation::Assign`, `CareDelegation::Revoke`, people and carer-relationship controllers |
| Household Access | Owns tenant participation and authorization: household membership, household roles, person-scoped grants, access levels, expiry, and revocation. | `Household`, `HouseholdMembership`, `PersonAccessGrant`, `AuthorizationContext`, `TenantContext`, Pundit policies and scopes |
| Identity | Owns authentication identities, credentials, account state, sessions, and tokens. It establishes who is acting but does not decide which person records they may access. | `Account`, `AccountIdentity`, Rodauth credential models, `User`, `ApiSession`, `ApiAppToken`, `OauthGrant`, authentication controllers and services |
| Interoperability | Translates authenticated, authorized domain data for external clients and applies imports through domain persistence boundaries. It does not own medication history, stock, people, or grants. | `Api::V1::*`, `Api::Fhir::R4::*`, `Fhir::R4::Serializer`, `PortableData::*`, `Api::SyncSnapshot`, MCP tools and resources |
| Reporting and Insights (supporting) | Builds read-only report projections and derived insights from records owned by other contexts. It owns presentation-specific calculations and detector results, not the underlying administration, inventory, health, or safety facts. | `Reports::*`, `SmartInsights::*`, report controllers and report views |
| Notifications (supporting) | Owns person notification preferences, delivery deduplication, subscriptions, and push transport. It consumes administration and inventory outcomes without owning the rule or fact that triggered a message. | `NotificationPreference`, `NotificationEvent`, `PushNotificationService`, `NativePush::*`, reminder and stock notification jobs |
| Audit and Compliance | Observes security and domain outcomes, preserves attributable change history and append-only evidence, and exports or verifies that evidence. It does not make medication or authorization decisions. | PaperTrail versions, `SecurityAuditEvent`, `Audit::Event`, `AuditLedgerEntry`, `AuditCheckpoint`, `Audit::EvidenceExporter`, audit verification and Object Lock services |

## Ownership Rules

### Medication Administration

`Schedule` and `PersonMedication` are the two retained administration sources.
`MedicationTake` records the dose snapshot and exactly one source through
`MedicationDoseSource`. Persisted takes are immutable. Administration depends
on Catalogue for medicine and selectable dose identity, Inventory for
availability and stock mutation, People for the care recipient, and Household
Access for the caller scope.

`Schedule` is a date-bounded source that supports scheduled types and the
retained `prn` type. `PersonMedication` is the direct routine or as-needed
source; it is not the only current representation of as-needed administration.

`MedicationAdministration::RecordDose` is the sole normal creator of
`MedicationTake` records. `MedicationAdministration::RestoreHistory` restores
immutable portable history without replaying stock mutation, while
`MedicationAdministration::HistoricalDataMigration` is limited to legacy
household and location metadata repair. These are explicit restoration and
migration exceptions, not alternative dose-recording workflows.

### Medication Catalogue and Inventory

`Medication` is currently a shared persistence adapter. Catalogue owns the
meaning of fields such as product names, dm+d identifiers, barcodes, category,
and selectable dose identity. Medication Administration owns regimen defaults,
including default schedule type and configuration, frequency, maximum daily
doses, minimum hours between doses, dose cycle, and age-based default selection
through `default_for_adults` and `default_for_children`. Inventory owns the
meaning of location, current supply, last-restock supply, reorder thresholds,
reorder state, and stock mutation. `MedicationDosageOption` is also shared: its
dose amount, unit, and description belong to Catalogue; its regimen and
age-based selection defaults belong to Medication Administration; and its
optional supply and threshold fields belong to Inventory.

Inventory may depend on Catalogue identifiers to determine which product or
dose is stocked. Catalogue lookup and import code must not infer household
stock, a person's regimen, or dose history.

### Health and Medication Safety

`HealthEvent` records illnesses and suspected side effects for a person.
`MedicationReviewEvidenceRecord` provides interaction evidence, while
`MedicationReviewPrompt` owns the resulting practitioner-review state and its
immutable evidence snapshot. This context may consume Catalogue identity and
Administration history without taking ownership of those source records.

### People, Care Delegation, and Household Access

`CarerRelationship` describes responsibility between a carer and a person
receiving care. It is not the authorization source. `PersonAccessGrant` is the
authority record for one `HouseholdMembership` and one `Person`, with `view`,
`record`, or `manage` access.

`CareDelegation::Assign` and `CareDelegation::Revoke` coordinate the two
contexts transactionally. Every assignment creates or reactivates the
descriptive relationship. When the carer has an account, assignment ensures the
carer's membership and standalone self grant. Account-backed non-self
delegation also creates or validates the relationship-owned person grant.
Non-self revocation disables the relationship and revokes only grants it owns;
it refuses to silently remove independent authority when an unowned grant
remains. An account-backed self relationship uses a standalone self grant whose
`carer_relationship` is absent, and deactivating the descriptive self
relationship does not revoke that grant. An accountless assignment, including
self assignment, creates no membership or access grant; revocation only
deactivates its descriptive relationship.

Person-scoped policy decisions derive authority from active
`PersonAccessGrant` records, not from relationship labels. Medication and
inventory policy decisions also recognize the current `owner` and
`administrator` household governance roles and the narrow creator-owned,
unlinked medication exception. Those household roles are distinct from the
obsolete application-wide role hierarchy.

### Identity and Household Access

Identity authenticates an `Account` and manages credential/session lifecycle.
Household Access binds the authenticated account to an active
`HouseholdMembership` and establishes `AuthorizationContext` and
`TenantContext`. Session and app-token records retain the membership and its
permissions version, but grant evaluation remains owned by Household Access.

### Interoperability and Audit

Interoperability is an adapter around the domain contexts. API, FHIR, portable
data, sync, and MCP paths authenticate through Identity and authorize through
Household Access. Each adapter reads or writes according to its published
contract; the hosted MCP surface is currently read-only. Importers may
coordinate a transaction, but imported clinical history remains owned by
Medication Administration and imported stock remains owned by Inventory.

Audit and Compliance consumes attributable outcomes and change records from
all contexts. Domain decisions must not depend on audit queries or evidence
exports succeeding, except where a workflow explicitly requires an audit write
to be atomic for compliance.

### Reporting, Insights, and Notifications

`Reports::*` and `SmartInsights::*` create read-only projections and derived
insights from records owned by other contexts. They own report calculations and
detector results, not administration, inventory, health, or safety source
facts.

Notifications own person preferences, subscription and delivery mechanics,
delivery deduplication, and push transport. Reminder and stock notification
jobs consume domain outcomes; they do not own the administration or inventory
rules that trigger delivery.

## Dependency Direction

- Web and API controllers are delivery adapters and call application services
  or domain records; controllers do not own domain rules.
- Medication Administration depends on Catalogue, Inventory, People, and
  Household Access, not the reverse.
- Health and Medication Safety depends on People and Catalogue identity and may
  consume Administration history; it owns its health-event and review records.
- Inventory depends on Catalogue identity, not on administration history for
  product meaning. An administration may trigger an inventory mutation.
- People and Care Delegation may coordinate Household Access writes, while
  Household Access remains the authority source.
- Identity establishes the actor; Household Access establishes the actor's
  tenant and person scope.
- Interoperability depends on the owning contexts and must not become a second
  source of truth for their records.
- Reporting and Insights reads owning-context records and produces derived
  projections; it does not write back new source facts.
- Notifications owns preferences and delivery mechanics while consuming
  outcomes from the contexts that define administration and inventory rules.
- Audit and Compliance observes outcomes and preserves evidence; it does not
  decide whether a dose or access request is allowed.

## Consequences

The context map gives maintainers concrete ownership language without a
physical reorganization. New rules should be placed with the owning context,
even when they currently use a shared Active Record adapter. Changes to shared
models must identify which context owns the changed behavior and which other
contexts consume it.

The current layout does not enforce these boundaries at compile time. Shared
models, callbacks, and transactions still couple contexts, so tests and review
must protect dependency direction. This ADR does not authorize a microservice,
database, Rails engine, or namespace split.

## Related Documents

- [Design and Architecture](../design.md)
- [Glossary](../glossary.md)
- [MedicationTake Aggregate Source Boundary](0006-medication-take-aggregate-source-boundary.md)
- [External App Integration Contract](0007-external-app-integration-contract.md)
- [Domain Events with ActiveSupport::Notifications](0004-domain-events-with-active-support-notifications.md)
- [Authentication and Authorization Strategy](0002-authentication-and-authorization-strategy.md),
  whose six-role hierarchy, role-based policy examples, and obsolete migration
  status are superseded by this ADR; its Rodauth, deny-by-default Pundit, and
  base PaperTrail audit decisions remain in force
