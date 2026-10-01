# Tasks

## 1. Preparation and shared boundaries

- [x] 1.1 Save research, ownership briefs and timed ledger; validate this change.
- [x] 1.2 Record failing shared-form and missing-route tests before implementing shared navigation/forms.
- [x] 1.3 Prove locale adapter compilation and key/plural/SSR consistency.

## 2. People

- [x] 2.1 Record Rails-derived failing list/detail/create/edit and permission tests.
- [x] 2.2 Implement People with preserved drafts and verified persistence.
- [ ] 2.3 Pass desktop/mobile, translations, security checks and independent review.

## 3. Locations

- [x] 3.1 Record Rails-derived failing location journey and isolation tests.
- [x] 3.2 Implement Locations and immediate medication-selector usability.
- [ ] 3.3 Pass desktop/mobile, translations, security checks and independent review.

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
- [ ] 6.3 Publish accepted work and record unfinished scope at two-hour assessment.

## Current evidence

The recovered household HTTP checks and English desktop/mobile workflows pass
for People, Locations and scalar medication creation/editing. All five catalogue
rendering checks pass. Full validation localisation remains incomplete because
unknown API messages retain English fallback text. Medication acceptance also
retains the documented between-read draft-retention race and dosage-option editor
gap. Combined acceptance passed 15 HTTP cases and all 21 browser cases on the
final frozen input, including dose permissions, stock effects, history, replay
protection and mobile page width. The broader journey requirements above remain
open where localisation or planned editors are incomplete.
