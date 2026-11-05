## Purpose

Defines a privacy-safe, OTel-standard observability contract for the Rust
application: structured JSON logs, OTLP traces and OTLP metrics that let
operators diagnose outages, errors and failed imports in Grafana. There is no
signal registry; coverage is enforced by a generic key rule and sentinel
privacy tests.

## REMOVED Requirements

### Requirement: Application events survive an available production logging pipeline

**Reason:** The Rails-era event-envelope contract (ActiveSupport::Notifications
subscribers, canonical publication records, collector-dedup semantics) is
replaced by the OTel log data model contract in "Structured logs follow the
OTel log data model".

### Requirement: Application-owned event coverage is exhaustive and declared

**Reason:** The reviewed signal registry for a frozen baseline caused missed
coverage and is explicitly abandoned; the generic key rule plus sentinel
privacy tests replace registry bookkeeping. There is no signal registry.

### Requirement: Application events use one queryable envelope

**Reason:** The custom application-event envelope is replaced by the OTel log
data model field contract in "Structured logs follow the OTel log data model".

### Requirement: Events and traces are correlated across execution boundaries

**Reason:** Custom workflow/causation/attempt identifier plumbing is replaced
by W3C trace context propagation and `trace_id`/`span_id` fields in
"Traces join inbound context and are recorded by default".

### Requirement: HTTP request completion has one application-level record

**Reason:** Replaced by the semconv request span and completion record in
"HTTP requests produce one server span and one completion log record".

### Requirement: Critical workflows expose decisions and outcomes

**Reason:** Replaced by the named domain-event contract in "Domain events use
named records with a stable outcome vocabulary".

### Requirement: Event delivery and transactional outcomes are truthful

**Reason:** Subscriber-dispatch semantics belonged to the Rails event bus; the
committed-only-after-commit rule is carried into "Domain events use named
records with a stable outcome vocabulary" and telemetry failures are covered
by "Telemetry failure is fail-open".

### Requirement: Trace retention reflects operational importance

**Reason:** Critical-path sampling tiers are replaced by 100% default sampling
with a ratio override in "Traces join inbound context and are recorded by
default".

### Requirement: Observability data is health-data safe

**Reason:** The per-event allowlist is replaced by the generic key rule in
"Telemetry attributes follow a generic privacy key rule"; sentinel tests
retain the same protection.

### Requirement: Operators can verify and use the observability contract

**Reason:** The production completion gate is superseded by the canary-only
export rollout defined in the `cluster-trace-backend` capability and the
fail-open contract in "Telemetry failure is fail-open".

## ADDED Requirements

### Requirement: Structured logs follow the OTel log data model

The application SHALL emit exactly one JSON object per line on stdout with no
ANSI colouring, using the OTel log data model fields: `timestamp` (RFC 3339
UTC with microseconds), `severity_text`, `severity_number` (TRACE=1, DEBUG=5,
INFO=9, WARN=13, ERROR=17), `body`, `trace_id` and `span_id` when a valid
context exists, `resource` (`service.name` = `medtracker`, `service.version`,
`deployment.environment.name`), `scope.name` (the tracing target) and
`attributes`.

#### Scenario: Request completes

- **GIVEN** an HTTP request is handled
- **WHEN** the application logs request completion
- **THEN** the record is one parseable JSON object containing `body`,
  `event.name` = `http.server.request`, `http.request.method`, `http.route`,
  `http.response.status_code` and `medtracker.duration_ms` in `attributes`
- **AND** `trace_id` and `span_id` equal the request span's W3C identifiers
- **AND** the line contains no ANSI escape sequences

#### Scenario: Event emitted inside nested spans

- **GIVEN** an event is emitted inside a span chain carrying fields
- **WHEN** the log record is formatted
- **THEN** span-chain fields are merged into `attributes` with the innermost
  span's fields winning on conflict

#### Scenario: Export disabled

- **GIVEN** no OTLP endpoint is configured
- **WHEN** an event is emitted inside a request span
- **THEN** `trace_id` and `span_id` still contain valid W3C-format identifiers
  so log records remain correlatable

#### Scenario: Logger misbehaves

- **GIVEN** the log writer fails
- **WHEN** the application continues handling work
- **THEN** the logging failure does not alter the domain operation

### Requirement: Telemetry attributes follow a generic privacy key rule

Log attributes and every exported span surface (attributes, events, links,
resource, status) SHALL admit only a fixed code-level set of OTel
semantic-convention keys plus keys prefixed `medtracker.`. String values
SHALL be capped at 256 bytes. Dropped key names SHALL be reported in
`medtracker.telemetry.dropped_keys`. There is no signal registry.

#### Scenario: Non-conforming field is emitted

- **GIVEN** code emits an attribute whose key is neither a listed
  semantic-convention key nor `medtracker.*`
- **WHEN** the log record or span is produced
- **THEN** the value is absent
- **AND** the key name appears in `medtracker.telemetry.dropped_keys`

#### Scenario: Over-long value

- **GIVEN** a conforming attribute with a string value longer than 256 bytes
- **WHEN** it is emitted
- **THEN** the value is truncated to the cap

#### Scenario: Sentinel secrets and clinical values

- **GIVEN** sentinel secrets or clinical strings present in request paths,
  query strings, headers, bodies, database errors, panics or import errors
- **WHEN** logs are written and spans are exported to the in-memory exporter
- **THEN** the sentinel values appear in neither
- **AND** raw error `Display` text is never emitted; a fixed `error.type`
  category is used instead

#### Scenario: Exported span status

- **GIVEN** a span ends with an error status
- **WHEN** the span is exported
- **THEN** the status description is stripped and only the conforming
  attribute surface remains

### Requirement: Traces join inbound context and are recorded by default

The root sampler SHALL apply `TraceIdRatioBased(trace_sample_ratio)` (default
1.0, clamped to 0..1, invalid values falling back to the default) and SHALL
ignore the inbound parent's sampled flag. A valid inbound W3C `traceparent`
SHALL be joined so the caller's trace ID is preserved.

#### Scenario: Unsampled inbound traceparent

- **GIVEN** a request arrives with a valid `traceparent` whose sampled flag is 0
- **WHEN** the request span is created
- **THEN** the span uses the inbound trace ID and is recorded and sampled
  according to the root ratio

#### Scenario: Malformed traceparent

- **GIVEN** a request arrives with a malformed `traceparent`
- **WHEN** the request span is created
- **THEN** a new trace ID is generated and the request proceeds normally

#### Scenario: Ratio override

- **GIVEN** `trace_sample_ratio` is set to a value in 0..1
- **WHEN** root sampling decisions are made
- **THEN** approximately that proportion of root traces is sampled

### Requirement: Telemetry export is disabled unless configured

When no OTLP endpoint is configured the application SHALL create no OTel SDK
objects and SHALL perform no telemetry network activity. Startup SHALL log
exactly one `info` line stating whether export is enabled, naming the endpoint
host only and never headers or credentials.

#### Scenario: Unconfigured startup

- **GIVEN** empty telemetry endpoint settings
- **WHEN** the application initialises logging
- **THEN** no provider, exporter or reader is constructed
- **AND** one `info` line states export is disabled
- **AND** no telemetry warning or error is emitted

#### Scenario: Configured startup

- **GIVEN** a configured OTLP traces endpoint
- **WHEN** the application initialises logging
- **THEN** one `info` line states export is enabled with the endpoint host

#### Scenario: Unreachable endpoint at startup

- **GIVEN** a configured endpoint that refuses connections
- **WHEN** the application starts and handles work
- **THEN** startup and request handling proceed normally

### Requirement: HTTP requests produce one server span and one completion log record

Each HTTP request SHALL produce one server span named `"{METHOD} {route}"`
with `http.request.method`, `http.route` (the matched route template or
`unmatched`), `http.response.status_code`, `error.type` equal to the status
code string on 5xx, and error span status on 5xx only. The
`x-medtracker-trace-id` response header SHALL carry the 32-hex W3C trace ID.

#### Scenario: Normal request

- **GIVEN** a request matching a route template
- **WHEN** it completes with a non-5xx status
- **THEN** the span records the route template, method and status
- **AND** the span status is not error
- **AND** the response header carries the request's trace ID as 32 lowercase
  hex characters

#### Scenario: Server error

- **GIVEN** a request completes with a 5xx status
- **WHEN** the span ends
- **THEN** `error.type` equals the status code string and the span status is
  error

#### Scenario: Unmatched route

- **GIVEN** a request matching no route
- **WHEN** the span is recorded
- **THEN** `http.route` is `unmatched` rather than the raw path

### Requirement: Database work is accounted on unit-of-work spans

The application SHALL NOT create per-query spans. It SHALL accumulate
`medtracker.db.query_count`, `medtracker.db.runtime_ms` and
`medtracker.db.failed_query_count` on the enclosing unit-of-work span
(request, job, task or import) and the corresponding completion log.

#### Scenario: Request performs queries

- **GIVEN** a request that performs several database queries including a
  failure
- **WHEN** the request span ends
- **THEN** the counters and total runtime are recorded on the span
- **AND** no individual query span exists

### Requirement: Outbound HTTP calls produce bounded client spans

Calls to external services (NHS terminology, web push, object storage, AI
help) SHALL create client spans carrying `http.request.method`,
`server.address`, `http.response.status_code` and `error.type`, and SHALL NOT
record request or response bodies.

#### Scenario: External call fails

- **GIVEN** an outbound call fails or returns an error status
- **WHEN** the client span ends
- **THEN** it carries the method, host, status class and a fixed `error.type`
- **AND** no request or response body is present

### Requirement: Telemetry emits the defined metrics when export is configured

When a metrics endpoint is configured the application SHALL export, via a
60-second periodic OTLP reader: `http.server.request.duration`,
`http.client.request.duration`, `db.client.connection.count`,
`db.client.connection.max`, `medtracker.job.duration`,
`medtracker.import.runs`, `medtracker.queue.jobs` and
`medtracker.queue.oldest_queued_age_s`. Queue gauges SHALL read cached
atomics so collection never blocks request handling.

#### Scenario: Metrics endpoint unconfigured

- **GIVEN** no OTLP metrics endpoint
- **WHEN** the application runs
- **THEN** no meter provider or reader is created

#### Scenario: Collection failure

- **GIVEN** a queue-statistics read fails
- **WHEN** the periodic collection runs
- **THEN** at most one warning per minute is emitted and one recovery line
  follows success

#### Scenario: Metrics without traces

- **GIVEN** an explicit metrics endpoint and no trace endpoint
- **WHEN** the application handles a request and metrics are flushed
- **THEN** the request-duration metric is exported without creating a tracer
  provider
- **AND** the startup record truthfully reports the enabled signals

#### Scenario: Metric labels preserve privacy

- **GIVEN** private sentinels in request paths, query strings, headers and bodies
- **WHEN** metrics are recorded and exported
- **THEN** those sentinels and person or medication identifiers are absent
- **AND** route labels contain matched templates or `unmatched`
- **AND** unsupported HTTP methods use a bounded fallback label

#### Scenario: Scheduled queue work is not overdue

- **GIVEN** queued jobs ready to run and jobs scheduled for the future
- **WHEN** the worker refreshes its cached observations
- **THEN** queue counts include the supported job states
- **AND** the oldest queued waiting time considers only ready-to-run work
- **AND** collection callbacks read cached atomics without querying the database

#### Scenario: Queue refresh fails after success

- **GIVEN** a successful cached queue snapshot
- **WHEN** a later refresh fails
- **THEN** the last successful values remain available
- **AND** a failure does not replace them with a misleading empty queue

#### Scenario: Reused instrumentation preserves actual outcomes

- **GIVEN** completed HTTP calls, background jobs and import attempts
- **WHEN** their metrics are recorded
- **THEN** durations use seconds and the actual terminal outcome
- **AND** import stage events do not increment the run count separately

#### Scenario: One provider stalls during shutdown

- **GIVEN** both providers are enabled and one SDK lifecycle call stalls
- **WHEN** the application flushes or shuts down
- **THEN** the healthy provider still receives its lifecycle request
- **AND** the caller's total wait is bounded at two seconds
- **AND** runtime teardown does not wait for the stalled SDK call

#### Scenario: Exporter recovery is independent

- **GIVEN** metric export is failing while trace export succeeds
- **WHEN** traces are exported
- **THEN** metric recovery is not reported
- **AND** one metric recovery record follows actual metric export success

### Requirement: Domain events use named records with a stable outcome vocabulary

Domain events SHALL be emitted as log records (and span events where a span
is active) carrying `event.name`, `medtracker.outcome` and optional
`error.type`. Committed-success events SHALL be emitted only after the
outermost transaction commit succeeds. No clinical values or
person/medication identifiers SHALL appear.

#### Scenario: Defined event catalogue

- **GIVEN** a domain workflow reaches a listed decision or side effect
- **WHEN** its event is emitted
- **THEN** `event.name` is one of: `medtracker.dose.attempted`,
  `medtracker.dose.committed`, `medtracker.dose.rolled_back`,
  `medtracker.dose.blocked`, `medtracker.dose.replayed`,
  `medtracker.reminder.scheduled`, `medtracker.reminder.suppressed`,
  `medtracker.reminder.sent`, `medtracker.reminder.provider_failed`,
  `medtracker.mail.enqueued`, `medtracker.mail.delivered`,
  `medtracker.mail.retried`, `medtracker.mail.failed`,
  `medtracker.auth.sign_in_failed`, `medtracker.auth.throttled`,
  `medtracker.support.session_started`, `medtracker.support.session_ended`,
  `medtracker.support.session_expired`
- **AND** `medtracker.outcome` is one of `success`, `failure`, `blocked` or
  `rolled_back`

#### Scenario: Transaction rolls back

- **GIVEN** a dose write is rolled back
- **WHEN** events are emitted
- **THEN** no committed event exists and a correlated `rolled_back` event does

#### Scenario: Idempotent replay

- **GIVEN** a replayed dose request
- **WHEN** it completes
- **THEN** exactly one `replayed` event is emitted

### Requirement: Panics are logged safely

The panic hook SHALL log `error.type` = `panic` with `code.*` location and
the active trace context, SHALL record span status error, and SHALL never
emit the panic payload message.

#### Scenario: Handler panics

- **GIVEN** a panic in request handling
- **WHEN** the panic hook runs
- **THEN** the log record contains the code location and trace identifiers
- **AND** the payload message is absent

### Requirement: Telemetry self-diagnostics are rate-limited

OTel SDK internal diagnostics SHALL be routed into the application tracing
pipeline. The first export failure SHALL emit one `warn` with
`event.name` = `medtracker.telemetry.export_failed` and an `error.type`;
further failures SHALL emit at most one warning per 5 minutes carrying a
suppressed count; recovery SHALL emit one `info` with `event.name` =
`medtracker.telemetry.export_recovered`. The exporter's own HTTP client SHALL
NOT produce spans or logs.

#### Scenario: Repeated export failure

- **GIVEN** a controllable failing exporter
- **WHEN** exports fail repeatedly
- **THEN** exactly one warning is emitted until the rate window passes
- **AND** the suppressed count increases
- **AND** no span or log is produced by the exporter HTTP client itself

#### Scenario: Export recovers

- **GIVEN** exports succeeded after a failing period
- **WHEN** the next export succeeds
- **THEN** one `medtracker.telemetry.export_recovered` info record is emitted

### Requirement: Telemetry failure is fail-open

No telemetry construction, write, flush or export failure SHALL block, roll
back, delay beyond its bound, or alter a domain operation. Shutdown flush
SHALL be bounded at 2 seconds.

#### Scenario: Failing exporter during a request

- **GIVEN** an exporter that always fails
- **WHEN** requests are handled
- **THEN** responses are unaffected
- **AND** only the rate-limited diagnostics appear

#### Scenario: Scheduler child shutdown

- **GIVEN** a scheduler child process exits
- **WHEN** telemetry shutdown runs
- **THEN** flush completes within the bound and a failed flush produces at
  most the rate-limited warning
