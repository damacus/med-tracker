# OpenTelemetry Sampling

Production tracing uses a parent-based trace-id ratio sampler wrapped with a critical-path sampler.

## Defaults

- Production baseline: `MEDTRACKER_OTEL_TRACE_SAMPLE_RATE=0.1`
- Non-production baseline: `1.0`
- Critical paths are retained even when the baseline sampler would drop the trace.

Critical defaults cover authentication, support access, audit logs, medication-take writes, offline medication-take sync, and `MedicationTake` model spans.

## Tuning

Set `MEDTRACKER_OTEL_TRACE_SAMPLE_RATE` to a value from `0.0` to `1.0` to adjust non-critical trace volume without changing code.

Set `MEDTRACKER_OTEL_CRITICAL_TRACE_MATCHERS` to comma-separated path or span-name fragments to replace the default critical matcher list.

Use `MEDTRACKER_OTEL_CRITICAL_TRACE_MATCHERS=none` to disable critical matcher retention for emergency volume reduction.

Standard OpenTelemetry sampler settings are still accepted through `OTEL_TRACES_SAMPLER` and `OTEL_TRACES_SAMPLER_ARG`. Critical-path retention wraps those delegate sampler choices unless the critical matcher list is disabled.

## Rollback

For maximum retention during incident review, set `MEDTRACKER_OTEL_TRACE_SAMPLE_RATE=1.0`.

For emergency volume reduction, set `MEDTRACKER_OTEL_TRACE_SAMPLE_RATE=0.0` and `MEDTRACKER_OTEL_CRITICAL_TRACE_MATCHERS=none`.

After changing sampling settings, restart the Rails process and confirm exporter throughput in the collector before applying the same setting to all production instances.
