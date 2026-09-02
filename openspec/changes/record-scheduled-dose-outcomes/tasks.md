## 1. Store Formal Occurrences

- [ ] 1.1 Red: specify exact-source, tenant, identity, state, audit, portable-ID, and take-link constraints.
- [ ] 1.2 Green: add dormant storage, row-level tenancy, purge inventory, associations, and model specs.

## 2. Project Formal Schedule Occurrences

- [ ] 2.1 Red: specify timed/untimed identities, ordinals, eligibility, pause exclusion, and PRN exclusion.
- [ ] 2.2 Green: add the Schedule occurrence projector without persisting open occurrences.

## 3. Record Not-Taken Outcomes

- [ ] 3.1 Red: specify due/overdue, future rejection, reason/note, no stock, replay, concurrency, and reopen.
- [ ] 3.2 Green: add the transactional not-taken resolver without creating a MedicationTake.

## 4. Link Taken Outcomes

- [ ] 4.1 Red: specify canonical RecordDose use, one take, one stock decrement, mismatch rollback, and immutability.
- [ ] 4.2 Green: link occurrences atomically through the existing dose-recording transaction.

## 5. Show Formal Outcomes on the Dashboard

- [ ] 5.1 Red: specify open, taken, not-taken, and unexplained-overdue standard dashboard states.
- [ ] 5.2 Green: update the standard query and task-card presentation only.

## 6. Stop Formal Outcome Reminders

- [ ] 6.1 Red: specify resolved suppression and unexplained-overdue eligibility.
- [ ] 6.2 Green: update reminder eligibility and directly affected jobs.

## 7R. Report Formal Outcomes

- [ ] 7R.1 Red: specify history and report categories for each formal outcome.
- [ ] 7R.2 Green: update history and report projections without changing insights.

## 7I. Interpret Formal Outcomes in Insights

- [ ] 7I.1 Red: specify that only unexplained misses drive affected insight patterns.
- [ ] 7I.2 Green: update only the affected insight context and detectors.

## 8. List Formal Occurrences through the API

- [ ] 8.1 Red: specify bounded dates, access, household isolation, and PHI-safe errors.
- [ ] 8.2 Green: add the read-only API route, controller, serializer, and request specs.

## 9N. Record Not-Taken Formal Outcomes through the API

- [ ] 9N.1 Red: specify not-taken, backdating, idempotency, and concurrency.
- [ ] 9N.2 Green: add the not-taken API adapter using the domain service.

## 9C. Correct Formal Outcomes through the API

- [ ] 9C.1 Red: specify audited reopen and replacement without mutating a MedicationTake.
- [ ] 9C.2 Green: add correction API adapters using the domain services.

## 10. Publish the Formal Outcome API Contract

- [ ] 10.1 Red: specify capability and OpenAPI coverage for S08/S09 operations.
- [ ] 10.2 Green: update metadata, OpenAPI, contract fixtures, and client sources within the tranche limit.

## 11. Link Queued Takes to Formal Occurrences

- [ ] 11.1 Red: specify optional identity, old-client compatibility, mismatch rollback, and replay safety.
- [ ] 11.2 Green: add atomic linkage to medication-take sync operations and batches.

## 12E. Export Formal Outcomes through Portable Data

- [ ] 12E.1 Red: specify the v2 formal-outcome export shape, references, order, and legacy compatibility.
- [ ] 12E.2 Green: add exporter, serializer, and format documentation support.

## 12I. Import Formal Outcomes through Portable Data

- [ ] 12I.1 Red: specify one v2 round trip, reference restoration, rollback, and no stock replay.
- [ ] 12I.2 Green: add importer, preflight, and transactional writer support.

## 13. Record Not-Taken Outcomes on the Web

- [ ] 13.1 Red: specify reason/note, access, validation errors, display, and accessible confirmation.
- [ ] 13.2 Green: add the standard web action and one desktop/mobile browser flow.

## 14. Correct Formal Outcomes on the Web

- [ ] 14.1 Red: specify reopen and replacement without MedicationTake mutation.
- [ ] 14.2 Green: add the audited correction action and one focused browser flow.

## 15R. Read Formal Outcomes through Sync

- [ ] 15R.1 Red: specify snapshot and feed visibility for occurrence rows and outcomes.
- [ ] 15R.2 Green: expose formal outcomes through existing sync read contracts.

## 15B. Write Formal Outcomes through Sync Batches

- [ ] 15B.1 Red: specify idempotent not-taken, ETag-protected reopen, and complete-batch rollback.
- [ ] 15B.2 Green: add batch outcome operations through the shared resolver.
