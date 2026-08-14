## Purpose

Defines a reliable, correlated, and privacy-safe observability contract for tracing application outcomes across requests, background jobs, domain decisions, and external effects.

## Requirements

### Requirement: Application events survive an available production logging pipeline
When the configured operational output path is available, the system SHALL preserve supported application events from emission through production formatting and collection so they remain queryable in the configured log backend. Total output failure MAY lose an operational event but SHALL NOT alter the domain operation.

#### Scenario: Event emitted during an HTTP request
- **GIVEN** an application event is emitted while handling an HTTP request
- **WHEN** production logging formats and collects the event
- **THEN** the application emits one parseable structured record for its stable event identifier
- **AND** request-log condensation does not suppress it

#### Scenario: Event emitted outside an HTTP request
- **GIVEN** an application event is emitted by a background job, scheduled task, subscriber, or service process
- **WHEN** production logging formats and collects the event
- **THEN** the application emits one parseable structured record for its stable event identifier without requiring controller request context

#### Scenario: Collector retries delivery
- **GIVEN** the application emitted one event and the collection pipeline retries delivery
- **WHEN** more than one backend record has the same event identifier
- **THEN** operators can detect and deduplicate the repeated ingestion
- **AND** the duplicate is not interpreted as another application attempt or outcome

#### Scenario: Application emits a new occurrence
- **GIVEN** an application operation reaches an observable occurrence
- **WHEN** its operational event is constructed
- **THEN** one collision-resistant event identifier is generated before serialization
- **AND** formatter or transport retries preserve that identifier
- **AND** a new application attempt or re-emission receives a new event identifier

#### Scenario: Production logging configuration changes
- **GIVEN** a logger, formatter, subscriber, middleware, or collector configuration change
- **WHEN** the production observability contract tests run
- **THEN** they prove that application info, warning, error, request, and job events are preserved

### Requirement: Application-owned event coverage is exhaustive and declared
The system SHALL maintain a reviewed registry for a frozen code baseline of
production output producers, application-owned event publishers and subscribers,
and direct logger call sites. Every baseline application event SHALL have a
production-visible, privacy-safe operational disposition.

#### Scenario: Custom event publisher exists
- **GIVEN** application code publishes a custom event
- **WHEN** observability coverage is validated
- **THEN** the registry identifies its publisher, payload contract, subscribers, transaction semantics, operational sink, privacy classification, owner, and verification
- **AND** its registered operational mapping emits the canonical safe application-event envelope independently of business-subscriber ordering

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
- **THEN** the pre-deployment gate fails until the registry, safe mapping, and automated verification are complete

#### Scenario: Completion census runs
- **GIVEN** the frozen baseline has been migrated and the change has introduced new operational signal paths
- **WHEN** final registry coverage is validated
- **THEN** every frozen baseline path has a tested disposition
- **AND** every publisher, subscriber, logger, emergency diagnostic, request subscriber, and stage event introduced by this change is registered and verified

#### Scenario: Baseline workflow inventory is frozen
- **GIVEN** work begins from the reviewed signal and workflow inventory
- **WHEN** the baseline is recorded
- **THEN** it identifies the source revision, production output producers, eight custom event types, direct logger call sites, and bounded workflows covered by this change
- **AND** completion is measured against that fixed inventory

#### Scenario: Out-of-baseline workflow gap is discovered
- **GIVEN** the work reveals a silent domain decision outside the frozen workflow baseline
- **WHEN** the discovery is reviewed
- **THEN** it is captured as a follow-up change with an owner
- **AND** it does not silently expand this change's completion boundary

### Requirement: Application events use one queryable envelope
The system SHALL emit application events as a single structured object using a documented field contract rather than nested JSON or incompatible message shapes.

#### Scenario: Successful domain outcome
- **GIVEN** a domain operation reaches an important successful outcome
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
The system SHALL attach opaque workflow, event, causation, and attempt identifiers plus available request, job, trace, and span context without inventing invalid identifiers or exposing domain identity.

#### Scenario: Event emitted in a traced request
- **GIVEN** a valid request identifier and sampled trace and span are active
- **WHEN** an application event is emitted
- **THEN** the event contains the request, trace, and span identifiers

#### Scenario: Request schedules a background job
- **GIVEN** a request enqueues a background job
- **WHEN** the job executes
- **THEN** job events contain the job identifier
- **AND** the job trace is linked to the originating trace using the configured propagation contract
- **AND** the workflow identifier is propagated even when the originating trace is not retained

#### Scenario: Scheduled work has no originating request
- **GIVEN** scheduled work begins without request or parent trace context
- **WHEN** it emits an application event
- **THEN** it contains its job or execution identifier and its own trace context
- **AND** absent request context is omitted rather than represented by a placeholder

#### Scenario: Unsampled trace context
- **GIVEN** an event occurs in an unsampled trace
- **WHEN** the application event is emitted
- **THEN** the event remains queryable by its non-trace correlation identifiers

#### Scenario: Workflow crosses requests and jobs
- **GIVEN** one logical workflow spans scheduling, later job execution, delivery attempts, and a related action in another request
- **WHEN** each stage emits an event
- **THEN** every stage shares one workflow identifier
- **AND** each emitted occurrence has its own event identifier
- **AND** each non-root event identifies its immediate cause
- **AND** each retry or external side-effect attempt has its own attempt identifier

#### Scenario: Later action has no trusted workflow association
- **GIVEN** a medication action occurs after a reminder workflow
- **WHEN** no trusted stored or client-carried occurrence association links the action to that workflow
- **THEN** the action starts a new workflow
- **AND** it omits reminder causation rather than inferring a relationship from time proximity or domain identity

#### Scenario: Correlation identifier privacy
- **GIVEN** an opaque correlation identifier is created
- **WHEN** it is stored, propagated, logged, or expired
- **THEN** its purpose, generation, lifetime, and rotation policy follow the documented contract
- **AND** it cannot be reversed into a raw person, medication, schedule, household, or notification identifier
- **AND** it is not used as a metric label

### Requirement: HTTP request completion has one application-level record
The system SHALL produce one canonical application-level request-completion event per non-silenced HTTP request.

#### Scenario: Completed application request
- **GIVEN** an HTTP request reaches the Rails application
- **WHEN** the application completes the request
- **THEN** one application-level request event records method, route template, status, duration, request identifier, and available trace identifiers
- **AND** it excludes raw query strings and unfiltered parameters

#### Scenario: Infrastructure also records the request
- **GIVEN** Thruster or another infrastructure layer emits its own access event
- **WHEN** both access and application events are collected
- **THEN** each event identifies its layer and dataset
- **AND** the two events are not indistinguishable duplicates

#### Scenario: Routine health check
- **GIVEN** a successful request targets a configured health or readiness endpoint
- **WHEN** request logging runs
- **THEN** routine success events are suppressed or sampled according to the volume policy
- **AND** failures remain visible

### Requirement: Critical workflows expose decisions and outcomes
The system SHALL maintain a reviewed inventory of critical workflows and emit
events at their externally visible decision and side-effect boundaries.

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
- **THEN** an in-baseline gap is completed with an owner and verification scenario
- **AND** an out-of-baseline gap is captured as a follow-up change

### Requirement: Event delivery and transactional outcomes are truthful
The system SHALL make application event dispatch and subscriber outcomes observable without reporting an uncommitted or rolled-back side effect as successful.

#### Scenario: Event is dispatched to production subscribers
- **GIVEN** an application-owned event is published
- **WHEN** registered production subscribers handle it
- **THEN** the application emits one canonical publication record identified by its event identifier
- **AND** externally visible subscriber decisions or side effects emit correlated stage outcomes

#### Scenario: Subscriber raises an exception
- **GIVEN** a registered subscriber raises while handling an event
- **WHEN** event dispatch handles the exception
- **THEN** a correlated failure event identifies the subscriber stage and allowlisted error type
- **AND** the failure policy states whether the originating operation fails, continues, or retries

#### Scenario: Side effect is retried
- **GIVEN** a subscriber or job retries an externally visible side effect
- **WHEN** attempts and outcomes are emitted
- **THEN** every attempt is distinguishable
- **AND** one successful side effect is not reported as multiple successful outcomes

#### Scenario: Event occurs inside a transaction
- **GIVEN** an event describes a domain write inside an open transaction
- **WHEN** the available operational output path observes it before the outermost transaction completes
- **THEN** its attempt and provisional-persistence events remain queryable
- **AND** neither is labelled as a committed success
- **AND** committed success is emitted only after commit

#### Scenario: Transaction rolls back
- **GIVEN** an event-related domain write is rolled back
- **WHEN** the transaction ends while the operational output path is available
- **THEN** no committed-success event is emitted
- **AND** the original attempt remains queryable
- **AND** a correlated rollback or failure outcome remains queryable

#### Scenario: Operational logging fails
- **GIVEN** the canonical operational logger or observer fails
- **WHEN** a medication or health-data operation continues
- **THEN** the observability failure does not block, roll back, or alter the domain operation
- **AND** a bounded non-recursive emergency diagnostic is attempted without sensitive payload data
- **AND** if both output paths fail, the operational event may be absent rather than falsely reported as preserved

#### Scenario: Business subscriber fails before another subscriber runs
- **GIVEN** a business subscriber raises during synchronous event dispatch
- **WHEN** later subscribers cannot run
- **THEN** the publication record and subscriber failure remain observable
- **AND** operational visibility does not depend on subscriber ordering

### Requirement: Trace retention reflects operational importance
The system SHALL retain traces for defined critical paths using a documented, testable sampling policy and SHALL preserve structured failure events when traces are not retained.

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

#### Scenario: Job arguments contain health context
- **GIVEN** a background job carries raw domain identifiers or schedule-revealing values
- **WHEN** job lifecycle logging runs
- **THEN** those arguments are absent from operational output

#### Scenario: Infrastructure access record contains request metadata
- **GIVEN** an access logger can emit raw request or network metadata, including user agents and path identifiers
- **WHEN** its output is retained
- **THEN** the same allowlist and privacy contract applies to that dataset

#### Scenario: Trace contains exception or event data
- **GIVEN** a span contains events, exception attributes, a status description, links, or resource metadata
- **WHEN** the span is exported
- **THEN** the complete exported span surface complies with the privacy allowlist

#### Scenario: Trace retains safe Kubernetes pod correlation
- **GIVEN** a span resource contains deployment-generated `host.name` together with process or arbitrary runtime attributes
- **WHEN** the span is exported
- **THEN** the non-empty `host.name` value is retained as safe deployment metadata
- **AND** `process.pid`, client and network identifiers, and unapproved resource attributes are absent

#### Scenario: Exception message contains sensitive input
- **GIVEN** an exception message may contain user or upstream input
- **WHEN** an error event is constructed
- **THEN** the event uses an allowlisted error type and stable reason
- **AND** raw exception text is included only when explicitly proven safe

### Requirement: Operators can verify and use the observability contract
The system SHALL provide test-mode, final production-image, and post-deployment checks that demonstrate the emitted data is useful, and the observability change SHALL remain incomplete until production acceptance succeeds for the exact deployed revision.

#### Scenario: Test-mode contract
- **GIVEN** the automated test environment exercises registered adapters and representative request, job, domain, warning, and error paths
- **WHEN** the observability contract runs
- **THEN** schema, correlation, privacy, transaction, subscriber-failure, retry, and exact application-emission behavior are verified

#### Scenario: Final production-image contract
- **GIVEN** the final application image boots with its production logger, request server, job backend, and telemetry configuration
- **WHEN** synthetic safe request, application-event, job, warning, error, and trace canaries run
- **THEN** captured output and exporter traffic contain the required fields and deployment identity
- **AND** decoded OTLP trace resource data contains a non-empty `host.name` for Kubernetes pod correlation
- **AND** forbidden data and nested application JSON are absent

#### Scenario: Post-deployment smoke verification
- **GIVEN** a revision containing observability changes is deployed and every target process reports the expected immutable image identity
- **WHEN** the documented smoke workflow emits synthetic safe events
- **THEN** operators can locate them by event, workflow, request, job, and trace identifiers where applicable
- **AND** the verification detects ingestion duplicates, parser failures, missing severity, missing traces, and mismatched deployment identity

#### Scenario: Production completion gate
- **GIVEN** automated checks and final-image checks are green
- **WHEN** production acceptance has not passed for the exact deployed revision
- **THEN** the change remains incomplete and is not archived
- **AND** rollback compatibility remains available

#### Scenario: Finite production evidence matrix
- **GIVEN** every target process runs the verified immutable image
- **WHEN** one safe request, application-event, job, log, metric, and sampled-trace canary is emitted
- **THEN** the deployment gate polls for at most fifteen minutes for all expected signals
- **AND** acceptance requires zero parser, schema, privacy, missing-severity, or deployment-identity failures for the canary identifiers
- **AND** collector duplicates are reported and deduplicatable rather than counted as application attempts
- **AND** every referenced sampled trace exists in the trace backend

#### Scenario: Unsafe or infrequent workflow outcome
- **GIVEN** a bounded workflow outcome cannot be induced safely or does not occur during the first twenty-four hours after deployment
- **WHEN** production evidence is recorded
- **THEN** the outcome is marked not applicable for production induction with its reason and final-image contract reference
- **AND** absence of that natural event does not extend the acceptance window

#### Scenario: Operational investigation
- **GIVEN** an operator starts with any supported correlation identifier
- **WHEN** they follow the documented query workflow
- **THEN** they can traverse the available request, job, decision, external-effect, and error events without searching for health data
