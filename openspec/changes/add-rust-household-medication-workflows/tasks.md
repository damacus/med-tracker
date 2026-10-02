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

- [x] 4.1 Record failing medication creation/edit and stock browser cases.
- [x] 4.2 Implement medication forms and verified inventory return.
- [x] 4.3 Independently accept medication before expanding to stock actions.
- [x] 4.4 Implement and accept adjustment/order/receipt with audit, concurrency and replay evidence.

## 5. Assignments and schedules

- [x] 5.1 Record failing assignment and seven-type schedule round-trip tests.
- [x] 5.2 Implement direct/scheduled assignment, all schedule editors and pause/resume.
- [x] 5.3 Verify taper boundaries, permissions, history and browser acceptance.

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
missing-token draft retention, a dosage option added while a medication form is open, and
limited-view dashboard access. All four bounded fixes are implemented and have
independent source-review passes. All 28 continuation HTTP cases passed on a
stable captured source. The separate browser run passed all 35 cases on the
identical source copy, including five-locale desktop/mobile People and Locations
journeys. Independent requirements, quality/security and visual review accepted
2.3 and 3.3. The separate legacy dashboard fixed-clock regression passed all
35 cases on the identical source copy. Final bounded A/B/C requirements and
code quality/security review passed; broader parity and planned editors remain
outside this acceptance.

The [continuation plan](../../../docs/plans/axum-leptos-port/next-slices-20261001.md) keeps the original 20 tasks. People and Locations completed 2.3/3.3. Medication and dosage management complete 4.3; stock management completes 4.4. Assignments complete 5.1–5.3 only after all seven schedule types pass their checks. Final review checks the whole programme. A task is complete when its required behaviour is verified and accepted.

The previous run paused new features at its 16:44 Europe/London assessment. The user subsequently authorised completion of dosage, stock, assignments and final checks, with another retrospective. The [delivery plan](../../../docs/plans/axum-leptos-port/household-delivery-20261001/plan.md) records responsibility for implementation, testing and independent review.

D medication/dosage management is independently accepted, bringing the original denominator to 15/20. Four browser HTTP cases and an isolated ordinary-member permission case passed. Twenty browser cases passed across all five languages at desktop/mobile, including immediate dose use, exact option/parent stock decrement and dose replay. The corrected pagination helper retained all assertions; prior household browser regressions passed on unchanged product source. The full `ci:rust-port` gate passed on final frozen inputs. Stock, all seven schedule editors and final whole-programme acceptance remain pending.

Stock management is independently accepted, bringing the original denominator
to 17/20. Existing stock API tests passed 17/17; the new stock checks passed
13/13. Browser acceptance passed 41 selected cases across all five languages at
desktop/mobile. A separate permission fixture passed its HTTP case and selector
check, and seven prior medication browser regressions passed. These checks cover
saved stock/status, correct dosage units, rejected conflicts with retained forms,
removal replay, revoked access and unchanged records after rejected writes.
The final Rust gate passed after four behaviour-preserving lint/type repairs,
which the reviewer checked against the runtime-tested source. Requirements and
quality/security review passed. Assignments, schedules and final combined work
remain pending; no merge, deployment or Rails cutover is authorised.

Assignments and schedules meet the original journey requirements, completing
the original 20 tasks. The final candidate passed 37 HTTP tests and 12 browser
journeys, plus separate revoked-access, date and taper-time checks. Five-language
desktop/mobile evidence and independent requirements review passed. The full
Rust gate passed on the unchanged source. Documentation checks and publication
follow before moving to the final planned compatibility work.

The remaining stock bugs, broader API compatibility, five additional large-file
refactors and runtime CI promotion are still tracked in the delivery plan and
issue #2345. Completing the original task list does not complete those follow-ups
or authorise merging, deployment or Rails cutover.
