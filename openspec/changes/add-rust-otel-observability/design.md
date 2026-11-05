# Design

## Context

The Rust application (Loco 1.2, SeaORM, PostgreSQL) replaced the archived Rails
application. Its logs are Loco's compact ANSI subscriber output, which Loki
cannot parse; `src/request_trace.rs` emits a UUID `x-medtracker-trace-id` that
is not a W3C trace ID; there are no traces or metrics. The home-ops cluster
runs Loki 3, Vector, VictoriaMetrics and Grafana; Tempo was retired on
2026-10-22 and no trace store is running. The approved plan is
`plan-c56595b5c01c6b7b`; owner decisions recorded there are settled.

## Goals / Non-Goals

Diagnose a production outage, error or failed import from Grafana using logs,
traces, metrics and alerts, without slowing production or tests and without
telemetry output that must later be ignored. Speed budget (measured by a
manual task, not CI): p95 request latency rises at most max(1 ms, 5%) with a
local export sink; nextest wall time within 3% of baseline with telemetry off.

Non-goals: OTLP log export, a collector, per-query spans, a signal registry,
production `home` rollout, and live cluster actions.

## Decisions

### Telemetry module (`src/telemetry/`)

One module owns configuration, subscriber initialisation, the JSON log layer,
the generic key rule, the sampler, the span-exporting wrapper, self-diagnostics
and shutdown. `Hooks::init_logger` builds the registry with Loco's env filter
and the JSON layer and returns `Ok(true)`; `Hooks::on_shutdown` flushes with a
2-second bound. When no OTLP endpoint is configured, no OTel SDK objects are
created, there is no network activity, and exactly one `info` line at startup
states export is disabled. When enabled, the startup line names the endpoint
host only, never headers. Settings live under `settings.telemetry` declared via
`get_env` in `config/*.yaml`; `std::env` is never read in `src/`.

Flush and shutdown wait at most two seconds in total for bounded detached
lifecycle work. Calls for each provider are serialised, overlapping requests
are coalesced and pending shutdown is retained. A stalled provider must not
prevent the other provider from flushing. Lifecycle work runs outside Tokio's
blocking pool so runtime teardown does not wait for a timed-out SDK call.
SDK calls cannot be cancelled: outstanding work may continue until completion
or process exit, and telemetry still pending at process exit may be lost.
Process-level tests cover stalled flush and shutdown, including runtime teardown.

### Log format

One JSON object per line on stdout, no ANSI, following the OTel log data
model: `timestamp` (RFC 3339 UTC microseconds), `severity_text`,
`severity_number` (TRACE=1, DEBUG=5, INFO=9, WARN=13, ERROR=17), `body`,
`trace_id`/`span_id` when a valid context exists, `resource`
(`service.name=medtracker`, `service.version`, `deployment.environment.name`),
`scope.name` (tracing target), and `attributes` merging the current span
chain's fields under the event's own fields (innermost wins). When export is
disabled, valid W3C-format trace IDs are still generated so Loki records stay
correlatable.

### Generic key rule

Allowed attribute keys are a fixed code-level set of OTel semantic-convention
keys plus any `medtracker.*` key. String values are capped at 256 bytes.
Disallowed keys are dropped and their names listed in
`medtracker.telemetry.dropped_keys`. The same rule applies to log attributes
and to exported span attributes, events, links, resource and status via an
exporter wrapper (the Rails `AllowlistedSpanExporter` equivalent). There is no
signal registry: coverage comes from the generic rule plus sentinel privacy
tests, not a frozen event inventory. JSON bodies use static tracing callsite metadata; formatted message prose is omitted for application and dependency events. Exported span-event names use a fixed redaction value because their provenance cannot distinguish literal text from formatted values. Reviewed structured event.name fields retain fixed diagnostic identifiers. This deliberately trades free-form prose for privacy.

### Traces

Root sampler is `TraceIdRatioBased(trace_sample_ratio)` (default 1.0) that
ignores the parent's sampled flag; the TraceContext propagator joins a valid
inbound `traceparent` trace ID. The request server span
(`"{METHOD} {route}"`) carries `http.request.method`, `http.route` (matched
template or `unmatched`), `http.response.status_code`, `error.type` on 5xx,
and error status on 5xx only. `x-medtracker-trace-id` returns the 32-hex trace
ID. Worker jobs get `job {name}` spans, scheduler children `task {name}` root
spans, and a panic hook logs `error.type="panic"` with `code.*` location and
never the payload message. Database work is accounted on the unit-of-work span
via SeaORM `set_metric_callback` (`medtracker.db.query_count`,
`medtracker.db.runtime_ms`, `medtracker.db.failed_query_count`); no per-query
spans. Outbound HTTP (NHS terminology, web push, S3/OpenDAL, AI help) gets
client spans with host, method, status and duration only.

### Metrics

When an endpoint is configured, a 60-second periodic reader exports OTLP to
VictoriaMetrics: `http.server.request.duration`,
`http.client.request.duration`, `db.client.connection.*` (via the SQLx pool
behind SeaORM — documented exception, SeaORM exposes no pool statistics),
`medtracker.job.duration`, `medtracker.import.runs`, and
`medtracker.queue.jobs`/`medtracker.queue.oldest_queued_age_s` read from
`pg_loco_queue` in a worker-only background task with cached atomics.

Only the explicit `settings.telemetry.otlp_metrics_endpoint` enables the meter
provider. The trace endpoint and generic endpoint do not enable metrics. A
metrics-only configuration works without a tracer provider. Durations use
seconds; import counts represent terminal attempts rather than individual
stage events. Labels use bounded methods, route templates, status and outcome
values without identifiers, concrete paths or clinical data.

Connection observations refer to the running application's pool, rather than
the separate context Loco creates to initialise logging. SeaORM 2.0.4 exposes
`get_postgres_connection_pool` but no pool-size or idle-count abstraction.
The narrow metadata exception reads only the exposed pool's size, idle count
and configured maximum; queries continue through SeaORM.

Queue job counts cover the fixed supported states, including scheduled work;
unrecognised framework states contribute to a fixed `other` bucket.
Waiting time starts when a queued job is ready to run (`run_at` is due), so
future jobs do not appear overdue. Collection callbacks only read cached
atomics. Each refresh has a five-second deadline. A failed refresh retains the
last successful snapshot, emits at most one warning per minute and emits one
recovery record after success. The
application context owns and stops the worker refresh task without retaining
an ownership cycle through the task or SDK callbacks.

Trace and metric exporter diagnostics have independent failure/recovery state.
A successful trace export cannot report recovery of a failing metric exporter.

### Self-diagnostics

OTel SDK internal diagnostics are routed into tracing at `warn`, rate-limited:
first export failure emits one `warn`
(`event.name=medtracker.telemetry.export_failed`, `error.type`), then at most
one per 5 minutes with a suppressed count, and one `info`
`medtracker.telemetry.export_recovered` on recovery. The exporter's own HTTP
client generates no spans or logs (no recursion). An unreachable endpoint at
startup is not an error. Every telemetry write is fail-open.

### Imports and API contract

NHS dm+d imports run under an `nhs_dmd.import` span with stage events and a
typed `ImportFailure { stage, category }` (`archive_invalid`, `xml_invalid`,
`storage`, `database`, `timeout`, `cancelled`, `panicked`, `stale`) whose
stored `error_message` is a fixed safe summary. A nullable `trace_id` column
on `nhs_dmd_imports` is set at run start and shown on the failed-import page.
Portable care imports and the `nhs_dmd_reconcile` task follow the same pattern.
API error bodies gain `trace_id` (32 lowercase hex); `x-medtracker-trace-id`
now carries the W3C trace ID (ADR 0020).

### Domain events

Named log records (also span events) with `event.name`: medication dose
attempted/committed/rolled_back/blocked/replayed, reminder scheduled/
suppressed/sent/provider_failed, mail outbox enqueued/delivered/retried/failed,
and auth/rate-limit/support-access events. Each carries `medtracker.outcome`
and optional `error.type`; `committed` is emitted only after the outermost
commit succeeds. No clinical values or person/medication identifiers.

### Build identity

`Dockerfile` build args `MEDTRACKER_VERSION`/`MEDTRACKER_COMMIT` are passed by
`.github/workflows/loco-images.yml`; `build.rs` prefers them over git;
`service.version` and `App::app_version()` use them.

## Risks / Trade-offs

- VictoriaTraces is pre-1.0 and Grafana Drilldown support is experimental;
  mitigated by the Tempo-type data source and config-only export disablement.
- 100% sampling volume is acceptable at household traffic; the ratio override
  exists.
- The generic key rule may drop fields developers expect; `dropped_keys`
  surfaces this.
- OTel Rust crate APIs churn between minor versions; exact pins and the
  isolated module limit churn.
- SQLx pool-stat access is a documented SeaORM exception.
