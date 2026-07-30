## Purpose

Defines a reliable, correlated, and privacy-safe observability contract for tracing application outcomes across requests, background jobs, domain decisions, and external effects.

## ADDED Requirements

### Requirement: Application events survive the production logging pipeline
The system SHALL preserve supported application events from emission through production formatting and collection so they remain queryable in the configured log backend.

#### Scenario: Event emitted during an HTTP request
- **GIVEN** an application event is emitted while handling an HTTP request
- **WHEN** production logging formats and collects the event
- **THEN** the event is present exactly once as a parseable structured application event
- **AND** request-log condensation does not suppress it

#### Scenario: Event emitted outside an HTTP request
- **GIVEN** an application event is emitted by a background job, scheduled task, subscriber, or service process
- **WHEN** production logging formats and collects the event
- **THEN** the event is present exactly once without requiring controller request context

#### Scenario: Production logging configuration changes
- **GIVEN** a logger, formatter, subscriber, middleware, or collector configuration change
- **WHEN** the production observability contract tests run
- **THEN** they prove that application info, warning, error, request, and job events are preserved

### Requirement: Application-owned event coverage is exhaustive and declared
The system SHALL maintain a reviewed registry of every application-owned event publisher, subscriber, and direct logger call site, and SHALL prevent application-owned events from existing without a production-visible, privacy-safe operational disposition.

#### Scenario: Custom event publisher exists
- **GIVEN** application code publishes a custom event
- **WHEN** observability coverage is validated
- **THEN** the registry identifies its publisher, payload contract, subscribers, transaction semantics, operational sink, privacy classification, owner, and verification
- **AND** a production observer translates the event into the canonical safe application-event envelope

#### Scenario: Publisher has no production sink
- **GIVEN** a custom application event has no registered production log, trace, metric, or subscriber-outcome adapter
- **WHEN** observability coverage validation runs
- **THEN** validation fails
- **AND** a test-only subscriber does not satisfy the production-sink requirement

#### Scenario: Direct application logger call exists
- **GIVEN** application code writes directly to the configured logger
- **WHEN** logging coverage is validated
- **THEN** the call site is registered as retained, migrated, or removed
- **AND** retained calls have a tested production output shape, severity, privacy classification, and correlation behavior

#### Scenario: New event or logger path is introduced
- **GIVEN** a change adds an application-owned publisher, subscriber, or direct logger call
- **WHEN** the observability coverage contract runs
- **THEN** it fails until the registry, safe mapping, and production verification are complete

### Requirement: Application events use one queryable envelope
The system SHALL emit application events as a single structured object using a documented field contract rather than nested JSON or incompatible message shapes.

#### Scenario: Successful domain outcome
- **GIVEN** a domain operation reaches a meaningful successful outcome
- **WHEN** its application event is emitted
- **THEN** the event contains a timestamp, severity, stable event name, outcome, service identity, deployment environment, and schema version
- **AND** each field is queryable without reparsing JSON stored inside a message string

#### Scenario: Failed domain outcome
- **GIVEN** a domain operation fails
- **WHEN** its application event is emitted
- **THEN** the event uses warning or error severity according to the documented severity policy
- **AND** it contains an allowlisted error type and stable failure reason
- **AND** it does not depend on free-text parsing to identify the failure

#### Scenario: Human-readable explanation is useful
- **GIVEN** a structured event benefits from a concise explanation
- **WHEN** the event is emitted
- **THEN** it may include a human-readable message
- **AND** automation can still identify the event using structured fields

### Requirement: Events and traces are correlated across execution boundaries
The system SHALL attach the available correlation context to application events without inventing invalid identifiers.

#### Scenario: Event emitted in a traced request
- **GIVEN** a valid request identifier and sampled trace and span are active
- **WHEN** an application event is emitted
- **THEN** the event contains the request, trace, and span identifiers

#### Scenario: Request schedules a background job
- **GIVEN** a request enqueues a background job
- **WHEN** the job executes
- **THEN** job events contain the job identifier
- **AND** the job trace is linked to the originating trace using the configured propagation contract

#### Scenario: Scheduled work has no originating request
- **GIVEN** scheduled work begins without request or parent trace context
- **WHEN** it emits an application event
- **THEN** it contains its job or execution identifier and its own trace context
- **AND** absent request context is omitted rather than represented by a placeholder

#### Scenario: Unsampled trace context
- **GIVEN** an event occurs in an unsampled trace
- **WHEN** the application event is emitted
- **THEN** the event remains queryable by its non-trace correlation identifiers

### Requirement: HTTP request completion has one application-level record
The system SHALL produce one canonical application-level request-completion event per non-silenced HTTP request.

#### Scenario: Completed application request
- **GIVEN** an HTTP request reaches the Rails application
- **WHEN** the application completes the request
- **THEN** one application-level request event records method, route template, status, duration, request identifier, and available trace identifiers
- **AND** it excludes raw query strings and unfiltered parameters

#### Scenario: Infrastructure also records the request
- **GIVEN** a proxy or process supervisor emits its own access event
- **WHEN** both access and application events are collected
- **THEN** each event identifies its layer and dataset
- **AND** the two events are not indistinguishable duplicates

#### Scenario: Routine health check
- **GIVEN** a successful request targets a configured health or readiness endpoint
- **WHEN** request logging runs
- **THEN** routine success events are suppressed or sampled according to the volume policy
- **AND** failures remain visible

### Requirement: Critical workflows expose decisions and outcomes
The system SHALL maintain a reviewed inventory of critical workflows and emit events at their externally meaningful decision and side-effect boundaries.

#### Scenario: Medication administration attempt
- **GIVEN** a medication administration is attempted
- **WHEN** it is recorded, blocked, rejected, or fails
- **THEN** an outcome event identifies the operation, source category, and stable outcome or reason
- **AND** the event can be correlated with its request or job

#### Scenario: Notification workflow
- **GIVEN** a reminder or alert workflow evaluates a notification
- **WHEN** it decides to send, suppress, skip, deduplicate, retry, or fail delivery
- **THEN** an outcome event identifies the workflow stage and stable decision reason
- **AND** delivery attempts can be correlated with the decision that caused them

#### Scenario: External dependency workflow
- **GIVEN** the application calls an external medication, identity, mail, or push service
- **WHEN** the call succeeds, times out, is rejected, or fails
- **THEN** the operation emits a bounded-cardinality outcome event or trace annotation
- **AND** it excludes request and response bodies

#### Scenario: Silent workflow discovered during inventory
- **GIVEN** the observability inventory finds a critical decision or side effect with no observable outcome
- **WHEN** the inventory is reviewed
- **THEN** the gap is added to the implementation scope with an owner and verification scenario

### Requirement: Event delivery and transactional outcomes are truthful
The system SHALL make application event dispatch and subscriber outcomes observable without reporting an uncommitted or rolled-back side effect as successful.

#### Scenario: Event is dispatched to production subscribers
- **GIVEN** an application-owned event is published
- **WHEN** registered production subscribers handle it
- **THEN** the canonical event identifies the event and dispatch outcome exactly once
- **AND** externally meaningful subscriber decisions or side effects emit correlated stage outcomes

#### Scenario: Subscriber raises an exception
- **GIVEN** a registered subscriber raises while handling an event
- **WHEN** event dispatch handles the exception
- **THEN** a correlated failure event identifies the subscriber stage and allowlisted error type
- **AND** the failure policy states whether the originating operation fails, continues, or retries

#### Scenario: Side effect is retried
- **GIVEN** a subscriber or job retries an externally meaningful side effect
- **WHEN** attempts and outcomes are emitted
- **THEN** every attempt is distinguishable
- **AND** one successful side effect is not reported as multiple successful outcomes

#### Scenario: Event occurs inside a transaction
- **GIVEN** an event describes a domain write inside an open transaction
- **WHEN** the event is observed before the outermost transaction completes
- **THEN** it is not labelled as a committed success
- **AND** committed success is emitted only after commit

#### Scenario: Transaction rolls back
- **GIVEN** an event-related domain write is rolled back
- **WHEN** the transaction terminates
- **THEN** no committed-success event is emitted
- **AND** a correlated rollback or failure outcome remains queryable

### Requirement: Trace retention reflects operational importance
The system SHALL retain traces for defined critical paths and failures using a documented, testable sampling policy.

#### Scenario: Critical medication or authentication path
- **GIVEN** an operation matches the configured critical-path policy
- **WHEN** a root trace sampling decision is made
- **THEN** the trace is sampled regardless of the general sampling rate

#### Scenario: Non-critical successful path
- **GIVEN** a successful operation does not match the critical-path policy
- **WHEN** a root trace sampling decision is made
- **THEN** the configured bounded sampling rate applies

#### Scenario: Failure occurs after an unsampled decision
- **GIVEN** an operation fails after its trace was not sampled
- **WHEN** failure handling emits observability data
- **THEN** a structured error event remains available with non-sensitive correlation
- **AND** the limitation of missing trace detail is explicit in operational documentation

### Requirement: Observability data is health-data safe
The system SHALL enforce an allowlist for event and trace attributes and SHALL treat unapproved values as sensitive.

#### Scenario: Event describes medication behavior
- **GIVEN** an event relates to a person, medication, dose, schedule, or notification
- **WHEN** attributes are constructed
- **THEN** it excludes names, medicine identity, dose values, free-text health details, raw database identifiers, and timestamps that reveal a medication schedule
- **AND** only approved categorical or opaque correlation values are emitted

#### Scenario: Request contains credentials or personal data
- **GIVEN** a request contains authorization headers, cookies, tokens, passwords, personal data, or health data
- **WHEN** request logging or tracing runs
- **THEN** those values are absent from logs and trace attributes

#### Scenario: Exception message contains sensitive input
- **GIVEN** an exception message may contain user or upstream input
- **WHEN** an error event is constructed
- **THEN** the event uses an allowlisted error type and stable reason
- **AND** raw exception text is included only when explicitly proven safe

### Requirement: Operators can verify and use the observability contract
The system SHALL provide automated and operational checks that demonstrate the emitted data is useful after deployment.

#### Scenario: Production-like contract test
- **GIVEN** the application boots with production-equivalent logging configuration and synthetic data
- **WHEN** every registered custom event and representative request, job, domain, warning, and error path is exercised
- **THEN** captured output contains parseable events with the required fields
- **AND** forbidden data and nested JSON messages are absent

#### Scenario: Post-deployment smoke verification
- **GIVEN** a revision containing observability changes is deployed
- **WHEN** the documented smoke workflow emits synthetic safe events
- **THEN** operators can locate them by event name, request identifier, job identifier, and trace identifier where applicable
- **AND** the verification detects duplicates and missing severity

#### Scenario: Operational investigation
- **GIVEN** an operator starts with any supported correlation identifier
- **WHEN** they follow the documented query workflow
- **THEN** they can traverse the available request, job, decision, external-effect, and error events without searching for health data
