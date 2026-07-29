## 1. Characterize the Production Pipeline

- [ ] 1.1 Add failing production-equivalent contract specs that capture direct info, warning, and error events, an Active Support subscriber event, and request and Active Job lifecycle events.
- [ ] 1.2 Extend the failing contract specs to identify duplicate events, nested JSON messages, missing severity, missing correlation, and unintended subscriber suppression.
- [ ] 1.3 Record the observed ownership and final shape of Rails, Lograge, proxy or supervisor, job, subscriber, and OpenTelemetry output in an observability coverage matrix.

## 2. Define the Canonical Application Event

- [ ] 2.1 Add failing unit specs for the canonical ECS-compatible field schema, registered event names, allowed value types, bounded cardinality, and schema version.
- [ ] 2.2 Add failing privacy specs using synthetic marker values for health data, credentials, tokens, cookies, raw domain identifiers, schedule timestamps, and unsafe exception text.
- [ ] 2.3 Implement the application-event boundary and serializer with stable event names, severity, outcome, safe reasons, service metadata, and optional human-readable messages.
- [ ] 2.4 Add request, job, trace, and span context enrichment that omits unavailable identifiers and preserves events when traces are unsampled.
- [ ] 2.5 Add a centralized adapter for selected Active Support notifications and current-span annotations without duplicating emitted events.
- [ ] 2.6 Refactor the event boundary while keeping all focused schema, privacy, correlation, and exact-count specs green.

## 3. Normalize HTTP Request Logging

- [ ] 3.1 Add a failing production-equivalent request spec requiring exactly one parseable Rails completion event with route template, status, duration, severity, request identifier, and available trace context.
- [ ] 3.2 Reconfigure, replace, or remove Lograge according to the characterization evidence so Rails emits the canonical request event without nested JSON or suppressed application events.
- [ ] 3.3 Remove the unused `keep_appenders` configuration and document the tested behavior of any retained Lograge settings.
- [ ] 3.4 Give proxy or supervisor access events a distinct documented dataset and verify they are distinguishable from Rails request events.
- [ ] 3.5 Suppress or sample routine successful health-check events at each controlled layer while retaining failures.
- [ ] 3.6 Run the focused production-equivalent logging contract and request specs and confirm exact event counts and parseable fields.

## 4. Close Critical Workflow Visibility Gaps

- [ ] 4.1 Complete the observability coverage matrix for medication administration, reminders, low stock, authentication, rate limiting, audit delivery, external lookups, mail, push, and scheduled imports.
- [ ] 4.2 Add failing outcome-event specs for medication recording success, blocking, rejection, rollback, and unexpected failure, then migrate its legacy free-text logs.
- [ ] 4.3 Add failing outcome-event specs for reminder scheduling, eligibility, suppression, deduplication, retry, send, and delivery failure, then instrument the missing boundaries.
- [ ] 4.4 Add failing outcome-event specs for low-stock decisions and delivery, then instrument the missing boundaries.
- [ ] 4.5 Add failing PHI-safe outcome-event specs for authentication, token exchange, rate limiting, audit delivery, and audit backlog health, then instrument the missing boundaries.
- [ ] 4.6 Add failing outcome-event specs for external medication, identity, mail, and push operations, then migrate legacy logs without recording bodies or upstream health data.
- [ ] 4.7 Add or update exact-count specs for retry and idempotency paths so retries remain visible without reporting one side effect as multiple successful outcomes.
- [ ] 4.8 Review the migrated workflows for transaction rollback, authorization, cross-household isolation, and accidental audit-log replacement.

## 5. Align Trace Coverage and Privacy

- [ ] 5.1 Add failing sampler specs for every critical medication, reminder, notification, authentication, and audit path identified by the coverage matrix.
- [ ] 5.2 Extend critical-path matching and job trace naming or linking only where the failing coverage specs prove a gap.
- [ ] 5.3 Add failing span-attribute privacy specs for each newly traced workflow and update the allowlist processor with bounded, non-sensitive attributes.
- [ ] 5.4 Verify that unsampled failures still emit correlated structured error events and document the head-sampling limitation.
- [ ] 5.5 Run the focused OpenTelemetry integration, active-job continuity, sampler, exporter-allowlist, and application-event specs.

## 6. Operational Verification and Documentation

- [ ] 6.1 Document the application-event schema, severity policy, event naming rules, privacy allowlist, correlation model, and ownership of request, job, proxy, log, trace, metric, and audit signals.
- [ ] 6.2 Add a synthetic, non-mutating smoke workflow that emits safe named events without creating medication history or sending real notifications.
- [ ] 6.3 Document Loki queries for event name, severity, request identifier, job identifier, trace identifier, missing fields, nested JSON, and duplicate counts.
- [ ] 6.4 Add post-deployment acceptance steps that prove synthetic events survive ingestion and that important saved queries work with the new schema.
- [ ] 6.5 Document the request-logger rollback switch and the compatibility window for legacy queries.

## 7. Quality Gates and Deployment Acceptance

- [ ] 7.1 Run focused specs after each Red-Green-Refactor slice using the repository task wrapper.
- [ ] 7.2 Run `task rubocop` and `task test` and fix every regression before publication.
- [ ] 7.3 Deploy with the previous request logger available as a rollback path and run the synthetic Loki acceptance checks.
- [ ] 7.4 Compare event volume, duplicate counts, unknown-severity counts, and critical-workflow coverage before and after deployment.
- [ ] 7.5 Remove compatibility output only after production verification is green and the operational queries have been updated.
