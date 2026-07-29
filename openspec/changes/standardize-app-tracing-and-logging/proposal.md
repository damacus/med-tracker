## Why

Production investigations cannot reliably connect a user-visible outcome to the request, job, domain decision, and external delivery that produced it. The missed-dose incident exposed this gap, while [issue #1707](https://github.com/damacus/med-tracker/issues/1707) demonstrates that the current observability pipeline can also generate high-volume noise that obscures useful signals.

## What Changes

- Characterize the production logging pipeline so Lograge, the Rails logger, Active Support subscribers, OpenTelemetry, the process supervisor, and log collection have explicit, tested responsibilities.
- Replace incompatible and nested log shapes with one queryable application-event envelope and an explicit request-completion contract.
- Preserve application events across requests, jobs, service objects, subscribers, retries, and failure paths, with request, job, trace, and span correlation where those contexts exist.
- Inventory safety-critical and operational workflows, then add outcome events at currently silent decision boundaries rather than logging internal implementation steps.
- Define severity, event naming, outcome, error, sampling, redaction, cardinality, and retention expectations suitable for health data.
- Add production-like characterization tests and post-deployment Loki queries that prove important events survive formatting, collection, and ingestion.
- Document migration and rollback criteria for retaining, reconfiguring, or removing Lograge based on evidence.

Explicit non-goals:

- Logging medication names, dose values, health details, request bodies, credentials, tokens, cookies, or raw identifiers.
- Replacing the immutable audit trail with operational logs.
- Logging every method call, SQL statement, successful health check, or framework lifecycle event.
- Rebuilding Grafana, Loki, or the OpenTelemetry backend as part of the application change.
- Treating logs or traces as a clinically reliable record of medication administration.

## Capabilities

### New Capabilities

- `application-observability`: Defines reliable, correlated, privacy-safe application events and traces across HTTP requests, jobs, domain decisions, and external effects.

### Modified Capabilities

None.

## Impact

- Affects production logger and Lograge configuration, OpenTelemetry correlation, Active Support notification subscribers, background-job context, and selected safety-critical services and jobs.
- Adds production-like observability contract specs and operational verification documentation.
- May remove Lograge or change its formatter/logger integration if characterization proves it duplicates, suppresses, or double-encodes events.
- Changes the shape of application logs consumed through Loki; migration must preserve a bounded compatibility window or update saved queries at the same time.
