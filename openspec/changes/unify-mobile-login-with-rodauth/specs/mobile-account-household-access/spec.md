## Purpose

Let first-party mobile users sign in once and navigate their authorised households while preserving household isolation and current person and action permissions.

## ADDED Requirements

### Requirement: Account-level mobile authentication
First-party mobile login SHALL issue an account-level credential without requiring a selected household. The authenticated account SHALL be able to list only its eligible households and manage its own device sessions.

#### Scenario: Login across membership counts
- **GIVEN** an otherwise eligible account with zero, one or several active household memberships
- **WHEN** mobile login succeeds
- **THEN** authentication completes without household selection and household listing contains only currently authorised operational households

### Requirement: Per-request household authorisation
Every household request SHALL resolve current account membership and enforce household status, role, person grants and action permissions before accessing household data. Sharing an instance SHALL NOT confer access.

#### Scenario: Same credential in two authorised households
- **GIVEN** an account has different permissions in households A and B
- **WHEN** it uses the same mobile credential to request data or actions in each
- **THEN** each request is evaluated using that household's membership and person permissions

#### Scenario: Cross-household record attack
- **GIVEN** a request targets household A
- **WHEN** it supplies a record belonging to household B or an unauthorised household identifier
- **THEN** access is denied without disclosing the other household's private data or modifying any record

#### Scenario: Membership removed
- **GIVEN** an account can access households A and B
- **WHEN** its membership in A is revoked
- **THEN** subsequent requests to A are denied while permitted requests to B can continue with the same credential

#### Scenario: Changed permissions and household state
- **GIVEN** an active account credential
- **WHEN** a role or person grant is reduced, or a household becomes non-operational
- **THEN** subsequent requests enforce the new permissions or household restriction without relying on stale token claims

### Requirement: Safe household navigation
Switching households SHALL reuse the account credential and update active household and visible data consistently. Cached data and queued work SHALL remain isolated by instance, account and household.

#### Scenario: Switch with in-flight requests
- **GIVEN** the phone displays A and a request for A is pending
- **WHEN** the user switches to B and the old response arrives
- **THEN** the phone displays only B's data and does not overwrite it with A's response

#### Scenario: Queued mutation and lost access
- **GIVEN** an offline mutation belongs to A
- **WHEN** the phone switches to B or loses access to A before replay
- **THEN** the mutation remains associated with A and is never submitted as an action in B; replay without authority is denied

#### Scenario: Idempotency across households
- **GIVEN** operations in A and B use the same idempotency key
- **WHEN** they are submitted using one account credential
- **THEN** neither operation receives the other household's stored response and existing replay and rollback guarantees remain effective

### Requirement: Restricted credentials and audit remain scoped
Existing integration credentials SHALL retain their household, person and scope restrictions. Audit evidence SHALL identify the membership and household actually used for each action.

#### Scenario: Legacy integration token
- **GIVEN** an integration credential limited to A and a subset of actions
- **WHEN** it targets B or requests an action beyond its scope
- **THEN** access is denied despite the account-level behaviour of first-party mobile credentials

#### Scenario: Concurrent requests in different households
- **GIVEN** one account sends simultaneous requests to A and B
- **WHEN** the requests execute
- **THEN** their authorisation contexts and audit membership identifiers remain separate
