# Person health and care journey comparison

Implementation started at `96682fff`, including `f3e7c132`. Current integration
base: `b5f38bfd4` (fast-forwarded with disjoint upstream dependency changes).
Approved owner packet:
`/private/tmp/medtracker-person-care-plan.md`.

This comparison includes source and actual rendered reference inspection.
The focused behavioural evidence below is partial; acceptance also requires
browser journeys, independent review and exact-head hosted gates.

| Concern | Rails browser | Documented public API | Loco delivery |
| --- | --- | --- | --- |
| Health reads | Person-scoped list, newest start date/id first | Household collection with pagination and updated-since; individual resource | Both adapters use person view access and household RLS |
| Creation | Person in URL; kind/title/start required | Person identifier in request; kind/title/start required | Record access, atomically persist event, links, audit and sync |
| Editing | Manage access; ongoing clears end date; action taken and medical help | Manage access; documented fields only; ETag precondition | Rich browser fields stay outside public API schema |
| Removal | Destroy event and link rows | No public DELETE route | Browser removal retains audit and sync tombstone evidence |
| Medication options | Policy-visible medications assigned by schedules or person medications | Policy-visible medication identifiers | Explicit adapter-specific eligibility; hidden/forged links fail atomically |
| Medication names | Snapshot names in links | Numeric and portable medication identifiers | Preserve snapshots for browser history |
| Manager carer assignment | Same-household capable adult with account | Existing administration contract | Person-scoped adapter reuses delegation effects |
| Nonmanager assignment | Parent email for authorised dependent manager | No new public endpoint specified | Parent-only existing-user/invitation flow reuses proof, expiry, delivery and acceptance |
| Removal/reactivation | Household manager | Existing administration contract | Relationship-owned grants revoked; unrelated grants preserved |
| Missing active carer | Legacy validation may veto writes | Descriptions at the delivery base require active carer | Approved correction allows create/edit/removal and warns authorised household/admin viewers |
| Dependent identity | minor=1, dependent adult=2; capacity false | Same enum and capacity constraints | Never change capacity or grant access merely because a warning appears |

Existing root Loco capabilities include sync health-event mutation/projection,
reports, person CRUD with audit/sync, administration delegation and invitations.
Direct health-event API routes and person health/carer browser journeys are absent
at the delivery base.

Cedar already provides `view_person`, `record_person`, `manage_person` and
`delegate_person`. Health adapters should use existing PersonAccess checks;
no new security protocol or custom policy evaluator is required.

Inventory delivery owns `medications.inventory` locale keys and inventory leaves.
Person-care owns health-event/person-carer leaves and the authorised care-warning
projection. Shared registrations and fixture helpers are additive.

## Focused evidence

All runs used owned synthetic databases and the coordinated local verification lane. No reference application or live data was changed.

| Selector | Observed result | Consequence |
| --- | --- | --- |
| `health_events_documented_routes_create_read_update_and_revoke` | Initial RED: four 404 responses | Added documented route adapters over existing sync mutations |
| `health_events::` | Three cases passed; missing request wrapper returned 422 instead of 400 | Request-envelope mapping corrected; subsequent focused run passed all four cases |
| `people_without_carer_can_be_created_and_edited_without_capacity_change` | Creation returned 422; subsequent edit lacked a created person | Creation and edit vetoes removed after independent RED; both dependent types now pass |
| `household_relationship_removal_preserves_unrelated_grant_without_veto` | Removal rejected while unrelated manual grant existed | Veto removed, owned-grant revocation retained; focused rerun passed |
| `people_existing_dependents_without_carer_can_be_edited` | Independent RED: 422/422; GREEN after correction: 200/200 | Two persisted edits and audits; capacity false and zero carers retained |

Reference source confirms separate manager and nonmanager forms. Parent email assignment uses an active account, capable adult person, no professional title, and no active self relationship. Unknown addresses use a member invitation with the selected person's parent/manage grant. The Loco adapter must validate pending invitation compatibility without silently widening an incompatible grant and preserve established token expiry and delivery.

The isolated Rails reference image and server were subsequently built and seeded; rendered inspection is recorded below.

## Rendered Rails reference (9 October 2026)

Isolated project `mt-person-care-reference`, loopback port `58662`, synthetic fixture database. Image build, server startup and seeding all completed successfully; logs are `/private/tmp/person-care-reference-{build,server,seed}.log`. No Rails source or live data changed.

Using the actual browser, Jane Doe's person page exposes Health events. Its empty state offers Record notable illness and Record suspected side effect. The side-effect form presents title, dates, ongoing, severity, assigned Ibuprofen/Vitamin D, notes, action taken and medical-help checkbox. An end date before the start produces an error while preserving the draft. Checking ongoing saves with a cleared end date. The resulting list shows the linked medication snapshot and action taken; edit retains the medication selection, action text and medical-help flag.

Jane Doe's Child Patient page exposes Manage parents; its nonmanager form contains only Parent email, Cancel and Save assignment. The synthetic owner sees Parent or carer and Relationship type selectors on the same person-scoped journey. The owner people index also renders NEEDS CARER and Assign Carer for the unassigned dependent fixture. These inspected states establish reference interaction evidence; they do not establish Loco browser acceptance or authorise copying reference discrepancies.


## Current candidate status (9 October 2026)

The health browser controller, forms, assigned-medication adapter, richer audit
fields, content-version checks and delete confirmation are implemented locally.
The shared permission-filtered no-carer warning is also implemented locally.
This health candidate now compiles and passes the six focused API cases and
three desktop browser journeys. This does not establish mobile, broader role,
independent-review or final integration acceptance.

Initial real browser RED reached the authenticated person page and failed on
the missing Health events link. The dependent-carer RED reached the minor page
and failed on the missing no-carer warning. Traces and contexts are retained
under `/private/tmp/person-care-browser-{health,carers}-red-*`.

The six focused API cases passed in run `84490` (compile 36.13 seconds;
tests 1.64 seconds), log `/private/tmp/person-care-health-green4.log`.
They cover documented routes, scoped access, record/view restrictions, stale
preconditions and rollback after forced audit and medication-link failures.

The three desktop health journeys passed in run `79915` (development build
30.87 seconds; browser tests 16.5 seconds), log
`/private/tmp/person-care-browser-health-green.log`. They cover forged medication
rejection, rich-field draft preservation, ongoing state, deletion audit and
tombstone evidence, snapshot preservation after rename/edit, and a two-tab stale
draft. Compiler sample: PID 46514, CPU 97.8%, RSS 2578448 KiB; this is a sample,
not a peak.

Person-scoped carer routes are implemented locally. Focused run `11026`
passed the renderer in all five supported languages and two domain permission
cases (compile 11.39 seconds; tests 1.33 seconds), log
`/private/tmp/person-care-carer-focused-green.log`. The permission cases reject
ineligible accounts, inactive relationships, withdrawn membership and unmanaged
or adult subjects; a forged nonmanager role still produces a parent relationship.

Desktop run `97102` passed manager assignment, last-carer removal, ordinary
editing without a carer, restoration and existing-parent email assignment.
Both invitation cases failed with HTTP 503 (two passed, two failed; 34.1 seconds),
log `/private/tmp/person-care-browser-carers-green3.log`. Inspection found an
incorrect PostgreSQL placeholder in the existing-user lookup. It is corrected
locally. The subsequent 14-case desktop/mobile run `81418` passed 12
checks; the two extended parent signup cases exposed the existing
household-manager-only acceptance guard. Log:
`/private/tmp/person-care-browser-complete-green.log`.

Acceptance now rechecks current parent/manage authority for every invited
dependent through existing Cedar, account and grant facilities. It rejects
withdrawn/expired access, incompatible grant or membership roles and adult
subjects before effects. All ten invitation regressions passed in run `10897`
(compile 28.28 seconds; tests 2.21 seconds), log
`/private/tmp/person-care-parent-invitation-green.log`. The two extended
desktop/mobile signup journeys then passed in run `95653` (compile 21.53
seconds; browser tests 14.7 seconds), log
`/private/tmp/person-care-parent-signup-green.log`. They verify one invitation
email on repeated assignment, signup, actual email verification, recovery-code
setup and parent access to edit/manage the selected dependent.

Eight actual desktop/mobile light/dark form screenshots are retained under
`docs/screenshots/person-{health,carers}-form-*.png`. Both viewport journeys
verified text contrast and horizontal reflow; representative mobile and desktop
screenshots were visually inspected. This is not screen-reader testing.
The final seven health-event cases and four carer cases passed in run
`83361`; logs are `/private/tmp/person-care-final-health-focused.log` and
`/private/tmp/person-care-final-carer-focused.log`. They include the browser
view/record/manage matrix for administrator, clinician, self, carer and parent
access, rejection of an active self-managing parent candidate, and denial of
real health reads/writes after relationship removal while a manager's unrelated
manual access remains usable. Health compile/tests: 6.41/6.81 seconds; carer
compile/tests: 0.64/1.34 seconds. Test process sample: PID 93461, CPU 42.1%,
RSS 167424 KiB; sample only, not peak.

Documentation build `23352` passed (2.39 seconds), log
`/private/tmp/person-care-docs-build.log`. The locale tree checker passed all
five files. Root lint initially identified one collapsible conditional in the success
notice; its correction and the added pending-invitation/original-inviter
regression passed in final focused run 64971 below.
Run `32034` confirmed both final REDs: the adapter accepted a pending
invitation attachment its original sender could not manage, and the browser
showed the wrong generic parent-email message. Logs:
`/private/tmp/person-care-pending-inviter-red.log` and
`/private/tmp/person-care-pending-error-red.log`. The correction rechecks
original-sender authority after attaching the grant inside the same transaction
and rejects unusable reuse atomically. Incompatible/expired invitations now
produce a specific accessible message in all five languages.

Final focused run `64971` passed all five carer domain cases (compile 23.26
seconds; tests 1.52 seconds), all 14 desktop/mobile health/carer journeys
(development build 20.84 seconds; browser tests 57.6 seconds), and root lint
(20.21 seconds). Logs: `/private/tmp/person-care-carer-final-green.log`,
`/private/tmp/person-care-browser-final-green.log`,
`/private/tmp/person-care-lint-green.log`. The browser evidence now also
includes keyboard order, 320-pixel reflow, missing/forged CSRF tokens with
unchanged persisted effects, and the translated invitation error's accessible
description. The locale tree checker still passes all five files.

The owner explicitly authorised sending any and all code to Devin. Devin
review session resisted-justice completed a static code/security review of the
392-file source/context packet. No builds, tests, source edits or publication
were delegated. Review logs:
 /private/tmp/person-care-devin-review.log
 /private/tmp/person-care-devin-review-conclusion.log

The reviewer returned two low-severity findings. API null/clear and PUT
replacement semantics were not changed: authoritative HealthEventAttributes
has nonnullable scalar inputs, PATCH and PUT share HealthEventUpdateRequest,
and retained merged-attribute behaviour is covered by the existing API test.
Widening those inputs or reinterpreting PUT is outside this migration.

The medication-link finding was confirmed: a browser save discarded a valid
API-created link when its medication was not assigned to the person. Regression
69998 genuinely failed with zero retained links. The correction derives all
editable medication IDs on the server, preserves links outside that set, and
still removes explicitly deselected eligible links. Audit snapshots now retain
medication link IDs and original names before/after updates and before deletion.
Run 22454 passed all eight health-event tests and strict lint. Logs:
 /private/tmp/person-care-link-review-red.log
 /private/tmp/person-care-link-review-green.log
 /private/tmp/person-care-link-review-lint.log

The same Devin session performed a bounded correction review and returned no
actionable findings. Log: /private/tmp/person-care-devin-correction.log.

Initial frozen root CI 41510 passed all Rust/tooling gates and 462 browser
cases (16.8 minutes). Corrected frozen root CI 94269 passed with exit status 0,
including all Rust/tooling gates and 462 desktop/mobile browser cases
(17.2 minutes). Logs: /private/tmp/person-care-root-ci.log and
/private/tmp/person-care-corrected-root-ci.log. Main remained b5f38bfd4.
No production source changed during either run.

The owner authorised the remaining local checks and PR publication. Separate
read-only code/security review person_care_final_review completed against the
corrected worktree with no actionable findings. It covered current membership,
household locks, CSRF, rollback, medication-link snapshots/audits, carer
eligibility, original-inviter authority, revocation and warning visibility.
No source edits, builds, tests or network operations were delegated.

Local verification and required reviews are complete. Hosted acceptance, merge
and deployment remain separate gates.
