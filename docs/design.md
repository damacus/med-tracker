# Design

## Purpose

MedTracker records medication schedules and administrations with guardrails
for timing and daily-dose safety, while preserving an auditable history.

## Architecture

- Backend: Ruby on Rails
- Frontend: Hotwire (Turbo + Stimulus) with Phlex components
- Authentication: account-based auth with password + generic OIDC (any provider) + passkey support
- Database: PostgreSQL in development, test, and production
- Audit trail: PaperTrail on critical clinical and identity models

Domain logic is enforced on the server. UI forms and pages render server-sent
HTML and use Turbo Streams for updates.

## Core domain entities

- `Person`: demographic and care-capacity record for an individual
- `User`: login-enabled identity linked one-to-one to `Person`
- `Medicine`: medicine catalog and supply attributes
- `Prescription`: prescribed regimen for a person
- `PersonMedicine`: non-prescription/ad-hoc medicine assignment
- `MedicationTake`: immutable dose record for safety and compliance
- `CarerRelationship`: mapping of carers to dependent people

## Safety rules

Timing restrictions are enforced in model logic:

- `max_daily_doses`
- `min_hours_between_doses`

These rules apply before a dose is persisted to prevent unsafe administration.

## Person types vs user roles

Person type models care needs:

- `adult`
- `minor`
- `dependent_adult`

User role models application permissions:

- `administrator`
- `doctor`
- `nurse`
- `carer`
- `parent`
- `minor`

## Auditing and compliance

Critical model changes are versioned for traceability, including medication
administrations and identity/relationship updates. See [Audit Trail](audit-trail.md).

## UI and accessibility direction

UI behavior and styling are guided by:

- [Accessibility](accessibility.md)
- [Theming](theming.md)
