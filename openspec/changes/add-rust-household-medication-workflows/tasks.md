# Tasks

## 1. Preparation and shared boundaries

- [x] 1.1 Save research, ownership briefs and timed ledger; validate this change.
- [x] 1.2 Record failing shared-form and missing-route tests before implementing shared navigation/forms.
- [x] 1.3 Prove locale adapter compilation and key/plural/SSR consistency.

## 2. People

- [x] 2.1 Record Rails-derived failing list/detail/create/edit and permission tests.
- [x] 2.2 Implement People with preserved drafts and verified persistence.
- [x] 2.3 Pass desktop/mobile, translations, security checks and independent review.

## 3. Locations

- [x] 3.1 Record Rails-derived failing location journey and isolation tests.
- [x] 3.2 Implement Locations and immediate medication-selector usability.
- [x] 3.3 Pass desktop/mobile, translations, security checks and independent review.

## 4. Medication and stock

- [ ] 4.1 Record failing medication creation/edit and stock browser cases.
- [x] 4.2 Implement medication forms and verified inventory return.
- [ ] 4.3 Independently accept medication before expanding to stock actions.
- [ ] 4.4 Implement and accept adjustment/order/receipt with audit, concurrency and replay evidence.

## 5. Assignments and schedules

- [ ] 5.1 Record failing assignment and seven-type schedule round-trip tests.
- [ ] 5.2 Implement direct/scheduled assignment, all schedule editors and pause/resume.
- [ ] 5.3 Verify taper boundaries, permissions, history and browser acceptance.

## 6. Combined acceptance and publication

- [x] 6.1 Run applicable Rust checks and prior browser/API regressions on stable input.
- [x] 6.2 Obtain broad independent review and resolve material findings.
- [x] 6.3 Publish accepted work and record unfinished scope at two-hour assessment.
- [x] 6.4 Address personal review: split mixed-purpose Rust files, preserve behaviour and independently verify before main.

## Previous bounded acceptance

The recovered household HTTP checks and English desktop/mobile workflows pass
for People, Locations and scalar medication creation/editing. All five catalogue
rendering checks pass. Full validation localisation was incomplete because
unknown API messages retained English fallback text in that accepted slice.
Medication acceptance also retained the documented between-read draft-retention race and dosage-option editor
gap. Combined acceptance passed 15 HTTP cases and all 21 browser cases on the
final frozen input, including dose permissions, stock effects, history, replay
protection and mobile page width. The broader journey requirements above remain
open where localisation or planned editors are incomplete.

The original two-hour assessment was delivered at 10:27 UTC. Recovery acceptance
finished at 11:52 UTC. The accepted bounded slice is published in PR #2346;
remaining scope is tracked in issue #2345. No merge, deployment or Rails cutover
has been performed.

## Continuation ownership and evidence

The continuation recorded actual failing tests for unknown validation messages,
missing-token draft retention, a deterministic first-option insertion race and
limited-view dashboard access. All four bounded fixes are implemented and have
independent source-review passes. All 28 continuation HTTP cases passed on a
stable captured source. The separate browser run passed all 35 cases on the
identical source copy, including five-locale desktop/mobile People and Locations
journeys. Independent requirements, quality/security and visual review accepted
2.3 and 3.3. The separate legacy dashboard fixed-clock regression passed all
35 cases on the identical source copy. Final bounded A/B/C requirements and
code quality/security review passed; broader parity and planned editors remain
outside this acceptance.

The [completion packet](../../../docs/plans/axum-leptos-port/next-slices-20261001.md) keeps the original 20-task denominator (14 accepted). Slice A completed 2.3/3.3; B/C close safety/usability gaps before 4.3. D completes dosage-option management before 4.3 acceptance; E targets 4.4; F targets 5.1–5.3 only after all seven actual schedule types pass. G re-evaluates the full requirements. No new completion box is checked merely because dispatch, source edits or a build finished.

One Luna Medium owner records all compiled/runtime jobs in the [build queue](../../../docs/plans/axum-leptos-port/next-slices/build-queue.md); named Sol owners write disjoint product/tests and independent review. The two-hour assessment was delivered at 16:44 Europe/London on 1 October 2026. New feature dispatch stopped; verification and publication continue as closure work.
