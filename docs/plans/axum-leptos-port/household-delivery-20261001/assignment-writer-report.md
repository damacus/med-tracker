# Assignment and schedule delivery

People need to assign medication, edit all seven supported schedule types and
pause or resume treatment without losing their entries. The existing assignment,
schedule and pause logic is now separated into smaller modules, with its function
bodies preserved. Assignment, seven schedule editors, pause, resume and history
screens are now added, with person-page links and translated labels. Their first
compile and browser checks are pending. All three existing API baselines pass;
the initial tests confirmed the missing screens.

## Existing behaviour

The original assignment baseline ran 11 cases: six passed and five stopped at
the same allowed-response-key assertion. The original schedule baseline ran ten
cases: three passed and seven stopped at the corresponding assertion. Both APIs
return the documented optional `can_record` and `eligible_stock_medication_ids`
fields. The test helpers omitted those keys. The pause baseline passed all ten
cases. These results precede any production change.

The narrow helper repair adds those two allowed keys and checks their types only
when present. Required fields and rejection of unknown keys are unchanged. Null
does not satisfy either optional field's type. No API or OpenAPI behaviour changes.

The verifier has the following stable request for meaningful repaired baselines:

```fish
rtk proxy task api:contract-selected-compile TEST_TARGET=openapi_person_medication_writes
rtk proxy task api:contract-selected-compile TEST_TARGET=openapi_schedule_writes
rtk proxy task api:openapi-person-medication-writes-acceptance
rtk proxy task api:openapi-schedule-writes-acceptance
```

Expected case counts are 11 assignment and ten schedule cases. The healthy pause
baseline is not repeated before extraction. Original run evidence is retained in
`/private/tmp/household-f-20261001/F-PERSON-BASELINE-001/runner.raw.log`,
`/private/tmp/household-f-20261001/F-SCHEDULE-BASELINE-001/runner.raw.log` and
`/private/tmp/household-f-20261001/F-PAUSE-BASELINE-001/runner.raw.log`.

The repaired assignment baseline passed 11/11 and the repaired schedule baseline
passed 10/10. The assignment runtime copy was validated before execution. For the
schedule rerun, the source digest and stable pre/post receipt are retained, but
the verifier missed per-file validation before the runner removed its temporary
copy. This evidence limitation does not change the actual test results.

## Browser-route test design

The new `household_treatments.rs` target is a test-only draft with nine cases:
direct assignment, seven separately selected schedule round trips, and canonical
pause/resume/history with exact replay. Resume submits the original pause-period
identity as well as its original period token and submission key. Fixtures use
native login, existing current
person management grants and real medication creation. No permission change is
part of this initial fixture. No runtime result is claimed for this draft.

The initial browser file has ten journeys covering all five languages at desktop
and mobile sizes. Each creates a direct assignment, corrects an invalid dose,
then saves and reopens all seven schedule forms with labelled native controls.
It requests screenshots of every editor. Per-editor evidence remains pending.

The first combined wrapper actually ran the ten browser journeys: all ten failed
at the missing assignment-create link on person detail. This is the observed UI
failure authorising the new screens. The nine HTTP cases did not execute because
their selector variables were passed as Task variables rather than exported
environment variables. No nine-case HTTP result is claimed. The verifier owns
the corrected exported-selector run; the same browser failure is not repeated.

The initial route checks use these existing selectors, after setup review and
the new target's compile check:

```fish
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatments
begin
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_treatments
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-treatments.test.mjs
end
rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-treatments.test.mjs
```

The HTTP run selects nine cases. Its missing-route failures should stop the
wrapper before browser execution, so the separate browser-only run selects ten
journeys with `HOUSEHOLD_ACCEPTANCE` and `HOUSEHOLD_TEST_FILE` unset. Existing
browser defaults are replaced by the explicit filename. No grants are changed.

The original module copies for the responsibility split are preserved in
`/private/tmp/household-f-20261001/extraction-before/`. Their SHA-256 digests are:

- `person_medication_writes.rs`: `0c6c667d82537de8aa39e302cc5bbf7ae24f7566afe7461157758206fc22798c`
- `schedule_writes.rs`: `5e92b91e697bd3a13c15240d7fe0d4075bd73114ecc011878911ca9b63a2f22c`
- `pause_lifecycle.rs`: `aabf0e62066bdaa5d9a78760160237dfc5664ee77bc77faebf019077f92c5cc6`

## Responsibility split

The three facades retain shared imports, domain types and explicit entry-point
exports. Their child modules separate parsing, access, representation, responses,
replay, writes and sync; assignment dose selection, pause persistence, pause
history reads and legacy pause actions have their own responsibilities. All 98
moved top-level function bodies are byte-identical before formatting. Their body
digests are retained in
`/private/tmp/household-f-20261001/extraction-function-bodies.tsv`. Existing
attributes and comments move with their functions. Entry-point visibility through
each facade is unchanged; internal functions remain limited to facade scope.

The split is stable for independent comparison and these verifier checks before
the corrected HTTP fixture capture:

```fish
rtk proxy task api:fmt:write
rtk proxy task api:check
```

Formatting, the API compile check and the selected treatment target compile all
pass. The API check reported ten unused-import warnings; exactly those imports
have now been removed without changing bodies or exports. The reviewer separately
verified all 98 bodies, the retained signatures, comments and attributes, and the
24 cohesive child modules. The post-cleanup formatter, compile check and
warning-denying Clippy pass. The extracted assignment, schedule and pause
baselines pass 11/11, 10/10 and 10/10 respectively. No new browser guard or API
behaviour is part of this split.

## Initial screens

The corrected exported-selector HTTP run reached all nine tests. All nine
failed at the absent editor: 404 instead of 200. Its raw assertions are retained
in `/private/tmp/household-f-20261001/F-TREATMENT-HTTP-RED-002/http.raw.log`.
The previously observed ten-browser missing-link failure remains the browser
starting evidence; no duplicate browser RED is needed.

The new renderer separates assignment fields, shared labelled fields, schedule
layout, type-specific controls, treatment overview and pause/history. Browser
adapters separately handle authorised context, drafts, dose selection, schedule
configuration, writes and canonical pause actions. The person page receives
preloaded authorised treatment rows. Current person management grants control
write affordances and form access. Existing renderer entry points remain usable.

Native forms preserve malformed dates, decimal strings, original tokens and
repeated time/date/taper rows. Schedule configuration uses the original draft
representation for stable retries; users edit labelled controls rather than JSON.
Canonical pause/resume routes retain the original pause identity and distinct
period token. The private source guard is deliberately absent until its dedicated
failing tests execute.

The initial screen input set is stable for this verifier request:

```fish
rtk proxy task api:fmt:write
rtk proxy task -d rust/web format
rtk proxy task api:check
rtk proxy task api:clippy
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatments
rtk proxy task -d rust/web test
```

These fast checks passed on the guard candidate. The verifier captured the
immutable application copy before starting eighteen guard cases and ten browser
journeys; their runtime results remain pending.

After that copy was released, two test-only key cases were appended to the same
guard target. Each covers absent, empty, whitespace and invalid UUID submission
keys for pause or resume, with full original draft and source/period no-write
checks. There is no local key-validation product change yet. Focused request:

```fish
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatment_pause_guards
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatment_pause_guards HOUSEHOLD_TEST_FILTER=requires_original_submission_key BROWSER_TEST_FILES=tests/household-treatments.test.mjs
```

The filter selects two HTTP cases from the twenty-case target. Calendar display
checks use a separate fixture and clock:

```fish
set -lx CONTRACT_DASHBOARD_NOW 2026-03-29T00:30:00Z
rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-treatment-calendar.test.mjs
```

The two-viewport actual administration draft remains outside the checkout at
`/private/tmp/household-f-20261001/household-treatment-administration.test.mjs`.
It uses real recording time, native selected-option assignment and taper forms,
actual stock deductions, and paused direct-dose rejection with unchanged
source, stock, takes and pause history. Native numeric option IDs and the direct
write error contract were checked against existing code. Permission-loss draft
work remains separate and must run last in its own fixture.

This is a compile and renderer check request, not final acceptance. Root owns the
separate Task selector repair; runtime selection will use its verified forwarding
or the explicit exported environment until that repair is accepted.

The first screen compile found two wiring errors: the overview export visibility
and an unavailable direct Decimal import. The child overview function now uses
the web-pages visibility boundary, and decimal selection uses the existing SeaORM
re-export. The next compile passed. Clippy then found a complex choices tuple and
an owned-string comparison; the choices now have named fields, and unchanged
numeric configuration compares its formatted string slice. These narrow repairs
remain pending the renewed lint gate.

Review found that the form adapter sends unsupported null values for empty
optional fields. The API contract supports null only for clearing minimum-hours;
blank maximum and cycle must preserve existing settings or API defaults. Notes
and frequency support empty strings. Schedule end date is required. The API
contract remains unchanged.

Five test-only regressions are added before correcting the adapter. Two create
ordinary assignment/schedule forms with optional guidance left blank; the
schedule has its required end date and expects the canonical default maximum of
four. Two edit real API-created assignments/schedules with an absent cycle,
preserving that absent cycle and the original dose/configuration/dates. A fifth
submits a blank required end date and requires 422, the full draft including the
original token/key, and unchanged saved data. The new HTTP target therefore has
14 cases. The captured run passed the original nine and failed all five added
regressions at their intended assertions. Ordinary blank-guidance creates and
null-cycle edits returned 422 instead of 303. The blank required end date was
rejected but replaced the unused original submission key. Actual assertions are
retained in `/private/tmp/household-f-20261001/F-TREATMENT-RED-003/runner.raw.log`.

The adapter now omits blank maximum and cycle, sends empty notes/frequency as
strings, and preserves null only for the supported minimum-hours clear. It checks
a blank required end date locally, keeping its unused key and full draft. Genuine
canonical API validation replies still issue a new key so correcting those cached
failures can succeed. All five locales explain the schedule default of four and
keeping existing maximum/cycle settings; the end-date label states it is required.

After source release, the ten browser journeys were strengthened to save actual
type-specific edits for all seven schedule types and compare exact saved/reopened
configuration. They capture assignment validation drafts and exercise assignment
and daily-schedule pause/resume/history with escaped notes and original period
identity. They remain ten journeys and do not alter the older captured input.

The form repair and strengthened browser file are stable for this renewed request:

```fish
rtk proxy task api:fmt:write
rtk proxy task -d rust/web format
rtk proxy task api:check
rtk proxy task api:clippy
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatments
rtk proxy task -d rust/web test
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatments BROWSER_TEST_FILES=tests/household-treatments.test.mjs
```

The wrapper selects fourteen HTTP tests before the ten browser journeys. Root's
Task forwarding regression and the actual fourteen-case run verify these CLI
selectors. A new eighteen-case pause guard target is separately drafted outside
the captured source; no private guard implementation is included. Calendar/timezone
checks require a separate fixed-clock fixture using the existing clock seam.
Permission-changing checks remain separate and last.

The repaired form run passed all fourteen HTTP cases. All ten browser journeys
then stopped at the same incorrect future-schedule expectation: resume was
required to make `active` true although the daily schedule starts in 2030. The
canonical representation combines the unpaused state with the current date
range. The captured editor screenshot confirms its original 2030 dates. The
test now requires restored original date eligibility, `paused:false`, unchanged
dose/configuration/dates and the same completed pause history. No product change
is made for this assertion. A focused browser-only retry selects the same ten
journeys with `HOUSEHOLD_ACCEPTANCE` unset and the explicit browser filename;
the healthy fourteen HTTP cases are not repeated for this test-only correction.

## Pause and resume preconditions

The reviewed eighteen-case test-only target is now
`rust/contract-tests/tests/household_treatment_pause_guards.rs`. It covers stale
assignment/schedule pause and resume, four deterministic source-read/write
interleavings, missing and whitespace source/period tokens, deliberately
mismatched period tokens, original source kind/identity mismatches, authorised
exact replay, payload conflicts and public API compatibility without browser
source tokens. The mismatched period-token case does not claim a real concurrent
period edit. Original period identity is preserved even on closed-period replay.

Disposable source-SHOW audit gates pause only the current browser session after
the original source row has been serialised. A real owner API write changes that
source before the browser action proceeds. Rejection assertions compare exact
source/history and scoped source versions/sync plus pause-period versions/sync.
The trigger and advisory gate are cleaned up; there is no production test hook.
No permission/profile change is made in this target.

The next test-only request is:

```fish
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatment_pause_guards
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatment_pause_guards BROWSER_TEST_FILES=tests/household-treatments.test.mjs
```

Expected selection is eighteen HTTP cases. Their actual guard failures should
stop the wrapper before browser execution. The private source guard is still
absent; implementation waits for these observed assertions. This input follows
the separate captured fourteen-form run and does not change that running copy.

## Remaining work

### Pause protection candidate

The eighteen-case guard run reached all assertions: two public API controls
passed and sixteen cases failed. Thirteen failures were unprotected original
source or period preconditions. Three failures occurred during rejected-draft
readback after valid conflict responses: the form omitted its required original
source identity fields. The unavailable test UUID import was replaced with a
distinct valid deterministic UUID before this runtime capture; no dependency
was added.

The candidate adds a private typed browser source guard to canonical pause and
resume, checked after current permission and authorised exact replay under the
existing household transaction. It compares original source kind, portable ID
and representation token. Resume additionally requires its separate original
period token. Public calls without the extension preserve their existing rules.
Original identity fields are rendered and retained; the earlier HTTP helper now
submits the actual form's identity fields. Verification remains pending.

The ten-browser retry now compares a resumed future schedule's eligibility with
its original eligibility, while still checking it is unpaused, its configuration
and dates are unchanged, and exactly one closed history entry exists.

The reviewed projection-only calendar file is now
`rust/web/tests/household-treatment-calendar.test.mjs` (two viewport cases).
It requires the isolated `CONTRACT_DASHBOARD_NOW=2026-03-29T00:30:00Z` clock and
restores the original profile timezone. It does not claim dose recording proof.

Stable candidate fast checks:

```fish
rtk proxy task api:fmt:write
rtk proxy task -d rust/web format
rtk proxy task api:check
rtk proxy task api:clippy
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatment_pause_guards
rtk proxy task -d rust/web test
```

The split, screens and private pause guard are implemented; final acceptance
remains pending. The filtered retry-key run reached both actual failures:
pause and resume accepted a missing key with 303 instead of 428. The candidate
now validates the original UUID locally before canonical dispatch and retains
the untouched draft. Public API keys remain optional.

The coordinator approved a small test-composition target containing only three
path module declarations: `household_treatment_acceptance` selects fourteen
forms, twenty pause guards and three boundary cases. The boundary cases cover
missing edit tokens and foreign person/medication/option rejection, including
an unrelated visible option. Single-page collection equality is deliberately
limited to the small isolated fixture, not general pagination evidence.

The strengthened calendar file now checks all seven types in both viewports.
Its profile changes are restored and its fixed dashboard clock remains separate
from the real-time administration file. The one-case permission target creates
an ordinary member's current manage grant, proves native assignment and daily
schedule creation, rejects wrong CSRF, and then revokes access. Captured writes
and an authorised prior replay must fail with unchanged source/history and eight
source/period version/sync counts. That target runs last in a separate fixture.

### Final stable verification request

All application, test and translation inputs are frozen for formatter/capture.
No Task or runner changes are part of this composition. Compile before fixtures:

```fish
rtk proxy task api:fmt:write
rtk proxy task -d rust/web format
rtk proxy task api:check
rtk proxy task api:clippy
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatment_acceptance
rtk proxy task api:contract-selected-compile TEST_TARGET=household_treatment_permissions
rtk proxy task -d rust/web test
```

Core fixture, with dashboard override unset and API timezone UTC: thirty-seven
HTTP cases followed by ten editor and two actual-administration browser cases:

```fish
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatment_acceptance BROWSER_TEST_FILES='tests/household-treatments.test.mjs tests/household-treatment-administration.test.mjs'
```

Separate projection-only fixture: two browser cases, all seven types, local
date/DST/taper assertions and restored timezone:

```fish
set -lx CONTRACT_DASHBOARD_NOW 2026-03-29T00:30:00Z
rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-treatment-calendar.test.mjs
```

Separate final permission fixture, dashboard override unset: one HTTP case,
then the seven existing medication browser regressions:

```fish
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_treatment_permissions BROWSER_TEST_FILES='tests/medication-journey.test.mjs tests/medication-journey-form.test.mjs'
```

Those existing browser filenames match the Task's current default selectors.
Full `rtk proxy task ci:rust-port` and independent written acceptance remain
required after the bounded checks. No F acceptance is claimed yet.

### Calendar fixture repair

The composed core runtime passed all thirty-seven HTTP cases and all twelve
editor/actual-administration browser cases. Raw evidence is
`/private/tmp/household-f-20261001/F-FINAL-GREEN-001/runner.raw.log`;
all 375 copied input paths matched. The verifier preserved and checksum-verified
102 screenshots under
`docs/screenshots/journey-medication-rust/f-20261001/final-core-green-001/`.
This proves actual current taper administration, including the recorded
`0.75 ml` and stock deduction; it does not yet establish dashboard display or
the separate permission fixture outcomes.

The first calendar run failed in both viewports at the seventh native source,
the PRN creation (line 111), after six preceding saves. Its explicit minimum
interval was zero; canonical schedule `parse_interval` accepts only whole
positive values or null. This is a fixture setup failure, not evidence about
date, daylight-saving or taper projection. Raw browser evidence is
`/private/tmp/household-f-20261001/F-CALENDAR-GREEN-001/browser.raw.log`.
No retained error page was captured by the original status-only helper.

Independent review approved the narrow test-only minimum change from zero to
one. The unique PRN source has no takes, so it remains available; maximum three
and every date/DST/taper assertion are unchanged. Unexpected create failures
now include only the visible alert text, without auth fields or full HTML.
The focused retry remains the same two-case, separately clocked browser request.

The second calendar run passed every native creation and reached the PRN card;
presence and `1.25 ml` passed, but its case-sensitive `Available` substring
assertion failed in both viewports before taper readback. The renderer emits
`Available now`, and CSS transforms the badge to uppercase in visible browser
text. Independent review approved a stricter contract assertion: exact card
`data-state=Available now` plus trimmed/lowercased visible badge `available now`.
Safe diagnostics include actual state and card text. Remaining time/quantity
assertions have no CSS case transformation and are unchanged. No eligibility or
projection product change is made. Raw evidence is
`/private/tmp/household-f-20261001/F-CALENDAR-GREEN-002/browser.raw.log`.

The third calendar run reached taper readback but its singular locator matched
four routine cards, all showing `1.25 ml`. The dashboard fetches occurrences only
for the fixed current local date; these are current-day positions, not future
cards. Source inspection confirms the formatter always uses the base source
dose, even during an effective taper step. This is tracked in issue #2360.
Independent review approved a stricter test: exact selected person heading,
routine task identifiers and exact medication text; require nonempty rows and
compare every current-day amount with ID, time and state diagnostics. The
corrected two-case amount assertion must run before the display fix. The fix is
prepared privately and remains absent from production pending that actual RED.

The separate permission run stopped at native assignment creation: the newly
created medication had no assignment making it visible to the ordinary member.
The person manage grant was valid, so the form existed, but canonical medication
scope correctly returned 404 for that forged stock ID. This is seed setup,
not a permission-policy failure. A reviewed correction will use existing
fixture stock already visible through a current person view grant, and assert
both member GET and an actual native select choice before posting. Revocation
and exact source/period/history/version/sync rejection proof remain required.

After strict calendar capture release, the permission seed correction was
applied with independent setup approval: existing `dose_write_medication_id`
is linked to the member's view-granted managed person. The test now explicitly
checks its real form select membership and member GET200 before posting to
the new managed person. No public scope or product permission rule changed.
The isolated one-case target and seven prior browser selectors are unchanged;
compile the corrected target before its focused retry.

### Precise taper display RED

The corrected strict calendar run reached the actual current-day amount
comparison in both viewports: expected `0.75 ml`, observed `1.25 ml`, with time
`Anytime` and state `Available now`. Raw evidence is
`/private/tmp/household-f-20261001/F-CALENDAR-RED-004/runner.raw.log`;
all 375 copied paths matched. This is the required product RED for #2360,
independent of earlier setup, casing and locator failures.

The reviewed minimal presentation draft uses existing current-step selection
and the canonical dose configuration amount/unit aliases and fallback. Only
two existing parser helpers need crate-private visibility; their bodies and
comments remain exact. Historical take displays, dose recording, occurrence
generation and public API response fields are unchanged. Before files are
preserved privately as `dose-before-2360.rs` and
`dashboard-projection-before-2360.rs` under the F evidence directory. The
permission source capture must release before applying this candidate.

That capture was validated and released. The reviewed candidate is now applied:
one taper-only display formatter and three current-task call sites, plus exactly
two `pub(crate)` visibility changes in existing dose parser signatures. A direct
before/after comparison confirms parser bodies and comments are unchanged.
Non-taper and historical formatting still uses the original formatter. No claim
is made about recurrence times or position counts; those paths are unchanged.

The stable fast-check request is:

```fish
rtk proxy task api:fmt:write
rtk proxy task api:check
rtk proxy task api:clippy
```

Then run the focused calendar two-case GREEN with its process-scoped clock,
followed by the existing `api:browser-dashboard-rust` regression. Production
and selected test inputs are frozen for capture. The earlier thirty-seven HTTP
and twelve actual-administration/editor passes remain recorded separately;
permission retry and final display verification are still pending.

After fast checks, rerun the same two calendar cases and the existing meaningful
dashboard regression using `rtk proxy task api:browser-dashboard-rust`. That
selector runs `tests/dashboard.test.mjs` and
`tests/leptodon-dashboard.test.mjs` with its established isolated clock, including
taper limit, minimum interval and gap checks. Full Rust verification remains
required before publication.

### Shared taper time controls (#2361)

The taper editor currently exposes only times stored inside each step. The
canonical occurrence builder uses the schedule's top-level `times` list instead.
This can make a saved step time appear to control doses when it does not. The
form needs shared dose-time controls and a distinct description for retained
step-time metadata, preserving the existing API recurrence rule.

The test-only draft is
`/private/tmp/household-f-20261001/taper-times-draft.test.mjs`. Its two viewport
cases use the isolated dashboard clock `2026-03-29T00:30:00Z` and restore the
original profile timezone. They require native creation at 07:30, native editing
to 08:15 and 21:00, exact current-day task times and effective `0.75 ml` amounts,
unchanged nested step metadata and base dose, reopened controls, distinct timing
labels, and invalid-time draft retention with an unchanged saved source.
This draft preceded setup review. No production timing change has been made.

The reviewer passed the two-case setup, including clearing an existing times
list back to canonical untimed dose slots while preserving all step metadata.
The stable file is now `rust/web/tests/household-taper-times.test.mjs`. Its clock
precondition checks the observable London dashboard date `Sunday, Mar 29`;
the browser service does not receive the API-only clock environment variable.
Production remains unchanged for this test-only RED request:

```fish
begin
    set -lx CONTRACT_DASHBOARD_NOW 2026-03-29T00:30:00Z
    set -lx API_TIME_ZONE UTC
    set -lx HOUSEHOLD_ACCEPTANCE false
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-taper-times.test.mjs
end
```

The expected first assertion is the missing native top-level `time_0` control,
checked explicitly before filling it. No HTTP selector, Task change or new
dependency is part of this request. Actual runtime evidence remains pending.

The first timing run stopped before the intended control assertion: CSS renders
the dashboard date as `SUNDAY, MAR 29`. The date precondition now trims and
lowercases visible text before exact comparison with `sunday, mar 29`. This is
a test-only setup correction. The remaining assertions were checked against
the stylesheet: dose and time text have no case transformation, status is read
from its semantic attribute, and labels are compared for distinction. No timing
production change or missing-control runtime result is claimed yet.

The corrected timing run reached native creation in both viewports and failed
on the absent `time_0` control (zero controls instead of one). Clock setup
passed. The actual failure is retained in
`/private/tmp/household-f-20261001/F-TAPER-TIMES-RED-002/runner.raw.log`;
project `mtcontract-2fec15ba18ae42f6` had matching copied application inputs.

The candidate now renders shared taper time controls before the step editors,
and writes their values to the canonical top-level `times` list. An existing
missing key stays missing when no shared time is entered; clearing a saved list
writes the supported empty list. All nested step data remains preserved. The
five languages explain that shared times determine when doses become due and
that retained step instruction times do not. There is no recurrence, recording,
stock, permission or public API change. The ten editor journeys now create,
edit and reopen shared taper times as well as their existing seven-type edits.

Stable final verification request:

```fish
rtk proxy task api:fmt:write
rtk proxy task -d rust/web format
rtk proxy task api:check
rtk proxy task api:clippy
rtk proxy task -d rust/web test
begin
    set -lx CONTRACT_DASHBOARD_NOW 2026-03-29T00:30:00Z
    set -lx API_TIME_ZONE UTC
    set -lx HOUSEHOLD_ACCEPTANCE false
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-taper-times.test.mjs
end
begin
    set -lx API_TIME_ZONE UTC
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_treatment_acceptance
    rtk proxy task api:browser-rust BROWSER_TEST_FILES='tests/household-treatments.test.mjs tests/household-treatment-administration.test.mjs'
end
rtk proxy task ci:rust-port
rtk proxy task ci:markdown
rtk proxy task docs:build
```

The focused timing job selects two cases; the real-time core job selects
thirty-seven HTTP and twelve browser cases with the dashboard override unset.
Final results, independent review and publication remain pending.
