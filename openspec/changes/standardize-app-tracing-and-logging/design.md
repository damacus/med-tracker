## Context

See [proposal.md](proposal.md) for motivation and scope.

Production currently has several overlapping observability paths:

- Rails uses `EcsLogging::Logger` on stdout.
- Lograge 0.15 formats request completion as JSON text and sends it through the Rails logger and ECS formatter.
- Thruster fronts Rails and emits its own structured access event to stdout.
- Puma, Solid Queue, and the OpenTelemetry SDK can also write directly to stdout or stderr.
- application code writes free-text `Rails.logger` messages from controllers, jobs, services, initializers, and subscribers;
- selected domain operations publish `ActiveSupport::Notifications`;
- OpenTelemetry instruments Rails, Rack, Active Job, Active Record, PostgreSQL, and outbound HTTP, with a ten-percent default production sample and an always-sampled critical-path matcher.

Production samples show that ordinary application logger messages reach Loki, so Lograge is not suppressing every direct `Rails.logger` call. They also show two request records for the same request in incompatible shapes: the Lograge JSON is collected as a string-valued message without usable severity, while the proxy record has its own schema. The initializer sets `config.lograge.keep_appenders = false`, but that option is not consumed by the locked Lograge 0.15 implementation. Lograge does intentionally detach the standard Action Controller and Action View log subscribers unless `keep_original_rails_log` is enabled.

The application currently publishes eight custom event types across medication administration, low-stock handling, audit backlog monitoring, and rate limiting. Only the low-stock event has an application subscriber, and that subscriber enqueues work without preserving an operational event. Medication event specs attach temporary test subscribers, while production has no subscriber, logger adapter, metric exporter, or trace adapter for those events. Consequently, a publisher can pass its unit specs and still produce no production-visible signal. The current medication event payload also contains raw health-domain identifiers and dose details, so it cannot be copied directly into logs.

The design must preserve useful custom events while reducing noise and must not make logs or traces a secondary health record.

## Goals / Non-Goals

**Goals:**

- Establish a tested end-to-end contract from event emission to parseable production output.
- Give requests, jobs, domain decisions, and external effects a common correlation model.
- Make important outcomes queryable by stable fields rather than message text.
- Make the complete set of application-owned publishers, subscribers, and direct logger calls reviewable and mechanically checked.
- Make dispatch, subscriber failure, retry, commit, and rollback outcomes truthful and queryable.
- Keep observability fail-open so logging cannot change medication or health-data behavior.
- Retain critical traces while bounding cost and cardinality.
- Centralize privacy enforcement and make accidental sensitive attributes fail tests.
- Migrate incrementally with explicit production verification and rollback.

**Non-Goals:**

- Replacing PaperTrail or compliance evidence.
- Capturing request or response bodies, SQL values, domain record snapshots, or medication facts.
- Guaranteeing that every application execution has a retained trace.
- Making framework debug output permanently available in production.
- Selecting a new observability backend.
- Instrumenting newly discovered domain workflows outside the frozen implementation baseline.

## Decisions

### 1. Characterize the current pipeline before changing it

The first implementation slice will characterize both the test environment and the final production image. The final-image check will use the real Rails/ECS logger, Lograge, Thruster, Puma, Solid Queue, and OpenTelemetry configuration and capture stdout and exporter traffic for:

- a normal request and redirect;
- direct info, warning, and error logger calls;
- an Active Support subscriber event;
- an Active Job enqueue and execution;
- an exception path; and
- a valid and invalid OpenTelemetry span context.

The characterization will record which producer owns each line, whether the application emitted one record for its event identifier, and the final parsed shape. It will separately identify duplicates introduced by collection retries. Tests will assert the desired contract only after the baseline has demonstrated the actual failure.

Alternative considered: remove Lograge immediately. Rejected because it would replace a suspected cause with an unverified one and could restore noisy framework subscribers without preserving a canonical request event.

### 2. Introduce one fail-open operational-event boundary

Application-owned operational events will be emitted through a small observability boundary accepting:

- stable event name;
- severity;
- outcome;
- allowlisted categorical attributes;
- safe error type and reason; and
- optional human-readable message.

The boundary will enrich events with service, environment, schema version, workflow, event, causation, attempt, request, job, trace, and span context. It will serialize one JSON object to the configured Rails logger and may add the same safe attributes to the current span.

The boundary rescues its own formatting and output failures. It makes one best-effort call to a fixed-shape, rate-limited emergency diagnostic path that contains only the boundary failure category, then returns control to the domain operation. The emergency path does not call the canonical boundary and cannot recursively log its own failure. If both paths fail, the operational event is lost; this is preferable to altering the medication or health-data operation, and the contract never claims otherwise.

Active Support notifications remain the domain-event mechanism established by ADR 0004. Operational publication is adjacent to, but independent from, domain-event dispatch: the publisher emits the safe publication or attempt record before invoking business subscribers. A publisher helper may rescue around `ActiveSupport::Notifications.instrument` only to emit a safe subscriber-failure outcome and then preserve the original propagation behavior; it does not replace Active Support or change its delivery semantics. Each subscriber that makes an externally meaningful decision or side effect emits its own safe stage outcome. A business subscriber failure therefore cannot suppress the publication record, and an operational failure cannot change the domain result. Event-specific adapters construct categorical attributes instead of serializing raw notification payloads.

Alternative considered: standardize direct `Rails.logger` strings by convention. Rejected because convention cannot consistently add context, validate an allowlist, prevent nested JSON, or keep event names stable.

Alternative considered: use traces as the only diagnostic signal. Rejected because sampling and exporter availability mean a trace may not exist when a critical error needs investigation.

Alternative considered: implement operational logging solely as another Active Support subscriber. Rejected because subscriber ordering and synchronous exceptions could suppress the publication record or let logging failure alter domain behavior.

ADR 0004 remains accepted. Implementation will append a follow-up note clarifying that Active Support continues to own in-process domain-event delivery while this boundary owns privacy-safe operational signals. The ADR will be superseded only if a later change replaces or wraps the domain-event mechanism itself.

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
- `event.id`
- `trace.id`
- `span.id`
- `error.type`

Application-specific bounded fields will live under a `medtracker` namespace, including schema version, stable reason, source category, workflow stage, `workflow.id`, `causation.id`, `attempt.id`, request identifier, and job identifier. Field names and types will be documented and contract-tested.

The identifiers have distinct semantics:

- `workflow.id` is stable for one logical workflow, such as a scheduled occurrence from evaluation through delivery and a related medication action.
- `event.id` is generated once using a collision-resistant generator before serialization, identifies one application emission, remains stable through formatter and transport retries, and is never reused for a new application attempt or re-emission.
- `causation.id` references the immediate preceding event and is absent for a root event.
- `attempt.id` identifies one retry or external side-effect attempt and changes for the next attempt.

No fifth global correlation identifier is introduced. Existing request, job, trace, and span identifiers keep their standard meanings.

Raw account, household, membership, person, medication, schedule, notification subscription, and medication-take identifiers will not be logged. Workflow correlation uses a purpose-specific opaque value with documented generation, propagation, lifetime, rotation, and expiry. It must be non-reversible, must not become a permanent pseudonymous health identifier, and must never be a metric label. A scheduled workflow propagates the same `workflow.id` through enqueue, evaluation, subscriber work, and delivery attempts. A later medication action joins that workflow only through a trusted stored or client-carried occurrence association; otherwise it starts a new workflow and omits reminder causation rather than guessing from time proximity or domain identity.

Alternative considered: emit arbitrary top-level fields. Rejected because collisions and type drift make Loki parsing and long-lived queries unreliable.

### 4. Keep layers distinct and emit one Rails request completion event

The Rails layer will emit one canonical request-completion application event. The characterization phase will choose one of these mechanisms:

1. reconfigure Lograge to pass a structured object through the canonical logger;
2. replace its subscriber with an application-owned request-completion subscriber; or
3. remove Lograge if Rails provides the same tested behavior with less coupling.

The choice is constrained by the spec: one application emission for each stable Rails request-event identifier, preserved application logs, correct severity, and correlation. The unused `keep_appenders` setting will be removed regardless of the selected mechanism.

Thruster access events may remain only if characterization proves their raw path, query, network, and user-agent fields can meet the privacy contract. Otherwise Thruster request logging is disabled and the canonical Rails request event is the retained application access record. Any retained infrastructure access event uses a distinct dataset/component so operators do not mistake it for a duplicate Rails event. Routine successful health checks are filtered at each retained producer; failures remain.

### 5. Freeze a finite signal registry and workflow matrix

Task 1 freezes the implementation baseline against a named source revision. The current census is eight custom event types and fifty direct logger call sites. The baseline also names the final-image output paths:

- Thruster access output;
- Rails/ECS output, including Lograge request formatting;
- Puma output;
- Solid Queue output; and
- OpenTelemetry SDK output.

The signal registry records every baseline custom-event publisher and subscriber, direct logger call, application event, custom metric and trace annotation, and operational job entrypoint. It also records every canonical boundary, emergency diagnostic, request subscriber, publisher helper, and workflow-stage signal introduced by this change. Each entry records its source, event or message contract, production consumers, output sink, correlation context, transaction timing, failure policy, privacy classification, owner, and verification. Static coverage specs compare known production signal APIs and call sites with the registry. Dynamic event-name wrappers require explicit registry entries; a text search is supporting evidence, not a claim of semantic completeness. A test-only subscriber is not a production consumer.

Every baseline direct logger call is classified as retained, migrated, or removed. Retained output must already satisfy the canonical privacy, severity, and correlation contract. Classification does not require adding new domain-decision events outside the bounded workflow set.

The workflow matrix is limited to:

- HTTP request completion;
- medication recording, blocking, rollback, and commit;
- scheduled reminder enqueue, eligibility, deduplication, and delivery;
- low-stock evaluation and delivery;
- web and native push attempts and outcomes;
- rate limiting; and
- audit-backlog monitoring.

Each row records entrypoint, decisions, side effects, retries, existing signals, correlation context, privacy classification, missing outcomes, and verification. A gap found inside this list is in scope. A silent decision found outside it becomes a named follow-up change with an owner and does not expand this checklist. The baseline is complete when every frozen registry entry has a disposition, every listed workflow meets its scenarios, and production acceptance passes for the deployed image.

### 6. Make event delivery and transaction state explicit

In-process domain-event publication and operational observation have different responsibilities. The operational boundary records that something was attempted or occurred independently from business-subscriber dispatch. Application subscribers that make decisions or cause external effects emit their own correlated stage outcomes through the same boundary.

Every event registration declares:

- whether subscriber failure propagates, is isolated, or schedules a retry;
- whether publication occurs before commit, after commit, or outside a transaction;
- which outcome represents an attempt, persistence, commit, rollback, or external side effect; and
- how retries and idempotent re-entry are distinguished without multiplying successful outcomes.

An action is observable even when its transaction rolls back. The attempt event is emitted when the action reaches the domain boundary. Provisional persistence receives a separate event and never claims committed success. A commit-aware hook emits the committed outcome only after the outermost transaction commits; rollback emits a correlated rollback outcome while preserving the original attempt and provisional records. Subscriber exceptions emit safe correlated failures. Operational logging never turns an otherwise successful health-data write into a failure.

Alternative considered: treat a successful model save as the final operational outcome. Rejected because an enclosing transaction can still roll back, producing a misleading success event.

### 7. Keep traces sampled and logs dependable

The existing critical-path sampler remains the basis for retaining medication-administration and authentication traces. The matrix will verify and extend matcher coverage for critical jobs and notification stages. General successful traffic remains sampled at a bounded rate.

Because head sampling cannot retain an already-unsampled trace after a later failure, every failure still emits a structured application event. Operational documentation will describe this limitation rather than claiming all error traces are retained. Tail sampling is deferred because it requires backend changes outside this repository.

Alternative considered: sample every trace. Rejected because database, job, and request instrumentation would create unnecessary volume and cost.

### 8. Enforce privacy through allowlists and tests

The event boundary will accept only registered keys and scalar, bounded-cardinality values. Error reasons will be stable enums; raw exception messages are excluded by default. Tests will use marker values representing names, medication details, tokens, cookies, network identifiers, job arguments, domain IDs, and schedules, then assert they never appear in serialized events, access records, job lifecycle output, or any exported span surface. Span privacy covers names, attributes, events and event attributes, status descriptions, links, and resource data.

The observability coverage matrix must assign each proposed attribute a privacy classification. Review will treat new free-text values and identifiers as unsafe until justified.

Alternative considered: redact with a denylist after serialization. Rejected because new sensitive fields and exception text can bypass a denylist.

### 9. Verify code, the final image, and production ingestion separately

Verification has four bounded layers:

- test-mode contracts prove registry coverage, mappings, schema, correlation, privacy, transaction truth, fail-open behavior, and one application emission per event identifier;
- a final production-image smoke runs the real Rails/ECS logger, Lograge, Thruster, Puma, Solid Queue, and telemetry configuration without a source bind mount and validates stdout and exporter traffic;
- a deployed transport canary verifies the exact image digest, application-event, request, job, log, metric, and trace paths through the collector and backends; and
- a twenty-four-hour, post-deployment observation window records naturally occurring safe workflow evidence without creating medication history or sending test notifications; an unsafe or absent outcome is marked not applicable with its reason and final-image contract reference.

The deployed canary starts only after every target process reports the verified immutable image digest. It emits one safe request, application-event, job, log, metric, and sampled-trace canary, then polls for at most fifteen minutes. Passing requires every expected canary signal, zero parser, schema, privacy, missing-severity, or deployment-identity failures for its identifiers, and a retained backend trace for every referenced sampled trace. Collector duplicates are recorded and deduplicatable but are not treated as extra application attempts. Workflow outcomes that cannot be induced safely use final-image evidence; no missing natural event can extend the twenty-four-hour observation window.

CI completion permits deployment but does not complete this change. The OpenSpec remains active until the finite production matrix passes for the exact deployed image revision, saved queries work, and rollback compatibility remains available. Production evidence uses synthetic identifiers and explicit time bounds; it never relies on health data as a search key.

## Risks / Trade-offs

- **[Risk] Logger configuration is initialized in an order different from tests.** → Boot a production-equivalent application process in an integration spec and include one container-level smoke check.
- **[Risk] Central subscribers emit duplicate events alongside legacy logger calls.** → Migrate one workflow at a time and assert exact event counts before removing the legacy call.
- **[Risk] A registry becomes stale while new publishers or logger calls bypass it.** → Compare code-discovered signal sites with registry entries in a required contract spec.
- **[Risk] Logging raw custom-event payloads exposes health data.** → Require event-specific safe mappings and reject unregistered payload keys before serialization.
- **[Risk] A subscriber exception changes domain behavior unintentionally.** → Register and test propagation, isolation, and retry policy for every production subscriber.
- **[Risk] The canonical logger fails while handling a health-data operation.** → Fail open and use one fixed-shape, rate-limited, non-recursive emergency diagnostic.
- **[Risk] Events report success before an outer transaction rolls back.** → Separate attempt, provisional persistence, committed, and rollback outcomes and test outer-transaction behavior.
- **[Risk] A single envelope becomes a generic dumping ground.** → Require registered event names, allowlisted attributes, and coverage-matrix ownership.
- **[Risk] Correlation identifiers become permanent pseudonymous health keys or metric-cardinality hazards.** → Use purpose-specific opaque values with bounded lifetime and rotation, keep them out of metric labels, and expire them with their operational retention contract.
- **[Risk] Removing framework subscribers hides useful diagnostics.** → Preserve exception/error behavior in characterization tests and retain traces or metrics for timing detail.
- **[Risk] Request schema changes break existing Loki queries.** → Provide compatibility queries during migration and update operational documentation before removing the old shape.
- **[Risk] Always-sampled critical paths expose sensitive attributes.** → Keep the existing span allowlist processor and add privacy assertions for every new attribute.

## Migration Plan

1. Freeze the source-revision baseline for output producers, eight custom events, direct logger sites, and bounded workflows.
2. Characterize test-mode and final-image output and document producer, formatter, transport, collector, and backend ownership.
3. Add the event schema, correlation lifecycle, privacy registry, fail-open boundary, safe custom-event mappings, and contract specs.
4. Add the ADR 0004 follow-up note without replacing the Active Support domain-event mechanism.
5. Normalize Rails request completion and distinguish or disable Thruster access output according to the privacy contract.
6. Migrate the eight custom events and bounded medication, reminder, low-stock, push, rate-limit, and audit-backlog workflows.
7. Classify every baseline direct logger call and migrate or remove output that fails the schema or privacy contract.
8. Run test-mode and final-image gates, then deploy with the previous request logger available as a rollback path.
9. Run production acceptance against the exact image digest and keep the change active until it passes.

Rollback restores the previous request logger configuration and disables the new operational boundary while leaving the Active Support domain-event mechanism and direct application logger calls intact. Compatibility queries and the rollback switch remain until production acceptance succeeds.
