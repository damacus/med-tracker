## Purpose

Keep mobile sign-in safe under invalid identity responses, concurrent redemption
and persistence failures, without weakening existing household access controls.

## ADDED Requirements

### Requirement: Bound and single-use code redemption

Mobile authorization SHALL require S256 and exact registered client and redirect
binding. Invalid, missing, expired or spent redemption inputs SHALL NOT issue
credentials. A single code SHALL produce at most one successful credential lineage.

#### Scenario: Invalid authorization or redemption

- **GIVEN** a registered public mobile application and a valid authorization setup
- **WHEN** authorization lacks S256, or redemption uses a wrong client, wrong or
  missing verifier, different redirect, expired code or spent code
- **THEN** no new usable credential is returned and the response contains no secrets

#### Scenario: Concurrent valid redemption

- **GIVEN** one unexpired authorization code and the matching client, redirect and verifier
- **WHEN** independent requests redeem that code concurrently
- **THEN** at most one request succeeds and there are never two independently usable
  credential lineages from the code

### Requirement: Atomic issuance and safe retry

Code consumption and credential persistence SHALL be atomic. Failed persistence
SHALL NOT expose a successful issuance response, usable partial credential or
successful-issuance audit. Retrying SHALL NOT duplicate a committed issuance.

#### Scenario: Persistence rolls back

- **GIVEN** an otherwise valid code redemption
- **WHEN** credential persistence fails and the issuance transaction rolls back
- **THEN** no usable credentials or successful-issuance audit remain, and retrying
  the unexpired code can complete no more than one successful issuance

#### Scenario: Response is lost after commit

- **GIVEN** code redemption committed but the client did not receive its response
- **WHEN** the client retries the same code
- **THEN** it receives no newly issued credentials and must restart authorization

### Requirement: Identity and callback failures do not authenticate

Delegated login SHALL reject invalid issuer, audience, signature, expiry or nonce.
Mobile callbacks SHALL remain bound to the initiating instance and state.
Cancellation or mismatched identity SHALL NOT resume a pending protected write.

#### Scenario: Invalid upstream identity

- **GIVEN** delegated browser login is in progress
- **WHEN** the provider result has an invalid issuer, audience, signature, expiry or nonce
- **THEN** no mobile authorization grant is issued from that result

#### Scenario: Cancelled or mismatched callback

- **GIVEN** a phone initiated login for a selected instance and pending account context
- **WHEN** the browser flow is cancelled or returns mismatched state, instance or account
- **THEN** the initiating context is not authenticated or upgraded and pending writes remain unperformed

### Requirement: Current household authority survives concurrent use

An account credential SHALL authorise each request using current membership and
person/action permission in the requested household. Concurrent requests SHALL
NOT share household context. Restricted integration credentials SHALL retain
their existing scope and household ceilings.

#### Scenario: Membership lost in one household

- **GIVEN** one mobile credential with access to households A and B
- **WHEN** membership in A is removed and later requests access A and B
- **THEN** A is denied without leaking records while otherwise authorised B access remains available

#### Scenario: Concurrent requests and rollback

- **GIVEN** concurrent writes in two authorised households using the same client idempotency key
- **WHEN** one household transaction fails while the other succeeds
- **THEN** the failed write leaves no domain changes or successful audit/idempotency result,
  and the successful write and audit belong only to their original household

#### Scenario: Restricted integration cannot adopt account access

- **GIVEN** an integration credential restricted to one household and scope
- **WHEN** it attempts another household or a wider mobile-account operation
- **THEN** the request is denied without broadening that credential

### Requirement: Private diagnostics and truthful discovery

Authentication responses, logs, traces and audit events SHALL exclude codes,
verifiers, access/refresh tokens and private provider payloads. Discovery SHALL
describe the available instance-owned authorization flow without exposing secrets
or advertising the retired exchange.

#### Scenario: Failure contains sensitive input

- **GIVEN** a failed authorization request or provider response contains credential material
- **WHEN** diagnostics and audit events are emitted
- **THEN** no credential material or private provider response body is exposed

#### Scenario: Phone discovers the supported flow

- **GIVEN** a configured instance with a registered public mobile client
- **WHEN** a phone discovers login configuration
- **THEN** it receives that instance's authorization metadata and public client/callback
  configuration without requiring the retired exchange or a mobile client secret
