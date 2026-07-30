## 1. Characterize the Production Pipeline

- [ ] 1.1 Add failing production-equivalent contract specs that capture direct info, warning, and error events, an Active Support subscriber event, and request and Active Job lifecycle events.
- [ ] 1.2 Extend the failing contract specs to identify duplicate events, nested JSON messages, missing severity, missing correlation, and unintended subscriber suppression.
- [ ] 1.3 Add a failing discovery spec that inventories every application-owned Active Support publisher and subscriber, direct logger call, application event, custom metric, trace annotation, and operational job entrypoint.
- [ ] 1.4 Record the observed ownership and final shape of Rails, Lograge, proxy or supervisor, job, subscriber, custom event, metric, direct logger, and OpenTelemetry output in the signal registry.
- [ ] 1.5 Baseline the eight current custom event types and prove which production subscribers and sinks are absent without treating temporary test subscribers as production coverage.

## 2. Define the Canonical Application Event

- [ ] 2.1 Add failing unit specs for the canonical ECS-compatible field schema, registered event names, allowed value types, bounded cardinality, and schema version.
- [ ] 2.2 Add failing privacy specs using synthetic marker values for health data, credentials, tokens, cookies, raw domain identifiers, schedule timestamps, and unsafe exception text.
- [ ] 2.3 Implement the application-event boundary and serializer with stable event names, severity, outcome, safe reasons, service metadata, and optional human-readable messages.
- [ ] 2.4 Add request, job, trace, and span context enrichment that omits unavailable identifiers and preserves events when traces are unsampled.
- [ ] 2.5 Add failing registry specs requiring every discovered custom event to declare its payload contract, subscribers, transaction timing, failure policy, safe mapping, production sink, owner, and verification.
- [ ] 2.6 Add production observers and event-specific safe adapters for every registered Active Support notification, plus current-span annotations where appropriate, without serializing raw payloads or duplicating events.
- [ ] 2.7 Add failing coverage for new unregistered publishers, subscribers, and direct logger calls so the contract cannot silently regress.
- [ ] 2.8 Refactor the event boundary and registry while keeping all focused schema, privacy, correlation, discovery, and exact-count specs green.

## 3. Normalize HTTP Request Logging

- [ ] 3.1 Add a failing production-equivalent request spec requiring exactly one parseable Rails completion event with route template, status, duration, severity, request identifier, and available trace context.
- [ ] 3.2 Reconfigure, replace, or remove Lograge according to the characterization evidence so Rails emits the canonical request event without nested JSON or suppressed application events.
- [ ] 3.3 Remove the unused `keep_appenders` configuration and document the tested behavior of any retained Lograge settings.
- [ ] 3.4 Give proxy or supervisor access events a distinct documented dataset and verify they are distinguishable from Rails request events.
- [ ] 3.5 Suppress or sample routine successful health-check events at each controlled layer while retaining failures.
- [ ] 3.6 Run the focused production-equivalent logging contract and request specs and confirm exact event counts and parseable fields.

## 4. Close Event Pipeline and Critical Workflow Visibility Gaps

- [ ] 4.1 Complete the exhaustive signal registry across the repository and separately complete the workflow matrix for medication administration, reminders, low stock, authentication, rate limiting, audit delivery, external lookups, mail, push, scheduled imports, and every additional operational workflow discovered.
- [ ] 4.2 Add failing production-observer specs for all current medication, low-stock, audit-backlog, and rate-limit events, then connect each to a canonical privacy-safe sink.
- [ ] 4.3 Add failing subscriber-pipeline specs for dispatch success, subscriber exception propagation or isolation, retries, and correlated stage outcomes.
- [ ] 4.4 Add failing outer-transaction specs proving provisional persistence is not reported as committed success and rollback produces no committed event.
- [ ] 4.5 Add failing outcome-event specs for medication recording attempt, commit, blocking, rejection, rollback, and unexpected failure, then migrate its legacy free-text logs.
- [ ] 4.6 Add failing outcome-event specs for reminder scheduling, eligibility, suppression, deduplication, retry, send, and delivery failure, then instrument the missing boundaries.
- [ ] 4.7 Add failing outcome-event specs for low-stock decisions and delivery, then instrument the missing boundaries.
- [ ] 4.8 Add failing PHI-safe outcome-event specs for authentication, token exchange, rate limiting, audit delivery, and audit backlog health, then instrument the missing boundaries.
- [ ] 4.9 Add failing outcome-event specs for external medication, identity, mail, and push operations, then migrate legacy logs without recording bodies or upstream health data.
- [ ] 4.10 Add or update exact-count specs for retry and idempotency paths so retries remain visible without reporting one side effect as multiple successful outcomes.
- [ ] 4.11 Review the migrated workflows for transaction rollback, authorization, cross-household isolation, subscriber failure policy, and accidental audit-log replacement.

## 5. Align Trace Coverage and Privacy

- [ ] 5.1 Add failing sampler specs for every critical medication, reminder, notification, authentication, and audit path identified by the coverage matrix.
- [ ] 5.2 Extend critical-path matching and job trace naming or linking only where the failing coverage specs prove a gap.
- [ ] 5.3 Add failing span-attribute privacy specs for each newly traced workflow and update the allowlist processor with bounded, non-sensitive attributes.
- [ ] 5.4 Verify that unsampled failures still emit correlated structured error events and document the head-sampling limitation.
- [ ] 5.5 Run the focused OpenTelemetry integration, active-job continuity, sampler, exporter-allowlist, and application-event specs.

## 6. Operational Verification and Documentation

- [ ] 6.1 Document the application-event schema, severity policy, event naming rules, privacy allowlist, correlation model, and ownership of request, job, proxy, log, trace, metric, and audit signals.
- [ ] 6.2 Document the exhaustive signal registry, workflow matrix, production-sink rule, subscriber failure policies, and transaction outcome semantics.
- [ ] 6.3 Add a synthetic, non-mutating smoke workflow that emits safe named events without creating medication history or sending real notifications.
- [ ] 6.4 Document Loki queries for event name, dispatch and subscriber stage, severity, request identifier, job identifier, trace identifier, missing fields, nested JSON, and duplicate counts.
- [ ] 6.5 Add post-deployment acceptance steps proving each registered custom-event family survives ingestion independently of its HTTP request and that important saved queries work with the new schema.
- [ ] 6.6 Document the request-logger rollback switch and the compatibility window for legacy queries.

## 7. Quality Gates and Deployment Acceptance

- [ ] 7.1 Run focused specs after each Red-Green-Refactor slice using the repository task wrapper.
- [ ] 7.2 Run `task rubocop` and `task test` and fix every regression before publication.
- [ ] 7.3 Deploy with the previous request logger available as a rollback path and run the synthetic Loki acceptance checks.
- [ ] 7.4 Compare event volume, duplicate counts, unknown-severity counts, registry coverage, subscriber failures, transaction outcomes, and critical-workflow coverage before and after deployment.
- [ ] 7.5 Remove compatibility output only after production verification is green and the operational queries have been updated.
