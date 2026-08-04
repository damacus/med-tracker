## Purpose

Defines safe persistence and replay of allowlisted response headers for idempotent API mutations.

## Requirements

### Requirement: Replayed mutations preserve allowlisted response headers

The system SHALL persist and restore only explicitly allowlisted, non-sensitive response headers for completed idempotent API mutations. The initial allowlist SHALL contain `ETag`.

#### Scenario: A create replay returns the original ETag

- **GIVEN** an authenticated account completes a keyed resource creation that returns an ETag
- **WHEN** the same account repeats the matching request in the same household before expiry
- **THEN** the system returns the stored status, body, and original ETag with `Idempotency-Replayed: true`
- **AND** no additional resource or mutation side effect is created

#### Scenario: An update replay returns the original ETag

- **GIVEN** an authenticated account completes a keyed resource update that returns a new ETag
- **WHEN** the same account repeats the matching request before expiry
- **THEN** the system returns the stored status, body, and exact post-update ETag
- **AND** it does not update the resource again

#### Scenario: A concurrent waiter returns the committed ETag

- **GIVEN** two matching authenticated mutations compete for the same household and idempotency key
- **WHEN** the first request commits its mutation and response under the serialized reservation
- **THEN** the waiting request returns the same status, body, and ETag with `Idempotency-Replayed: true`
- **AND** exactly one mutation and one set of mutation side effects commit

### Requirement: Replayable response headers use a strict allowlist

The system MUST persist and restore response headers only when their names are present in the explicit replay allowlist. Stored headers SHALL contain no PHI, credentials, cookies, request identifiers, cache metadata, or rate-limit timing.

#### Scenario: Sensitive and delivery-specific headers are excluded

- **GIVEN** a completed keyed mutation response contains ETag plus non-allowlisted headers
- **WHEN** the system persists and later replays that response
- **THEN** the stored and replayed header set contains the ETag
- **AND** it does not contain `Set-Cookie`, `Authorization`, `X-Request-Id`, `Cache-Control`, `Retry-After`, or rate-limit headers

#### Scenario: Stored data cannot bypass the current allowlist

- **GIVEN** an idempotency row contains a manually inserted non-allowlisted response-header key
- **WHEN** a matching request replays the row
- **THEN** the system does not add that header to the HTTP response

### Requirement: Header replay preserves tenant and account boundaries

The system SHALL expose stored allowlisted headers only when the authenticated account, household, method, path, and request digest match the completed idempotency row.

#### Scenario: A different account cannot receive stored headers

- **GIVEN** a completed idempotent response belongs to one account in a household
- **WHEN** another account attempts to reuse the key in that household
- **THEN** the system returns HTTP 409 `idempotency_key_reused`
- **AND** it does not expose the stored status, body, or response headers

#### Scenario: Households retain independent replay namespaces

- **GIVEN** two households use the same raw idempotency key
- **WHEN** each household completes and replays its own matching request
- **THEN** each replay returns only its household's stored response and allowlisted headers

### Requirement: Legacy headerless rows age out without duplicate mutation risk

The system SHALL keep an unexpired legacy idempotency row with empty response headers replayable and SHALL remove an expired matching row only while holding the existing serialized reservation.

#### Scenario: An unexpired legacy row remains replayable

- **GIVEN** a matching pre-deployment row has stored status/body, empty response headers, and a future expiry
- **WHEN** the authenticated account repeats the request
- **THEN** the system replays the stored status/body without inventing an ETag
- **AND** it does not execute the mutation again

#### Scenario: An expired row is replaced under serialization

- **GIVEN** a matching idempotency row has reached its recorded expiry
- **WHEN** an authenticated mutation reuses the key
- **THEN** the system removes the expired row after acquiring the reservation lock
- **AND** it executes the mutation once and atomically stores the new response with its allowlisted headers

#### Scenario: Concurrent callers cannot both reopen an expired key

- **GIVEN** two matching requests encounter the same expired idempotency row
- **WHEN** they compete for the existing reservation namespace
- **THEN** one request replaces the expired row and the other replays the replacement
- **AND** exactly one new mutation commits
