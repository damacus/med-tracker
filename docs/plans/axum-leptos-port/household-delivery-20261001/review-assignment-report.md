# Independent assignment and schedule review

The assignment journey now passes its requirements checks. People can save and
edit assignments and all seven schedule types, pause/resume treatment, and use
the saved dose without losing rejected entries. Requirements review is PASS.
Code quality/security review is PASS on the verified final source. Documentation
validation and publication remain separate checks. Accepted dosage and stock
reports remain unchanged.

This reviewer reads source and matching verifier receipts only. Product edits,
compiled/runtime checks and publication remain with their designated owners.
The [assignment brief](assignment-writer-brief.md),
[later journey requirements](later-journeys.md) and OpenSpec treatment scenarios
govern acceptance.

## Baseline and extraction setup

The proposed baseline wrappers select openapi_person_medication_writes,
openapi_schedule_writes and openapi_pause_lifecycle. Static runner inspection
confirms their historical rails argument dispatches to the Rust API image lane
and exact Cargo targets. The schedule option helper supplies its required
default_dose_cycle; the known missing-cycle direct SQL fixture issue is not
present in these targeted files.

The unchanged-source assignment baseline passed 6/11 tests and the schedule
baseline passed 3/10. All five assignment failures stopped at the allowed-key
assertion in openapi_person_medication_writes.rs:416; all seven schedule
failures stopped at its equivalent in openapi_schedule_writes.rs:332. The
authoritative receipts are
/private/tmp/household-f-20261001/F-PERSON-BASELINE-001/runner.raw.log and
/private/tmp/household-f-20261001/F-SCHEDULE-BASELINE-001/runner.raw.log.
Assertions after those failures did not execute. These are existing schema-helper
omissions, not failing tests for the new assignment journey.

OpenAPI Schedule properties at 7111–7120 and PersonMedication properties at
7507–7516 document optional can_record (boolean) and
eligible_stock_medication_ids (integer array). Current source_projection.rs
schedule_row/assignment_row emit both. The two test helpers omit them from their
allowed lists while correctly keeping them out of required lists. Approve a
test-only repair adding these two allowed keys and checking their documented
types when present; neither optional field is nullable. Exact patched-helper
review PASS: required lists and unknown-key rejection remain intact; each present
can_record must be boolean and each present eligible_stock_medication_ids must
be an array containing only as_i64 integer values. No public contract or product
change is needed.

Repaired baseline receipts independently read: F-PERSON-BASELINE-002 passed
11/11, F-SCHEDULE-BASELINE-002 passed 10/10. Both pre/post input manifests are
identical. Schedule runner source digest is
231828dc97cf0ce899f7f7dee6770e10c02a0fe594938314d6bb5157d04fe968;
its original run did not save an individual copied-input receipt, so this review
relies on frozen inputs, runner digest and unchanged pre/post manifest for that
run. No stronger copy-provenance claim is made. The unchanged pause baseline
F-PAUSE-BASELINE-001 independently passed 10/10 and is not repeated before
extraction. All three are existing behaviour baselines, not editor acceptance.

Supported extraction boundaries are attribute/config validation, scoped
person/source access, representations, persistence/replay, write orchestration
and sync adapters. Preserve public create/patch/put and sync entry points,
comments, original bodies and transaction/lock ownership. The additional
dose/occurrence/invitation/OAuth extraction remains after this journey's
acceptance; it does not interrupt the existing three prerequisite splits.

## Mechanical extraction review

The three existing write modules now separate input validation, scoped access,
representations/responses, replay, write orchestration and sync adapters. Pause
also separates read queries, locked persistence and legacy entry points; direct
assignments have a separate dose-resolution module. This gives each child a
concrete responsibility while shared domain types stay in its facade.

Requirements for the mechanical split: static PASS. Independent read-only source
comparison against /private/tmp/household-f-20261001/extraction-before/ confirms
all 98 top-level function bodies are byte-identical: 28 assignment, 28 schedule
and 42 pause functions, with no extra/missing functions. The accompanying
extraction-function-bodies.tsv hashes independently match body text including
its trailing newline. Function signatures are equivalent apart from the scoped
visibility changes and formatting. Attributes and comments are preserved. After
removing relocated functions and the new module/import wiring, each facade's
remaining imports, constants, shared structs/enums and impls is unchanged apart
from whitespace.

Quality/security for the mechanical split: static PASS. The 24 child modules are
private. Helpers are pub(super) for facade/sibling access; moved entry functions
are pub(crate) to permit the original pub(super) facade re-exports. Router calls
in api_routes.rs, sync dispatch/reauthorisation in sync_batch.rs and
portable_projection.rs:607 retain their original facade paths. No external API
surface or policy is added by that internal visibility change.

Transaction and replay order remain in the exact preserved orchestration bodies:
household locking and reauthentication, scoped source resolution and current
person manage permission precede authorised replay. Assignment and schedule
update keep their original optional If-Match check after replay; canonical pause
create keeps its absent public token contract, while resume retains its distinct
period token. Versions, request audit, sync events, idempotency storage and commit
remain on the same transaction. Sync adapters and legacy pause/reorder paths are
also byte-preserved. Existing allow attributes moved intact; no warning
suppression or comment change was added.

The subsequent narrow import cleanup also passes static review. The verifier's
F-EXTRACTION-FAST-001/api-check.raw.log reports ten unused-import diagnostics,
covering 25 individual helper bindings in the three facade wiring sections.
Only those bindings were removed; original crate/library imports and public
re-exports remain. An independent rehash after cleanup again matches all 98
function-body receipts with zero differences. The first compiled check completed
successfully with those warnings; a clean check and runtime regressions remain
with the verifier. No behaviour change or new warning suppression is introduced.

This verdict covers source preservation and coherent boundaries only.
F-EXTRACTION-FAST-002 compiled API check and warning-denying Clippy completed
successfully. This reviewer independently read api-check.raw.log and
api-clippy.raw.log and compared its unchanged pre/post input manifests, both
b5aec893f9660499e4d38f2df9d3194b451c4d2f1a501b19cc324739e26849ad.
Formatting success is reported by the sole verifier separately from these raw
reads. The existing proc-macro-error2 future-incompatibility notice remains in
the logs; no new local-code warning is shown. Post-extraction 11/10/10 baseline
runtime proof is still pending. New browser editors and the proposed trusted
pause guard are not accepted by this extraction review.

## Initial treatment test setup

The new household_treatments.rs nine-case packet uses real public login,
cookie-authenticated medication creation, genuine current manage access to the
fixture journey person, and API read-back. Its seven native configuration
payloads agree with canonical validation, including both taper steps and string
decimal values. The top-level whole-hour interval projects as 8.0; taper config
is stored as supplied JSON. The direct-assignment stale submission uses a fresh
submission key before the concurrent API update while retaining the original
edit ETag, so its expected conflict cannot be caused solely by an older key's
different payload. These setup boundaries pass static review.

The resume setup correction is independently confirmed: the same captured
vector now retains source etag, original pause_period_id, distinct period_etag
and submission_id, and is retried after closure. Source identity comes from the
unchanged addressed assignment route; the implementation must bind the period
to that source rather than adopt a latest/open period. Initial nine-case HTTP
and ten-case browser setup static PASS. Compiled and runtime evidence, including
dedicated stale-source/no-write and replay guard RED, remains pending.

The new household-treatments.test.mjs ten-case packet uses all five locales and
desktop/mobile viewports, real cookie/CSRF creation, bounded paginated API
read-back and native controls with JavaScript disabled. Static setup is sound
for the initial create/read-form RED. Its schedule paths create seven records
and inspect populated edit forms; they do not yet save schedule edits. Browser
assertions currently verify an alert and locale tag, rather than every translated
message. All-seven saved edits, full invalid drafts, assignment screenshots,
permission loss, original-token boundaries, date/timezone/DST administration
eligibility and strengthened pause replay/concurrency remain final acceptance
work. Initial setup review does not certify those pending requirements.

The amended HTTP packet adds four ordinary-blank and existing-null-cycle cases
(13 total). Setup review is PENDING two concrete test corrections. The shared
blank helper expects max_daily_doses null for both resource types, but canonical
ScheduleFields::new defaults an omitted maximum to 4; assignment creation leaves
an omitted maximum null. The schedule create success case must retain a valid
end_date: OpenAPI ScheduleCreateRequest requires it (7304), and canonical
ScheduleFields::validate rejects an absent end date (schedule_writes.rs:137).
The new null-cycle schedule API seed also omits that required end date and would
fail before reaching the intended browser edit. These findings were sent to the
writer and coordinator before capture. A separate blank-end 422/draft/no-write
check should preserve that existing validation rather than expect success.

Field-by-field adapter ruling: supplied null is valid only for the nullable
min_hours_between_doses field, including its explicit clear semantics. Blank
notes and schedule frequency must be supplied as empty strings, which both
canonical parsers accept. Blank max_daily_doses and dose_cycle must be omitted
to preserve existing values/defaults; their optional request fields are not
nullable. A supplied end_date must be a valid date string and cannot be cleared
with null. It is required on schedule creation; omission on update preserves
the original date. OpenAPI request types at 7317–7330, 7373–7386, 7595–7602 and
7638–7645 agree with these supplied-value rules. API-created null cycles are
valid when cycle is omitted; the new edit cases correctly retain the original
source ETag/key and compare persisted dose/config/dates. Empty notes/frequency
expectations are valid. Their actual runtime outcomes remain pending.

The corrected 14-case packet now passes static setup review. Successful schedule
creation and the null-cycle seed/edit supply 2030-04-30 as a valid end date;
blank maximum expectations distinguish schedule default 4 from assignment null.
The separate blank-end rejection starts from an API-created valid schedule,
supplies otherwise valid guidance, requires 422 with every submitted field and
original ETag retained, and compares the entire saved representation unchanged.
The coordinator clarified the agreed boundary: this required-field rejection
must occur locally before API dispatch, so its unused original submission key
is retained. The current adapter instead dispatches invalid null and rotates
the key on API 422; that is an intended product RED, not a setup mismatch.
Genuine API 422 responses continue rotating cached keys for corrected payloads;
409/428 retain the original key. No other seed/assertion blocker was found.
Compilation is reported by the sole verifier; actual runtime evidence remains
pending. This setup PASS does not accept the incomplete adapter or pause guard.

## Public wrapper selector regression setup

The root-owned household_selector_test.fish extension exercises real public
api:browser-rust Task execution with only the downstream fish runner replaced
by a controlled PATH probe. The probe verifies the intended runner invocation
and records six actual environment values. Exact expected/actual receipt
comparison tests defaults, explicit false flags, explicit CLI file/filter/browser
selectors and inherited export compatibility. Existing strict Cargo dry-render
assertions remain intact. This is meaningful observable forwarding coverage;
it does not invoke Docker or simulate Task's variable resolution.

The generated temporary directory is uniquely named and removed on fish_exit
only when its narrow path pattern matches. Expected empty values remain separate
arguments and six receipt lines. Current Fish primary documentation confirms an
out-of-bounds positive-start/negative-end slice such as argv[8..-1] returns an
empty list when no extra arguments exist; the initial reviewer concern about
that slice was withdrawn after checking the documented example.

One verified setup blocker at first handoff: the copied executable probe has
#!/bin/fish, while this host resolves fish to /opt/homebrew/bin/fish and has no
executable /bin/fish. The generated shim must use the resolved real interpreter,
without recursive /usr/bin/env fish resolution through its own PATH entry.
The exact setup repair passes static review: command -s fish resolves the real
interpreter before PATH replacement; a generated header uses that absolute path
and appends the probe body. There is no fixed /bin/fish or env recursion. The
optional argv[8] guard was added for clarity. Test-only setup verdict PASS;
production Task remains unchanged and actual forwarding RED is pending.

The actual F-SELECTOR-RED-001 receipt was independently read: all five requested
household selectors were blank at the probe while the browser filename was
forwarded, so this was a forwarding failure after successful setup. The root's
subsequent Task patch passes static review: api:browser-rust uses per-command env
assignments, each value passed through shellQuote, for the five household
selectors and browser filename list. Empty/false values and the original two
default browser files are preserved; no global environment-precedence experiment
or runner logic change is introduced. The added CLI-over-conflicting-export case
and exact space/semicolon/double-quote/apostrophe filter case check real runner
values. Actual GREEN remains pending.

## Initial screen source review

People can reach ordinary treatment editors from their person page, and the new
native controls cover all seven schedule shapes. Initial source review found one
blocking problem: blank optional fields are encoded as null even where the
existing APIs require a supplied value to be a string or positive integer.
Default browser saves would therefore fail. Requirements remain PENDING and
quality/security acceptance is withheld until this adapter defect and the
remaining guard/permission/eligibility evidence are resolved.

Confirmed static P2 finding, recommended labels rust and bug if tracked beyond
this implementation: web_pages/treatments/payload.rs always inserts null for
blank max_daily_doses, dose_cycle and notes; schedule also inserts null for blank
frequency and end_date. Existing assignment input::positive_integer and
optional_string reject present null, and its dose_cycle enum requires a string.
Schedule apply_attributes likewise rejects null max, cycle, notes and frequency;
parse_date rejects null end_date. The initial ten browser journeys leave max
blank and schedule frequency blank, while the nine HTTP helper common values
supply max 4 and frequency, masking these ordinary defaults. Editing an existing
null cycle would also submit an unsupported null. Send only values supported by
the canonical request contract, with meaningful ordinary-default and
existing-null-cycle regression evidence; do not relax the API. Null
min_hours_between_doses is supported and retains its explicit-clear semantics.
The writer and coordinator were notified before acceptance.

Other initial source boundaries inspected: current people/manage_ids drive form
access and person-page affordances; canonical writes recheck actual current
person grants under their existing locks. Source member reads compare person_id
to the addressed route, so a source for another person cannot be edited through
this form. Cookie Origin and current CSRF are checked before adapter writes.
Update sends the exact submitted ETag and idempotency key; rejected forms retain
that ETag rather than adopting the context GET token. A fresh submission key is
generated after 422 to allow a corrected payload following a cached validation
response; conflicts retain the existing key.

Schedule editing expands original JSON into native times, weekdays, dates and
taper steps, with original config retained in an escaped hidden field. The
builder retains untouched config and taper amount/unit aliases while overlaying
submitted fields; weekly aliases are preserved when selected days do not change.
No JavaScript is required for adding/removing rows or applying a selected dose.
Dose selection filters to the chosen medication before populating the snapshot;
canonical ownership/snapshot validation remains authoritative on save. Actual
saved-edit and full-draft evidence is still pending.

Renderer strings and hidden values are built with escaped Leptos nodes and
existing field helpers, including original config, notes, medication/person
names and unavailable submitted selections. Error messages use the existing
safe form_error path. Existing page response/cache/CSRF helpers are reused.
New treatments namespaces are present with aligned controls/types/weekdays in
all five locale files; actual translated content and desktop/mobile accessibility
still need runtime and visual evidence.

Pause/history resolves the addressed source, displays authorised paginated
periods and retains original period ID/token on resume. It verifies that the
submitted period belongs to the addressed source before calling its canonical
resume route. The private source guard is intentionally absent in this initial
handoff and is not accepted as an implemented safeguard. Original source-token
validation, required browser replay/period tokens, dedicated concurrent/stale
RED and no-write outcomes remain outstanding before final pause acceptance.

## Pause form compatibility ruling

A pause form needs to reject a treatment changed since the form was opened.
The public pause-create API currently does not inspect If-Match. Its documented
parameters contain household and idempotency key only. Canonical resume accepts
an optional pause-period If-Match; this is a different token from the assignment
or schedule ETag.

Source evidence before extraction: pause_lifecycle.rs create at 1179 resolves
the source and checks current manage access before keyed replay; resume at 1322
does the same for the addressed period, then compares its optional nonempty
If-Match against period_body. Public OpenAPI paths at lines 2118 and 2192 agree
with this distinction. source_body at 413 uses the same authorised assignment/
schedule serialization and wrapped representation shape as their member reads.
person_medication_writes.rs and schedule_writes.rs use the existing household
lock before mutation, as do pause create/resume and legacy source actions.
mutation_idempotency.rs:42 locks the household and then reauthenticates.

Recommended ruling: approve a private typed request extension carrying the
browser's original source kind, portable identity and canonical source ETag.
The canonical handler should reauthenticate under its existing household lock,
resolve the visible source and current person manage grant, honour authorised
exact replay, then compare original source identity/version before mutation.
Requests without the extension retain existing public behaviour. Public headers
or JSON must not be able to create that trusted extension. Keep source mutation,
pause history, audit and sync within the same existing transaction and lock order.

Reject a missing/blank browser source token with 428 and a stale or mismatched
identity/version with 409, preserving the exact submitted token, reason, note
and replay key. Do not adopt the newly read context token in a rejected form.
Canonical resume must also retain and forward its originally captured period
ETag, distinct from a browser source ETag. Guard evaluation after authorised
exact replay allows a retry of the successful pause to survive its own source
version change, while current grant checks still prevent replay after access loss.

Require actual stale and deterministic concurrent-change no-write evidence,
missing/blank draft retention, exact replay after the original mutation,
changed-payload conflict, current permission loss, and public API compatibility
before accepting this additive browser guard. This is a concrete design ruling,
not acceptance of an implementation or runtime outcome. Exact source line
references will move during the approved extraction.

## Private pause guard test setup

The eighteen-case draft at
/private/tmp/household-f-20261001/pause_guard_draft.rs passes static setup review.
Assignment and schedule seeds use genuine canonical writes; schedules supply
the required end date. Its imported current-bearer helper has the compatible
fixture/login imports and provides a separately authenticated real bearer for
concurrent source changes. Source token and source identity are taken from the
original addressed source; resume independently retains its original period ID
and token. Forthcoming hidden identity fields and guard rejections are intended
RED assertions, not claims that the current UI already implements them.

The four source-change interleavings gate the browser session's source SHOW
audit before insertion, after the authorised source row is serialized. The
actor/household/controller/method/status/session scope and positive advisory-lock
wait select the submitted browser read, while the owner bearer PATCH updates
notes. The BEFORE audit gate avoids holding the audit insert's household foreign
key lock while that mutation runs. Its bounded wait and Drop release/teardown
are consistent with the existing test-only gate; no product hook is introduced.

Replay cases retry identical original vectors after pause/resume changed the
source version, and resume still addresses the original closed period. Changed
pause notes under the same key exercise a genuine canonical payload conflict.
Missing/blank source and period tokens, mismatched source kind/identity and
separate mismatched period token test the intended private browser contract.
The period-token stimulus is an invalid mismatched token string, not an actual
concurrent period mutation. Public canonical requests without source tokens
provide legitimate absent-extension compatibility checks.

No-write assertions compare the complete source and period history, source
versions/sync counts and period versions. The later fourth count now explicitly
checks household-scoped MedicationPausePeriod sync events using the internal IDs
of periods belonging to the addressed source. That strengthening passes static
review and closes the earlier period-sync count gap.
Current permission loss remains a separately required packet. Static setup PASS
does not certify compilation, actual RED/GREEN, the unimplemented guard or final
requirements/security acceptance.

## Bounded form repair review

The initial blank-field defect now passes scoped static source review. The
adapter omits unsupported blank maximum/cycle fields, sends notes/frequency as
strings and retains supported null clearing for minimum hours. Blank required
end dates return local 422 before canonical dispatch. Local rejection renders
the unchanged draft and unused key; only an actual canonical 422 response
replaces its cached submission key. Original ETags and keys remain unchanged
on 409/428. Existing current-person permission, Origin, CSRF, route ownership
and original-token forwarding boundaries are preserved.

The typed schedule flag chooses default-four help only for new schedules;
assignment maximum help remains optional, while edit help explains preservation
of the current maximum/cycle. Required end-date and these help entries exist in
all five locale trees and are wired through escaped renderer helpers. Visual
and runtime confirmation remain pending.

The strengthened ten browser journeys now actually save and reopen all seven
schedule edits in each locale/viewport. They compare the exact resulting config,
notes/frequency and unchanged source dose/effective dates; PRN also checks its
changed limits. They retain assignment validation fields, read back the unchanged
dose, save a corrected assignment and exercise assignment/daily pause-resume
history with escaped notes and original source/period fields. This supersedes
the earlier create/read-form-only setup limitation. It does not prove actual
administration eligibility, permission loss, DST boundaries or completed guard
behaviour. Scoped requirements and quality/security static PASS for this repair;
overall F acceptance remains PENDING matching runtime and screenshots.

The subsequent narrow browser expectation repair passes static review.
schedule_row publishes active only within its effective date range, separately
from paused, which reflects the raw source state. A future 2030 schedule must
therefore resume with paused false while retaining its original date-ineligible
active value. Comparing resumed active to captured source.active is correct for
this fixture; exact original start/end dates were added alongside the existing
dose/config and single closed history checks. This is a test expectation repair,
not a product change or proof that the focused ten-browser retry has passed.

## Calendar projection setup

The separate two-viewport draft at
/private/tmp/household-f-20261001/household-treatment-calendar.test.mjs passes
static projection setup review only with an explicitly isolated
CONTRACT_DASHBOARD_NOW=2026-03-29T00:30:00Z job. That instant is 29 March in London
and 28 March in Los Angeles. The start-28-March every-other-day parity, selected
specific dates, inclusive taper step boundaries and missing 29-March taper step
match schedule_applies/schedule_config_on. scheduled_time_in_zone shifts the
nonexistent London 01:30 by the offset gap to 02:30. Native creates supply valid
dates, use authorised cookie/CSRF requests and read back persisted configs.

The profile read serializes a timezone string, including its UTC default, and
finally restores that original value. Keep this profile-mutating packet on its
separate fixture. Desktop/mobile width and screenshot checks are meaningful,
but no runtime or visual result is accepted yet. The draft does not submit doses,
compare take/history counts or verify stock decrements/rejected no-write.
dashboard_now is a debug/project-gated presentation override; dose recording
still uses real Utc::now. The coordinator agreed actual administration and
rejected-write proof must use a separate real-time fixture.

The checked-in guard target's compile repair uses the valid UUID constant
00000000-0000-4000-8000-000000000042 and explicitly asserts it differs from the
source ID. No dependency is added. The older private guard draft still contains
the UUID crate call; runtime capture must use the corrected checked-in target.

## Private source guard implementation review

The current BrowserSourceGuard and identity fields pass scoped static
requirements and quality/security review. Both canonical handlers retain the
existing household lock and reauthentication, resolve the authorised addressed
source/current person manage grant, then honour authorised keyed replay before
new-state guard checks. The guard compares exact source kind, portable identity
and the ETag of the same wrapped canonical source representation as member
reads. Resume additionally requires a separate nonblank period If-Match for
guarded calls, then applies its unchanged original-period comparison.

The browser keeps the originally captured period ID, verifies it belongs to the
addressed source, and can address that same closed period on exact replay.
Original source identity/ETag and period fields are forwarded from the submitted
draft and escaped in hidden controls; rejected forms do not adopt reread tokens.
The dedicated internal WebApi method inserts the crate-private typed extension.
Ordinary public JSON/headers cannot create it, and requests without the extension
retain existing public create/resume preconditions. Scalar adjustment keeps its
separate existing typed intent. Guard failures precede source/period/version/sync
mutation; ordinary denial audits may still persist.

One separate outstanding browser replay blocker remains: pause::save silently
omits a malformed submission_id header, and canonical key parsing ignores
empty/whitespace keys. With valid original source/period tokens, these requests
can mutate without a retry key. The coordinator agreed targeted missing/blank/
malformed-key RED followed by browser-only local 428 validation retaining the
entire original draft. Public API keys remain optional. This does not discard
meaningful eighteen-case guard evidence; it prevents final F acceptance until
the separate browser requirement is implemented and verified. No actual GREEN
receipt or final guard acceptance is claimed by this static review.

The two appended requires_original_submission_key cases pass setup review.
Their filter selects exactly those two names in the now-twenty-case checked-in
guard target. Each starts from valid current identity/source token (and original
period identity/token for resume), tests absent/empty/whitespace/invalid-UUID keys,
then requires local 428 with exact draft/key retention and unchanged source,
period history and all four version/sync counts. Current missing-key behaviour
is an intended RED. The preceding eighteen-case run uses its prior immutable
copy and is unaffected by these added source cases.

## Real-time administration setup

The corrected two-viewport draft at
/private/tmp/household-f-20261001/household-treatment-administration.test.mjs
passes static setup review. An initial portable-ID option selection was repaired
to the actual native control's numeric ID; an unsupported assignment-response
source_dosage_option_id assertion was removed without expanding the API.
The draft applies the selected option through native controls, then requires
an actual 1.25 take and option/parent supply 12.25 to 11.0. This remains meaningful
observable proof of selected stock consumption.

The native taper uses valid yesterday/today/tomorrow bounds and today's 0.75 ml
step, then requires one actual 0.75 ml take and parent stock 20 to 19.25 while
preserving the persisted config. After native pause, the forged direct take
requires 422, code unprocessable_content and message Cannot take medication:
paused, with complete source/stock/period-history/take readbacks unchanged.
The original code paused expectation belonged to the occurrence action and was
corrected to this direct endpoint's actual contract before capture.

Run on a separate real-time fixture with the dashboard override unset and the
API's default UTC timezone. These checks are distinct from fixed-clock calendar
projection and do not assume that presentation time changes recording time.
No actual run or administration acceptance is claimed yet; separate current
permission evidence and final Rust/visual gates remain pending.

## Guard and treatment editor evidence

Independent receipt review confirms F-GUARD-GREEN-001 passed all eighteen
selected HTTP guard cases and all ten native browser journeys, covering en,
cy, ga, es and pt at desktop/mobile. The HTTP results are in the outer
/private/tmp/household-f-20261001/F-GUARD-GREEN-001/runner.raw.log; the file
http.raw.log contains build output and is not the HTTP result authority.
browser.raw.log contains the actual ten-pass, zero-failure Node summary.
The 371-path copy-hashes.manifest and live-hashes.manifest compare identically.
The verifier records source digest
1c3083109189123ffee69a456c02caacc7068bcbd9f335c7e2c455b25b02519f
and fixture SHA
04c005434dedc285c114cd3b9693ea8df6a2d8e2500be9e5e85a1078ccf61692.

This batch proves the four controlled source interleavings, stale/missing source
tokens, separate period preconditions, original identity retention, exact
pause/resume replay and payload conflict, and the two public compatibility
controls. The later two missing-submission-key cases were outside this eighteen
case capture. The ten browser journeys actually save/reopen all seven schedule
edits and compare persisted configs, guidance, dose and effective dates. They
also verify assignment validation/corrected saving and assignment/daily pause,
resume and the single retained closed history period. The future daily schedule
correctly retains its original date-ineligible active value after resume.

All 100 archived checksums verify in
docs/screenshots/journey-medication-rust/f-20261001/guard-green-001/.
Representative visual inspection covered daily-en-desktop,
every_other_day-cy-mobile, multiple_daily-ga-desktop, prn-pt-mobile,
specific_dates-es-desktop, tapering-en-mobile, weekly-cy-mobile,
assignment-validation-ga-mobile, assignments-history-pt-desktop and
schedules-history-es-mobile. The forms show saved edited values, readable
translated labels/help, retained invalid dose and literal script-like history
notes. Form controls and history cards fit their viewports. This is a visual
PASS for these inspected screens, not a claim that all hundred images were
visually inspected or that screenshots alone prove keyboard interaction.

Some mobile images clip the final shared Locations navigation label. The shared
nav.med-sidebar has overflow:auto in medication.css:52, allowing horizontal
scrolling by source design. No explicit scroll cue is rendered. A static image
cannot verify the gesture, so this is a recorded shared-layout limit rather
than proof of an inaccessible action or a confirmed treatment bug.

Scoped requirements PASS for the eighteen-case source guard and native saved
editor/history journey. Scoped quality/security PASS for that captured work,
with the separate missing-key blocker still open. Overall F acceptance remains
PENDING missing-key repair/runtime, fixed-clock calendar projection, real-time
administration/rejected writes, current-grant revocation and final gates.

## Remaining isolated test setup

The private one-case household_treatment_permissions.rs draft passes setup
review. It creates an adult and a real manage grant for an ordinary member,
then exercises native assignment/daily creation, CSRF rejection, authorised
pause/replay and denial of captured actions after real public grant revocation.
Source/history and eight scoped source/period version/sync counters must remain
unchanged after denial. Cookie authentication reloads current membership, so
the creator's People permission-version bump does not create the stale bearer
setup failure seen in earlier work. Keep this grant-mutating packet separate
and last; no actual permission-loss runtime is accepted yet.

The strengthened private calendar draft also passes projection-only setup:
Sunday weekly inclusion, two multiple-daily local times and PRN availability
match the explicit March29 London/March28 Los Angeles clock. It does not submit
doses or change the distinction between presentation and recording clocks.

The three private household_treatment_boundaries.rs cases pass setup review:
missing original edit tokens retain every entry without changing the source;
foreign person/medication/option inputs are denied; a visible unrelated option
is rejected. The option seed includes its required cycle and frequency. The
single-page final collection equality is fixture-bounded, not a general
pagination guarantee. These drafts have no accepted runtime results yet.

## Retry key repair and composed acceptance setup

The actual F-RETRYKEY-RED-001 outer receipt selects two cases, with eighteen
filtered out. Both stop at 303 versus required 428 for an absent key; subsequent
retention/no-write assertions and the remaining loop inputs did not execute.
The verifier qualifies independent source-copy equivalence as unavailable for
this RED. Its emitted runner source digest and assertion log remain evidence;
the broad copy/live manifest pair must not be described as unchanged.

The local pause::save repair passes static requirements/security review.
After current access, origin, CSRF and addressed-source checks, it rejects an
unparseable submitted UUID with local 428 before canonical mutation dispatch.
The renderer receives the unchanged submitted draft, preserving source/period
identity, original tokens and missing/invalid key rather than inventing a new
key. Valid keys still reach the existing typed guard/canonical replay path.
Public API optional-key semantics and authorised replay-before-stale ordering
remain unchanged. This closes the source blocker; matching GREEN for both
cases remains required before acceptance.

household_treatment_acceptance.rs passes static composition review: its three
path modules select fourteen form, twenty guard and three boundary cases.
These create distinct medication/source rows without person, profile or grant
mutation. Current-bearer helpers revoke their own tokens, and advisory gate
guards release/drop owned database objects. Run serially. Fixed-clock profile
changes and permission grant/revocation remain separate fixtures. No thirty-seven
case runtime PASS is claimed by this setup approval.

The separate permission fixture may run the seven existing medication browser
regressions after its HTTP case. medication-journey.test.mjs has five cases and
medication-journey-form.test.mjs has two; both use fresh public primary-account
logins and browser cookies, with no fixture bearer token read or sent. Browser
authentication reloads current membership. The permission case's People create
therefore does not invalidate these cookie journeys, and grant creation/revocation
changes the separate view membership. Its new adult/medication/sources do not
change the existing journey stock/assignment fixtures. Sequencing setup PASS;
these seven regressions still require actual matching runtime evidence.

## Treatment error localisation audit

Static tracing finds no raw English fallback in the new treatment validation
views. The error summary and field errors use medication_management::messages,
which resolves known messages or the safe translated form_error fallback.
The referenced keys have local values in en, cy, ga, es and pt.

| Server-produced message | Rendered catalogue key |
| --- | --- |
| missing_browser_precondition (local missing retry/edit token) | medications.stock.original_token |
| Record has changed since it was last read (stale source/period) | dosages.management.conflict |
| is invalid (including locally required end date) | errors.messages.invalid |
| Other allowlisted blank/inclusion/number/range messages | Corresponding errors.messages key |
| If-Match is required (canonical missing source/period token) | errors.messages.form_invalid via safe generic fallback |
| Idempotency key has already been used for a different request | errors.messages.form_invalid via safe generic fallback |

The exact canonical guard 428 message at pause_lifecycle/responses.rs:16 lacks
a specific recovery mapping. It currently shows a translated generic failure,
not an English diagnostic. This is a guidance gap if original-form reload
instructions are required for every guard failure. Before changing it, require
targeted five-locale renderer RED using the exact If-Match is required stimulus
and the intended reload text. The browser invalid-dose screenshots prove the
ordinary invalid field case; HTTP guard assertions do not visually certify
these other messages. No mapping implementation or new runtime is claimed here.
The coordinator accepts the safe translated fallback for current F requirements
and queues this reload-guidance improvement as shared form polish in G, with
the focused renderer tests before a mapping change. It is not an F security or
English-leak blocker and does not require public API changes.

The first strengthened calendar run stopped during its seventh native schedule
create, before projection assertions: PRN supplied min_hours_between_doses zero.
schedule_writes/input.rs:63–82 requires a whole positive interval or null for
clearing. The earlier strengthened setup review missed this constraint. The
test-only correction from zero to one passes static review; maximum three stays
unchanged and the unique source has no prior takes, so its PRN availability
expectations remain meaningful. All strict local-date, DST and taper assertions
remain. This is a seed repair rather than a product failure, and the corrected
calendar runtime is still pending.

The subsequent calendar run passed PRN creation/presence/dose and stopped at
the case-sensitive availability substring assertion before later taper checks.
dashboard.rs:317–320 publishes Available as Available now, task_card stores
that exact data-state and badge text, and dashboard.css:67 transforms the visible
badge to uppercase. Requiring exact data-state Available now and trimmed,
lowercased .dashboard-status innerText exactly available now passes static
review. It preserves positive availability and is stronger than a substring
check; quantities and all date/taper expectations remain unchanged. This is a
test display expectation repair, not a projection/eligibility change. Later
calendar assertions remain unverified until the corrected run completes.

The next calendar receipt exposes a real display problem: the dashboard shows
the original 1.25 ml dose on all four current-day taper tasks when the current
saved step requires 0.75 ml. F-CALENDAR-GREEN-003/browser.raw.log records those
four values in its strict-mode diagnostics. dashboard.rs:209 requests only the
selected local today's occurrence window; dashboard_projection.rs:282 formats
every task from dashboard_dose(source), which reads only the base amount/unit
at lines11–14. This is a Rust display correctness finding (rust, bug), distinct
from recording-time taper proof and from the locator cardinality failure.

The test should scope the selected person's routine, exact medication heading
and current routine task IDs, require nonempty matches and check every dose,
with row ID/time/state diagnostics. Choosing an arbitrary first row or inventing
an 08:00 occurrence would weaken the evidence: the current occurrence generator
can create multiple untimed positions from the canonical maximum. Capture the
strict all-row wrong-dose RED before the presentation fix.

The existing occurrence representation does not publish effective amount/unit.
Do not expand the public API for this repair. Reuse existing source_config_on/
taper_step_on and the inclusive current-step amount/dose_amount, unit/dose_unit
aliases and source fallback used by dose::effective_source (dose.rs:159–200).
No new taper policy, stock or administration change is justified. Fix/runtime
and later local-date/taper-gap assertions remain pending.

The private calendar-strict.mjs setup now passes review: it reads the exact
authorised person's name, resolves one person card, scopes current routine
test IDs and exact medication text, requires nonempty matches and compares
every amount with ID/time/state diagnostics. All timezone round-trip and gap
assertions remain. This draft is ready for the actual two-case amount RED.

The private dashboard_projection_draft.rs also passes bounded static review.
Its new dose-on-date helper is taper-only and uses the existing source_config_on,
canonical config_decimal/config_value aliases, base source fallback and existing
decimal presentation. Three current-task display sites change; historical
actual-take formatting remains unchanged. Widening only the two canonical
helper visibilities within the crate is acceptable if their bodies/comments
remain exact. No public API, dose recording or recurrence behaviour changes.
This repair does not fix or prove taper recurrence times/count. Production
application and matching strict amount GREEN remain pending.

The isolated permission run exposed another seed visibility mismatch: a new
owner-created medication without a person source is correctly unavailable to
the ordinary member. The earlier setup review missed this medication scope
boundary. The proposed test-only correction uses dose_write_medication_id:
scripts/contract_provision.rb:577–579 gives the member a current view grant to
managed_person, and lines686–694 create that person's linked 1.25 ml medication.
medication_reads::scope makes this medication visible while the new adult's
manage grant authorises the treatment target. Require actual native select
membership and member medication GET200 before the POST.

The new adult avoids duplicate assignment. Assignment/schedule creation and
pause change only its source/period records, not medication supply or the
original seeded source. Later seven primary-cookie browser stock assertions
remain valid. Revocation, captured replay denial and all eight no-write counters
stay unchanged. This correction passes static setup review; the initial404 is
not a product defect, and corrected permission runtime remains pending.

The exact taper amount RED is now recorded in F-CALENDAR-RED-004/runner.raw.log
and full RTK tee1790894214_task_api_698f4f.log. Both desktop/mobile fail at the
first scoped current row with 1.25 ml versus 0.75 ml; the diagnostic shows
Anytime and Available now. Later loop rows and timezone assertions were not
reached, so this receipt is not a completed calendar result.

The applied display fix passes scoped static review against the saved
dose-before-2360.rs and dashboard-projection-before-2360.rs. dose.rs changes
only config_value/config_decimal to pub(crate); both bodies/comments remain
unchanged. The projection adds the reviewed taper-only helper and replaces
exactly three current-task displays. Its reference date is the dashboard's
now.with_timezone(timezone).date_naive(), matching the source/occurrence local
date boundary. Canonical aliases/Decimal parsing/base fallback remain intact.
Non-taper display and historical actual-take formatting stay unchanged, as do
recording, recurrence and public representations. Matching GREEN and visual
proof for this applied fix remain pending.

## Taper occurrence compatibility ruling for G

The four untimed taper tasks are supported by both existing implementations,
so they are not a confirmed Rust parity defect. Rails
MedicationAdministration::OccurrenceProjection#rows_on reads only top-level
schedule_config times (occurrence_projection.rb:123–130). If absent, untimed_rows
uses Schedule#expected_doses_on (lines133–137). Schedule#expected_doses_on uses
top-level configured_times, then effective current-step maximum or the source
maximum (schedule.rb:70–86,189). FamilyDashboard::ScheduleQuery also reads only
top-level times at schedule_query.rb:255–263.

Rust config_times likewise reads top-level times (dose_occurrences.rs:392–399).
The schedule projection calls it with the whole schedule config; when empty,
effective_count receives the current step config and source maximum
(lines597–620). With nested step times only, no step maximum and base maximum
four, both implementations therefore derive four untimed positions. This does
not say that the step times are used as occurrence times. The corrected amount
display is independent of this timing/count compatibility rule.

This ruling is a source comparison, not an executed Rails reproduction. Final
G should preserve a targeted parity case for nested-only step times and step
versus base maximum, rather than label the behaviour a speculative Rust bug or
change recurrence policy during the display repair. Any future change to honour
nested times needs an explicit compatibility decision and meaningful tests.

There is, however, a concrete new Rust form usability finding for G: people
enter a time inside each taper step, but the current APIs do not use those
nested values for occurrence cards. The form offers no top-level occurrence
time control. schedule_controls::taper renders step_{index}_time_{index} fields;
the tapering render branch bypasses the ordinary top-level time_ fields.
schedule_config::build writes taper_steps in that branch and only the ordinary
schedule branches write top-level times. Existing top-level values can survive
through config_original, but the taper editor cannot show or change them.

Label this rust, bug: a saved apparent administration time produces untimed
cards, despite the recurrence API behaving compatibly. G should expose truthful
top-level occurrence-time controls and explain/label any retained nested step
times, backed by a failing native round-trip and occurrence test before changes.
Do not change public recurrence policy to hide the form problem. This finding
does not change the separate amount-display fix or certify missing G work.

## Matching taper display GREEN and time-control test setup

The calendar screens now show the current taper dose correctly. Independent
review of F-CALENDAR-GREEN-004/runner.raw.log confirms both strict desktop and
mobile projection cases passed, with two tests, zero failures and task_exit=0.
The 375-path source premanifest and copied-input manifest are identical. The
runner emitted source digest
a6429066ce424d0348c3f6d3fe57945cf9a39cceeae29106ae38365c3601cabc;
the current dose.rs, dashboard_projection.rs and calendar test hashes still
match those captured inputs. Fixture rehash is
881af988e73bc358405529a215c399209ae69c517175cf605a6a5eb42996bd6f.
No independent full live postmanifest is present in this receipt directory;
this review does not invent one.

Both archived screenshot checksums pass in
docs/screenshots/journey-medication-rust/f-20261001/calendar-green-004/.
I inspected treatment-calendar-desktop.png and treatment-calendar-mobile.png:
current taper cards show 0.75 ml, the spring-gap task shows 02:30, and routine
cards fit both widths. The mobile full-page image is very tall, so its overview
has limited fine-detail legibility. The actual two passing cases retain all
strict date, timezone, taper and availability assertions. This is projection
and display evidence; the dashboard clock does not change recording time.

#2361 remains open before F acceptance. The private
/private/tmp/household-f-20261001/taper-times-draft.test.mjs setup passes static
review, provided its isolated job uses dashboard clock 2026-03-29T00:30:00Z.
I requested an explicit test precondition for that value. London 07:30 and
then 08:15/21:00 are future current-day slots, so exact Upcoming state is valid.
The test requires native top-level time controls directly, saves and reopens
one then two times, checks exact occurrence count/time/current-step 0.75 ml,
preserves nested 09:00/10:00 metadata and base dose, and proves invalid-time
422 retains the original token and draft without changing the saved source.
Its final clearing check expects top-level times=[] and the existing base
maximum of untimed Anytime positions, while preserving all step metadata.
These expectations follow existing recurrence rules; no policy change is
needed. Actual RED, the eventual form fix and matching runtime remain pending.

## Applied shared taper-time controls: static review

The taper editor now separates the times that set due tasks from the step
instructions saved with each dose change. The bounded applied candidate passes
static requirements and security review. Matching runtime and visual checks
remain pending, so #2361 stays open before F acceptance.

schedule_controls.rs renders the ordinary shared time controls before taper
steps, reusing the existing labelled, escaped inputs and add/remove intents.
Nested step inputs have a distinct instruction-time label and an explanation
that they do not set due times. The three new keys are present in English,
Welsh, Irish, Spanish and Portuguese; their wording preserves this distinction.
No new authorised data projection, raw user-content rendering or write path is
introduced.

schedule_config.rs builds top-level taper times from the shared time_ fields.
Nonempty rows write the times array. Blank rows preserve an originally missing
times key; blanking a previously present times array writes an explicit [].
Original config cloning and existing taper_steps handling preserve other
metadata and supported amount/unit aliases. Existing recurrence rules consume
these top-level times; canonical recurrence and recording code are unchanged.

The ten existing editor journeys retain all seven schedule types and their
actual save/reopen assertions. Taper now creates shared 07:30, edits to 08:15,
checks exact saved config, and also retains the previous step amount/time edits,
dates and base dose checks. The focused two-case time test remains unchanged
after its actual missing-control RED, except the already approved observable
clock setup uses normalized exact dashboard date text. Browser containers do
not receive CONTRACT_DASHBOARD_NOW; checking the rendered London March 29 date
therefore correctly replaces the earlier environment-precondition suggestion.
F-TAPER-TIMES-RED-002 reports two failures at the explicit native time_0 count,
observed zero versus one. Later persistence, timing and invalid-draft assertions
were not reached in that RED.

This candidate does not change permission, CSRF, original-token or replay
handling, public API schema, historical dose records or recurrence semantics.
Its source changes remain separate from overall F acceptance and final quality
gates.

## Focused shared-time GREEN and visual review

Taper times entered in the form now produce the intended dashboard tasks.
Independent review of F-TAPER-TIMES-GREEN-001/runner.raw.log confirms both
desktop/mobile cases passed: two tests, two passes and zero failures. This
executes native creation at 07:30, save/reopen/edit to 08:15 and 21:00, exact
current-day task count/time/effective 0.75 ml and Upcoming state, original step
metadata/base-dose preservation, invalid 25:99 draft/token retention with saved
source unchanged, and deliberate clearing to times=[] with canonical untimed
positions. The isolated presentation clock is separate from recording time.

Runner digest is
b7b76b52d9061904bebb33bca03f3357e970f32e903c1cf057dbf40c47367c62;
all 371 authored premanifest paths match the copied-input hashes. The copy has
five additional config/hidden paths, explicitly listed in source-copy.diff;
the two complete manifests are therefore not identical. Emitted fixture hash
is 5aa7fbeabfcfe6269efce99f697951cb8528deaefa050481fd66671b2faa9f24.
The authoritative wrapper log checksum is
e552d6b847bc875746107be37655fca3e80f522573df142441b4e767b4b4f944.
The verifier reports wrapper exit zero; the saved wrapper itself has no exit
marker. The separately linked Docker build tee is not browser assertion
evidence.

All six archived image checksums pass against screenshots.manifest, checksum
5b23d9dda1f6325418258d352ebc4d92b588dd2cf960ce6a1226693574012b61.
I inspected all six files in
docs/screenshots/journey-medication-rust/f-20261001/taper-times-green-001/:
desktop/mobile edit, due tasks and invalid draft. Shared controls and step
instructions are readable and visually distinct; 08:15/21:00 tasks display
0.75 ml and Upcoming; the rejected 25:99 remains in its control. Forms fit the
mobile width, including wrapped step actions and the save button. The tall
mobile dashboard overview has limited fine-detail resolution, while the
focused rows remain legible and exact DOM assertions passed. No new visual
blocker was found within this bounded check.

The focused #2361 requirement now has matching source, runtime and visual
proof. Overall F acceptance remains pending the final composed 37-case HTTP,
12-case browser and Rust/documentation receipts; this is not a final quality
gate verdict or a claim that all jobs shared one fixture.

## Final requirements evidence

Requirements verdict: PASS for the bounded F journey. Independent reading of
F-CORE-GREEN-002/runner.raw.log confirms all 37 composed HTTP cases and all 12
browser cases passed. The HTTP target retains fourteen form cases, twenty pause
guard cases and three original-token/foreign-record boundaries; nothing was
filtered out. Browser results name both real-time administration viewports and
all ten five-locale/editor viewports. Each locale journey saves and reopens all
seven editors, including shared taper times and exact type-specific config,
dates, dose and notes. The real-time cases prove selected-option consumption,
the current taper dose, and rejection while paused with unchanged source,
stock, pause history and takes.

Core wrapper checksum is
588181c504d5166e55d1afc2f3f3230262078694868fbdeec57d0b1670865a44;
runner copy digest is
b7b76b52d9061904bebb33bca03f3357e970f32e903c1cf057dbf40c47367c62.
Authored pre/post manifests match at
677bde3ee6254bbed9e893e97263d23af2d0846b4ef02b2db75e4ba11aace14d;
all 371 authored copied inputs match, with five separately listed extra
config/hidden paths. I also rechecked all authored input hashes against current
source: all match. Task input pre/post manifests match. Emitted fixture hash is
98221eb28948e56820740c972537444beca1055d1493c54ead2fce99d56ce12a,
independently validated by the verifier before cleanup.

All 102 core screenshot checksums pass under
docs/screenshots/journey-medication-rust/f-20261001/core-green-002/.
I inspected the refreshed Welsh mobile, Irish desktop, Spanish mobile and
Portuguese desktop taper forms and the mobile administration history, alongside
the earlier representative all-seven/five-locale form, validation and history
review. The new shared time and instruction distinction is translated and
readable, entered values are preserved, step actions wrap within mobile width,
and the paused history names its reason, actor and time. No new F visual blocker
was found. The previously recorded shared navigation layout limit is separate
G work, not proof of a newly broken treatment action.

F-PERMISSION-GREEN-002/runner.raw.log independently confirms the ordinary
current-manage-grant creation/pause/replay/revocation case passed 1/1, then
the seven explicitly selected fresh-login medication browser regressions passed
7/7; exit.txt records task_exit=0. Its 375-path copy receipt and fixture
provenance remain as recorded in the verification queue. A path/hash comparison
against current core inputs finds only the previously reviewed taper display,
helper visibility, shared-time controls/config, five locale files and editor
test changes. Permission, replay, API authentication and clinical write code
are unchanged, so that permission result remains applicable.

The separate fixed-clock calendar result is 2/2, focused shared-time result
2/2, and existing dashboard regression browser.full.log independently records
35/35. These jobs use separate fixtures and scoped clocks. Calendar covers all
seven projected schedule types, local-date changes, daylight-saving gap and
taper step/gap boundaries; real-time administration is proved by the core
cases, not by the calendar clock. Dashboard regression predates the shared-time
form change but its projection/recording inputs are unchanged by that change.

Final quality/security verdict: PASS on the verified final candidate. Static
review finds no unresolved F authorisation, CSRF, original
identity/token, keyed-replay, transaction/lock, draft privacy or public-contract
blocker. The reviewed extractions and private browser guards retain their
earlier scoped preservation rulings. Remaining planned module refactors,
known collection-cap/fallback-stock follow-ups and shared error-guidance polish
belong to G; they are not silently accepted or fixed by these F results.

## Final quality gate and review freeze

Independent reading of
/private/tmp/household-f-20261001/F-FINAL-RUST-001/ci-rust-port.raw.log
confirms successful format checking, warning-denying Clippy, SSR and hydration
builds, API/web unit and renderer tests, and contract-test compilation. All
reported test groups pass, including the API's 36 tests. The full log checksum
matches d7c8f81e55f55fe33769a2c3f14dbfe7542b09a03a7b28fdb698f221eb07c62d.
The final authored postmanifest exactly matches the runtime premanifest at
677bde3ee6254bbed9e893e97263d23af2d0846b4ef02b2db75e4ba11aace14d,
and Task input receipts also match. The sole verifier reports the full
ci:rust-port command exited zero. The saved log contains the underlying
successful commands/results rather than an embedded exit marker.

Existing dependency future-compatibility, Browserslist data and optional
fsevents script notices remain visible in the receipt; no warning suppression
or new dependency change is used to pass this candidate. The source remains
the reviewed and runtime-tested candidate. Independent requirements verdict
PASS and independent code quality/security verdict PASS are separate findings;
neither certifies the remaining G work or an unexecuted Rails suite.

#2359 forwarding, #2360 current taper amount display and #2361 shared taper
time controls are fixed and verified locally. Publication/issue closure is the
coordinator's responsibility. Remaining G work is the authorised 501-option
pagination fix (#2353), all-untracked fallback stock-loss choice (#2358), the
recorded #2347 compatibility/helper checks, accepted-journey CI selection,
the five planned responsibility splits (portable imports, dose recording,
occurrences, invitations and OAuth), and the recorded shared error-guidance
and navigation layout follow-ups. The longer audit-reason boundary is an
unverified planned check, not a confirmed 255-character schema defect.

This report is now frozen for the separate documentation validation and
publication packet. No further source review scope or optional polish is added.
