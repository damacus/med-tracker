# Proposal

## Why

The Rust application emits Loco's default compact logs with ANSI codes that the
production Vector to Loki path cannot parse, has no trace or metrics export, and
correlates requests with a non-standard UUID header. Operators cannot diagnose a
canary outage, failed NHS dm+d import or scheduled task failure from Grafana.
Originating plan: `plan-c56595b5c01c6b7b` (owner-approved 2026-11-02).

## What Changes

- Emit OTel log-data-model JSON records on stdout, one object per line, shipped
  by the existing Vector to Loki path; no OTLP log export and no extra container.
- Export OTLP/HTTP traces to a single-node VictoriaTraces backend in home-ops
  with 7-day retention, queried through a Grafana Tempo-type data source.
- Export OTLP metrics straight to VictoriaMetrics when configured; no collector.
- Adopt 100% default trace sampling that joins an inbound W3C `traceparent`
  trace ID but ignores the caller's sampled flag.
- Apply a generic privacy key rule (semantic-convention keys plus `medtracker.*`
  only, capped value lengths, dropped key names reported) to logs and exported
  spans instead of a per-event allowlist or signal registry.
- Record database query accounting on unit-of-work spans; no per-query spans.
- Add typed import failure categories and store/display a `trace_id` reference
  for failed imports; add `trace_id` to API error bodies.
- Enable trace export only for the `medtracker-canary` deployment; production
  `home` remains unchanged.

Explicit non-goals: OTLP log export, a collector, per-query database spans, a
signal registry, production `home` rollout, live cluster changes from this
repository, and CI performance assertions.

## Capabilities

### New Capabilities

None. All behaviour lands in existing capabilities.

### Modified Capabilities

- `application-observability`: replace the Rails-era event registry, envelope
  and correlation contract with the OTel log data model, generic key rule,
  W3C tracing, OTLP export and self-diagnostics contract for the Rust
  application.
- `cluster-trace-backend`: replace the retired Tempo backend with single-node
  VictoriaTraces, shorten retention to 7 days, and restrict export enablement
  to `medtracker-canary`.
- `dmd-import-reliability`: add typed failure categories, safe stored
  summaries and a stored, displayed trace reference for failed imports.

## Impact

Adds an isolated `src/telemetry/` module, OTel crates pinned to exact published
versions, `settings.telemetry` configuration, a request server span replacing
`request_trace`, a nullable `trace_id` column on `nhs_dmd_imports`, API error
body and OpenAPI changes, build-identity plumbing, and a home-ops change adding
VictoriaTraces, Grafana wiring, network policies and alerts. Existing request,
job and import behaviour is preserved; telemetry failures are fail-open.
