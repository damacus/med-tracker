# Application Observability

MedTracker emits privacy-safe operational events for requests, jobs, medication
actions, notification workflows, external adapters, and process diagnostics.
These records complement traces and in-process domain events. They are not a
health-data audit log and must never be used to reconstruct a person's
medication history.

The reviewed signal inventory is
`config/observability/signal_registry.yml`. A new publisher, subscriber,
direct logger, or signal adapter is incomplete until it has a registry entry,
an owner, a privacy classification, and automated verification.

## Canonical event schema

Application-owned events are one flat JSON object on stdout. They use
`medtracker.application`, `medtracker.request`, or `medtracker.job` as their
dataset. Puma, Solid Queue, and OpenTelemetry SDK process diagnostics use their
own datasets.

| Field | Contract |
| --- | --- |
| `@timestamp` | UTC ISO 8601 timestamp with milliseconds. |
| `log.level` | One of `debug`, `info`, `warn`, `error`, or `fatal`. |
| `event.name` | Registered stable event name; never inferred from message text. |
| `event.outcome` | `success`, `failure`, or `unknown`. |
| `event.dataset` | Producer-scoped dataset. |
| `event.id` | Fresh UUID for one application emission. Transport retries preserve it. |
| `service.name` | `medtracker`. |
| `service.version` | Deployed application or image identity. |
| `service.environment` | Rails environment. |
| `medtracker.schema.version` | Integer schema version. |
| `medtracker.reason` | Registered stable reason code. |
| `error.type` | Allowlisted exception class only; exception text is excluded. |

Event-specific fields are accepted only when registered. A human-readable
`message` is optional and comes from a fixed internal mapping. JSON encoded
inside `message` is invalid.

## Correlation identifiers

| Identifier | Purpose and lifecycle |
| --- | --- |
| `event.id` | Identifies one emitted occurrence. Re-emission creates a new value; collector retries retain it so duplicate ingestion is detectable. |
| `medtracker.workflow.id` | Opaque UUID shared by one logical workflow. It defaults to a twenty-four-hour lifetime and rotates when invalid or expired. |
| `medtracker.causation.id` | The immediate causing event UUID. It is omitted when no trusted association exists. |
| `medtracker.attempt.id` | Fresh UUID for each retry or external side-effect attempt. |
| `medtracker.request.id` | Bounded Rails request identifier when a request exists. |
| `medtracker.job.id` | Bounded Active Job identifier when a job exists. |
| `trace.id` and `span.id` | Lowercase trace identifiers only when valid context exists. |

Request-to-job propagation serializes only the opaque workflow, causation,
attempt, and expiry values. A scheduled reminder workflow carries the same
workflow identifier through enqueue, evaluation, intent, recipient, channel,
and provider stages. A later medication action joins it only through a trusted
stored or client-carried occurrence association. Time proximity, person
identity, or medication identity must never be used to guess causation.

Correlation values are not domain identifiers, are not reversible, expire with
their operational purpose, and must not be metric labels.

## Privacy contract

Operational output is allowlist-only. It excludes account, household,
membership, person, medication, schedule, notification-subscription, and
medication-take identifiers; names; medicine identity; dose values; schedule
times; request parameters; raw paths with identifiers; query strings; IP
addresses; user agents; cookies; credentials; tokens; job arguments; provider
endpoints; notification content; upstream bodies; and exception messages.

Unknown fields are discarded. Error records use a stable reason and exception
class. Exported spans are copied through an allowlist covering the span name,
attributes, events and event attributes, status description, links and link
attributes, and resource attributes. Treat a proposed free-text or identifier
field as sensitive until its registry and tests prove otherwise.

## Severity policy

| Severity | Use |
| --- | --- |
| `debug` | Optional bounded diagnostics that are not required for an investigation. |
| `info` | Normal attempts, accepted work, committed outcomes, suppression decisions, and recovery. |
| `warn` | Expected business blocking, throttling, partial provider failure, configuration warnings, or recoverable dependency degradation. |
| `error` | Unexpected operation failure, permanent delivery failure, job failure, or subscriber failure. |
| `fatal` | Reserved for process-level conditions that prevent the service from continuing. |

Outcome and reason remain authoritative; operators must not classify an event
by parsing a message.

## Transaction truth

A medication action emits an attempt when it reaches the service boundary.
Successful persistence inside a transaction is provisional and is not a
committed success. The committed event is registered on the outermost
transaction and emitted only after commit. An outer rollback emits a correlated
rollback outcome while leaving the attempt and provisional records queryable.

Business-rule blocks, persistence failures, an unavailable household, and
unexpected failures use separate stable outcomes. Idempotent re-entry does not
multiply a committed side effect; a genuine retry receives a distinct attempt
identifier.

## Failure isolation

Operational publication is fail-open. Canonical serialization or output failure
must not block, alter, or roll back a medication or other health-data operation.
The publisher attempts one fixed-shape emergency diagnostic on stderr containing
only the failed event name and exception class. Failure of that path is
suppressed and cannot recurse.

Active Support domain subscribers retain their existing synchronous business
semantics. The publication attempt is recorded before dispatch. If a business
subscriber raises, a safe subscriber-failure event is attempted and the
original exception is re-raised.

Repeated database-pool collection failures emit at most one warning per minute.
The first successful collection after degradation emits one recovery event.

`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` enables trace export only. Set the generic
`OTEL_EXPORTER_OTLP_ENDPOINT` separately when metrics must also be exported.

## Ownership

| Area | Owner |
| --- | --- |
| Canonical schema, request and job output, process datasets, exporter privacy | `platform_observability` |
| Medication attempt, persistence, commit, rollback, and blocking | `medication_workflows` |
| Reminder, missed-dose, low-stock, and push stages | `medication_notifications` |
| Rate limiting and security diagnostics | `application_security` |
| NHS, catalogue, barcode, mail, and external adapter diagnostics | `data_integrations` |
| Audit backlog signals | `audit_security` |

The signal registry is authoritative for individual paths and named follow-up
changes. Out-of-baseline discoveries receive a bounded follow-up with an owner;
they do not expand this implementation indefinitely.

## Rollback and compatibility

Keep the previously deployed immutable image available throughout production
acceptance. Rollback restores that image and its request-log shape; it does not
change the `ActiveSupport::Notifications` domain-event contract. During the
compatibility window, saved queries must support both the previous request
fields and the canonical `event.name`, `event.dataset`, `event.id`,
`service.version`, and correlation fields.

Do not remove the previous-image reference or compatibility queries until the
exact new image passes the production matrix. If parser, privacy, severity,
trace-retention, deployment-identity, or producer-count thresholds fail, stop
the canary, preserve its opaque identifiers, and roll back the workload.

## Verification gates

Pre-deployment verification has three layers:

1. Test-mode contracts validate schema, registry coverage, privacy,
   correlation, transaction outcomes, retry semantics, and failure isolation.
2. The final production-image characterization boots the real server, worker,
   logger, process formatters, and OTLP exporter without a source bind mount.
3. Static checks run RuboCop, the full test suite, documentation build,
   `git diff --check`, and strict OpenSpec validation.

Passing these gates permits deployment but does not complete the OpenSpec change.
Production acceptance starts only when every target process reports the
exact immutable image digest. Safe canaries are polled for at most fifteen
minutes. Acceptance requires the request, application event, job, process log,
metric, and sampled trace; zero parser, privacy, schema, severity, and
deployment-identity failures; and reported, deduplicatable ingestion retries.

After the canary passes, record naturally occurring safe workflow evidence for
a fixed twenty-four-hour window. Do not induce medication history or test
notifications in production. Unsafe or absent outcomes are marked not
applicable with a reason and final-image contract reference. The change remains
active and unarchived until this matrix passes.

## Deployed canary procedure

The query catalogue is
`config/observability/production_acceptance_queries.yml`. It is scoped to the
`home` namespace, the production and canary MedTracker apps, and either a
fifteen-minute canary window or the fixed twenty-four-hour evidence window.

Before emitting anything, record the expected application version and immutable
container digest for every web and worker pod. Confirm that a Tempo-compatible
trace datasource is connected; absence of the required trace datasource blocks
acceptance.

Generate a UUID locally for `X-Request-ID`, then make one safe request to the
login page:

```console
curl --fail --silent --show-error \
  -H "X-Request-ID: $REQUEST_ID" \
  https://medtracker.damacus.io/login >/dev/null
```

Run the application-owned canary once in one pod:

```console
kubectl -n home exec deploy/med-tracker -c app -- \
  bin/rails observability:canary
```

The task creates a new opaque workflow, enqueues one no-argument job, and
flushes the remaining enqueue trace before the short-lived command exits. The
job inherits the workflow and emits both `application_event` and `job` canaries
inside always-retained `observability.canary` spans, with distinct attempts.
The application-event and job canaries increment the bounded metric. It reads
and changes no account or medication data.

Use the catalogue queries with the emitted event, workflow, request, job, and
trace identifiers. Poll only until all expected signals appear or fifteen
minutes elapse. The duplicate query may return rows, but every row must be
explainable as repeated ingestion of one stable event identifier. Parser
failure, missing severity, mismatched application version or image digest, a
missing canary metric, or an absent sampled trace is a failed gate.
