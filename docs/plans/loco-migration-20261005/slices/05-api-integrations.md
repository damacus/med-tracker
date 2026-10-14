# API, native clients and integrations Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Preserve every public and native-client API plus complete integration journeys on Loco.
**Architecture:** API controllers serialize shared operation outputs into authoritative envelopes.
FHIR/SMART and MCP adapt the same authorised operations; external services use maintained clients.
**Tech Stack:** Loco, OpenAPI/client generation, maintained FHIR/MCP/storage/HTTP libraries.
**Spec:** [implementation-spec.md](../implementation-spec.md), authoritative `docs/api/openapi.v1.yaml`.

## Global constraints

C1–C3 supply care operations; I3 supplies protocol flows. Queue infrastructure from W1 precedes
any endpoint that enqueues work. No private compatibility adapter becomes a second business-logic owner.
Writer owns API/integration code; existing contracts/native pinned files are reference inputs.

## The work in plain English

This slice is about programmatic interfaces: APIs used by software, not the app features behind them.

1. **Complete the public API.** Preserve the documented API v1 routes, permissions, responses and
   errors on Loco, and check compatibility with the pinned iOS and Android API definitions.
2. **Add the other API protocols.** Preserve FHIR/SMART and MCP interfaces. They can follow the first
   production release, but remain part of the full migration.
3. **Check the API contracts for data services.** Keep the routes and responses for files, reports,
   portability, lookup and reviews documented and authorised. Their product behavior belongs to the
   care, browser and worker slices; this slice owns only the programmatic interface.
4. **Prove the interfaces.** Check documented routes, client contracts, permissions, errors and safe
   handling of retries and service failures.

A1–A3 below retain the detailed API implementation and acceptance checks.

## s05-i01 execution agreement — 13 October 2026

The 13 October baseline contains 119 OpenAPI v1 operations. Care/review PR #2617
merged on 14 October as `672dbc0d1520ee586ce625f0dbfec7fb3eba0544`, adding three
approved review-lease operations. Current acceptance therefore covers all 122
operations, including portable imports and AI suggestions. Preserve the earlier
119-operation evidence as historical; route registration does not complete the
three added operations. Browser/native journeys, live-provider acceptance, FHIR/SMART, MCP
and production cutover remain separate. Preserve the authoritative wire contracts
and approved migration differences; historical legacy-Rust acceptance is not Loco
evidence.

Use `rust-testing` and the upstream Loco guide vendored in `rust/AGENTS.md`.
Demonstrate failing behavioural tests before production corrections. Keep pure
logic tests fast, async tests deterministic, and integration tests on real restricted
database roles. Use Loco testing facilities over the fully booted router for new
request acceptance, retaining owned PostgreSQL 18/template fixtures and the
documented departures in #2592. Do not enable global truncation or replace RLS
credentials with administrator connections. Retain socket tests where transport
behaviour specifically requires them. Coordinate coverage reporting with #2583;
coverage numbers do not replace operation-level assertions.

The shared rate limiter uses the socket peer unless `API_TRUSTED_PROXY_IPS`
explicitly names a proxy. Configure each trusted proxy to overwrite incoming
`X-Forwarded-For` with the observed client address, or append that observed address
to the incoming chain. Passing client-supplied forwarding values through unchanged
does not establish a trusted client identity. List only controlled proxy addresses,
not ordinary clients. The application walks the chain from the socket peer and
stops at the nearest untrusted address; earlier supplied values cannot change
that client's bucket.

The coordinator owns integration, shared-file allocation, evidence and publication.
One Sol API writer owns each active area's implementation and regression tests.
One verifier owns builds, disposable resources and live job handles. Devin SWE-2
Max reviews each PR and resumes its review for corrections. Transfer overlapping
paths from existing owners before editing.

Delivery uses `gh stack`, rooted on refreshed `main`, with branches named
`codex/s05-i01-*`. Each PR targets its predecessor. The hard limit is 2,000 added
plus deleted lines against that immediate base, including tests, documentation
and generated changes; aim below 1,500 before review. Split at complete operation
or behaviour boundaries, keeping fixes with tests and every cumulative branch
passing. Correct lower-layer defects there and propagate them upwards.

| Order | Area | Operations |
| --- | --- | ---: |
| 1 | Operation inventory and Loco test integration | — |
| 2 | Shared errors, request IDs, conditional headers, replay and rate limits | — |
| 3 | Capability discovery and account sessions | 5 |
| 4 | People, profile, avatar and UI capabilities | 13 |
| 5 | Household administration | 14 |
| 6 | Invitations | 5 |
| 7 | Locations and memberships | 8 |
| 8 | Medication inventory and dosage options | 15 |
| 9 | Schedules and person-medication assignments | 15 |
| 10 | Dose occurrences and medication takes | 10 |
| 11 | Pause periods and health events | 8 |
| 12 | Notifications and device subscriptions | 8 |
| 13 | Review prompts, edit leases and JSON/PDF reports | 11 |
| 14 | Exports, imports and mobile snapshot | 5 |
| 15 | Synchronisation | 3 |
| 16 | Medication lookup and AI suggestions | 2 |

For each operation retain method/path, schemas, security, statuses, headers,
handler, executed assertions, candidate revision and gaps. Compare the document
with registered Loco routes, then exercise behaviour. Route presence, a test file
or historical evidence alone earns no completed coverage. Reconcile extra routes
without deleting them automatically.

`task slice:contracts GROUP=api-v1` must dispatch root Loco tests through existing
owned-resource helpers, reject unknown groups and fail final incomplete coverage.
Focused area selectors support red/green development. Verify middleware, RLS,
isolation and cleanup in the harness change. Preserve documented success/error
envelopes, permissions for each credential class, replay, concurrency, audit/sync
effects and durable queue outcomes. Use deterministic external service doubles.

Every PR needs focused evidence, resolved Devin findings, applicable repository
gates and exact-head hosted checks. Final acceptance also needs cumulative
contracts and `task ci`, coverage evidence, generated Swift/Kotlin checks and
pinned Android/iOS API checks in isolated native checkouts. Do not silently update
native pins. Record PR/base/head, line count, owners, review IDs, jobs and results
in `../progress.md`; update the existing reports from the same evidence.

Stop dependent work for unresolved contract decisions, ownership collisions,
failed required checks, unavailable review or oversized PRs. Acceptance,
publication, merge and deployment are separate; no merge or deployment is granted.

Account evidence, 14 October: source candidate `2ae0b911` has four exact operation
records, exercised by 13 credited request/database tests through dispatcher
`58221`. Its final completion gate correctly rejects 118 remaining operations;
the inventory is four complete, two partial and 116 unverified. Same independent
SWE-2 Max review `important-platypus` accepted the final correction in `59730`.
Cumulative CI and publication remain separate gates recorded in `../progress.md`.

## Review focus

Malformed requests preserve status/envelope (A1). Stale tokens and wrong households fail all transports
(A1/A2). Signed URLs cannot expose foreign blobs (A3). External failures do not commit partial work
(A3). Exports/imports preserve encryption, identity and retention boundaries (A3).

### A1: Complete API v1 surface and native contracts

**Files:** Extend the existing `src/controllers/api/` controllers, shared domain models and
explicit response views as evidence requires. New request acceptance lives in `tests/requests/`;
`scripts/migration/run-contracts.mjs` dispatches through the existing owned-resource test runner.
The operation inventory and registered-route snapshot live beside the request tests.
**Interfaces:** `slice:contracts GROUP=api-v1` runs cumulative acceptance; focused area groups
support development. Reuse existing error rendering and narrow contract adapters to preserve
documented details, request IDs, envelopes, pagination, ETags and preconditions without leaking
SQL or internal failures.

- [ ] Add route-inventory assertions against every authoritative OpenAPI operation; test absent/extra
  fields, malformed JSON, stale preconditions and permissions. Assert `unimplemented_operations == []`,
  `unauthorised_status == expected_status`, `response_schema_valid == true`.
- [ ] Run `rtk task slice:contracts GROUP=api-v1-inventory` for the initial harness;
  run focused area selectors to demonstrate each missing behaviour before correction.
- [ ] Implement remaining read/write/profile/device/admin/platform routes with standard `Routes`;
  preserve app-token/session revocation and support-access expiry. Build the Loco-targeted contract Task.
- [ ] Run `rtk task slice:contracts GROUP=api-v1`, root CI and repository-native pinned-client generation
  checks from `client-tools/openapi-generator/Taskfile.yml`. Record exact native commands in the report;
  ordinary native builds must not generate from live Rails or silently update pinned contracts.
- [ ] Review and commit `feat(api): preserve versioned API and native client contracts`.

### A2: FHIR R4, SMART and MCP use the same permission boundary

**Files:** Create `src/controllers/{fhir,smart,mcp}.rs`, `src/models/integrations/{fhir,mcp}.rs`,
`src/models/integrations.rs`, `tests/integration_protocols.rs`; modify dependencies and route registration.
Read retained `fhir.rs`, `smart_fhir.rs`, `mcp.rs` and Rails MCP/FHIR owners.
**Interfaces:** Each controller exposes `routes() -> Routes`; discovery metadata reflects I3;
adapters invoke shared care operations with verified actors and current scopes/consent.

- [ ] Test resource read/write/search, audience/patient/scope mismatch, withdrawn consent and MCP
  transport/tool errors. Assert `foreign_patient_results == []`, `stale_consent_write_count == 0`,
  `mcp_write_state == equivalent_api_write_state`.
- [ ] Run `rtk task slice:test TARGET=integration_protocols`; record unsupported route/transport failures.
- [ ] Select maintained FHIR/MCP libraries from current docs; record any actual capability limitation.
  Implement adapters without a custom OAuth engine or copied domain transactions.
- [ ] Run `rtk task slice:contracts GROUP=integrations`, I3 interoperability and root CI.
- [ ] Review and commit `feat(integrations): preserve FHIR SMART and MCP access`.

### A3: Attachments, reports, portability, lookup and review workflows

This sub-plan owns the programmatic API contracts and adapters for these capabilities. The underlying
product behavior is owned by its care, browser or worker slice; API routes do not make that feature
implementation API work.

On 6 October, the owner excluded historical download-signature and system-export
format compatibility. Preserve underlying files/data, issue new authorised links
and generate new Loco exports. PDF reports must match existing appearance; inspect
the current renderer, fonts and retained Rust implementation before selecting a
renderer and prove it in scratch. FHIR/SMART, MCP, AI and external lookup remain in
full migration scope but are deferred from the first production gate. Exact AI/
lookup provider configuration parity is unnecessary; core medication entry remains.

The later first-release answers require automated NHS dm+d import/reconciliation,
scanner and medication review generation/background refresh. Those outcomes are
core despite optional AI/lookup deferral. Portable imports and acceptance of both
native apps are deferred from first production, not from the full goal. Core API
correctness, avatar uploads and complete device/session management remain required.

**Files:** Create `src/models/integrations/{attachments,reports,portability,lookup,reviews}.rs`,
matching API controller modules, `tests/integration_data.rs`; add fixtures under `tests/fixtures/integrations/`.
Read contract `uploads.rs`, `envelopes.rs`, `reports.rs`, `portability.rs`, `lookup.rs`, `reviews.rs`,
`nhs_dmd_import_api.rs`; Rails portable-data/report/storage services and retained PR #2418 assets.
**Interfaces:** Each operation module exposes specification `execute`; reports/portability produce
authorised private result references, never an unverified caller-supplied storage path.

- [ ] Test signed upload/download expiry and foreign IDs; encrypted export round-trip; identity-stable
  import retries; unavailable AI/lookup; failed PDF/font rendering; review refresh and import job state.
  Assert `foreign_blob_exposed == false`, `expired_download_rejected == true`,
  `roundtrip_portable_ids == original_ids`, `retry_duplicate_records == 0`.
- [ ] Run `rtk task slice:test TARGET=integration_data`; record missing operations/format support.
- [ ] Integrate maintained signing/encryption/storage clients and proven format compatibility.
  Use W1 durable enqueue for imports/reviews. Keep private storage, retention and error behaviour explicit.
- [ ] Run `rtk task slice:contracts GROUP=data-integrations` plus root CI, including real private storage
  and matching PDF fixtures. Verify new authorised links and expiry; old Rails links
  may expire under the approved transition.
- [ ] Review, integrate and publish; reconcile every capability-inventory API row and update progress.

**Done:** Every authoritative API operation and inventoried integration passes on Loco, pinned clients
remain compatible, and no response success conceals missing jobs, unauthorised access or partial writes.
