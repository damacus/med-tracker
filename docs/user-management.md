# User Management

## Overview

MedTracker separates care identity (`Person`) from authentication identity
(`User`).

- `Person`: clinical/care context (capacity, care relationships, demographics)
- `User`: can sign in and act in the system

Not every `Person` must have a `User`.

## Person types

`Person.person_type` values:

- `adult` (`0`): self-managing adult
- `minor` (`1`): child requiring parental/carer support
- `dependent_adult` (`2`): adult requiring carer support

Capacity (`has_capacity`) is tracked separately and used in care rules.

## User roles

`User.role` values:

- `administrator`
- `doctor`
- `nurse`
- `carer`
- `parent`
- `minor`

Roles define authorization scope; person type defines care requirements.

## Carer relationships

`CarerRelationship` links carers to patients/dependents:

- `carer_id` references a `Person`
- `patient_id` references a `Person`
- relationships can be active/inactive

A dependent person can have multiple carers. A carer can support multiple people.

## Key constraints

- Person emails are normalized and unique when present.
- People without capacity must have at least one active carer relationship.
- Carer relationship uniqueness is enforced for each carer/patient pair.

## Practical flow

1. Create a `Person`.
2. Optionally create a linked `User` for sign-in.
3. If the person lacks capacity, assign at least one active carer.
4. Use role-based policies to control who can prescribe/administer/manage.

## Related guides

- [Carer Onboarding: First Dose](user-onboarding-carer-first-dose.md)
- [Design](design.md)
- [Audit Trail](audit-trail.md)
