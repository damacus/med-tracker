## Purpose

Let authorized native users manage household locations while retaining history and person-level access boundaries.

## ADDED Requirements

### Requirement: Manage locations safely
The API SHALL support location create/update/delete using existing authorization, validation and retained-history rules, with ETags on update/delete.

#### Scenario: Create a location
- **GIVEN** an authorized household manager
- **WHEN** a valid location is submitted
- **THEN** a household-owned location is returned and immediately readable

#### Scenario: Reject stale edit
- **GIVEN** a location changed after it was read
- **WHEN** the client updates using the old ETag
- **THEN** a conflict leaves the newer version intact

#### Scenario: Retain referenced history
- **GIVEN** a location referenced by administration history
- **WHEN** the client deletes it
- **THEN** deletion is refused without losing history

#### Scenario: Reject cross-tenant writes
- **GIVEN** a foreign household location identifier
- **WHEN** the client updates it
- **THEN** a non-disclosing failure leaves both households unchanged

### Requirement: Manage person location memberships
The API SHALL create and remove location-person memberships only with authorization for the location action and selected person's management; retries SHALL not duplicate membership.

#### Scenario: Assign an authorized person
- **GIVEN** a managed person and authorized location
- **WHEN** membership creation is repeated
- **THEN** one household-consistent membership exists

#### Scenario: Reject unauthorized person assignment
- **GIVEN** a view-only or inaccessible person
- **WHEN** the client changes their location membership
- **THEN** no membership or access change occurs
