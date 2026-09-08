## 1. Store Pause Periods

- [x] 1.1 Red: specify exact-source, tenant, interval, reason, actor, audit, portable-ID, and one-open-period constraints.
- [x] 1.2 Green: add dormant storage, row-level tenancy, purge inventory, associations, and focused model specs.

## 1B. Reconcile Legacy Inactive Sources

- [x] 1B.1 Red: specify inactive, active, already-reconciled, and idempotent rerun behaviour for both source types.
- [x] 1B.2 Green: add bounded reconciliation that creates one unknown-start `reason_not_recorded` period per uncovered inactive source.

## 2. Record the Pause Lifecycle

- [x] 2.1 Red: specify atomic, idempotent, concurrent, and legacy pause and resume transitions.
- [x] 2.2 Green: add shared lifecycle services while retaining `active` compatibility.

## 3. Apply Pause Intervals to the Dashboard

- [x] 3.1 Red: specify partial-day, full-day, current, and resumed dashboard behaviour.
- [x] 3.2 Green: add the shared pause projection and update the standard dashboard query.

## 4. Apply Pause Intervals to Reminders

- [x] 4.1 Red: specify reminder exclusion inside intervals and unchanged eligibility after resume.
- [x] 4.2 Green: update reminder eligibility and directly affected jobs through the shared projection.

## 5R. Apply Pause Intervals to History and Reports

- [x] 5R.1 Red: specify history and report boundaries, including unknown legacy starts.
- [x] 5R.2 Green: update history and report queries without changing unrelated results.

## 5I. Apply Pause Intervals to Insights

- [x] 5I.1 Red: specify insight boundaries and preserve unrelated detector results.
- [x] 5I.2 Green: update only the affected insight context and detectors.

## 6. Collect Schedule Pause Context on the Web

- [x] 6.1 Red: specify required reason, optional note, access, validation errors, and active context for Schedule.
- [x] 6.2 Green: add the shared accessible pause form and Schedule web workflow.

## 7. Collect Assignment Pause Context on the Web

- [x] 7.1 Red: specify the same contract for routine and as-needed PersonMedication assignments.
- [x] 7.2 Green: reuse the Schedule pause form in the PersonMedication workflow.

## 8. Show Completed Pause History

- [x] 8.1 Red: specify preloaded, authorised completed-period history without view queries.
- [x] 8.2 Green: add the history query and shared component to the existing medication history surface.

## 9. Add the Pause API Server Operations

- [x] 9.1 Red: specify reason-required create/close, legacy compatibility, idempotency, isolation, and safe errors.
- [x] 9.2 Green: add server routes, controller adapters, serializers, and focused request specs.

## 10. Publish the Pause API Contract

- [x] 10.1 Red: specify capability and OpenAPI coverage for P09 operations and legacy deprecation.
- [x] 10.2 Green: update metadata, OpenAPI, contract fixtures, and client sources within the tranche limit.

## 11E. Export Pause Periods through Portable Data

- [x] 11E.1 Red: specify the v2 pause-period export shape, references, and legacy state.
- [x] 11E.2 Green: add exporter, serializer, and format documentation support.

## 11I. Import Pause Periods through Portable Data

- [x] 11I.1 Red: specify one v2 round trip, reference resolution, final active state, and rollback.
- [x] 11I.2 Green: add importer, preflight, and transactional writer support without replaying effects.

## 12R. Read Pause Periods through Sync

- [x] 12R.1 Red: specify snapshot and feed visibility with household isolation.
- [x] 12R.2 Green: expose pause periods through existing sync read contracts.

## 12B. Write Pause Periods through Sync Batches

- [x] 12B.1 Red: specify idempotent create/close, ETag conflict, and complete-batch rollback.
- [x] 12B.2 Green: add batch operations through the lifecycle service without wiring-only tests.

## 13. Native iOS Pause Controls

- [ ] 13.1 Red: specify reason validation, online-only writes, capability gating, retained input on failure, person filtering and completed history.
- [ ] 13.2 Green: add native pause form, labelled card actions and Paused treatments on Today, using the new API contract.
- [ ] 13.3 Verify: run iOS API, lint, unit/UI and CI tasks; capture iPhone/iPad accessibility and appearance evidence.

## 14. Native Android Pause Controls

- [x] 14.1 Red: specify the same native pause, resume, history and connection contract.
- [x] 14.2 Green: add native pause form, labelled card actions and Paused treatments on Today, using the new API contract.
- [x] 14.3 Verify: run Android contract, unit/UI, lint and build tasks and capture emulator evidence.

## 15. Completion Review

- [x] 15.1 Reconcile existing reminders, reports and insights against focused verification before marking their earlier tasks complete.
- [ ] 15.2 Independently review each delivery slice and the integrated feature, resolve findings and run applicable broad checks.
- [ ] 15.3 Publish focused feature PRs and record exact verification and remaining environment limits; do not merge or deploy.
