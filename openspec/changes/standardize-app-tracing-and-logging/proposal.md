## Why

Production investigations cannot reliably connect a user-visible outcome to the request, job, domain decision, and external delivery that produced it. The missed-dose incident exposed this gap, while [issue #1707](https://github.com/damacus/med-tracker/issues/1707) demonstrates that the current observability pipeline can also generate high-volume noise that obscures useful signals. A production check also confirmed that custom application events can be published successfully without any production subscriber, logger, metric exporter, or trace adapter preserving them.

## What Changes

- Characterize the production logging pipeline so Lograge, the Rails/ECS logger, Active Support publishers and subscribers, OpenTelemetry, Thruster, Puma, Solid Queue, and log collection have explicit, tested responsibilities.
- Replace incompatible and nested log shapes with one queryable application-event envelope and an explicit request-completion contract.
- Preserve application events across requests, jobs, service objects, subscribers, retries, and failure paths while the operational output path is available, using opaque workflow, event, causation, and attempt identifiers alongside available request, job, trace, and span context.
- Freeze a finite implementation baseline covering the production stdout/stderr producers, the eight existing custom events, existing direct logger call sites, and the explicitly named workflows; require every baseline signal to have a production-visible, privacy-safe operational disposition.
- Instrument the bounded workflow set: HTTP requests, medication administration, scheduled and missed-dose reminders, low-stock notifications, push delivery, rate limiting, and audit-backlog monitoring.
- Make attempts, provisional persistence, commit, rollback, subscriber outcomes, retries, and failures observable without reporting an uncommitted side effect as successful.
- Keep logging fail-open so an observability failure cannot block or roll back a medication or health-data operation.
- Define severity, event naming, outcome, error, sampling, redaction, cardinality, and retention expectations suitable for health data.
- Preserve deployment-generated `host.name` as the sole pod-correlation resource attribute needed for trace-to-Loki navigation while continuing to remove process, client, network, and unapproved runtime resource data.
- Separate test-mode contracts, final production-image checks, and post-deployment acceptance; the change is not complete until the exact deployed revision passes production ingestion checks.
- Document migration and rollback criteria for retaining, reconfiguring, or removing Lograge based on evidence.

Explicit non-goals:

- Logging medication names, dose values, health details, request bodies, credentials, tokens, cookies, or raw identifiers.
- Replacing the immutable audit trail with operational logs.
- Logging every method call, SQL statement, successful health check, or framework lifecycle event.
- Adding newly discovered workflow instrumentation outside the frozen baseline; those gaps become follow-up changes rather than expanding this change.
- Replacing Active Support notifications as the domain-event mechanism.
- Rebuilding Grafana, Loki, or the OpenTelemetry backend as part of the application change.
- Treating logs or traces as a clinically reliable record of medication administration.

## Capabilities

### New Capabilities

- `application-observability`: Defines reliable, correlated, privacy-safe application events and traces across HTTP requests, jobs, domain decisions, and external effects.

### Modified Capabilities

None.

## Impact

- Affects production logger and Lograge configuration, production stdout/stderr producers, OpenTelemetry correlation, the eight existing Active Support notification event types, existing direct application logger calls, background-job context, and the bounded medication, notification, rate-limit, and audit-backlog workflows.
- Adds production-like observability contract specs and operational verification documentation.
- Adds an event and logging registry whose coverage tests prevent newly introduced silent publishers, subscribers, or logger paths.
- Requires a follow-up note in ADR 0004 clarifying that Active Support remains the domain-event mechanism while the new boundary owns privacy-safe operational signals.
- May remove Lograge or change its formatter/logger integration if characterization proves it duplicates, suppresses, or double-encodes events.
- Changes the shape of application logs consumed through Loki; migration must preserve a bounded compatibility window or update saved queries at the same time.
