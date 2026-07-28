## 1. Red: Define the batch contract

- [ ] 1.1 Add request examples in `spec/requests/api/v1/sync_spec.rb` for a successful medication-take create, required `client_uuid`, the `replayed` result flag, and rejected update/delete actions.
- [ ] 1.2 Add request examples for schedule and person-medication sources covering source visibility, person `record` access, stale sources, and cross-household references.
- [ ] 1.3 Add request examples for unavailable stock, timing conflicts, invalid timestamps/doses/selections, and the exact PHI-safe error codes.
- [ ] 1.4 Add mixed-batch rollback examples in both operation orders and assert medication takes, inventory, audit versions, tombstones, and sync change records leave no committed side effects.
- [ ] 1.5 Add focused `Api::Sync` service examples for attribute validation, canonical `MedicationAdministration::RecordDose` delegation, success/replay outcomes, domain-error mapping, and named UUID-constraint detection.
- [ ] 1.6 Add a PostgreSQL concurrency example proving two batches with one `client_uuid` converge on one take and one set of stock, audit, and sync side effects.
- [ ] 1.7 Run `task test TEST_FILE=spec/requests/api/v1/sync_spec.rb` and the new focused service spec; confirm the new examples fail for the intended missing behavior.

## 2. Green: Add the explicit medication-take operation

- [ ] 2.1 Implement a small `Api::Sync` medication-take operation service that validates the create attributes, invokes `MedicationAdministration::RecordDose`, and returns typed success, replay, or sanitized failure outcomes.
- [ ] 2.2 Extend the batch dispatcher only for `action: create` with `resource_type: medication_take`; keep existing update/delete handling and ETag requirements unchanged.
- [ ] 2.3 Resolve schedules and person medications through their policy scopes, enforce `take_medication?`, and replay matching takes only through the authorized medication-take scope and `create?` policy.
- [ ] 2.4 Add `replayed` to medication-take operation results and render the approved machine-readable error codes without echoing submitted or inaccessible health data.
- [ ] 2.5 Detect only `index_medication_takes_on_client_uuid`, unwind the failed transaction, and retry the complete batch at most once before returning `idempotency_key_unavailable`.
- [ ] 2.6 Run the focused request, service, authorization, architecture, audit/change-record, and concurrency examples until green.

## 3. Refactor: Preserve existing boundaries

- [ ] 3.1 Remove duplicated parsing or failure mapping while keeping the controller responsible for HTTP, policy, and transaction concerns and the canonical service responsible for medication rules.
- [ ] 3.2 Confirm no raw medication-take insert, direct stock mutation, unscoped source lookup, new model callback, or medication-take update/delete path was introduced.
- [ ] 3.3 Run `task rubocop` and correct any offenses without broad unrelated formatting.

## 4. Verification and handoff

- [ ] 4.1 Run `task openspec:validate` and verify the implemented behavior still matches the approved proposal, spec, and design.
- [ ] 4.2 Run `task test` and confirm the full Docker-backed suite passes.
- [ ] 4.3 Review the diff against #1675 boundaries, confirm `docs/api/openapi.v1.yaml` and generated client types remain for #1656, and update the issue with any intentionally deferred follow-up.
