## 1. Freeze and Characterize the Baseline

- [x] 1.1 Add a registry-census spec that pins the source revision and enumerates Thruster, Rails/ECS with Lograge, Puma, Solid Queue, OpenTelemetry SDK output, the eight custom event types, every direct logger site, and the bounded workflows.
- [x] 1.2 Add failing test-mode characterization specs for request, job, custom-event, warning, error, exception, subscriber-failure, and valid, invalid, and unsampled trace-context output.
- [x] 1.3 Add a failing final production-image characterization task, without a source bind mount, that captures real stdout and exporter traffic from the production logger, request server, job backend, and telemetry configuration.
- [x] 1.4 Record the frozen producers, transports, event mappings, direct logger dispositions, privacy classifications, transaction timing, failure policy, and owners; record any discovery outside that baseline as a named follow-up change.

## 2. Establish the Correlated Fail-Open Boundary

- [x] 2.1 Add failing schema and correlation specs for collision-resistant `event.id` generation and non-reuse, ECS fields, severity, outcome, safe reasons, service metadata, schema version, and the generation, propagation, lifetime, rotation, expiry, and retry semantics of `workflow.id`, `causation.id`, and `attempt.id`.
- [x] 2.2 Add failing privacy specs for health data, credentials, tokens, cookies, network identifiers, job arguments, raw domain identifiers, schedule timestamps, unsafe exception text, and the complete exported span surface.
- [x] 2.3 Add failing resilience specs proving canonical logging and emergency-diagnostic failures never alter, block, or roll back a medication or health-data operation and never recurse.
- [x] 2.4 Implement the operational-event boundary, serializer, context enrichment, event-specific safe mappings, and bounded non-recursive emergency diagnostic.
- [x] 2.5 Enforce the frozen registry and add duplicate-ingestion fixtures proving collector retries are detectable and deduplicatable by stable event identifier without becoming another application attempt.

## 3. Normalize Request and Process Output

- [x] 3.1 Use failing final-image request specs to reconfigure, replace, or remove Lograge so Rails emits one canonical request event with route template, status, duration, severity, request identifier, deployment identity, and safe trace context.
- [x] 3.2 Remove unused request-logging options and retain Thruster access output only if its fields meet the privacy contract; otherwise disable it and rely on the canonical Rails request event.
- [x] 3.3 Give Rails/ECS, Thruster if retained, Puma, Solid Queue, and the OpenTelemetry SDK distinct datasets and tested severity contracts, and suppress routine successful health-check output while preserving failures.
- [x] 3.4 Run the test-mode and final-image request contracts and prove producer-scoped counts, parseable fields, privacy, request-tag propagation, deployment identity, and no nested JSON message.

## 4. Connect Domain Events and Transaction Outcomes

- [x] 4.1 Add failing safe-mapping specs for `take_attempted`, `take_recorded`, `take_blocked_by_rules`, `take_errors`, `dose_taken`, `low_stock_threshold_reached`, `audit_delivery_backlog`, and `rack_attack.throttled`.
- [x] 4.2 Publish operational records for all eight events without replacing Active Support as the domain-event bus, and use a publisher-side rescue boundary that records subscriber failure while preserving the original propagation semantics.
- [x] 4.3 Add failing medication outcome specs for attempt, rule blocking, provisional persistence, committed success, rollback, persistence failure, unavailable household, and unexpected failure.
- [x] 4.4 Implement outer-transaction-aware medication outcomes so attempt and provisional events remain queryable after rollback while committed success is emitted only after the outermost commit.
- [x] 4.5 Remove replaced medication free-text logs and verify authorization, household isolation, idempotency, stable causation, distinct retry attempts, and exact application-emission counts.

## 5. Trace Reminder, Low-Stock, and Push Outcomes

- [x] 5.1 Add safe outcome events for reminder enqueue, eligibility, past-occurrence handling, suppression, deduplication, retry, and job failure using stable reason codes.
- [x] 5.2 Propagate one workflow identifier across scheduling, missed-dose evaluation, notification intent, recipient and channel attempts, and provider outcomes; join a later medication action only through a trusted stored or client-carried occurrence association, otherwise start a new workflow without guessed causation.
- [x] 5.3 Add push outcomes for attempted, provider accepted, partial failure, permanent failure, and delivery unknown without claiming device delivery.
- [x] 5.4 Add low-stock evaluation and delivery outcomes plus retry and idempotency coverage, migrate the bounded notification logger sites, and capture notification-state, timezone, preference-ownership, and channel-parity correctness gaps as named follow-up changes.

## 6. Close Baseline Privacy Gaps and Document the Contract

- [x] 6.1 Add Active Job and exporter allowlist specs covering job arguments and the complete span surface, including names, attributes, events, exception attributes, status descriptions, links, and resource data.
- [x] 6.2 Migrate or remove unsafe infrastructure, telemetry, boot, and database-pool logger sites, including repeated-noise and recovery behavior.
- [x] 6.3 Migrate or remove unsafe authentication, authorization, security, rate-limit, and administrative logger sites without adding new out-of-baseline workflow events.
- [x] 6.4 Migrate or remove unsafe external lookup, import, mail, audit-export, rendering, and UI logger sites without recording upstream bodies or health data.
- [x] 6.5 Run the final census and prove every frozen baseline path plus every canonical boundary, emergency diagnostic, request subscriber, publisher helper, workflow-stage event, and trace surface introduced by this change is registered and has a tested disposition.
- [x] 6.6 Amend ADR 0004 with a follow-up note preserving Active Support as the domain-event mechanism, then document the operational schema, identifiers, privacy, severity, transaction, failure-isolation, ownership, rollback, and verification contracts.

## 7. Verify the Exact Production Revision

- [x] 7.1 Add a synthetic safe deployed canary and bounded Loki and trace queries for identifiers, deployment identity, ingestion duplicates, parser failures, missing severity, absent retained traces, and producer-scoped counts.
- [x] 7.2 Run focused Red-Green-Refactor specs, `task rubocop`, `task test`, `git diff --check`, strict OpenSpec validation, and the final production-image observability smoke.
- [x] 7.2.1 Correct the trace exporter resource allowlist so deployment-generated `host.name` survives for Kubernetes pod correlation while `process.pid` and every unapproved resource attribute remain removed; decode final-image OTLP traffic to enforce the non-empty resource value.
- [ ] 7.3 Deploy the exact verified image with the previous request logger and compatibility queries available for rollback, then confirm every target process runs its immutable digest.
- [ ] 7.4 Run the finite production matrix: poll the safe canaries for at most fifteen minutes, then record safe naturally occurring workflow evidence for twenty-four hours and mark unsafe or absent outcomes not applicable with a final-image contract reference.
- [ ] 7.5 Keep this change active and unarchived until that matrix passes for the exact deployed revision with its stated zero-failure thresholds; confirm the frozen baseline and all newly introduced signal paths are registered and every out-of-baseline discovery has a separate follow-up change.
