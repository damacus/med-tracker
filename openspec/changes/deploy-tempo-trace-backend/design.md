## Context

See [proposal.md](proposal.md) for motivation. The private Tempo backend, least-privilege RustFS storage, retention, network policy, alerts, Grafana correlation, canary exporter wiring, and focused validation planned here landed in `damacus/home-ops` PR #3974 at merge commit `517a51f668425115f89be9be98961005b4a99011`. This merged implementation boundary does not establish current operational acceptance: the final cumulative canary configuration after the current pause/migration-mount stack still requires the value-aware privacy gate, outage rollback, 24-hour evidence, and production promotion.

This MedTracker OpenSpec change is the single planning authority. The `damacus/home-ops` deployment repository owns runtime implementation and reconciliation under its own instructions; these planning artifacts remain here and are not copied into the deployment repository.

Grafana documents a monolithic Kubernetes deployment as a supported Helm path and notes that Tempo has no built-in authentication layer. Grafana also supports provisioning Tempo as a built-in data source at the backend query URL and configuring trace-to-log correlation. The design therefore keeps ingestion and query services cluster-internal and enforces access at the network boundary.

## Goals / Non-Goals

**Goals:**

- Add the smallest production-appropriate trace backend that satisfies MedTracker's production acceptance contract.
- Reuse existing Flux, HelmRelease, RustFS, Grafana, Loki, VictoriaMetrics, ExternalSecret, and network-policy patterns.
- Make the canary rollout finite, evidence-based, and reversible.
- Preserve a clear migration path to a collector or distributed backend without introducing either now.

**Non-Goals:**

- Provide a shared trace gateway for every cluster workload.
- Add span-derived metrics, service graphs, tail sampling, or multi-tenancy.
- Make a single-binary deployment highly available.
- Change application instrumentation or use traces as an audit record.
- Expand this change into broader TLS or encryption hardening, which remains separately owned follow-up.

## Decisions

### 1. Keep planning in MedTracker and runtime implementation in `home-ops`

This MedTracker change remains the sole planning authority. Runtime manifests, secret references, bucket provisioning, Grafana configuration, and rollout controls belong to `damacus/home-ops` and must be changed under that repository's instructions without copying this OpenSpec ledger there. The application repository supplies the emitter and acceptance canary but does not own the backend runtime.

Alternative considered: implement Tempo in MedTracker Compose. Rejected because it would not prove the production transport and would add local services the cluster can already host centrally.

### 2. Use the pinned official monolithic Tempo Helm chart

Deploy one single-binary Tempo replica in the monitoring namespace through the current Flux `OCIRepository` plus `HelmRelease` pattern. Pin the chart and image through the same dependency-update conventions as the rest of `home-ops`. Enable only the components required for OTLP ingestion, durable trace storage, TraceQL query, health, and metrics.

The implementation must verify the selected chart's current values schema and Tempo major-version compatibility before committing rendered values. Tempo 3 monolithic mode remains a single process and does not require Kafka; configuration copied from a Tempo 2 deployment must not be assumed compatible.

Alternatives considered:

- Distributed Tempo: rejected because the present load and availability requirement do not justify Kafka or multiple backend components.
- Tempo Operator: rejected because it adds CRDs and another controller for one small deployment.
- Jaeger: rejected because Grafana and the existing acceptance queries are already designed around Tempo and TraceQL.

### 3. Send OTLP/HTTP directly from MedTracker to Tempo

Expose the OTLP/HTTP receiver on cluster port `4318` and the Tempo query API on cluster port `3200`. MedTracker uses its existing `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` and `OTEL_TRACES_EXPORTER` contract. No collector is introduced in this slice.

A NetworkPolicy permits ingestion from the explicitly selected MedTracker canary and, only after promotion, production workload. Query access is limited to Grafana and bounded operator diagnostics. Neither service receives an HTTPRoute, Ingress, LoadBalancer, or external tunnel.

Alternatives considered:

- Reuse OpenEBS Alloy: rejected because it is scoped to storage telemetry and has no configured OTLP receiver or trace exporter.
- Add a general Alloy or OpenTelemetry Collector: useful later for buffering, central sampling, and multi-backend routing, but outside the minimum backend slice.
- Export spans to stdout and Loki: rejected because logs cannot provide native trace trees, TraceQL search, or backend retention semantics.

### 4. Store traces in a dedicated RustFS bucket for 14 days

Add a `tempo-traces` bucket and dedicated least-privilege RustFS identity through the existing bucket-provisioning and external-secret patterns. Tempo uses the internal S3-compatible endpoint with path-style access. Credentials are injected from a Kubernetes Secret and never embedded in Helm values, rendered validation output, or acceptance evidence.

Set backend trace retention to 14 days, matching the current Loki retention window. Retention and compaction configuration must follow the selected Tempo version's current schema. The bucket survives workload replacement and chart rollback.

Alternatives considered:

- Local filesystem or a Tempo PVC: acceptable for a disposable demo, but rejected because the production acceptance trace must survive pod replacement and the cluster already provides object storage.
- Reuse Loki buckets or credentials: rejected because it violates least privilege and couples independent retention lifecycles.

### 5. Provision bidirectional Grafana correlation

Add a provisioned Tempo data source with stable UID `tempo` and query URL `http://tempo.monitoring.svc.cluster.local:3200`. Keep Prometheus as the default data source.

Configure:

- Trace-ID lookup and TraceQL search.
- Trace-to-logs using Loki UID `loki`, a narrow time window, MedTracker service identity, the trace identifier, and `host.name` mapped to the Kubernetes pod label.
- A Loki derived field that recognizes the canonical `trace.id` field and opens the matching Tempo trace.

Do not enable service graphs or trace-to-metrics in this change because no span-metrics generator is being deployed.

### 6. Stage exporter enablement through MedTracker canary

Deployment order is backend, storage, monitoring, Grafana data source, then MedTracker canary. Production remains `OTEL_TRACES_EXPORTER=none` until canary acceptance passes.

Canary uses:

- `OTEL_TRACES_EXPORTER=otlp`
- `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`
- The cluster-internal OTLP endpoint through the secret-backed `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` contract
- The exact application image containing the approved tracing implementation

Run the application's safe observability canary and retain only identifiers, image digests, timestamps, queries, counts, and pass/fail results as evidence. Do not record trace payloads containing health data.

Promotion changes only the production exporter configuration after the finite checks pass. Sampling remains owned by the application change.

### 7. Monitor the backend without creating a platform epic

Expose Tempo's own metrics to VictoriaMetrics using the repository's established scrape mechanism. Add only actionable signals needed for this deployment: workload unavailable or restarting, durable-storage failure, sustained rejected spans, and resource or storage pressure supported by available metrics.

Add a focused repository task that renders and asserts the Tempo, RustFS, Grafana, NetworkPolicy, and staged MedTracker contracts. Broader cluster checks remain existing `home-ops` gates.

## Risks / Trade-offs

- [Single replica is a trace-visibility single point of failure] → Keep application export fail-open, persist blocks in RustFS, alert on unavailability, and allow exporter disablement without changing domain behavior.
- [Tempo has no built-in authentication] → Expose no public route and enforce least-privilege NetworkPolicy on ingest and query ports.
- [Direct export has no collector buffer or tail sampling] → Retain application batching and sampling, monitor rejected exports, and defer a collector until measured need justifies it.
- [Trace volume could pressure memory or object storage] → Set explicit requests and limits, keep 14-day retention, monitor usage, and enable only MedTracker after canary measurement.
- [Tempo major versions and Helm values can drift] → Pin chart and image versions, verify the current schema, render in CI, and test upgrade and rollback against retained storage.
- [Trace-log linking can break when field names drift] → Assert the canonical `trace.id`, service identity, and data-source UIDs in focused validation and the deployed canary.
- [Traces may contain health data if application filtering regresses] → Make privacy inspection a promotion gate; disable export immediately if prohibited attributes appear.

## Migration Plan

1. Add the dedicated RustFS identity, bucket, and Kubernetes secret wiring.
2. Add the pinned Tempo source, HelmRelease, internal services, NetworkPolicy, metrics discovery, and bounded alerts.
3. Provision the Grafana Tempo data source and bidirectional Loki correlation.
4. Reconcile and verify backend health, storage access, data-source health, and absence of public routes before enabling an emitter.
5. Configure only MedTracker canary for OTLP/HTTP and deploy the exact approved application digest.
6. Run the synthetic canary checks for at most 15 minutes, then collect the bounded naturally occurring evidence required by the application observability change.
7. Promote the same exporter contract to production only after all gates pass and record the exact digest and configuration revision.

Steps 1 through 5 of this implementation sequence landed through task 3.4 in `damacus/home-ops` PR #3974 at merge commit `517a51f668425115f89be9be98961005b4a99011`. The initial demonstrations are not final acceptance evidence because later canary/reset changes superseded them. After the current pause/migration-mount stack, the final cumulative canary configuration must be re-proven before production promotion.

Rollback begins by restoring `OTEL_TRACES_EXPORTER=none` for the affected MedTracker lane. Backend and Grafana resources may remain available while export is disabled. If the Tempo release itself must be removed, stop exporters first and retain the bucket and credentials until the rollback decision and evidence-retention window are complete.
