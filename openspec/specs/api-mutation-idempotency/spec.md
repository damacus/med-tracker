## Purpose

Defines serialization and atomic replay behavior for concurrent idempotent API mutations.

## Requirements

### Requirement: Matching concurrent mutations execute once
The system SHALL serialize authenticated mutating API requests before controller
mutation when their request identity matches. This identity contains the routed
household and authenticated account scope. It also contains the request path,
HTTP method, and `Idempotency-Key`. After the first request commits, every
matching waiter SHALL replay its stored status and body with
`Idempotency-Replayed: true` without executing the mutation again.

#### Scenario: Concurrent People creates converge
- **GIVEN** two database connections submit the same valid People create for the same household and authenticated account with the same path, method, key, and request digest
- **WHEN** both requests pass the initial idempotency boundary concurrently
- **THEN** one request performs the create and stores its response
- **AND** the other request waits, then returns the stored status and body with `Idempotency-Replayed: true`
- **AND** exactly one Person, one applicable access grant or carer relationship, and one Person mutation audit version are committed

#### Scenario: A later retry replays the committed response
- **GIVEN** a matching idempotent mutation has committed successfully
- **WHEN** the same authenticated account later repeats the request within the retention period
- **THEN** the system returns the stored status and body with `Idempotency-Replayed: true`
- **AND** it produces no additional mutation or mutation side effect

### Requirement: Mutation and idempotency response commit atomically
The system SHALL hold serialization across the post-lock lookup, controller mutation, mutation side effects, and response storage. A successful mutation SHALL NOT commit unless its replayable response also commits.

#### Scenario: Response storage fails
- **GIVEN** a keyed People create has produced a successful response but its idempotency response cannot be stored
- **WHEN** the idempotent attempt finishes
- **THEN** the Person and every access, audit, and other mutation side effect are rolled back
- **AND** the key remains retryable

#### Scenario: Successful response storage releases the waiter
- **GIVEN** a matching request is waiting for the active idempotent attempt
- **WHEN** the first mutation and stored response commit
- **THEN** serialization is released
- **AND** the waiter observes the committed response before deciding to replay

### Requirement: Key reuse conflicts remain scoped and PHI-safe
The system SHALL replay a stored response only when the authenticated account scope, HTTP method, request path, and request digest match the stored request. Reuse outside that request identity SHALL return HTTP 409 with error code `idempotency_key_reused` and SHALL NOT reveal the stored request or response.

#### Scenario: Same key with a different request digest conflicts
- **GIVEN** an idempotency key has a stored response for one request body
- **WHEN** the same account reuses the key with different filtered parameters
- **THEN** the system returns HTTP 409 with error code `idempotency_key_reused`
- **AND** neither request's submitted values or stored response data are disclosed
- **AND** no second mutation runs

#### Scenario: A different account cannot replay the response
- **GIVEN** an account has stored a keyed response inside a household
- **WHEN** another authorized account in that household submits the same key and request
- **THEN** the system returns HTTP 409 with error code `idempotency_key_reused`
- **AND** it does not return the first account's response or run a mutation

#### Scenario: The same key is independent across households
- **GIVEN** two authenticated accounts are routed to different households
- **WHEN** each submits the same key, method, path shape, and request body in its own household
- **THEN** each household may execute and store its own mutation
- **AND** neither request can observe the other household's response or health data

### Requirement: Failed attempts do not poison an idempotency key
The system SHALL release transaction-scoped serialization on commit, rollback,
database disconnect, or raised exception. Responses below HTTP 500 MAY be stored
as completed outcomes under the existing response policy. HTTP 500 responses
and raised exceptions SHALL NOT store a completed response and SHALL leave the
key retryable.

#### Scenario: A deterministic client error is replayed
- **GIVEN** a keyed mutation completes with a deterministic HTTP 4xx response that is eligible for storage
- **WHEN** a matching request repeats within the retention period
- **THEN** the system replays the stored error status and body with `Idempotency-Replayed: true`
- **AND** no mutation side effect exists

#### Scenario: An exception rolls back and releases the key
- **GIVEN** a keyed mutation raises after starting database work
- **WHEN** the idempotent attempt exits
- **THEN** all mutation and response-storage work is rolled back
- **AND** serialization is released automatically
- **AND** a later matching request may attempt the mutation again

#### Scenario: A server-error response is retryable
- **GIVEN** a keyed mutation returns HTTP 500 or higher
- **WHEN** the attempt finishes
- **THEN** no completed response is stored
- **AND** the key is not permanently reserved
- **AND** a later matching request may attempt the mutation again

### Requirement: Serialization metadata remains non-sensitive
The system SHALL derive its serialization identity without including request bodies, filtered parameters, person details, medication details, or other protected health information in database locks, application logs, or public errors.

#### Scenario: A lock wait or failure is reported safely
- **GIVEN** an idempotent request waits for serialization or fails while holding it
- **WHEN** the condition is logged or returned as an error
- **THEN** diagnostics contain no raw idempotency key, submitted health data, or stored response body
- **AND** the public response uses the existing generic API error envelope
