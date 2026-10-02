# Assignments and schedules

Start only after the coordinator accepts and publishes stock management.
Keep the existing writer, verifier and independent reviewer. The coordinator
creates the next branch and owns publication.

## What people need to do

Assign a medication to a person, choose a supported dose, and edit the assignment.
Create and edit daily, multiple daily, weekly, specific-date, as-needed, tapering
and every-other-day schedules. Pause and resume treatment, with a truthful history.
After saving, the person’s treatment and dashboard must use the saved dose and dates.

Use ordinary form controls. People must not need to write JSON. Keep their entries
when validation fails or another change makes their form stale. Show useful errors
in all five supported languages, and make the forms usable on desktop and mobile.

## Implementation boundaries

Follow the existing [later journey packet](later-journeys.md) and OpenSpec treatment
requirements. Use existing API routes, current person management permissions and
the established browser session and CSRF checks. There is no assignment DELETE
endpoint. Monthly is a dose cycle, not an eighth schedule type.

Before extending the large assignment, schedule and pause files, establish their
existing test results and split them by responsibility. Preserve comments,
transactions, lock order, original edit tokens, replay outcomes, audit and sync
records. Keep public API behaviour unchanged unless the coordinator records a
specific compatibility ruling.

Preserve the dose’s decimal precision and the API’s rules for dose limits and
minimum intervals. A selected dosage must belong to the selected medication and
match the saved dose. Keep every taper step, time, effective date and timezone
when creating, editing or rejecting a draft.

## Checks before acceptance

Write failing browser-route and seven-type round-trip tests before adding the UI.
Check new fixtures against successful API test helpers before requesting a runtime
run. Check API response formatting and stored audit formatting separately; an
API quantity such as `20.0` can have an audit snapshot of `20.00`. Preserve the
documented precision and meaningful before/after assertions. Ask the verifier
to compile new test files before creating a disposable
fixture. Keep permission-changing tests in a separate final fixture.

Verify actual persisted values and dashboard dose eligibility for all seven
schedule types. Check taper step boundaries, dates, timezones and daylight-saving
changes, as-needed dosing, and paused intervals. Reject foreign records,
view-only access, lost permissions, missing or stale edit tokens, and conflicting
repeated submissions without changing treatment or dose history.

The verifier owns all builds and runtime tests. Send one stable source handoff,
with exact Task commands and the required test files. Release the writer after
the verifier validates the captured test inputs. Do not hold the whole run while
waiting for an intentionally uncopied command file.

Write assignment-writer-report.md with changes, test results and remaining work.
The reviewer records separate requirements and code-quality verdicts in
review-assignment-report.md. Acceptance needs all seven editors, permissions,
history, translations, desktop/mobile evidence and the relevant regression tests.
Do not start the final compatibility work until this journey is accepted.
