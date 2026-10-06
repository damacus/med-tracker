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

## Review focus

Malformed requests preserve status/envelope (A1). Stale tokens and wrong households fail all transports
(A1/A2). Signed URLs cannot expose foreign blobs (A3). External failures do not commit partial work
(A3). Exports/imports preserve encryption, identity and retention boundaries (A3).

### A1: Complete API v1 surface and native contracts

**Files:** Modify care API controllers; create `src/controllers/api/{profile,devices,admin,platform}.rs`,
`src/views/api.rs`, `tests/api_v1.rs`, `scripts/migration/loco-contracts.mjs`; modify `Taskfile.yml`.
**Interfaces:** `slice:contracts GROUP=<group>` starts owned Loco/fixtures and runs the retained black-box
cases. `api::error_response(OperationError, request_id: &str) -> Response` preserves documented details,
request IDs, envelopes, pagination, ETags and preconditions without leaking SQL/internal failures.

- [ ] Add route-inventory assertions against every authoritative OpenAPI operation; test absent/extra
  fields, malformed JSON, stale preconditions and permissions. Assert `unimplemented_operations == []`,
  `unauthorised_status == expected_status`, `response_schema_valid == true`.
- [ ] Run `rtk task slice:test TARGET=api_v1`; expected missing route/envelope failures.
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
