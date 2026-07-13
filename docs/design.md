# Design

## Purpose

MedTracker records medication schedules, direct assignments, and
administrations with guardrails for timing, dose limits, and stock, while
preserving attributable history inside a household authorization boundary.

## Architecture

- Backend: Ruby on Rails
- Frontend: Hotwire (Turbo and Stimulus) with Phlex components
- Authentication: Rodauth accounts with password, generic OIDC, passkeys, and
  household-bound API credentials
- Authorization: household memberships plus person-scoped access grants
- Database: PostgreSQL in development, test, and production
- Audit trail: PaperTrail change history plus security and compliance evidence

MedTracker is a modular monolith. Domain logic is enforced on the server. UI
forms and pages render server-sent HTML and use Turbo Streams for updates.

The [bounded-context map](adrs/0009-bounded-context-map.md) defines present
ownership and dependency direction for Medication Administration, Medication
Catalogue, Inventory, Health and Medication Safety, People and Care Delegation,
Household Access, Identity, Interoperability, Audit and Compliance, and the
supporting Reporting and Insights and Notifications capabilities. These are
not separate services or databases. Shared Rails models and tables may act as
persistence adapters for more than one context.

Phlex components are composition roots and rendering units, not the home of
core medication business rules. UI code consumes domain records, value objects,
presenters, and query objects rather than reimplementing supply, filtering,
authorization, or administration rules.

## Core Domain Records

| Record | Present responsibility |
| --- | --- |
| `Person` | Demographic details, care capacity, and the individual whose care data is tracked |
| `Account` and `User` | Authentication identity and the active application user linked to a person |
| `HouseholdMembership` | An account's participation and role in one household |
| `PersonAccessGrant` | Authority for a membership to view, record for, or manage one person |
| `CarerRelationship` | Descriptive care responsibility between two people; not authority by itself |
| `Medication` | Shared catalogue, administration-default, and inventory persistence record for one medicine/product |
| `MedicationDosageOption` | Shared selectable dose identity, age-based and regimen administration defaults, and optional dose-specific inventory record |
| `Schedule` | Date-bounded administration source supporting scheduled types and retained PRN semantics |
| `PersonMedication` | Direct routine or as-needed administration source without a schedule |
| `MedicationTake` | Immutable record of one completed administration from exactly one source |
| `HealthEvent` | Recorded illness or suspected side effect for a person |
| `MedicationReviewPrompt` | Practitioner-review state with an immutable medication-interaction evidence snapshot |

Use the [Glossary](glossary.md) as the source of truth for these terms and for
the distinction between care responsibility and authority.

## Medication Safety Rules

`Schedule` and `PersonMedication` expose the applicable dose and timing rules.
`TakeMedicationService` checks source state, stock availability, dose amount,
timing restrictions, and overlapping administration rules before creating a
`MedicationTake`. The take stores a dose snapshot and its concrete source.
Persisting a valid take mutates only the selected tracked inventory source when
inventory tracking is enabled and retains the selected source on the historical
record. A valid take may use untracked inventory without changing a quantity.

Important timing concepts include:

- `max_daily_doses`
- `min_hours_between_doses`
- `dose_cycle`
- schedule-specific effective doses and dates

## Person Types and Authority

Person type models care capacity:

- `adult`
- `minor`
- `dependent_adult`

Household membership role models tenant governance:

- `owner`
- `administrator`
- `member`

Person access level models authority over a specific person's records:

- `view`
- `record`
- `manage`

Relationship labels describe why access may exist. They are not substitutes
for an active `PersonAccessGrant`. Every care assignment creates or reactivates
the descriptive relationship. Only an account-backed carer receives household
membership and a standalone self grant; account-backed non-self delegation also
coordinates relationship-owned access. Deactivating an account-backed self
relationship leaves its standalone grant active. Accountless delegation,
including self delegation, has no access records, so revocation only deactivates
the description. Person-scoped authority comes from active grants; medication
and inventory policies additionally recognize household `owner` and
`administrator` governance and the narrow creator-owned, unlinked medication
exception.

## Supporting Read Models and Delivery

`Reports::*` and `SmartInsights::*` build read-only projections and derived
insights from administration, inventory, health, and safety records. They own
their report calculations and detector results, not the source facts.

Notifications own person preferences, subscriptions, delivery deduplication,
and push transport. Reminder and stock notification jobs consume outcomes from
the contexts that own administration and inventory rules.

## Auditing and Compliance

Critical model changes are versioned for traceability. Security and workflow
events are recorded with household, account, membership, and request context,
then projected into append-only compliance evidence. Audit records observe
domain outcomes; they do not own medication, inventory, or authorization
decisions. See [Audit Trail](audit-trail.md).

## UI and Accessibility Direction

UI behavior and styling are guided by:

- [Accessibility](accessibility.md)
- [Theming](theming.md)
