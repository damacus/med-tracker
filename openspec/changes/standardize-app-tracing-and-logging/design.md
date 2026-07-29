## Context

See [proposal.md](proposal.md) for motivation and scope.

Production currently has several overlapping observability paths:

- Rails uses `EcsLogging::Logger` on stdout.
- Lograge 0.15 emits a separate JSON-formatted request-completion line.
- The process supervisor or proxy emits its own structured access event.
- application code writes free-text `Rails.logger` messages from controllers, jobs, services, initializers, and subscribers;
- selected domain operations publish `ActiveSupport::Notifications`;
- OpenTelemetry instruments Rails, Rack, Active Job, Active Record, PostgreSQL, and outbound HTTP, with a ten-percent default production sample and an always-sampled critical-path matcher.

Production samples show that ordinary application logger messages reach Loki, so Lograge is not suppressing every direct `Rails.logger` call. They also show two request records for the same request in incompatible shapes: the Lograge JSON is collected as a string-valued message without usable severity, while the proxy record has its own schema. The initializer sets `config.lograge.keep_appenders = false`, but that option is not consumed by the locked Lograge 0.15 implementation. Lograge does intentionally detach the standard Action Controller and Action View log subscribers unless `keep_original_rails_log` is enabled.

The design must preserve useful custom events while reducing noise and must not make logs or traces a secondary health record.

## Goals / Non-Goals

**Goals:**

- Establish a tested end-to-end contract from event emission to parseable production output.
- Give requests, jobs, domain decisions, and external effects a common correlation model.
- Make important outcomes queryable by stable fields rather than message text.
- Retain critical traces while bounding cost and cardinality.
- Centralize privacy enforcement and make accidental sensitive attributes fail tests.
- Migrate incrementally with explicit production verification and rollback.

**Non-Goals:**

- Replacing PaperTrail or compliance evidence.
- Capturing request or response bodies, SQL values, domain record snapshots, or medication facts.
- Guaranteeing that every application execution has a retained trace.
- Making framework debug output permanently available in production.
- Selecting a new observability backend.

## Decisions

### 1. Characterize the current pipeline before changing it

The first implementation slice will boot the application with production-equivalent logger and Lograge configuration and capture stdout for:

- a normal request and redirect;
- direct info, warning, and error logger calls;
- an Active Support subscriber event;
- an Active Job enqueue and execution;
- an exception path; and
- a valid and invalid OpenTelemetry span context.

The characterization will record which subscriber owns each line, whether it is emitted once, and the final parsed shape. Tests will assert the desired contract only after the baseline has demonstrated the actual failure.

Alternative considered: remove Lograge immediately. Rejected because it would replace a suspected cause with an unverified one and could restore noisy framework subscribers without preserving a canonical request event.

### 2. Introduce one application-event boundary

Application-owned operational events will be emitted through a small observability boundary accepting:

- stable event name;
- severity;
- outcome;
- allowlisted categorical attributes;
- safe error type and reason; and
- optional human-readable message.

The boundary will enrich events with service, environment, schema version, request, job, trace, and span context. It will serialize one JSON object to the configured Rails logger and may add the same safe attributes to the current span. Existing `ActiveSupport::Notifications` events remain valid domain instrumentation; centralized subscribers will translate selected events into the application envelope rather than each caller hand-formatting log strings.

Alternative considered: standardize direct `Rails.logger` strings by convention. Rejected because convention cannot consistently add context, validate an allowlist, prevent nested JSON, or keep event names stable.

Alternative considered: use traces as the only diagnostic signal. Rejected because sampling and exporter availability mean a trace may not exist when a critical error needs investigation.

### 3. Use an ECS-compatible envelope with a MedTracker namespace

The serialized event will use ECS-compatible fields where semantics match:

- `@timestamp`
- `log.level`
- `message`
- `service.name`
- `service.version`
- `service.environment`
- `event.name`
- `event.outcome`
- `event.dataset`
- `trace.id`
- `span.id`
- `error.type`

Application-specific bounded fields will live under a `medtracker` namespace, including schema version, stable reason, source category, workflow stage, request identifier, and job identifier. Field names and types will be documented and contract-tested.

Raw account, household, membership, person, medication, schedule, notification subscription, and medication-take identifiers will not be logged. Where an investigation genuinely needs cross-event identity beyond request, job, or trace context, the design will require a purpose-specific opaque correlation token with a documented lifetime rather than reusing a domain key.

Alternative considered: emit arbitrary top-level fields. Rejected because collisions and type drift make Loki parsing and long-lived queries unreliable.

### 4. Keep layers distinct and emit one Rails request completion event

The Rails layer will emit one canonical request-completion application event. The characterization phase will choose one of these mechanisms:

1. reconfigure Lograge to pass a structured object through the canonical logger;
2. replace its subscriber with an application-owned request-completion subscriber; or
3. remove Lograge if Rails provides the same tested behavior with less coupling.

The choice is constrained by the spec: exactly one parseable Rails completion event, preserved application logs, correct severity, and correlation. The unused `keep_appenders` setting will be removed regardless of the selected mechanism.

Proxy or supervisor access events may remain because they observe a different layer, but they must use a distinct dataset/component so operators do not mistake them for duplicate Rails events. Routine successful health checks will be filtered at each producing layer where practical; failures remain.

### 5. Build and close an observability coverage matrix

Implementation will inventory workflows by operational question rather than file count. The initial matrix covers:

- medication recording and rejection;
- scheduled reminder enqueue, eligibility, deduplication, and delivery;
- low-stock evaluation and delivery;
- authentication, token exchange, and rate limiting;
- audit delivery and backlog health;
- external medication catalogue/content lookups;
- mail and push delivery; and
- scheduled imports.

Each row records entrypoint, decisions, side effects, retries, existing logs, existing notifications, existing spans, correlation context, privacy classification, missing outcomes, and verification. A gap is complete only when an operator-facing question maps to a stable event or trace; logging every branch is explicitly discouraged.

### 6. Keep traces sampled and logs dependable

The existing critical-path sampler remains the basis for retaining medication-administration and authentication traces. The matrix will verify and extend matcher coverage for critical jobs and notification stages. General successful traffic remains sampled at a bounded rate.

Because head sampling cannot retain an already-unsampled trace after a later failure, every failure still emits a structured application event. Operational documentation will describe this limitation rather than claiming all error traces are retained. Tail sampling is deferred because it requires backend changes outside this repository.

Alternative considered: sample every trace. Rejected because database, job, and request instrumentation would create unnecessary volume and cost.

### 7. Enforce privacy through allowlists and tests

The event boundary will accept only registered keys and scalar, bounded-cardinality values. Error reasons will be stable enums; raw exception messages are excluded by default. Tests will use marker values representing names, medication details, tokens, cookies, domain IDs, and schedules, then assert they never appear in serialized events or exported span attributes.

The observability coverage matrix must assign each proposed attribute a privacy classification. Review will treat new free-text values and identifiers as unsafe until justified.

Alternative considered: redact with a denylist after serialization. Rejected because new sensitive fields and exception text can bypass a denylist.

### 8. Verify both application output and production ingestion

Automated verification has two layers:

- production-like contract specs capture stdout and validate cardinality, JSON parsing, schema, correlation, severity, and privacy; and
- a synthetic post-deployment smoke workflow emits safe named events and supplies Loki queries that verify ingestion, field extraction, duplicate count, and correlation.

The smoke workflow must not create medication history, enqueue real notifications, or include real household data. Saved queries and runbook examples will be updated in the same deployment window as a schema change.

## Risks / Trade-offs

- **[Risk] Logger configuration is initialized in an order different from tests.** → Boot a production-equivalent application process in an integration spec and include one container-level smoke check.
- **[Risk] Central subscribers emit duplicate events alongside legacy logger calls.** → Migrate one workflow at a time and assert exact event counts before removing the legacy call.
- **[Risk] A single envelope becomes a generic dumping ground.** → Require registered event names, allowlisted attributes, and coverage-matrix ownership.
- **[Risk] Correlation identifiers increase cardinality.** → Keep them in structured metadata/fields intended for exact lookup, never as unbounded stream labels.
- **[Risk] Removing framework subscribers hides useful diagnostics.** → Preserve exception/error behavior in characterization tests and retain traces or metrics for timing detail.
- **[Risk] Request schema changes break existing Loki queries.** → Provide compatibility queries during migration and update operational documentation before removing the old shape.
- **[Risk] Always-sampled critical paths expose sensitive attributes.** → Keep the existing span allowlist processor and add privacy assertions for every new attribute.

## Migration Plan

1. Capture the current production-like outputs and document Lograge, Rails logger, proxy, subscriber, job, and OpenTelemetry ownership.
2. Add the event schema, privacy registry, event boundary, and contract specs without migrating callers.
3. Normalize Rails request completion and remove the unused Lograge option; deploy behind a configuration switch that can restore the previous request logger.
4. Verify synthetic events and request output in Loki, including severity, parseability, correlation, and duplicate counts.
5. Build the workflow coverage matrix and migrate one bounded workflow at a time, beginning with medication administration and notification delivery.
6. Expand critical trace matching and operational queries as each workflow migrates.
7. Remove legacy free-text events only after their replacement is visible in production and referenced queries are updated.

Rollback restores the previous request logger configuration and disables centralized event subscribers while leaving direct application logger calls intact. Schema consumers retain compatibility queries until the migration is complete.

## Open Questions

- Whether the proxy access dataset can be filtered or relabelled in the application deployment repository; the application contract does not depend on that answer.
- How long compatibility queries should remain after the canonical schema is fully deployed; this affects operations documentation, not application behavior.
