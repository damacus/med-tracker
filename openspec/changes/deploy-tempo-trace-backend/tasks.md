## 1. Freeze the `home-ops` Contract

Implementation tasks 1.1 through 3.4 landed in `damacus/home-ops` PR #3974 at merge commit `517a51f668425115f89be9be98961005b4a99011`. This MedTracker change remains the single planning authority; runtime work stays in the deployment repository under that repository's instructions.

- [x] 1.1 Start from a fresh writable deployment-repository worktree while retaining the planning ledger here, and record the deployment revision plus a PHI-safe baseline of Tempo, OTLP services, and the Grafana trace data source before implementation.
- [x] 1.2 Add a failing focused repository task that asserts the pinned Tempo source, private ports, dedicated secret and bucket, 14-day retention, NetworkPolicy peers, Grafana data-source UIDs and correlations, canary OTLP settings, and production's disabled exporter.
- [x] 1.3 Confirm the current official monolithic Tempo chart and major-version values schema, then pin the compatible chart and image through the repository's dependency-management conventions.

## 2. Deploy the Private Trace Backend

- [x] 2.1 Extend the RustFS provisioning and ExternalSecret contracts with a dedicated `tempo-traces` bucket and least-privilege Tempo identity, keeping every credential out of Git and rendered verification output.
- [x] 2.2 Add the monitoring Tempo Flux resources with one monolithic replica, OTLP/HTTP on cluster port 4318, query API on cluster port 3200, explicit health probes and resources, RustFS trace storage, and 14-day retention.
- [x] 2.3 Add NetworkPolicy that permits ingestion only from the selected MedTracker lane and queries only from Grafana and bounded operator diagnostics, and prove that no Ingress, HTTPRoute, LoadBalancer, or external tunnel exposes Tempo.
- [x] 2.4 Add Tempo metric discovery and only the bounded workload, durable-storage, sustained-rejection, restart, and resource-pressure alerts supported by the deployed version; make the focused backend contract green.

## 3. Connect Grafana and MedTracker Canary

- [x] 3.1 Add a failing Grafana provisioning check for stable Tempo UID `tempo`, the internal query URL, TraceQL search, trace-to-Loki correlation, and Loki `trace.id` derived links.
- [x] 3.2 Provision the Tempo data source and bidirectional Loki correlation without changing Prometheus as Grafana's default or enabling service graphs and span-derived metrics.
- [x] 3.3 Configure only MedTracker canary with secret-backed `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`, `OTEL_TRACES_EXPORTER=otlp`, and `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`; assert production remains `OTEL_TRACES_EXPORTER=none`.
- [x] 3.4 Run the focused observability task, secret and policy checks, repository YAML/schema validation, Flux rendering and diff, and the normal `home-ops` quality gates until they pass.

## 4. Prove, Promote, and Stop

- [ ] 4.1 After the current pause/migration-mount stack, reconcile the final cumulative RustFS, Tempo, monitoring, Grafana, and canary configuration in dependency order; verify pinned workload versions, healthy storage, a healthy Grafana data source, scrape targets, alert state, and cluster-only endpoints before acceptance.
- [ ] 4.2 Deploy the exact approved MedTracker canary digest and re-prove within 15 minutes that its safe observability trace is searchable by trace ID and bounded TraceQL, displays the expected parent-child spans, links bidirectionally with Loki, and passes the value-aware gate for prohibited data.
- [ ] 4.3 Exercise a controlled canary-only exporter outage against the final cumulative configuration, prove application requests and the safe operation remain available, restore the tested endpoint, and record the successful rollback without exporting sensitive evidence.
- [ ] 4.4 Collect the bounded 24-hour naturally occurring evidence required by `standardize-app-tracing-and-logging` from the final cumulative configuration; treat absent unsafe workflows as not applicable with final-image contract references rather than generating health-data actions.
- [ ] 4.5 Promote the same endpoint, protocol, privacy, and fail-open contract to the exact production digest only after tasks 4.1 through 4.4 pass, rerun the finite production checks, record immutable revisions and safe query evidence, and stop implementation; file any broader collector, HA, service-graph, multi-application, capacity, TLS, or encryption work as separate changes.

## 5. Sync and Archive

- [ ] 5.1 After production acceptance is complete, use the final PR for this OpenSpec change to sync the delta into the main `cluster-trace-backend` spec and archive the change.
