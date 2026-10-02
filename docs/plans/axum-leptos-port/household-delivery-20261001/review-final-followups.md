# Final journey setup and current PR follow-ups

Focused checks now pass for large medication-option collections, parent-stock
fallback, translated editing guidance and the minor's readable person page.
The page clearly marks forbidden schedules as unavailable. Broader collection
and permission checks, native desktop/mobile screenshots, compatibility and
planned refactors remain pending. This report does not accept final readiness.

## Initial four-case inventory setup

Corrected static setup verdict: PASS for
`rust/contract-tests/tests/household_inventory_completion.rs`. My initial PASS
missed the medication threshold rule: all four cases stopped in the shared
medication helper with 422 instead of 201. None reached its intended product
failure, so this run is setup evidence, not product RED.

The correction changes the parent medication's `reorder_threshold` from null
to the valid decimal string `"3"`. Current
`medication_management/validation.rs:125–150` requires this field on creation
and rejects null, while permitting null `current_supply`. Current
`dosage_options/validation.rs:139–168` explicitly permits null option supply
and threshold. Both helpers now capture the public JSON response alongside the
201 assertion to identify setup errors. Production code is unchanged; intended
null, zero and finite stock stimuli and regression assertions remain intact.

Real API seeds include dosage frequency, cycle, maximum and minimum defaults. Zero minimum is
valid for dosage options; this differs from the positive schedule interval
rule. Medication supplies remain explicitly null, zero or finite as intended.
Canonical medication decimal strings and separately normalized removal-history
quantities are asserted separately. Removal audit versions use medication.id
as item_id in the current API, matching the scoped counts.

The public legacy parent adjustment deliberately supplies finite parent stock
while zero/null option balances remain unchanged. This is an explicit fixture
stimulus, not a proposed browser or API policy change. Replay, conflicting
payload and insufficient stock assertions preserve source, option, history and
scoped version/sync evidence.

The large case copies required fields from two real seeds into unique disposable
parents: 199 extra plus one seed and 300 extra plus one seed. UUID and default
boolean columns have database defaults. Null option supply avoids inventing
stock or requiring an aggregate repair. The serial case runs last; no grants,
profile or person identities are mutated. Before HTML assertions it proves
complete authoritative pages, stable totals, unique IDs and foreign exclusion,
then exact parent option counts and full assignment choices.

Expected finite-parent fallback 422 versus 303 and large-page 503 versus 200
are intended product failures, not fixture mismatches. The other two cases are
negative controls that may already pass. Compilation and actual assertions
remain with the sole verifier. Browser/permission/final quality acceptance is
not supplied by static setup approval.

## Narrow stock and pagination candidate

The corrected run reached the intended behaviours: finite parent fallback
returned 422 instead of 303, and the large-option page returned 503 instead of
200. The null-parent and tracked-zero/mixed controls passed. The writer records
the authoritative receipt in
`/private/tmp/household-g-20261002/G-INITIAL-RED-002/runner.raw.log`; final runtime
acceptance is now supported by the focused GREEN and the combined inventory
journey receipt described below.

Requirements verdict: PASS for the bounded stock/pagination repair. The shared
`web_pages/api_client.rs` collection reader has no arbitrary five-page ceiling.
It requests canonical authorised pages of 100, verifies the reported page and
size, requires a stable total and unique row IDs, and returns only exact total
coverage. Missing metadata, duplicates, empty incomplete pages, excess rows or
changed totals fail closed. Current dosage, medication and shared resource
responses provide the metadata this reader requires. Existing callers use base
paths without a query, so the appended pagination parameters remain valid.

In `web_pages/stock.rs`, blank dosage identity is accepted only when every option
has null supply and the parent supply is a string. Zero is a string balance and
therefore tracked; mixed tracked/null options still require a matching tracked
option. The web renderer omits the unusable empty selector only in the all-null
option case and keeps the existing parent quantity and fallback explanation.
Null parent stock remains rejected. No public API rule is relaxed.

Quality and security verdict: PASS for this bounded candidate, with no
blocking source finding. Pagination still dispatches each page through the
existing authenticated API. It does not return collected rows after a later
error or weaken visibility filtering. The canonical removal handler is
unchanged: it reauthenticates under the household lock, checks current manager
permission and dosage ownership, locks the parent, performs authorised replay,
and rechecks whether any tracked option exists before parent removal. Audit,
sync, stock versioning, quantity precision and CSRF paths are unchanged.

The page checks establish consistency of the observed totals and IDs, not a
transactional snapshot. A same-count replacement between pages can evade these
checks; no snapshot or permission-version guarantee is claimed. The separate
permission-change fixture, audit and compatibility follow-ups, other refactors
and overall G readiness remain outside this acceptance. This review ran no
tests and made no product edits.

### Current authorisation boundaries and coverage

Independent bounded source review finds no stock/collection permission bypass.
Every complete collection page still uses authenticated canonical get_reply;
a later denied/error response aborts without returning accumulated rows.
Medication reads retain household scope and membership/source visibility.
People, assignments and schedules filter household and current granted people
before pagination; schedules retain the adult-member index check. This repair
does not widen any of those API scopes.

Parent-stock removal still locks the household and reauthenticates, then checks
visible medication and manager role before parsing/mutating. It verifies dosage
ownership, locks stock, performs authorised exact replay, and rechecks tracked
options before scalar fallback. Browser allowance of blank dosage in the
all-null case cannot override those canonical locked checks.

Existing meaningful tests include `web_reads_api` visible/hidden/foreign
collection and detail checks, stable authorised pagination and grant revocation
on next read; `household_stock_permissions` manager denial and revoked-access
404 with no writes; and `household_stock_boundaries` unrelated/foreign dosage
rejection. The final seven-case receipt adds complete large-page canonical IDs,
foreign dosage/medication exclusion and exact People/form coverage. Reading
these tests does not claim they were all rerun in the final seven-case job.

The genuine remaining coverage limit is that the new greater-than-500 cases
use the owner. They do not directly combine a limited member's hidden records
with WebApi's later 100-row pages. A focused restricted-member later-page
regression would strengthen integration coverage; no current source disclosure
or authorization failure is established by that omission. This check makes no
transactional snapshot or instant revocation guarantee between page reads.

### Matching inventory runtime and visual evidence

The reviewer independently read
`/private/tmp/household-g-20261002/G-FINAL-INVENTORY-GREEN-001/runner.raw.log`.
It records seven HTTP tests passing with no failures: two linked/unlinked
assignment controls, four inventory fallback/null/zero/mixed/large-option cases,
and the complete large People collection case. All ten inventory browser cases
pass across five locales and desktop/mobile. `task.exit` is zero. No additional
default browser files or permission cases are claimed from this selection.

The emitted source digest is
`8611a75140874abd71a8d904ae6f7209e13f422d85095628e0482c7505f71592`;
fixture hash is
`220a619a8521fafd479f2e3fbfd4a108b3ce674917f5c378805b8ce264131862`.
Independent comparison finds the saved 393-entry `copy.manifest` and
`live-app.manifest` exactly equal. All 25 archived screenshot checksums match
the evidence directory's `screenshots.sha256` manifest.

Representative visual review covered rejected EN desktop and CY mobile forms,
saved ES desktop and PT mobile stock pages, and GA/PT mobile Locations pages
under `docs/screenshots/journey-medication-rust/g-20261002/inventory/`. The
rejected quantity 99, selected reason and literal escaped note are retained and
readable. Saved pages show 18.0 ml parent stock separately from the untracked
tablet option and explain fallback truthfully. Controls wrap within mobile
width and translated labels/guidance remain readable.

The Portuguese navigation label truncates at its initial position, including
after a fresh destination render. The actual successful browser test hovers the
navigation, performs a horizontal wheel gesture, observes increased scrollLeft,
requires the last link's full bounding box inside the navigation viewport, and
activates it with Enter before asserting the Locations URL. Thus this is the
previously qualified scrollable layout limit, not evidence of an inaccessible
last action. Destination screenshots alone do not prove the gesture; the
strict passed browser assertions supply that evidence.

## Minor person page and translated guidance RED setup

Static setup verdict: PASS for the isolated
`household_minor_readiness.rs` case and ten `readiness_locale.rs` renderer cases.
No production translation or rendering changes were present at review.

The minor case uses the actual ordinary viewer account email and a fresh public
login. `scripts/contract_provision.rb:570–581` gives that member a view grant to
the managed person; the fixture contains actual assignments to that person.
The test proves person and assignment API reads succeed before requiring the
browser page to succeed despite schedule-index 403. Current
`read_resources/schedules.rs:24–68` bases that denial on the viewer's person age
and type. The temporary age/type/capacity mutation and restoration/Drop bodies
match the previously accepted `web_reads_api.rs` helper. It must run in its own
fixture. No grant or production policy is changed.

Assignment history link selectors match the existing overview renderer's
`/people/{person}/assignments/{id}/history` paths. The test requires those links
to remain, no schedule links to appear, no data in the schedule denial, and
hidden/foreign person pages to remain 404. Its five-language section-unavailable
copy explains the schedule omission rather than presenting it as an empty
authorised collection.

The five missing-version renderer cases require known `If-Match is required`
to use the existing translated `medications.stock.original_token` guidance.
All five catalogues contain that key. Unknown private diagnostics must remain
hidden and must not acquire a false missing-version explanation. The five
editing cases require an honest keep-current placeholder and guidance that a
blank option preserves any existing link; create keeps Default dose. The copy
does not claim that an unknown linked option has been identified or unlinked.

The literal selected-option substring check is sensitive to HTML attribute
serialization. It is a supporting renderer assertion, not proof of persisted
link identity; the separately planned real linked/unlinked write tests remain
necessary. Runtime, source repair and final requirements/security acceptance
are not established by this setup review. Verifier and writer were notified.

## Minor page candidate and remaining empty-state wording

Independent reading of `G-MINOR-RED-001/runner.raw.log` confirms one intended
failure: the allowed person page returned 403 instead of 200 after successful
person/assignment reads and the expected schedule-index denial.

Access and security verdict: static PASS for the focused candidate. The person
read remains mandatory. Medication and assignment collection errors propagate;
only schedule-specific `PageError::Status(FORBIDDEN)` becomes an explicit
unavailable flag. All other schedule errors propagate. No denied rows are
recovered through another credential, and no public schedule policy changes.
People integration receives authorised rows and the explicit flag. Previous
renderer entry points remain wrappers with schedules available, preserving
normal adult rendering. The new five-language warning is escaped text and
distinguishes unavailable data from an authorised empty collection.

Initial requirements verdict: conditional, pending one confirmed P2 wording repair and
runtime. When assignments are empty and schedules are unavailable,
`rust/web/src/treatments/overview.rs` still renders the legacy statement
`No medication or schedules have been assigned.` This implies absent schedules
without permission to establish their absence. The smallest fix is to render
that statement only when rows are empty and schedules are available. First add
a focused empty/unavailable renderer failure and an available-empty control;
no API or policy change is needed. Recommended classification: rust, bug.

The current isolated minor fixture contains readable assignments, so it does
not reach this wording branch. Its focused GREEN can proceed and remains
meaningful, but it cannot certify the empty/unavailable state. Writer, root and
verifier were notified. No product changes or runtime were performed by review.

Final bounded minor requirements verdict: PASS. The applied one-condition
repair is `rows.is_empty() && !overview.schedules_unavailable`; the unavailable
warning remains visible, while the ordinary available-empty statement is
preserved. Independent raw review confirms `G-OVERVIEW-LOCALE-RED-001` had one
intended failure and one control pass, followed by `G-OVERVIEW-LOCALE-GREEN-001`
with both cases passing (Task exit 0). Both cases loop all five locales.
`G-MINOR-GREEN-001/runner.raw.log` confirms one HTTP case passed, including its
five-language HTML and access-boundary assertions, plus six routes smoke cases.
The empty-state finding is resolved locally; no absence claim remains when
schedules are forbidden.

Final bounded quality and security verdict: PASS. The condition changes only
truthful presentation; mandatory reads, exact schedule-403 handling, other
errors, current permissions and renderer wrappers remain as reviewed. Raw
guidance GREEN confirms ten cases, and focused stock GREEN confirms four HTTP
plus six routes smoke cases. Neither smoke run supplies native journey screenshots.

Evidence qualification: the overview RED/GREEN `source.pre.sha256` lists omit
the changed `rust/web/src/treatments/overview.rs` renderer. Their identical
70947c prefix proves stability only for the listed locale/test/helper/Task
inputs, not the complete renderer. The actual raw assertions remain valid,
and current condition was independently read, but complete renderer input
stability cannot be verified from that list. Root and verifier were notified;
the final capture must include overview, people and renderer module dependencies.
No repeat solely to reconstruct missing metadata is requested. Broader
collection/permission, browser visuals, compatibility and refactor acceptance
remain pending; accepted D/E/F reports remain unchanged.

## Empty-state and large People collection test setup

Static setup verdict: PASS for the private `overview_access_locale.rs` draft.
Its two public-renderer cases loop all five locales. Unavailable schedules with
empty assignment rows must show the translated warning and omit the statement
that no treatments were assigned. The ordinary available-empty control must
retain that statement and exclude the warning. The product condition is still
unchanged; the failing case must run before its narrow repair.

Static setup verdict: PASS for the separate private
`household_people_collections.rs` draft. The medication uses the required valid
threshold, location, dose/unit and stock. Assignment seeds use the currently
manageable fixture person. The daily schedule has positive precise dose,
permitted unit, valid 08:00 config and required ordered start/end dates. Omitted
optional interval and cycle are supported, and the canonical default maximum
is four; no unsupported null or zero schedule interval is submitted.

The bulk fixture copies all mandatory medication, assignment and schedule
columns from real API-created seeds. Database UUID/default columns remain valid;
new medication IDs avoid the active person/medication assignment uniqueness
constraint, and copied positions are advanced. All inserts remain in the
disposable household and person without changing grants, identities or profile.
Authoritative pages require stable totals, exact coverage and unique IDs. People
history sets and all create/edit medication choices must equal those complete
canonical sets. The draft remains separate from active captures; compilation,
runtime and final permission-change proof are pending. Writer, root and verifier
received these bounded setup approvals.

## Minimal translated guidance candidate

Requirements verdict: static PASS. Independent reading of
`G-READINESS-RED-001/readiness-locale.raw.log` confirms ten intended failures:
five known missing-version cases and five editing placeholders. The candidate
maps only the exact `If-Match is required` message alongside the existing
browser-precondition marker to `medications.stock.original_token`. Unknown
messages still use `Text::form_error`; they do not acquire a false reopen
explanation or expose diagnostics.

Editing now uses keep-current dose-choice wording and conditional help. Creating
still uses Default dose and does not show the editing hint. The two new nodes
have matching structure and reviewed copy in all five locale files. The shared
schedule editor also retains its stored option through canonical fields and
validates matching amount/unit, so the common editing explanation is truthful.

Quality and security verdict: static PASS, with no blocking source finding.
Guidance is rendered as ordinary escaped Leptos text. No option identity is
fabricated, no link is silently cleared, and payload, public API, permissions,
CSRF and original-token handling are unchanged. Minor access handling remains
unchanged pending its separate actual failure. Focused GREEN and real
linked/unlinked persistence results remain pending with the sole verifier.

## Linked and unlinked assignment persistence setup

Static setup verdict: PASS for the two cases in
`household_assignment_readiness.rs`. These test existing canonical behaviour
and are expected to pass before the guidance repair; they are not proposed
product REDs.

Medication seeds supply the required valid `"3"` threshold, positive dose,
permitted unit, current household location and string stock. Dosage seeds
supply medication identity, positive amount, unit, frequency, cycle, maximum
four, minimum zero and supported null supply/threshold. Fresh public owner
login and cookie/CSRF writes use the fixture's current management rights.

Current `person_medication_writes/writing.rs:353` selects the stored option when
the replacement field is omitted; `dosing.rs` rejects a dose that disagrees with
that option. Consequently notes-only blank selection must preserve the link,
manual amount 2.5 against the linked 1.25 option must reject, and explicitly
selecting the matching 2.5 option must replace the saved link and amount/unit.
The separate manual case creates no options, so edits cannot silently infer
an option link.

The scoped read-only SQL lookup checks the actual household and internal
assignment ID, compensating for the intentionally absent public link field.
Version and sync counts use the canonical `PersonMedication` item/record types
and source ID; take counts address the exact assignment. Rejection must retain
original ETag and supplied entries, rotate only the cached validation retry key,
leave source JSON/link and all three counts unchanged, and preserve parent stock.
Success checks stored replacement identity, precise dose, escaped note readback
and no takes. No public API, permission or persistence changes are proposed.
Actual runtime results remain pending with the verifier.

## Final browser drafts and seven-case composition setup

The two private browser drafts require corrections before runtime. Both close
the shared browser with top-level `await browser.close()` after registering
asynchronous test cases. Use the accepted suites' `test.after` teardown instead,
so pending cases retain a live browser.

The inventory draft's seeds, null versus finite quantities, exact removal
payload/history, replay/conflict, retained key and source no-write assertions
match the current contract. Its mobile test uses an actual horizontal wheel,
observed scrollLeft, full last-link containment within the navigation box and
keyboard navigation to Locations. It does not assign scrollLeft or rely only
on the link's DOM presence. However, the requested specific insufficient-stock
copy is not emitted by this API: `stock_removals.rs:93` returns the generic
`Stock removal could not be recorded`, which the safe renderer maps to generic
translated form-invalid text. Preserve exact 422/draft/no-write assertions and
assert that existing translated message, unless a separate reason-specific
behaviour change is explicitly approved and tested RED first. Do not infer a
specific rejection cause from the generic API message.

The minor browser draft's public setup is unsupported. The actual viewer has
no own-person grant, so `/profile` is intentionally 403. The primary owner has
no management grant to that viewer person, so an owner person GET/PATCH cannot
be assumed authorised either. Furthermore, person validation requires an
active carer relationship for no-capacity people; converting an otherwise
granted adult does not automatically supply that relationship. No production
grants or policy relaxation should be added to make this fixture work.

Recommended bounded replacement: run the existing isolated minor HTTP case,
which restores its temporary fields, then use a root-owned fixture-preparation
Task with existing test-container database access. Capture and modify only the
viewer person addressed by household, membership and account, using the
accepted TemporaryMinor stimulus. Run fresh browser viewer sessions and prove
identity/type through public `/me`, then restore captured fields in guaranteed
orchestration cleanup before destroying the owned disposable fixture. Browser
history access is supported by the existing read-only treatment context; hidden
and foreign person denial remains required. No Node database dependency is
needed. Exact replacement setup remains pending root/writer handoff.

The private `household_final_acceptance.rs` composition has static setup PASS:
unchanged path modules select two assignment controls, four inventory cases and
one People collection case. Serial alphabetical module order runs assignments
before the inventory's large option case and People cloning last. These modules
do not mutate membership, grant, account or profile state, and browser sessions
must be fresh after them. Minor and later permission mutation fixtures remain
separate. Installation is conditional on the standalone People test passing.
No browser runtime or final acceptance is claimed by this review.

Subsequent private inventory-draft review: both inventory setup corrections are
now applied and statically PASS. Browser cleanup uses `test.after`, and all five
rejection strings exactly match existing `errors.messages.form_invalid` values.
The strict 422, retained fields/key, history and parent/option no-write checks
remain. This does not approve the still-private minor fixture-preparation setup.

## Portable import responsibility split

Requirements verdict: PASS for this single mechanical split after the two
necessary internal field visibility repairs and matching post-split checks.
The saved original hashes to
`d24868946b11011dd9fd100f01f56479abc7a59e9653e0cf05c0277bd05c64d7`.
Independent extraction/comparison against the actual facade and nine children
found all 49 function/method bodies preserved: 41 moved helpers/methods, seven
facade routines and the existing unit test. All five constants and three structs
retain their values/types, allowing only parent-scope visibility changes.
Test attributes are preserved, and both sources contain zero comment lines.

The compiler-required `ExistingIndex.ids` and `.names` bridges are now
`pub(super)`, matching their sibling access/preflight users. Child modules and
other bridges remain private to the importer facade; the external route export
and router caller are unchanged. Formatter wrapping and trailing commas change
two method signatures, so raw whole-file byte identity is not claimed. Their
parameter/return types and bodies are unchanged.

Quality and security verdict: PASS for the reviewed extraction. Boundaries separate schema/contracts,
validation, existing identity lookup, access checks, preflight, field mapping,
record writing, pause handling and relationship links. The retained handle
orchestration commits before blocking decryption, reacquires request context,
starts the write transaction and reauthenticates under the household lock before
access/preflight checks. The unchanged writing routines preserve immutable and
pause checks, import grants/links, request-linked versions and sync events,
pause reconciliation and rollback on failure. Successful import audit and final
commit remain in the same transaction/order. No encryption, replay, tenant or
clinical policy change is introduced.

The first post-split compile stopped on the two field-privacy errors; it did not
reach Clippy or runtime. The repaired candidate then passed API formatting,
check and warning-denying Clippy, plus selected compilation of portable writes,
portability and the final acceptance target. The reviewer independently read
the successful raw check/Clippy logs and all six zero exit receipts under
`/private/tmp/household-g-20261002/G-PORTABLE-SPLIT-FAST-002/`.

The authoritative post-split `G-PORTABLE-SPLIT-GREEN-001/runner.raw.log` records
both named portable-write tests passing: strict encrypted import shapes/dry-run/
apply and the portable sync-batch contract. The separate
`G-PORTABILITY-SPLIT-GREEN-001/runner.raw.log` records all three portability
tests passing: bundle dry-run/apply/public readback, current household/account
authority, and cross-person reference rejection without changes. Each summary
has zero failures and its saved `task.exit` is zero. These are fresh fixtures,
not one combined fixture: their emitted fixture hashes are respectively
`6c9db253dcaf73a94d92974f6a7a42cb2f721e27fcf143525da9a94b2a5649f8` and
`ff8c995a735d28dc8ad18b04cc62420458ab015ae1129db5bfb6d1332c2fafa4`.

Both runs emit the same source digest
`959deae28f2e1a8f41f7752ac2422f53c73b6c41751ed73b0049148c38a84d19`.
The reviewer independently compared each saved 392-entry `copy.manifest` with
its `live-app.manifest`; both are exact matches. Together with the preservation
review, these results accept the one import extraction. No runtime command was
executed by review. The remaining four refactors, final browser groups and
overall readiness remain outside this verdict.

## Five current PR comments

Thread state and full comment bodies were fetched read-only from PR #2362.
All five threads were unresolved and not marked outdated at inspection. No
reply, resolution or issue mutation was performed.

### Taper step times: comment 4161530906

Verdict: not a current defect under the recorded compatibility ruling. Both
Rust `dose_occurrences.rs:600` and Rails occurrence projection consume canonical
top-level times. The new taper controls save those shared times, while nested
step times are explicitly labelled saved instructions that do not determine
due tasks (`schedule_controls.rs`, `schedule_config.rs:137`). Current shared-time
tests prove native create/edit/clear and actual occurrence times/counts. The bot's
proposal to use nested times would change existing recurrence policy. Explain
the preserved rule and verified UI distinction; do not make that policy change.

### Removed taper-step metadata: comment 4161531037

Verdict: false positive for the described remove/save/reopen sequence.
`intentions.rs:145` removes the selected step's fields without reindexing the
others. `schedule_config.rs:89` clones original[index] before appending to the
new compacted steps array. Metadata therefore moves with the retained logical
step. On reopening, `forms.rs` uses the newly saved compacted config as both
original config and indexed field source. A later save still addresses the same
object. Stable new IDs are not required to fix the alleged cross-step copy.
A targeted removal/reopen preservation regression would be useful evidence,
but no current source defect or runtime failure is established here.

### Linked assignment selection: comment 4161531154

Verdict: partly valid browser usability finding, not an API permission or
clinical-write defect. `forms.rs:38` does not receive/initialize the existing
option ID because `source_projection.rs:320` omits it from public read data.
`fields.rs:89` consequently displays Default dose. When no replacement is
submitted, `writing.rs:353` retains the existing option and `dosing.rs:63`
correctly rejects an amount/unit that disagrees with it. A replacement option
can already be selected and applied through the native form.

The user-visible problem is that the form appears unlinked and freely editable
even when the saved option constrains the dose. The smallest policy-preserving
repair is truthful existing-dose/retained-link guidance and a tested explicit
replacement flow. If the exact linked ID is displayed/preselected, obtain it
through an authorised browser-only context; do not infer identity from possibly
ambiguous equal amount/unit or casually widen the public schema. There is no
supported unlink behaviour established by the bot's recommendation. Test linked
and unlinked reopening, notes-only saves, permitted replacement, mismatched
manual entries and preserved original tokens before a UI repair. Classify the
confirmed misleading UI as rust, bug; do not describe canonical 422 as an API
defect.

### Large People collections: comment 4161531302

Verdict: confirmed, part of the #2353 root problem. `people.rs:124` calls
`treatments::rows`; `overview.rs:10–24` reads complete medication, assignment
and schedule collections before filtering to the addressed person.
`api_client.rs:242` stops after five pages/500 records and returns 503, so
otherwise readable People detail fails when any required collection exceeds
the ceiling. Extend the scoped pagination repair and actual large-fixture
regression to People treatment collections as well as dosage/inventory paths.
Use the documented API pages with consistency/current-permission checks; do
not invent an unsupported person filter or treat a partial page as complete.
Labels: rust, bug. No separate speculative policy change is required.

### Minor People detail: comment 4161531413

Verdict: confirmed browser regression. The mandatory authorised person detail
can succeed, then `overview.rs:15` calls the household schedule index.
`read_resources/schedules.rs:26–67` deliberately requires an adult household
member, matching Rails SchedulePolicy#index?. Its policy-correct 403 propagates
through the new treatment subsection and prevents the whole person page from
rendering. This does not depend on the target person's age; it concerns the
viewer's membership person.

Keep person access mandatory and current permissions unchanged. Treat only
the forbidden optional schedule subsection as unavailable, render the allowed
person/assignment data with truthful guidance, and continue failing genuine
server errors. Do not relax SchedulePolicy or query protected schedules through
a bypass. Require a realistic minor member's allowed person GET/page success,
unchanged schedule-index 403, no hidden schedules and unchanged management denial
before the narrow browser repair. Labels: rust, bug; G writer owns the RED and
fix, independently of the accepted F report.

### Prepared minor browser fixture: static setup review

The minor browser checks can now use the existing viewer without adding grants
or depending on forbidden profile/person-edit access. The private SQL pair and
browser setup pass static review. Actual execution and the final Task wiring
remain pending.

Reviewed private inputs are `minor-viewer-prepare.sql`,
`minor-viewer-restore.sql` and `household-minor-readiness.test.mjs` under
`/private/tmp/household-g-20261002/`. The SQL identity join checks membership,
household, account, person and account email against the fixture. It requires
an active member and exactly one matching row. Table and field types match
`db/schema.rb`: bigint identities, integer person type, date of birth and
boolean capacity. Preparation changes only person type, date of birth and
capacity. This is an explicit disposable-fixture stimulus, not proof that a
public person-edit request can create this state.

Restoration checks the same identity, restores all three original fields and
uses `IS NOT DISTINCT FROM` to verify nullable originals before dropping the
snapshot. Every update must affect exactly one row. A duplicate preparation
fails rather than overwriting another snapshot. PostgreSQL 18 documentation
confirms that each file run with `psql -1 -v ON_ERROR_STOP=1 -f` is atomic on
command failure and that `:'variable'` performs SQL-literal quoting. Neither
file contains transaction commands that would defeat that wrapping. See the
[official psql documentation](https://www.postgresql.org/docs/18/app-psql.html).

The browser uses a fresh viewer login and the actual `/me` data shape, including
account ID/email, membership role and person type/capacity. It verifies readable
person/assignment data, forbidden schedule collection access, translated
partial-section guidance, assignment history and hidden/foreign person denial.
It has no owner mutation, profile-read assumption or Node database dependency.
Browser shutdown is registered with `test.after`; contexts close in `finally`.

The private `browser_minor_wrapper.fish`, its nine-case test, fake Task and
legacy helper also pass static setup review for returned task statuses. Exact
minor-file isolation, disposable project format, resolved fixture filename,
owner marker and fixture identities are checked before preparation. Arguments
remain separate quoted values. Preparation failure stops Node; browser failure
still invokes restoration; restoration failure overrides the browser result.
Ordinary browser selection remains node-only. The legacy helper is a meaningful
RED baseline because it lacks preparation/restoration while the same assertions
require both. No runtime result is claimed by this review.

The wrapper has no signal/exit restoration handler. These cases cover ordinary
returned errors, not forced process termination. Cancellation must continue to
use the outer owned disposable-runner cleanup; final Task/SQL wiring and that
failure-path integration need separate review before browser acceptance.

The first wrapper probe failed to intercept real Task because local Fish
configuration reset PATH; that is a setup failure, not product RED. The test
then launches the helper with `fish --no-config`, leaving the wrapper behaviour
unchanged. Independently inspected
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-002/red.raw.log` is meaningful
legacy RED: exactly eight assertion failures report the fake receipt's `node`
phase, including the injected browser exit 7. The ordinary node-only control
does not fail, and no missing-Taskfile error remains. This proves interception
and the legacy helper's missing preparation/restoration/isolation behaviour.
Candidate GREEN and final Task integration remain pending.

Subsequent candidate probes exposed two setup issues: Fish/jq handling of the
escaped domain dot, and fake Task expecting the unresolved temporary-directory
alias while the wrapper intentionally forwards `realpath`. The literal-dot
expression now uses `[.]`; the fake expectation now resolves existing fixture
files only. The normal missing-path control remains unchanged. Static review
accepts these bounded corrections: project/files/Task names, exits and phase
order remain strict. The earlier static setup review did not catch the
Fish/jq escape issue; no candidate runtime success is inferred from it.

The private `minor-browser-task-draft.yml` also passes static review for the
existing owned runner path. Browser image build stays before preparation.
The wrapper path and both SQL phases use exported `CONTRACT_API_BUILD_CONTEXT`;
fixture forwarding uses `CONTRACT_FIXTURE_DIR/fixture.json`, matching the
runner's realpath export and browser mount. Snapshot copying includes the
contract-tests directory, so the installed helper and SQL files will be captured
with application input rather than read from changing live source.

Wrapper arguments, host paths, SQL input redirection and psql variable values
use `shellQuote`. The SQL Task checks the disposable project format and phase,
then streams the reviewed script into `db-test`/`medtracker_contract` with
`psql -X -1 -v ON_ERROR_STOP=1 -f -`. The full ownership-marker and positive
identity checks are performed by the wrapper; the SQL checks the actual database
identity again. Direct SQL Task entry is only phase/project-format guarded and
must remain an internal fixture operation. Ordinary browser defaults and the
existing Node command are preserved.

Browser or restoration errors propagate to the existing runner's exit cleanup.
Build failure precedes preparation; preparation failure is atomic and stops
Node. Forced termination remains covered by disposal of the owned fixture,
not by a claim that the wrapper always executes restoration. Installation,
fake-case GREEN and real minor-browser/restoration proof are still pending.

Installed wiring static review now passes: the four actual Task blocks exactly
match the reviewed private draft, the wrapper is identical, and its nine-case
test changes only the fake Task path under `fixtures/`. CI runs the wrapper
failure-handling test only on the isolated minor row, after prerequisites and
before the browser runner. Installed execution and fresh prepared-minor proof
remain pending; no ordinary-browser, clock or clinical-policy change is added.

The first installed minor packet (`G-MINOR-ACCEPTANCE-001/runner.raw.log`)
passes its one HTTP case, then stops at the SQL Task precondition before SQL or
browser assertions execute. Owned fixture cleanup still runs. This is setup
failure, not a minor-page product failure or browser acceptance. The earlier
static wiring PASS missed an unescaped end-anchor dollar sign inside Fish's
double-quoted regex. The coordinator reproduced the parse error independently.
The Task and private draft now escape that dollar sign so Fish passes the
literal regex anchor. Static review accepts this one-character correction;
the project pattern and allowed phases are unchanged. The coordinator reports
the corrected private Task accepts valid project/prepare and rejects production
project and activate phase. Fresh actual minor browser/restoration proof remains
pending.

## Explicit first-option race assertions

The existing race test already rejects a scalar stock adjustment when the first
dosage option appears after its browser read. The proposed test-only extension
makes that transition and both unchanged stock balances explicit.

Reviewed private proposal:
`/private/tmp/household-g-20261002/stock-first-option-proposal/proposal.diff`.
Only household_stock_concurrency.rs changes. It asserts zero options before
the gated read and exactly one after the real public API create, then asserts
parent and option current_supply equality in addition to the retained full-row
equalities. The deterministic gate, original draft/token retention, 409 status,
version/sync counts and cleanup are unchanged. The synthetic timestamp restore
case remains explicitly distinguished from the ordinary concurrent create.
No production code, comments, sleeps or weaker assertions are introduced.

Requirements and quality/security verdicts for this assertion-only proposal:
PASS; installation is released after the completed original run. The extended
test's actual execution remains pending.

The original unchanged run passed all four HTTP cases, with no ignored or
filtered cases, and all six route checks. Its outer exit receipt is zero.
Raw result: `/private/tmp/household-g-20261002/G-STOCK-CONCURRENCY-FINAL-001/stock-concurrency.raw.log`,
SHA-256 c21800bb4a5803a4e87a02f442a2922aa066de40205a97a7fba90e0b8c0daf15.
Source identity is runner-emitted
310de470b49cd66e4f73e4720afe6bd74c05d90a46b67012a3e6084d5985b078;
fixture identity is runner-emitted
6d70c0e1c515717b6857f4e532e3715c3e781d3927ebb2aba44155c0ff071465.
No independent original copy/live or fixture comparison is claimed for this
packet. Overall G acceptance remains pending.

### Prepared minor acceptance and visual evidence

Bounded minor-page requirements and quality/security verdicts: PASS.
Independently read `G-MINOR-ACCEPTANCE-003/runner.raw.log` and
`browser.raw.log`: the isolated HTTP case passes, preparation performs all three
exactly-one checks, ten browser cases pass across five locales and both
viewports, restoration performs its three checks and drops the snapshot, and
saved `task.exit` is zero. Original schedule policy remains forbidden while
permitted person/assignment/history pages are readable. No new grant is added.

The runner emits source digest
`9bae6448d851ce01846b1630c7c3f9d4f43ba20ecf67af2b855132d860dcaf2f`
and fixture hash
`54a19104a27f8215c6e8c550ff5ad911eebbc3abacccd9b932c232ea93f83e73`.
Original per-file copy/live manifests were not retained. The verifier later
reconstructed frozen current source using the existing source-snapshot Task
under `G-MINOR-SOURCE-RECON-001/source`. The reviewer independently checked its
400-entry `source.manifest`: its aggregate hash exactly equals the emitted
runtime digest above, and all 400 reconstructed files match their checksums.
This corroborates current frozen source identity against the emitted digest;
it is not an original per-file comparison with the deleted runtime copy.

All 20 archived screenshots match `G-MINOR-ACCEPTANCE-003/screenshots.sha256`.
Representative EN desktop, CY/GA mobile person pages and ES desktop/PT mobile
history pages under `g-20261002/minor-readiness/` show readable translated
unavailable guidance, permitted assignment cards and history links, without
schedule management controls. History empty-state wording is translated and
does not claim forbidden schedules are absent. No new visual defect is found;
the already-qualified horizontal navigation limit is unchanged. This accepts
the minor-page repair only, not remaining audit, permissions, compatibility or
refactor work.

## Private dose baseline sharing proposal

Composition verdict: NOT APPROVED pending concrete fixture-safety proof.
The private six-module facade preserves original tests, but serial execution
alone does not establish independent UUIDs or mutable fixture assumptions.
`dose_mode_transition_api.rs:38` omits three non-null columns without schema
defaults: default_dose_cycle, default_max_daily_doses and
default_min_hours_between_doses. The note identified only the first; the
unchanged helper needs actual baseline classification and an approved narrow
setup repair before a meaningful extraction baseline.

The doses/direct-write 111 namespace and direct-write/replay 333 namespace can
share a UUID if allocated medication IDs equal the respective household/sequence
tail. Actual prepared-fixture IDs must prove inequality before composition;
do not reset UUID/cache/history or weaken zero-count assertions to manufacture
sharing. Source capabilities reads only the default collection page of 20;
canonical assignment/schedule ordering is ascending ID, so appended sources
do not displace earlier IDs, but selected seed membership still needs proof.

The proposed last-suite temporary restorers and replay-specific permanent grant
mutation are acknowledged, not guarantees after forced termination. Run the
isolated source-capability compatibility checks first, as root selected. Known
paused-stock and location-order expectations must be ruled against canonical
behaviour, not used to change policy merely to make a composed baseline green.
No dose extraction or runtime was performed by review.

## Adjustment reason boundary probe setup

Static setup verdict: PASS for the private two-case
`/private/tmp/household-g-20261002/household_adjustment_reason.rs`. This is a
probe of the actual audit storage boundary, not confirmation of the bot's
255-character claim. `db/schema.rb:1251` declares event as an unbounded string;
line 1262 defines the named event index. The authoritative adjustment schema
permits an optional reason string without maxLength, and
`medication_management/inventory.rs` checks its type but introduces no length
restriction.

Medication setup matches the accepted stock helper: threshold 3, supply 20,
dose 1.25 ml and a current owner bearer. The probe introspects actual event
column type/length and collects the named index definition. It tests a
300-character control and a deterministic varied 8192-byte printable reason.
After the request it reads public stock and scoped version/sync counts before
asserting expected success, preserving useful diagnostics on a later failure.
Index definitions are diagnostic, not asserted nonempty; the current schema
does supply that index.

If successful, each case requires full reason preservation in the event or
structured audit context, stock 15.12, exact persisted 20.00-to-15.12 changes,
one additional version/sync event and matching request, household and actor
linkage. A failed large request must be classified from actual status,
transaction/readback and database evidence; neither truncation nor a database
index failure is claimed before execution. Installation and runtime remain
pending, with no production change approved by this setup review.

### Observed audit failure and storage recommendation

Long adjustment notes can currently prevent an otherwise valid stock change.
Independent reading of `G-AUDIT-PROBE-001/runner.raw.log` confirms the
300-character control passes, while the varied 8192-byte reason returns
500/internal_error. Stock remains 20.0 and version/sync counts remain (1, 1).
The transaction therefore rolls back rather than leaving a partial stock
change. Only the short case reaches the complete successful linkage assertions.

Runtime introspection reports unbounded character varying, no column character
limit, and `index_versions_on_event` as B-tree. The alleged 255-character
column limit is false. The actual large-reason failure is a confirmed Rust bug
(rust, bug); the index-size explanation is strongly supported but remains an
inference because the saved receipt contains no underlying database exception.
PostgreSQL 18 limits B-tree entries to roughly a third of a page after possible
compression, consistent with inserting the full varied reason into the indexed
event. See [official B-tree documentation](https://www.postgresql.org/docs/18/btree.html).
Rails `AdjustMedicationInventoryService#build_event_string` uses the same full
interpolation, but no Ruby runtime was run and its equivalent failure is not
certified here.

Preserve short event compatibility rather than making every event concise:
`household_stock.rs:107` requires the existing exact short reason event, and
Rails service specs require both quantity-only and short reason strings.
Use a conservative total UTF-8 byte budget for the indexed event (for example
1024 bytes, explicitly an internal storage choice rather than the database's
exact maximum). Above that budget, retain a bounded quantity-only event and
store the complete original nonblank reason under a namespaced adjustment
entry in existing audit_context. Do not truncate the reason or introduce a
public maxLength, rejection rule, migration or index removal. Preserve trusted
request/actor/household metadata and the same lock/transaction/version/sync
order; do not merge user JSON over security context.

Admin audit filtering uses the `adjust inventory` prefix, so that remains
compatible. Existing admin event labels display `version.event`; they do not
render a structured adjustment reason. Therefore a concise large event with
full structured persistence must not be described as showing that full reason
in the current admin UI. Require successful short and large probes, exact
structured reason/quantity/linkage and conservative byte-boundary/multibyte
coverage before accepting the storage fix. No production code was changed by
review, and the final repair remains pending.

### Stable audit repair: independent source review

The proposed repair keeps long adjustment notes without putting them into the
indexed event label. Requirements verdict: static PASS. Quality and security
verdict: static PASS. The six-case runtime and existing audit compatibility
checks remain pending; this is not final acceptance of the repair.

`medication_management/persistence.rs::record_inventory_adjustment` applies a
1,024-byte budget to the complete formatted event through `String::len`, which
counts UTF-8 bytes. At or below the budget the legacy event remains exact;
above it the event becomes quantity-only. It does not slice, truncate or reject
the reason. Every adjustment stores the exact original optional reason and
normalized quantity in `audit_context.inventory_adjustment`. Missing reason
is stored as null; blank and whitespace strings remain exact in metadata but
produce a quantity-only event. Existing non-string and null rejection in
`inventory.rs` remains unchanged and precedes stock mutation.

The private `VersionChange` payload and common insertion function preserve the
existing `record_version` signature and other callers. Their ordinary versions
do not receive adjustment metadata. Trusted actor, household and request fields
are constructed from authenticated context, with only the dedicated adjustment
namespace added; request JSON is not merged over those fields. The medication
update, version insertion and request-linked sync event still use the same
transaction and existing locks. Any insertion failure propagates before commit.
There is no public reason limit, schema change, Rails change or new warning
suppression.

The stable six-case target tests exact complete-event boundaries at 1,024 and
1,025 bytes for ASCII and multibyte reasons, the existing 300-character control
and the varied 8,192-byte note. Each successful case requires exact structured
reason and normalized quantity, persisted stock/change values, one added version
and sync event, and matching request/actor/household linkage. The boundary helper
subtracts the formatted event overhead, so it tests the total byte budget rather
than reason length alone. Existing short-event compatibility is retained.

No concrete source or test setup blocker was found. The budget is a conservative
internal storage rule, not a claim about PostgreSQL's exact maximum index entry
or a promise that current admin labels display the full structured long note.

### Audit repair runtime receipt

Independent reading of
`/private/tmp/household-g-20261002/G-AUDIT-BUDGET-GREEN-001/runner.raw.log`
confirms all six named adjustment-reason cases pass, with zero filtered cases,
and all six route smoke cases pass. The raw receipt SHA-256 is
`5d914a7a310538a908af57f35500cd7c14a4a4fd3c2954bddca7877c372962b0`.
Format, API check, warning-denying Clippy, selected compilation and wrapper
exit receipts are all zero. The runner emits source digest
`93dfde80f4cc6755aac1bcacb07642e21304243ea8ceabd7d89e1a804c91b113`
and fixture SHA-256
`c1a4aa1823c3485462fe23393e33d579b9752cdf76013c373b45c52a6bce677c`.

This demonstrates the boundary and large-note cases reach complete successful
stock, exact structured reason/quantity, version and request-linked sync
assertions. The separate existing short-event stock test and the verifier's
source-match metadata remain pending at this update. No original per-file
copy/live equality is claimed from an emitted digest alone. The original
PostgreSQL exception was not captured, and Rails remains untested and unchanged.

### Bounded audit acceptance

Long stock-adjustment notes now preserve the complete reason and allow the
stock change to finish. Final bounded requirements verdict: PASS. Quality and
security verdict: PASS for the reviewed Rust repair. The six reason/boundary
cases above and the separate existing short-event compatibility check provide
the required behavioural evidence; this does not accept remaining G work.

Independent reading of
`/private/tmp/household-g-20261002/G-AUDIT-STOCK-REGRESSION-001/runner.raw.log`
confirms `scalar_adjustment_records_reason_quantity_and_request_linkage` passes
(one case, three filtered), followed by six passing route smoke cases. Its raw
SHA-256 is
`415ff9a0b72ecc42211611587e7575930691bd3fac8fe99cabe7f6fc810d5314`.
It emits the same application digest `93dfde80...` as the six-case audit GREEN,
with separate fixture SHA-256
`1986b449173f25466a6b3107d95e29a7be2286b6935575d02797bd9f58d40756`.

Provenance qualification: original per-file copy/live manifests were not
retained in the audit evidence directories. The acceptance relies on the
passing raw behavioural receipts, their matching emitted application digest
and the independently reviewed frozen patch. It does not claim original
per-file equality or a later reconstruction. No check was repeated solely to
recover missing metadata. Future actual runs must retain those manifests
before cleanup.

The original large-note failure, rollback evidence and unrecorded PostgreSQL
exception remain part of the record. The precise underlying database exception
is still unverified, and no equivalent Rails runtime or Rails repair is claimed.
Existing admin event labels still show the bounded event, while full long notes
are preserved in structured audit context rather than displayed by those labels.

### Limited-member collection coverage and acceptance composition

Static setup verdict: PASS for the two test-only extensions. Runtime remains
pending. `household_final_acceptance.rs` now includes the unchanged audit target
as `stock_adjustment_audit`. Serial alphabetical module order is assignments,
inventory, people, then stock_adjustment_audit: two, four, one and six cases,
respectively, totaling thirteen. The verifier must confirm the actual selected
list before execution. Audit cases create unique medication parents and revoke
only their own temporary bearer tokens; the unrelated six-suite dose-sharing
proposal remains unapproved.

The People collection test uses the existing adult viewer's managed-person view
grant and a fresh cookie login, with account email checked against the fixture.
Provisioning and the test both establish that journey_browser_person_id equals
managed_person_id. Expected medication IDs derive from linked sources for that
person plus the viewer's own unlinked medications, matching the current canonical
scope rather than the owner's complete collection. It adds no grant, profile or
person mutation.

The extension checks all authoritative 100-row pages, stable totals, unique IDs
and exact permitted medication, assignment and schedule sets of at least 501
rows. Hidden and foreign medications remain excluded; every returned treatment
source belongs to the managed person. Native history links must cover the exact
full permitted source sets, without creation, editing, pause or resume links.
This directly addresses the earlier large-page limited-member coverage limit
once the new assertions pass; it does not invent transactional snapshot safety.

### Source-capability compatibility baseline

The standalone source-capability baseline reached the two known disagreements.
Independent reading of
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-BASELINE-001/runner.raw.log`
confirms two HTTP assertion failures and no subsequent route smoke execution.
After changing the candidate stock location, eligible medication IDs remain
medication-ID ordered rather than location-name ordered. The paused-source
test also fails its old empty-eligibility assertion. This distinguishes a
stock display ordering defect from an incorrect pause projection expectation;
neither failure authorises a change to recording policy.

The raw receipt hashes to
`33af16383f06ee3129b2ce14e5ab0c3a8cb09f5929a5489b86734a92042241fb`.
The preserved 401-entry pre-run, copied and live manifests each hash to the
emitted source digest
`ef0f3abf34f8dddd590969e9a042d1e21f533c6d484fe99a8532c0053ae393b9`;
fixture SHA-256 is
`1ddc7796d9fe929eb4f04f353a329a1498a36705c5ebc556eec723cb9ebbc84a`.
Compilation passed. Source repair and its new rejection/no-write proof remain
pending independent review and GREEN. No Rails change or claim that Rails
publishes the optional eligibility JSON field is made.

### Stable source-capability repair review

Stock choices now follow the location order users expect. Static requirements
verdict: PASS. Static quality and security verdict: PASS. The same standalone
two-case suite and route smoke GREEN remain pending; no dose refactor is accepted
by this review.

In `read_resources/source_stock.rs`, candidate collection still starts from the
canonical authorised household scope, constrained by the existing source names
and person links. The additional rank query binds every candidate ID and the
household ID as values. It uses a household-matching LEFT JOIN to locations and
orders by `l.name ASC, m.id ASC`, preserving the database collation, medication-ID
ties and database null ordering rather than applying a separate Rust text sort.
The rank map must cover the exact candidate set before it is used; inconsistency
fails closed. This query ranks authorised candidates and does not introduce new
ones. Eligibility preserves that order while retaining the existing household,
person-link, signature and positive-or-untracked supply predicates. It adds no
transactional snapshot guarantee.

The corrected paused assertion keeps permission-derived can_record and the
source's suitable medication choice. A valid, distinct UUID and actual current
time are used for a canonical medication-take request against the paused source.
The test requires 422, `unprocessable_content` and the exact existing paused
message. It then compares complete household-scoped medication, assignment and
dosage rows, household take/version/sync counts, and the public source projection
before and after. Security audit activity is separate from those domain records.
Thus a suitable stock choice does not imply that administering a paused source
is allowed. No recording, pause or Rails policy changes are introduced.

### Positive stock suitability and sufficient-dose check

The first candidate run passes the paused-source case but stops at a later
pre-existing eligibility expectation. Independent reading of
`G-SOURCE-CAPABILITIES-GREEN-001/runner.raw.log` confirms one pass and one
failure at line 455: actual stock IDs include the original and alternate
medications, while the test expects only the original. Its later tracked-stock
assertion has not executed, and no route smoke ran. The preserved pre/copy/live
manifests match at
`5e0e6d21a54b486b2bc6f3878fe9bf6d025b84aef38b086ab19eded4b7943de6`.
This is an incomplete candidate result, not acceptance of the whole suite.

The compatibility ruling is confirmed: positive stock remains a suitable
choice even when it cannot cover a complete dose. OpenAPI descriptions at
lines 7115 and 7511 explicitly specify untracked or positive supply and a
submission-time stock recheck. Rails
`MedicationStockSourceResolver#available_medications` rejects only
`out_of_stock?`; `SupplyLevel#out_of_stock?` means tracked current supply is
zero or negative. This comparison does not claim that Rails emits the optional
Rust eligibility field.

Rust `dose::decrement_stock` separately checks sufficient stock under the
existing medication/selected-option locks and returns the canonical out-of-stock
422 before scalar or tracked decrement. Preserve those production checks and
correct the positive-but-insufficient projection expectations only in tests.
The strengthened tests must submit actual 1.25 ml doses against 1.00 ml scalar
and tracked balances, require that exact rejection, and compare complete source,
original and selected medication, dosage, take/version/sync state before and
after. In the scalar case the selected alternate and original medication are
different rows; both must be covered. Exact test patch and GREEN remain pending.

The resulting test-only patch has now received independent setup PASS. The two
requests use distinct valid UUIDs ending 235 and 236, actual current time,
canonical assignment portable identities and the intended selected scalar or
tracked medication. Provisioned sources require 1.25 ml; the controlled selected
balances are 1.00 ml. Both require the exact canonical out-of-stock 422 rather
than accepting any failure. `dose_write_state` captures both original-source and
selected medication rows, complete associated dosage arrays, the complete
assignment and household takes/version/sync counts. Exact public source
readback is also required. The paused case reuses the expanded snapshot without
changing its rejection semantics. Product ordering and stock predicates remain
unchanged. Same-suite GREEN is still required before final compatibility PASS.

### Bounded source-capability compatibility acceptance

Final requirements verdict: PASS. Quality and security verdict: PASS for the
reviewed ordering change and corrected capability tests. Stock choices follow
location-name order, while suitable positive stock and recording permission do
not bypass paused-source or sufficient-dose checks.

Independent reading of
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-CORRECTED-002/runner.raw.log`
confirms both HTTP cases pass with zero filtering, followed by six passing route
smoke cases. Selected compilation and wrapper exit receipts are zero. The full
raw receipt SHA-256 is
`166452cd0317362b2b31ab8f2bca500730639d0e9e6e76b4cdf0361173433109`.
The source suite therefore reaches the paused and scalar/tracked insufficient
recording assertions, including complete domain no-write comparisons, rather
than stopping at the earlier projection expectations.

The original retained 401-entry pre-run, copied and live manifests each hash to
the emitted application digest
`f9c97a8221a6ccb9eca5a08e786b612cf23c243f5fa658a2c637e900de444286`.
The two reviewed current source/test file hashes match their retained live
manifest entries. Fixture SHA-256 is
`1d51b3e699c8f438ffbe2df37d01ba71a3aa9ae13f83117fd331d50c51e684df`.
These are original retained comparisons, not reconstructed source identity.

The initial two-failure baseline and intermediate one-pass/one-failure candidate
remain in the record. This acceptance introduces no Rails changes, new public
field promise, pause policy or stock predicate change. Oversized People page
compatibility and the remaining mechanical refactors remain separate pending
work; no refactor has been accepted from this receipt.

### People page-size compatibility

The existing People collection request with per_page=999 incorrectly returns
422 rather than the documented compatibility response with page size 100.
Independent reading of `G-WEB-READS-PAGINATION-RED-001/runner.raw.log` confirms
the selected existing test reaches this assertion: one failure, five filtered,
no route smoke. Original 401-entry pre/copy/live manifests match unchanged
source digest `f9c97a82...`. This is a real request failure, not setup evidence.

The stable local repair has requirements and quality/security static PASS.
`read_resources/people.rs::people_index` caps the successfully extracted
typed per_page value with min(100) before the unchanged strict parser. Zero and
negative values remain unchanged and invalid. Malformed, fractional and integer
overflow inputs still fail typed query extraction. Invalid page numbers and
blank or invalid timestamps still fail the existing strict checks. Authentication,
person scope, audit, ordering and shared pagination helpers are untouched.

The same existing test retains the 999 expectation and checks 100, 101 and
i64::MAX return identical authorised data, totals and page size 100. It checks
zero/negative sizes and pages, malformed values, empty/fractional/overflow sizes,
bad timestamps and combinations of invalid input with a clamped size. Unrelated
locations, medications and assignments still require 422 for 999. Schedules
retain their existing lenient clamp and empty-timestamp handling. Source and
setup PASS have been sent to the verifier; the selected HTTP case and six route
GREEN remain pending. No global pagination policy change is introduced.

### Bounded People page-size acceptance

People now accept a requested size above 100 by returning at most 100 records,
without accepting malformed or otherwise invalid queries. Requirements verdict:
PASS. Quality and security verdict: PASS for this local compatibility repair.

Independent reading of
`/private/tmp/household-g-20261002/G-WEB-READS-PAGINATION-GREEN-001/runner.raw.log`
confirms the same selected HTTP case passes (five other cases filtered), followed
by six passing route smoke cases. Its controls reach oversized sizes and valid
boundaries, invalid People queries and unchanged other-resource behaviour.
Format, API check, Clippy, selected compile and wrapper exit receipts are zero.
The raw receipt SHA-256 is
`2e30b0e048cbcececbc4d90bd76ee28732c27f068443ae9d60446378455e2270`.

The original retained 401-entry pre-run, copied and live manifests each hash to
the emitted application digest
`1e1786fc8e7842d5329f91df9ccc103633f66c4d031e588f6442ff9bfc94e3bc`.
Current reviewed People source and web-read test hashes match their retained
live entries. Fixture SHA-256 is
`dfcf717d227acd5afd42ee0624ef2b044634bb79f29293d986941f9ee083253b`.
The initial actual failure remains recorded above. This accepts neither a global
pagination change nor the pending dose-mode baseline or mechanical dose split.

### Dose-mode baseline fixture repair

The two existing dose-mode tests stop before their behaviour checks because
their direct dosage INSERT omits required defaults. Independent reading of
`G-DOSE-MODE-BASELINE-001/runner.raw.log` confirms PostgreSQL 23502 for
default_dose_cycle in both helpers. No route smoke runs. These are setup
failures, not product REDs or evidence that dose-mode behaviour is wrong.

The frozen test-only repair has setup PASS. `db/schema.rb:430`, 433 and 434
require cycle, maximum daily doses and minimum hours without database defaults.
The existing dosage validator accepts cycle 0 (daily), maximum 4 and minimum 0.
Those are the only added INSERT columns/values; amount, unit and frequency
remain unchanged. Independently reversing that one string in memory reconstructs
the baseline file SHA-256
`cf5e2889c0bb0964943ed64627c2dd0cd9d4994ff11a582f1030e2db63ff2ccf`,
proving every other test/helper/assertion/comment byte remains unchanged.

The same two-case fresh-fixture rerun is required before any behavioural
acceptance. No production dose code or extraction has been reviewed or changed
at this checkpoint.

### Dose-mode before-split baseline acceptance

The repaired existing baseline now reaches and passes both behaviour cases.
Bounded requirements verdict: PASS for this before-split baseline. Quality and
security verdict: PASS for the test-only required-default repair and its scope;
no dose extraction or production change is accepted here.

Independent reading of
`/private/tmp/household-g-20261002/G-DOSE-MODE-CORRECTED-002/runner.raw.log`
confirms two HTTP cases pass, zero filtered, followed by six route smoke passes.
They cover clearing options while preserving a direct source, and rejecting a
single-dose switch linked to a schedule without version/sync/tombstone effects.
Selected compilation and wrapper exit receipts are zero. The full raw SHA-256
is `6e989af9d8a57fc28fe28e9f3ec08e3312174fdd8d39364dd5b6cfdd71b47985`.

Original retained 401-entry pre-run, copied and live manifests each hash to
the emitted application digest
`e750f56aac2f42c5fc04da134cc23c9b870846ef9d6faa6a5f9bdc4f799f2e45`.
The current repaired test file matches its retained live entry. Fixture SHA-256
is `29f62f2792db0bae608b2da9076a48b5ec7933128512c27dffc4a5858824d636`.
The initial two helper failures remain recorded as setup failures. Separate
doses, dose-write, full sync and replay baselines remain required before any
mechanical dose extraction.

### Doses helper and unsupported source-type ruling

The doses baseline has eleven medication-creation setup failures, two executable
passes, one unsupported-source status failure and one existing ignored timestamp
case. These categories must stay separate. The frozen helper repair adds the
required reorder_threshold value 3 and safe public status/body diagnostics only.
Current medication validation explicitly requires that field and accepts 3.
Reversing those two edits in memory reconstructs the baseline file SHA-256
`ef4c2b9951719d8752ee33f5d040bed8f1b767ee00263a63ba4752d39d24f8ad`.
Helper setup verdict: PASS; every behavioural assertion and the ignore remain
unchanged.

The existing unknown-source 404 assertion has authoritative compatibility
support. Rails `MedicationTakesController#medication_take_source` raises
RecordNotFound for unsupported kinds, and its request spec explicitly requires
404 with a valid source identifier and client UUID. OpenAPI defines the two
supported enum values and lists both 404 and 422 response categories, but does
not settle the unsupported-enum edge by itself. Do not weaken the doses assertion
to 422 merely because the current Rust early validator returns it.

There is an existing test conflict: `dose_write_api.rs` case 2 of
`invalid_time_source_unit_and_stock_selection_leave_no_partial_take` expects 422
for the same direct unsupported-kind request with a valid portable ID and UUID.
That expectation must be explicitly ruled and captured in its own baseline;
it must not be silently changed or bypassed.

Rust `create_in_transaction` is shared by the direct take route, sync medication
take writes and occurrence recording. Sync also has separate replay authority
checks; occurrences construct supported source kinds internally. A direct-route
compatibility classification can keep shared contracts unchanged. Restrict it
to an otherwise valid source ID and supplied nonblank unsupported string kind,
preserving shape/error priority, malformed or missing/nonstring/blank source
422, authentication, rollback and audit behaviour. Any proposed shared-core
taxonomy change instead needs an explicit sync/replay ruling and focused failing
controls first. No production change is accepted by this diagnosis.

Root subsequently authorised the shared-path classification, contingent on
focused direct and sync RED/GREEN tests. Independent source review supports
that bounded ruling: Rails MedicationTakeOperation requires nonblank references
then yields to BatchesController, whose resolver raises RecordNotFound for an
unsupported kind. OperationCatalog only validates resource/action. The earlier
direct-only recommendation is therefore superseded by an explicit shared
compatibility ruling, rather than silently extending a route-only fix.

The forthcoming controls must distinguish a valid identifier with a supplied
nonblank unsupported string (404) from missing, blank, whitespace or non-string
kind and malformed identifier (existing 422). Outer shape and unknown-field
priority, authentication, rollback, replay authority and trusted occurrence
construction must remain intact. Require focused direct and sync failures
before changing the shared production validator, then matching GREEN with
complete no-write evidence. The conflicting dose_write_api expectation remains
held until its unchanged six-case baseline is captured. No production patch or
new focused test setup has yet been accepted at this update.

The private focused two-case draft at
`/private/tmp/household-g-20261002/dose_source_errors.rs` checks twenty attempts
per direct/sync path. Valid seeded source readback, current timestamps and
distinct request UUID ranges are sound. Missing/null/blank/whitespace/non-string
references and malformed IDs remain 422; an unexpected inner field requires
the unchanged exact priority message before the final well-formed unsupported
kind expects 404. Complete household medication, dosage, assignment, schedule,
take, version and sync row snapshots are compared before status assertions, so
the intended RED still proves its actual rejection writes no clinical data.

One narrow amendment was requested before installation: the draft's sync call
uses post_json_authorized and sends no Idempotency-Key. Keys are optional and
this cannot itself trigger a key replay conflict, but it does not exercise the
requested keyed-path coverage. Use the existing post_json_with_key helper with
a distinct valid key for every sync attempt, keeping the direct calls and
assertions unchanged. Twenty sync attempts remain below the route's 30-request
bucket. Final setup approval awaits that amendment; production remains unchanged.

The amended private packet now has setup PASS. Every sync attempt uses the
existing post_json_with_key helper with a distinct valid key whose final segment
is its attempt index. These keys are separate from the already distinct direct
and sync client UUID ranges. Direct requests, malformed-reference controls,
unknown-field priority and full clinical snapshots remain unchanged. Fresh key
identity ensures an earlier rejected request cannot mask the intended source
classification behind cached replay or a payload conflict. Installation and
actual direct/sync RED remain required before the production validator changes.

The separate unchanged replay baseline is independently verified at
`G-REPLAY-BASELINE-001/runner.raw.log`: all eight named HTTP cases pass, zero
filtered, followed by six route smoke passes. Original pre/copy/live manifests
each hash to emitted source digest
`43009eccbfe57883dbefca91688ccddc20286fef741b1ea80bf531373699912a`;
the full raw SHA-256 is
`acaffdf3a2c9749953af752a9aceb284c31c80ae231a929c48c9dba05bf7abaf`.
This is useful before-change replay evidence, not a replacement for focused
source-error failures or post-change replay compatibility.

### Dose extraction map review

Planning requirements verdict: PASS with the safeguards below. Planning quality
and security verdict: PASS. This approves cohesive boundaries in the read-only
map, not source extraction, body preservation or runtime behaviour.

Independent comparison of
`/private/tmp/household-g-20261002/dose-extraction-map.md` with current dose.rs
supports the nine responsibilities: source/effective configuration, current
access, history representation/query, submitted dose preparation, timing,
UUID replay, stock selection/decrement, request/domain audit and take insertion.
The named external exports and current callers are complete, including accepted
taper-display helpers. The public facade continues owning index/create response
transactions and create_in_transaction orchestration.

Preserve the existing write sequence exactly: input checks, household lock,
current membership/account/lockout checks, client UUID lock and authorised
replay, preparation and timing, take insertion, stock decrement, then domain
audit on the same supplied transaction. Existing source permission and replay
authority helpers must move intact, with no new policy, recurrence, effective
dose or stock logic. Index authentication/audit/commit and create rollback/error
audit also remain in their original wrappers.

Capture the final repaired unsplit source after the approved source-error change
as the preservation baseline, rather than comparing against the older map-era
file. Add the focused dose_source_errors target to both before/after checks
alongside the six named existing targets. Retain the four unit tests as one
unchanged block; the doses target has fourteen executable cases plus one
pre-existing ignored case, not fifteen executed cases. Current dose.rs has no
comment lines; preserve attributes, signatures, types and complete bodies, and
report formatter normalisation separately from exact byte proof.

Private child re-exports require appropriate internal visibility: a child
pub(super) item cannot be re-exported to the facade's parent. Use only necessary
pub(crate) bridges inside private modules, public Pagination type where needed,
and unchanged facade export visibility. Source/ProposedTake fields need only
sibling-facing access, not public exposure. Compiled visibility checks and
post-split runtime remain mandatory; no extraction is currently accepted.

### Shared source-error RED and stable guard review

Independent reading of `G-DOSE-SOURCE-ERRORS-RED-001/runner.raw.log` confirms
both paths reach zero-based attempt 19 and fail only at 422 versus required 404.
The eighteen malformed-reference controls and one unknown-field priority
control precede it successfully. Clinical no-write snapshots also pass before
the final status assertion. No route smoke runs. Original 402-entry pre/copy/live
manifests match digest
`6362974393e8b5f945782c2580da5892aa9c279c29cf558cb812ca1c499820cf`.

The resulting minimal shared guard has requirements and quality/security
static PASS. It keeps valid_identifier and supplied nonblank string validation
at 422, then classifies unsupported enum values through ApiError::not_found.
This happens after unchanged outer/unknown-field validation and before the
existing UUID checks. Missing/null/non-string/blank/whitespace references and
malformed identifiers retain their old rejection. Authentication, locks,
replay, occurrence construction and write/rollback sequences remain unchanged.

Reversing just the two agreed guard edits in memory exactly reconstructs the
captured dose.rs SHA-256
`bbd57158e95e2205181a3efdf93c7916fabb08daffff811b89b2a4936f682d57`.
Reversing only the documented dose_write_api unknown-kind expectation 404 to
422 reconstructs its captured SHA-256
`bcc545509344006441ba11cc23e263535811534ac7d1368a76fd80c1dad52125`.
This independently proves the remaining source bodies/assertions/comments are
unchanged. That obsolete expectation correction follows the recorded unchanged
six-case dose-write baseline and the explicit Rails-compatible ruling.

The final acceptance facade now adds the unchanged focused source-error target
as source_errors, giving fifteen cases in serial alphabetical module order:
assignments, inventory, people, source_errors, stock_adjustment_audit. The new
controls create no clinical rows or grant/profile changes, and use fresh sync
keys. Static composition PASS does not replace the final selected list/runtime.
Matching guard GREEN and dose extraction remain pending.

### Remaining extraction maps: planning review

The three plans divide the remaining modules along existing responsibilities
without changing how requests authenticate, retry or write. They are ready for
their separate before-split checks. No source split or runtime behaviour is
accepted by this planning review.

Planning requirements verdict: PASS with the dispatch safeguards below.
Planning quality and security verdict: PASS. I compared the occurrence,
invitation and OAuth maps in `/private/tmp/household-g-20261002/` against current
source declarations, external callers and named test targets.

The occurrence facade retains list/mutation and sync orchestration, including
household locking, current access, authorised replay, original preconditions,
decision/take linkage and delegation to dose::create_in_transaction on the same
transaction. Source identity/access, signed keys, calendar projection,
representation, errors/audit, replay, submitted fields and decision persistence
are supported boundaries. Keep the dashboard timezone task-local declaration
with its scope helper, complete Kind/Source impls, RangeQuery visibility and
original facade exports. Preserve signed-key bytes and recurrence rules.

Its named targets are openapi_dose_occurrences (ten source-declared cases),
doses (fourteen executable and one existing ignored), sync (twenty-three),
replay (eight), openapi_sync_reads (two), and the existing occurrence unit test.
Retain dose_source_errors as a focused shared-creation compatibility check.
Name the actual browser selection as `tests/dashboard.test.mjs`; use its
fixed-clock fixture separately from real-time recording checks. These are
planned selections and source counts, not completed test results.

The invitation facade retains every request and transaction orchestration
body, including resend_inner, accept_inner and accept_pending. Mail transport,
input/representation, token creation, responses/audit, keyed replay and
acceptance effects are coherent boundaries. Keep MailConfig and route exports
at their current visibility. Membership locks, permission versions, accepted
retry, grant effects and audit/sync linkage must stay in their original order.
SMTP must remain at its current position before transaction completion.
Before/after targets are openapi_invitations (six source-declared cases) and
invitations (eight), using the named existing self-provisioning Tasks. The
SMTP-failure case must actually execute against rust-api-mail-fail and Mailpit;
ordinary successful mail delivery does not prove rollback on delivery failure.

The OAuth map preserves complete transaction-owning handlers, routes/api_routes
and all existing externally used exports. Signing/configuration, cookie/session
access, shared HTTP/database helpers, discovery, client authorisation, browser
login/logout and token lifecycle are cohesive boundaries. Shared signing and
cookie constants stay unique. Preserve cookie bytes, PKCE/client/redirect/scope
checks, current account and membership checks, MFA, origin/CSRF, password-worker
limits, single-use codes, refresh rotation, revocation and renewal headers.
Current callers include request authentication/guards, WebApi, native browser
write adapters and occurrence-key secret access; none may change during the
move. Targets are oauth (eleven source-declared cases), web_session_api (nine),
medication_mobile_oauth_api (seven), openapi_auth_sessions (eight), plus the
explicit `tests/login.smoke.test.mjs` browser selection. That browser file
checks standalone login/logout, failed login and mobile authorisation/consent.

Each split remains serial and held until its own meaningful unsplit baselines
pass. Capture the final accepted source immediately before moving it; compare
complete function/type/impl bodies, attributes and comments independently, and
qualify formatter changes separately. Internal bridges may widen only enough
for existing sibling callers/re-exports while facade visibility stays exact.
Compiled visibility and post-split runtime proof remain pending. Do not share
fixtures across credential, membership, refresh/logout or grant mutations
without a separate mutation-order review.

### Remaining issue 2347 readiness check

Stock timestamp checks are already fixed and passed. Two fixture/property-list
repairs and an explicit cache-header contract clarification remain. The
view-only dashboard has its accepted product fix, but its original session
cases still need matching runtime results. This review made no test or product
changes and ran no runtime commands.

I fetched the live issue body through the GitHub connector. It remains open
with its original checklist; the checklist has not incorporated the later
accepted local corrections. Source-capability ordering/stock eligibility,
People pagination and dose-mode corrections are already reviewed separately
and are not re-reviewed here.

| Item and exact target | Current status | Narrow correction or next check |
| --- | --- | --- |
| management_sync_events_api | Its two tracked cases still call an invalid direct dosage INSERT. | After retained current failure evidence, supply the three required dosage defaults in that helper only. Keep all event/replay/no-write assertions. |
| medication_stock | Completed locally: shared UTC/microsecond helper is present, and the recorded E baseline passed 9/9. | No new timestamp or production correction is needed. |
| openapi_medications | The exact Medication key helper still omits three documented optional fields; the earlier focused baseline was 7/9. | After current failure capture, distinguish required keys from allowed keys; permit only the three documented nullable strings when present. |
| openapi_read_completion | Two exact header assertions still require no-store while Rust emits private, no-store. | Agree and record the safe representation clarification below before changing tests or schema. |
| web_session_api | Source includes the accepted optional-profile dashboard fallback; exact original session-case runtime remains pending. | Run the unchanged nine-case target in its own fresh fixture. Preserve all permission/session assertions. |

For management_sync_events_api, the affected names are
tracked_removal_emits_option_and_parent_updates_without_replay_events and
tracked_dose_emits_take_person_metadata_and_stock_updates_once. The shared
tracked_option INSERT at line 125 supplies amount/unit/frequency/supply but
omits default_dose_cycle, default_max_daily_doses and
default_min_hours_between_doses. db/schema.rb lines 430–434 require all three
without defaults. Daily cycle 0, maximum 4 and minimum hours 0 match the
accepted dose-mode helper and dosage validation; leave its supplied stock,
amount/unit/frequency unchanged. The adjacent person_medications INSERT uses
existing nullable/defaulted fields correctly. The ordinary management case
does not call this dosage helper. There is no dedicated self-provisioning
management-sync wrapper in the current Taskfile; use the established exact
per-file selector and report all three actual case results.

The completed medication_stock repair at lines 55–60 parses RFC3339, requires
UTC plus the Z suffix and nanoseconds divisible by 1,000. It preserves
microsecond precision and optimistic ETags without demanding a fixed string
length. The original E baseline failed seven cases at length 27 versus 20;
the repaired E-FINAL-BASELINE-001 receipt recorded medication_stock 9/9,
openapi_stock_workflows 3/3 and medication_management_security_api 5/5. These
are completed prior receipts, not a new runtime on the current G candidate.

The two focused Medication shape failures are
medication_list_create_get_patch_and_replace_match_openapi_shapes and
medication_write_requests_validate_openapi_shape_types_and_nonmutation.
Both use assert_medication, whose MEDICATION_FIELDS currently contains only
the required keys. OpenAPI Medication at lines 6290–6340 separately allows
friendly_name, barcode and warnings as nullable strings; they are optional,
not required. medication_projection.rs lines 36–38 emits all three. A correct
helper preserves every current required key and additionalProperties:false,
accepts only these three extra names, and checks string-or-null whenever each
is present. Do not add optional keys to the required list or permit arbitrary
extra properties. Other shape/nonmutation assertions stay intact.

The read-completion failures are
capabilities_and_household_reads_match_openapi_shapes at line 110 and
household_list_scopes_app_tokens_and_keeps_mobile_account_scope at line 218.
api_routes.rs merges the OAuth API routes before applying private_api_cache;
api_request_guards.rs lines 62–69 then writes private, no-store for every API
response. Both spellings prohibit storage. Rails CapabilitiesController#show
explicitly sets no-store, while its request spec checks that the header
includes no-store. Rails Auth::SessionsController#households has no explicit
Cache-Control assignment; no Rails runtime header was inspected here.

The schema distinction matters: /capabilities has an exact enum containing
only no-store at OpenAPI lines 133–139. /auth/households declares no response
Cache-Control header at present. Therefore the capability case has both a
literal schema and test disagreement; the household case has only the exact
test disagreement. Recommended smallest explicit clarification, if Rust's
additional private directive is the accepted behaviour: allow no-store and
private, no-store for the capability header while retaining required:true,
and test the mandatory no-store directive with optional private on these two
paths. Document a household header guarantee separately if desired. Do not
silently change the shared no_store/export schemas or accept cacheable medical
responses, missing no-store, public/max-age variants or English-like string
containment as a substitute for directive validation. This recommendation is
a proposed clarification, not an applied repair. Root subsequently adopted
that bounded ruling: change only the capability enum to those two values, and
the two focused tests to mandatory no-store with optional private. Unrelated
shared/export header contracts stay unchanged. The compatibility repair
remains held while dose inputs are frozen, and current failure receipts must
precede installation. Rails household headers remain source-only and not
runtime-verified.

The two original view-only cases are
cookie_access_respects_foreign_households_and_ungranted_people and
suspended_membership_denies_cookie_access_without_changing_other_sessions.
Both BrowserClient::login calls follow the login redirect and require a 200
dashboard before their permission assertions. Current dashboard.rs reads
mandatory /me, tolerates only profile 403/404 as optional, and propagates all
other required-read failures. The earlier accepted nine-case completion
dashboard target proved the limited-viewer fallback, access and role bounds.
That evidence supports the repair but does not claim these two original
session tests executed. Keep their unchanged hidden/foreign denial and current
membership checks; no grant, login shortcut or permission relaxation is
indicated. The planned OAuth before-split web_session_api baseline is the
appropriate fresh-fixture proof.

Requirements verdict: outstanding readiness checks are identified; no full
issue acceptance is claimed. Quality/security verdict: preserve the existing
stock precision, required keys, unknown-key rejection, no-store protection and
current access checks. Keep proposed remaining test corrections private until
the verifier captures their actual current failures.

### Shared source-error fix: focused acceptance

Well-formed unsupported dose source types now return 404 through direct and
sync writes. Malformed references still return 422, and rejected requests
leave clinical records unchanged.

Requirements verdict: PASS for this bounded fix. Quality and security verdict:
PASS, combining the independently reviewed minimal guard with matching focused
runtime evidence. This does not accept the pending full dose baselines,
fifteen-case final composition or any source extraction.

I independently read G-SOURCE-ERRORS-GREEN-001/runner.raw.log. Both named direct
and sync cases pass, with zero failed/ignored/filtered cases, followed by six
route smoke passes and zero browser failures. The cases retain all twenty
requests each, unknown-field priority, malformed-reference controls, distinct
request/replay keys, valid unsupported-kind 404 and complete clinical no-write
snapshots. The formatter, API check, warning-denying Clippy, both selected
compile receipts and wrapper exit files each record 0.

The full raw SHA-256 matches
`0a99a785207817d5b799ecb0150fba2a7335cc74f89174e7b16ff5cb2738f6a5`.
All three original pre/copy/live manifests contain 402 entries and independently
hash to
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`;
retained pre-copy and pre-live differences are empty. This full value replaces
the earlier inaccurate abbreviated digest. The emitted fixture SHA-256 is
`6f5f87d27b0c1a75fa9bcff3ae066505cf9e4296d6bb8a8c916d9674267d2e6b`.
The prior two-case 422-versus-404 RED and static inverse-hash preservation proof
remain recorded above; no permission, lock/replay or successful-write policy
change is claimed beyond the agreed unsupported-source classification.

### Private remaining parity proposals: setup review

Proposal requirements verdict: PASS. Proposal quality/security verdict: PASS.
The four private files in
`/private/tmp/household-g-20261002/parity-proposals/` match the recorded narrow
rulings. I compared each directly against its checked-in counterpart; no
proposal is installed or runtime-accepted by this review.

management_sync_events_api changes only the tracked dosage INSERT to provide
daily cycle 0, maximum 4 and minimum hours 0. Its original amount, unit,
frequency, supplied stock and all behavioural assertions remain unchanged.

openapi_medications replaces only the Medication exact-key assertion with
separate required/allowed checks. Every MEDICATION_FIELDS member remains
required. Only friendly_name, barcode and warnings are permitted as additional
keys, and each must be string-or-null when present. Missing required keys and
every other extra property still fail; no required schema list changes.

openapi_read_completion adds one header helper and uses it only in the two
agreed capability/household cases. The helper reads every Cache-Control header
value, splits directives, trims whitespace and normalises case, then sorts
without deduplication. It accepts exactly one no-store directive and optionally
exactly one private directive. Missing headers, empty directives, duplicate
no-store/private, extra cache directives and parameterised variants fail.
Multiple header values cannot hide an unsafe or repeated directive. Existing
shape, credential scope and access assertions remain intact.

The private OpenAPI change adds only private, no-store to the capability
response's exact enum. Its required:true and existing no-store remain. No
household, shared no_store or export schema is changed. No production response
policy, comment or unrelated test assertion is modified. Keep these proposals
private while dose inputs are frozen and obtain current failing receipts
before installation, followed by the affected target results.

### Newly reached dose failures: independent classification

The repaired fixture now reaches real dose behaviour. Eight cases pass, six
fail at assertions, and the existing timestamp case remains ignored. Three
failures show occurrence requests being recorded under the wrong controller;
two expose Rails error-code differences. The remaining failure is a legacy
pagination expectation that conflicts with the published strict bounds.

I independently read G-DOSES-GREEN-001/runner.raw.log and verified its SHA-256
`16b874a716881b7ccb3d4312c18ce47d6858dcb580a36049c882501e6e5bf5a1`.
The raw result is eight passed, six failed, one ignored, zero filtered and no
route smoke. Original pre/live manifests both hash to
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`.
No retained original copy manifest exists for this job; no independent
copy/live equality is claimed. The job name does not make its failing outcome
GREEN. No new runtime or source edits were made by this review.

| Reached test failure | Classification and recommended ruling |
| --- | --- |
| direct_assignment_outcome_reopens_then_records_one_take, line 205 | Confirmed audit attribution difference; retain expected api/v1/dose_occurrences. |
| schedule_not_taken_reopens_with_a_current_version_and_retains_context, line 205 | Same confirmed audit attribution difference. |
| schedule_take_replaces_not_taken_once_and_stays_immutable, line 205 | Same confirmed audit attribution difference. |
| direct_take_failures_leave_stock_and_take_history_unchanged, line 1672 | Confirmed occurrence-domain error difference: preserve paused, rather than the generic direct-take code. |
| occurrence_writes_reject_invalid_identity_context_time_and_amount_without_mutation, line 1487 | Confirmed direct numeric-contract validation difference: preserve validation_failed. |
| medication_takes_create_filters_paginates_and_preserves_precision, line 1076 | Conflicting legacy pagination expectation; retain published strict take bounds and add a valid filtered-read control. |

For the three audit failures, Rust dose_occurrences.rs Kind::controller at
lines 59–64 returns api/v1/schedules or api/v1/person_medications, and all
occurrence response/error audit calls use it. Rails routes these requests to
Api::V1::DoseOccurrencesController; BaseController#record_api_request_event at
lines 138–151 records controller_path, hence api/v1/dose_occurrences. The
expected value is supported by observable Rails code, not merely a test name.
Correct request-controller attribution without changing the source policy,
source identifiers, current permission checks or domain event linkage. The
later outcome/replay assertions in these cases were not reached yet.

For the paused occurrence take, Rust forwards the ApiError from shared
dose::create_in_transaction after rolling back the write transaction, then
audits/caches it through the existing response path. Shared direct-dose errors
use unprocessable_content. Rails OccurrenceResolver#record_dose at lines 66–73
raises its typed Error with the RecordDose result symbol as code;
DoseOccurrencesController#render_occurrence_error at lines 81–84 preserves that
code and returns 422 for ordinary domain failures. The test's valid current-day
untimed schedule remains projectable because the just-created pause interval
does not cover the whole local day; the current active=false source then
causes RecordDose's paused result. Keep direct medication-take errors at their
current generic/message contract unless separately ruled; preserve occurrence
domain codes at the occurrence boundary. The later out_of_stock assertion in
this same test follows the same Rails rule, but was not executed after paused
failed. Do not claim its no-write, resume or cooldown assertions passed yet.

For the numeric amount, the failing request is a direct medication_take with
JSON dose_amount 1.25, not an occurrence request. Rails
MedicationTakesController#medication_take_params rejects numeric source_id and
dose_amount before source lookup. BaseController#render_invalid_contract_value
at lines 282–289 emits validation_failed with errors for that field. Rust
decimal_from_json at lines 725–735 instead returns a generic
unprocessable_content error for non-string values. The OpenAPI request
requires a decimal string, while the generic Error schema permits either code
and optional errors; Rails is the precise code/field-shape reference. Keep
unknown-field/malformed-reference priorities and no-write assertions. Any
repair should distinguish typed numeric-contract failures from domain failures
and be checked through direct and sync callers, rather than change all generic
422 errors or infer codes from untrusted diagnostic text.

For pagination, Rails MedicationTakesController#index calls the shared
BaseController#paginate, which clamps page at one and per_page to 1–100. Its
request spec explicitly proves page=0/per_page=500 becomes 1/100. However the
authoritative OpenAPI page/per_page schemas at lines 4329–4346 require
page>=1 and per_page<=100, and the existing focused openapi_medications tests
at lines 680–716 explicitly require 422 for invalid take page/size values.
Current Rust index_in_transaction matches that strict published contract.
Recommended bounded clarification: repair this legacy doses expectation to
assert 422 for its invalid request, then make a separate valid page=1,
per_page=100, updated_since request retaining the original take-presence and
filter/pagination checks. Preserve invalid-filter errors and all other
resource rules. This remains a coordinator ruling, not an applied test change.

Requirements verdict: not accepted; the compatibility differences and
contradictory pagination assertion need explicit scoped resolution. Quality
and security verdict: preserve rejection/no-write guarantees, source policy,
authorised replay, lock ordering, audit linkage and ignored-case status. Dose
extraction and subsequent acceptance remain held pending meaningful passing
baselines. No Rails runtime was run by this review.

### Private precise dose compatibility candidate

The private candidate corrects occurrence audit attribution and gives direct
numeric amount failures their field error, while preserving the separate
occurrence paused code. It leaves out-of-stock mapping unchanged. The public
sync path retains its existing error responses.

Proposal requirements verdict: PASS for the reviewed boundaries. Proposal
quality/security verdict: PASS. This is source review of the three private
files under `/private/tmp/household-g-20261002/dose-compatibility-proposal/`,
not installation, compilation or runtime acceptance.

I compared their exact diffs against checked-in source. Kind::controller now
labels occurrence request audits api/v1/dose_occurrences, while source kind,
policy and request identity remain unchanged. All success/failure/cached-replay
audit paths use the same method; there is no new policy or domain event.

TakeFailure is dose-local and carries the original ApiError plus an optional
typed cause. From<ApiError> assigns no cause. Only the existing inactive-source
check assigns Paused, and the existing amount-parsing position assigns
NumericDoseAmount for a JSON number. Null, boolean, array and string inputs
continue through their existing parsing behaviour. Earlier outer/unknown-field,
source-reference, UUID/unit/stock-ID checks, household lock, current membership
and account/lockout checks and authorised UUID replay still precede preparation.
Source permission, date/window checks, effective configuration, stock selection,
timing, insertion, decrement and domain audit retain their existing sequence.
Explicit Err conversions only wrap the existing error.

The original create_in_transaction facade signature remains and strips the
typed cause, so sync_batch continues receiving the old ApiError. Direct create
uses the same core and adapts only NumericDoseAmount after its original rollback
and failure-audit sequence. The response carries validation_failed, Validation
failed, errors.dose_amount=[must be a string] and the same request ID/status;
all other direct errors are unchanged. Occurrence create uses the same core
and adapts only Paused to the Rails resolver code/message before its unchanged
rollback, current reauthentication and audit/cache path. No raw-message match,
global ApiError widening, error suppression or out-of-stock adaptation exists.

The private doses change rejects the invalid page-zero/size-500 request at 422,
then makes the original filtered collection read with valid page one/size 100.
Original metadata/take-presence/filter assertions and the ignored timestamp
case remain. Before installation, strengthen the numeric assertion with the
exact message/field errors and focused earlier-validation/direct-versus-sync
controls promised in the rationale. Compiled visibility and meaningful affected
runtime remain required; no extraction starts from this static verdict.

The proposed independent zero-stock exposure is a valid next RED design:
create a unique current schedule, capture its valid occurrence, set its own
medication supply to zero through the real API, and capture rejection snapshots
after that successful setup write. Compare stock, takes, occurrence and scoped
clinical version/sync counts before asserting 422/out_of_stock. Keep the direct
take generic error as a control. This exposes the currently blocked occurrence
assertion without relying on the paused case succeeding. A reached failing
receipt is required before introducing any out-of-stock mapping; this review
does not claim that new case is authored or executed.

### Private numeric and zero-stock test amendments

The independent zero-stock case has setup PASS. The numeric-priority case
needs one control expectation corrected before setup PASS; no production
behaviour change is justified by that control.

I read the appended cases in the private doses candidate and the separate
zero-stock-case.rs. Both use accepted medication/schedule helpers, valid
portable source IDs and the real current UTC clock. The zero-stock case
captures a current occurrence, writes zero through the real medication API,
verifies that setup succeeded, then takes its rejection baseline. It compares
exact stock, take IDs/count, occurrence rows and full household medications,
dosages, assignments, schedules, occurrences, takes, versions and sync rows
after both rejected writes, before checking error codes. Security request
audits and retry cache are correctly outside the clinical no-write snapshot.
The direct generic out-of-stock control executes first; the occurrence code
expectation remains an intended RED on the unchanged mapping. Client UUIDs
ending 201 and 202 are distinct from the numeric-control ranges and existing
11111111-prefixed dose requests.

The numeric case has six direct controls and four sync controls, with distinct
client UUID ranges ending 300–305 and 402–405 and four distinct valid sync
idempotency keys. It compares the same complete clinical state before each
status assertion. Unknown envelope/attribute fields, malformed source and
unsupported source remain earlier than amount parsing; direct numeric failure
expects the exact message/field errors while sync keeps its generic response.
The existing numeric assertion also gains exact message and field-error checks.

One concrete setup error was sent to the writer: direct index zero supplies
medication_take:null and expects 400/bad_request. Both checked-in and private
core call dose::error(BAD_REQUEST, medication_take is required), whose code is
unprocessable_content for every non-conflict status. The new typed failure
has no cause here and intentionally preserves that code. Correct only the
control to 400/unprocessable_content, retaining its exact message and no-write
check. Do not expand the product adapter to satisfy this invented expectation.
All other setup checks are sound; compilation/runtime and actual zero-stock
failure evidence remain pending.

For eventual CI, the doses target belongs in its own fresh serial fixture,
not the final acceptance composition. Its current-day occurrence/take and
cooldown/pause/replay stimuli use the real UTC clock. Do not set
CONTRACT_DASHBOARD_NOW or share its fixture with calendar, permission, lifecycle
or other dose suites. Preserve the existing ignored timestamp case in reported
counts. No CI row is installed or accepted by this recommendation.

The writer corrected that sole numeric setup error in both the private
standalone case and appended candidate: index zero now expects
400/unprocessable_content with medication_take is required. I verified the
current values; the original status, message and full no-write checks remain.
Both private appended cases now have setup PASS. Checked-in files and the
product candidate remain unchanged; actual failure/compiled checks are still
required and no runtime acceptance follows from setup approval.

### Standalone doses CI route smoke selection

The existing route smoke file can accompany the doses target in its own fresh
real-clock fixture without introducing another clinical journey. Static setup
PASS: tests/household-routes.test.mjs contains one parent and five nested tests,
which account for the six reported Node cases. It uses a fresh primary-owner
cookie login, then reads three SSR pages, current UI capabilities and an existing
medication. It never submits clinical forms or changes grants, people, profiles,
memberships, stock or lifecycle state. Login activity and security request audits
remain expected side effects. Browser and context cleanup are explicit.

The doses target's ungranted-person case creates a separate adult; it does not
revoke the primary owner's grants or alter their profile. No fixture bearer is
used by the smoke file, and no fixed dashboard clock is required. Select this
file explicitly rather than claiming wrapper defaults ran. The final actual
doses-plus-route run and CI wiring remain pending; this is a source-based setup
ruling, not runtime acceptance.

### Read-completion oversized People query control

The frozen read-completion baseline failed all three cases: two strict header
values differed from private, no-store, and per_page=101 returned 200 where its
old control expected 422. No route smoke followed. The latter expectation is
obsolete under the accepted People-specific positive-size clamp: people_index
caps the typed size at 100 before the existing strict parser, then applies its
unchanged authorised person scope.

The exact private amendment has setup PASS. It removes only per_page=101 from
the invalid-query loop and adds a separate 200 response assertion with exact
data/meta keys, page 1, size 100, unchanged authorised total, at most 100 rows,
valid person shape for every row, hidden-person exclusion and managed-person
presence. Page zero, malformed page, invalid timestamp and all later detail,
viewer, hidden and foreign access assertions remain unchanged. Those later
checks were not reached by the failed baseline, so their passing result remains
pending. There is no production change or checked-in installation in this
proposal review.

Baseline source evidence records matching original 402-path pre/copy/live
manifests at e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920.
The fixture hash is only the runner-emitted receipt; independent rehash missed
cleanup. The complete baseline log is G-OPENAPI-READ-COMPLETION-BASELINE-001/
runner.full.log under the private G evidence directory, with SHA-256
24972cfd7c216d2f79970a7e2c6ab5eb32717d5a88fc2eb7495ec92f4b222f8f.

### Zero-stock selection setup correction

The original added zero-stock test did not reach its intended error branch.
It explicitly selected the zero-supply medication, but prepare removes zero
supply from matching stock before looking up an explicit selection. That
request correctly fails as an unavailable location. My earlier setup PASS
missed this branch; any resulting invalid-selection failure is setup evidence,
not an out-of-stock product RED.

The corrected private stimulus has setup PASS. It removes only
taken_from_medication_id from both the direct and occurrence requests. The
occurrence adapter forwards this field only when supplied. The canonical
medication helper creates a nanosecond-unique name, and same_stock_signature
requires the exact name, amount and unit in the same authorised household.
After setting this sole matching medication's supply to zero, the unselected
request reaches the empty matching-stock branch. Other fixture medications
cannot satisfy its signature. The two UUIDs, source/occurrence identity and all
before-status stock, takes, occurrence and complete clinical/version/sync
snapshots remain unchanged. No product mapping or installed test changes are
accepted by this correction; a genuine domain-error runtime result is still
required.

The parent identified an additional Rails priority that qualifies the earlier
classification. MedicationAdministration::RecordDose.prepare_valid_take checks
resolver.blocked_reason at lines 126–127 before resolve_stock_source at line
132. MedicationStockSourceResolver.blocked_reason returns out_of_stock when
available_medications is empty, after its paused/inactive checks and before
explicit selection validation. Thus explicitly selecting the sole zero-stock
medication is also a valid Rails parity stimulus. My earlier invalid-selection
description explains current Rust behaviour, but does not settle the required
compatible response. It must not dismiss that independent parity case.

Keep the no-selection test to isolate the empty-stock branch and add a separate
explicit-selection test with its own unique source and request UUIDs, retaining
complete no-write snapshots before assertions. Actual failures must precede
any change to error priority or occurrence mapping. This priority ruling is
based on Rails source, not a Rails runtime result. The independent private
packet still needs exact setup review.

The exact zero-stock-two-cases.diff and private doses.rs now have setup PASS.
The automatic case omits selection and retains UUIDs ending 201/202. The new
explicit case independently creates a fresh schedule/unique medication,
captures its real portable source ID and current-day occurrence key, and uses
UUIDs ending 601/602 with the sole zero-stock medication explicitly selected.
Each case records its baseline after the successful stock-zero PATCH and
compares stock, takes, occurrence rows and complete clinical/version/sync
state before response assertions. The unique name/amount/unit signature keeps
their stock candidate scopes independent. No grants or profile changes are
introduced.

The explicit direct generic out-of-stock response is an intended Rails-priority
failure, while the automatic occurrence request independently exposes domain
error mapping. If the explicit direct assertion fails first, its later
occurrence assertions remain unexecuted. No product change or runtime
acceptance follows from this setup approval; the parent owns test-only
installation and the fresh failing run.

The fresh two-case runtime is now a verified RED. In
G-DOSES-ZERO-STOCK-RED-002, the explicit test fails at the direct error message:
unavailable location instead of out of stock, after its complete no-write
snapshots. Its occurrence assertions were not reached. The automatic case
passes the direct generic out-of-stock control and snapshots, then fails the
occurrence code: unprocessable_content instead of out_of_stock, after its
snapshots. Exactly two tests ran, both failed, with sixteen filtered out and
no route smoke after Cargo failure. Selected compilation passed.

I independently verified raw SHA-256
8e5ff19b611c9b0a2d93b7c17a5e04b436cf46b0458e0a0df820befea937d5ec
and all three original pre/copy/live manifest hashes at
70536947125c1d262d62becbae1d48b22b34ecca0671c281d71717624affccc5.
The fixture was independently captured before teardown. This establishes both
the explicit-selection priority mismatch and occurrence domain-code mismatch;
it does not accept a product fix. The updated private typed candidate still
requires exact review, compilation and meaningful GREEN results.

The updated private typed candidate now has requirements static PASS and
quality/security static PASS for this bounded repair. It adds OutOfStock beside
Paused and NumericDoseAmount. Only the empty authorised, same-signature,
null-or-positive stock set receives this cause, before explicit selected-ID
lookup. Malformed outer stock references are still rejected before household
locking; source access, date/amount validation and positive-stock selection or
ambiguity remain in their existing order. The occurrence adapter maps only
Paused and OutOfStock causes to their domain codes, retaining status and
activity semantics. Direct and sync paths retain the original generic stock
error through the existing ApiError facade.

The reviewed numeric direct-only field-error adapter and occurrence controller
label remain unchanged. Household locks, current authentication, authorised
UUID replay, preparation, insert, decrement, audit and sync order are preserved;
the diff contains no global ApiError expansion or raw message matching.
Insufficient supply detected later inside decrement_stock remains unchanged
and untyped by this patch. Newly reached assertions for that separate branch
must be diagnosed from actual results. Compilation and meaningful focused/full
GREEN are pending, and no extraction has been accepted.

### Revised extraction sizes and test boundaries

The private extraction-size-review.md has planning PASS. Moving complete HTTP
and shared transaction owners into cohesive private handlers makes the public
facades clearer without fragmenting lock, authentication, replay or write
clauses. The occurrence writing module remains larger because its roughly
388-line mutate routine owns one complete transaction. Invitation issuing,
resending and acceptance are separate whole journeys; SMTP must retain its
current pre-completion position. Sizes are estimates, not acceptance evidence.

The final accepted compatibility source must be the before-copy. The older dose
map needs its new create_with_failure and TakeFailure facade access and
into_occurrence_error caller added; occurrence now uses that typed path while
sync keeps create_in_transaction. Preserve exact current public visibility:
dose index/create are public and their moved entry items must support the same
facade re-export, even if that requires public items inside a private module.
Do not apply a blanket crate-private restriction. Keep occurrence RangeQuery,
record_etag and with_dashboard_timezone paths/visibility and invitation
MailConfig construction through lib.rs intact. Widen sibling bridges only where
actual callers require them.

The proposed root-level include! test split preserves the existing Cargo target
and full libtest names, unlike named module wrappers. Whole helper/test bodies,
original relative include order and the existing ignored attribute stay intact.
Capture and compare the exact registered test-name/ignore list before and after;
new private child directories must not introduce extra Cargo test targets.
Seventeen executable dose cases plus one existing ignored case are the current
source inventory, not an executed acceptance result.

Before and after proof must still cover the exact independent dose targets,
full unfiltered sync and replay, the direct/sync source-error controls and
preserved API unit tests. Occurrence fixed-clock dashboard tests stay separate
from real-time recording, and invitations need the actual isolated SMTP failure
rollback target. Capture normalized function/type/attribute/comment comparisons
separately from formatter changes. No source move or runtime behaviour is
accepted by this planning review; each split remains serial after its meaningful
unsplit baselines pass.

### Corrected dose journey acceptance before extraction

The bounded dose repair has requirements PASS and quality/security PASS on
G-DOSES-FIX-GREEN-001. I independently read all seventeen passing executable
tests, zero failures and zero filtered cases. The original fractional-second
timestamp test remains ignored with its original reason. Both independent
zero-stock cases now reach and pass their complete assertions, including
explicit-selection priority, direct generic response, occurrence domain code
and full clinical no-write snapshots. Numeric direct field errors and earlier
direct/sync validation priorities pass, as do current-person access,
occurrence audit controller, replay, history, timing and stock effects in this
target. This does not silently accept the ignored timestamp behaviour.

The subsequent browser route smoke reports six passing Node cases: one parent
and five nested checks. The raw output explicitly says tests 6, pass 6, fail 0;
describing it as one passing top-level journey is possible, but 1/1 must not
replace the actual six-case count. These checks read authenticated private SSR
pages and capabilities after a fresh owner login. No other browser suite ran.

Raw runner SHA-256 is
b4f8e1f114b9310987b60020e13e8428b308e9ac0b6f1f9826a4ca04f0a9455e.
All original 402-path pre/copy/live manifests hash to
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
with empty comparison diffs. The independently captured fixture hash is
9d11ac8108e6d1815f6323bc1f38f67d097236f66a970c0b3b8506235dc51ab0.
I verified these retained receipts under the private G evidence directory.
The installed dose source differs from the reviewed private candidate only
by formatting; the occurrence candidate matches exactly. Typed causes retain
the reviewed direct/sync facade and current authentication, lock, replay and
transaction boundaries without message matching or global error changes.

Earlier failed/setup receipts remain above. This is acceptance of the bounded
corrected doses target, not all G work or a dose extraction. Fresh focused
source-error/direct-write checks and the other meaningful before-split
baselines still need their current-source receipts before extraction proceeds.

### Fresh focused dose compatibility and amended extraction maps

The same frozen repaired source now has bounded requirements PASS and
quality/security PASS for the fresh focused source-error and direct-write
targets. G-DOSE-SOURCE-ERRORS-GREEN-002 passes both direct/sync control cases
without filtering, then all six route-smoke cases. G-DOSE-WRITE-GREEN-002 passes
all six direct-write tests without filtering, including contending identical
requests, precise scalar/tracked stock effects and rejected partial writes,
then all six route-smoke cases. Both retained runner exit receipts are zero.

I independently verified the full raw hashes:

- Source errors: b65a5b3395b39f569fb2b7cfd4ec5e371dea6438227cf30bef7905a2cbff652d.
- Direct writes: cbe493c921d9912a2ef40d41f8eee065cd0ad0fdacbc6d6f423ed23c27f5bc2d.

For each job, all original 402-path pre/copy/live manifests match
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722
and both comparison diffs are empty. Retained independent fixture hashes are
e8f8e5b39864fc62ef04eca018cc327fd4d94b776517a507763c541cf04dc759
and 66fb96c2d7f6913ec9366e5131fcd91036fac94937a27028407178ce0843466f,
respectively. The verification queue agrees with the actual logs and receipts.

The amended private dose, occurrence and invitation maps also retain planning
PASS. They now explicitly preserve create_with_failure, TakeFailure, both
conversion methods and their current caller, exact public facade visibility,
occurrence timezone/query/ETag paths and invitation MailConfig construction.
The same-target root include test split preserves full names, filters, order
and the ignored timestamp attribute, with complete comparison receipts still
required after any move. These amended plans do not constitute implementation
acceptance. Full current-source sync/replay and other required unsplit baselines
remain pending, and extraction stays held.

### Current-source full sync baseline

G-SYNC-FIX-GREEN-001 has bounded requirements and quality/security acceptance
as a meaningful before-split baseline. I independently read all 23 passing
HTTP tests, zero ignored/filtered/failed cases and the following six passing
route checks. This is the complete sync target rather than the batch_ filtered
wrapper. Executed cases include late invalid/stale rollback, dose occurrence
and pause replay, current saved outcome feed, hidden-person event/tombstone
exclusion and current household permissions. Runner exit is zero.

Raw SHA-256 is
da95f1f4315302a3fb381089f2d809eaa0607f2ca3490af0f5b87b7680aeb3f9.
All original 402-path pre/copy/live manifests hash to
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
with empty diffs. The retained independent fixture hash is
e74f1158bc5420caf50f1900340ea8983240894970c849f67ea4042b4f2b0b3f.
Other required current-source baselines and any extraction remain pending.

### Additional G module size check

The bounded source inventory found no new actionable oversized G module beyond
the agreed four production splits and doses test split. Current G transport,
stock adapter/renderer, adjustment persistence, People reads and source-stock
modules range from 162 to 373 lines and retain clear request, presentation or
domain boundaries. New G HTTP targets are 88–259 lines; the final acceptance
facade composes existing path modules. WebApi deliberately keeps authenticated
cookie transport, private browser intents and complete authorised pagination
together. No separate ownership or dependency smell warrants another split.

Larger health-events, profile, review-prompt, integration and historical
contract-test files predate this G work and lie outside the earlier accepted
eight-file extraction. Their size alone is not a new defect or reason to widen
the final refactors. No additional before-main extraction is recommended from
this check. The agreed serial splits and their preservation/runtime safeguards
remain the authorised maintainability work.

### Current-source full replay baseline

G-REPLAY-FIX-GREEN-001 has bounded requirements PASS and quality/security
PASS as a meaningful before-split baseline. I independently verified eight
passing HTTP cases, no failures/ignored/filtered cases, six passing route-smoke
checks and runner exit zero. Executed tests cover current-grant rechecking after
revocation, persistent UUID replay, concurrent matching UUID/request keys,
distinct-key stale edits, changed payload/account rejection, invisible UUID
collision non-disclosure and saved validation-error replay.

Raw SHA-256 is
9f5b8dc383a1a52ee28e2937c052a7872e91bb8afe6704121a209fd64f709eab.
All original 402-path pre/copy/live manifests hash to
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
with empty diffs. The independently captured fixture hash is
205c081dc31deb377bb8f11bea1eff01a10b7eaa2cf6e8f94cb7b39d79561c41.
Other required current-source checks remain pending. This receipt accepts
neither the private extraction candidate nor the complete G delivery.

### Private dose and test split preservation review

I independently compared the READY private dose-extraction-candidate against
its retained before files, which exactly match the current accepted live dose
and doses files. All 55 production whole items match after removing only
visibility prefixes; complete function bodies match exact bytes. All 40 test
and helper items match exact bytes. The complete four-test API unit block is
unchanged in the facade. All 18 full contract test names equal both the before
source and expected-name receipt, and the original ignore attribute matches.
There are no original or added Rust comment lines in the inspected source.

The 164-line facade and twelve cohesive children keep complete write/history
owners and the supplied transaction intact. Root-level test includes retain
names, filters, target and shared helper namespace without fixture composition.
Exports for public index/create and parent/crate-only helpers remain at their
original facade scopes. Typed causes, preparation, current authentication,
household/UUID locks, authorised replay, insert/decrement and audit ordering
are preserved by the independently compared bodies.

One concrete visibility defect blocks this first candidate: into_api_error
remains pub(super) inside the new responses child. Before the move, that method
was visible at the crate-parent scope; afterward it would be visible only inside
dose. The already-bridged into_occurrence_error correctly uses pub(crate).
The writer must add the same necessary pub(crate) bridge for into_api_error and
record it separately. No method body or caller change is needed. Compilation
and actual registered-name checks remain pending; source preservation alone is
not runtime or extraction acceptance.

The writer repaired that sole private visibility defect: into_api_error now
uses pub(crate), with the bridge recorded in visibility.tsv and README. I
rechecked its exact signature and unchanged body. The repaired private dose
and test candidate has requirements static PASS and quality/security static
PASS. Actual installation, compilation, selected-name discovery and after-split
runtime acceptance remain held until the required unsplit baselines pass and
the coordinator releases this single extraction.

### Fresh dose-mode before-split baseline

G-DOSE-MODE-FIX-GREEN-001 has bounded requirements and quality/security PASS.
Both unfiltered behaviour tests and six route checks passed, with runner exit
zero. The cases exercise linked-schedule mode-switch denial without sync effects
and successful single-dose conversion preserving the direct source.
I verified raw SHA-256
5a938423c7ac71c0c7042d476cb72d7d7c44e0ce0b5390bff5a1315b2c8098b9,
matching original 402-path pre/copy/live manifests at
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
empty diffs and independent fixture hash
53d4f0c66aac78950c5a9c01ade26c807febaa8b99ce5cec3bdd95423a774b4d.
Source-capability and API-unit current-source receipts remain pending, along
with actual acceptance of any extraction.

### Private occurrence split preservation review

The READY occurrence-extraction-candidate has requirements static PASS and
quality/security static PASS. I independently compared all 63 whole items
after removing only visibility prefixes, all 52 complete function bodies as
exact bytes, the exact preserved unit-test block and the before-file against
current live source. No mismatch or comment-line change was found.

The 111-line facade and thirteen cohesive children retain the complete mutate
and sync transaction owners. Signed-key encoding, task-local timezone scope,
calendar/taper/pause projection, typed dose delegation, current access,
reauthorisation, locks, replay, preconditions, rollback, audit and sync ordering
remain in their original bodies. The scheduling child avoids the imported
schedule entity's namespace. Necessary type/field/method bridges remain private
to the occurrence module or retain the original facade scope. RangeQuery,
record_etag, with_dashboard_timezone, eight route wrappers and both sync exports
keep their original paths/visibility.

This is private-source review only. Installation remains serial after dose
acceptance and meaningful occurrence baselines; format, compiler checks,
independent after-source comparisons and actual same-target regressions are
still required. No occurrence extraction or runtime behaviour is accepted yet.

### Fresh source-capability before-split baseline

G-SOURCE-CAPABILITIES-FIX-GREEN-001 has bounded requirements and
quality/security PASS: both unfiltered HTTP cases and six route-smoke cases
passed, with runner exit zero. The target retains the accepted location ordering,
current record/manage distinction, paused and positive-but-insufficient stock
projection controls and actual rejected-dose no-write proofs.
I independently verified raw SHA-256
8b3d971ed597200dd8921c0f96e25a165c5d741228f669c1a555f1f8f06fd9c3,
all original 402-path pre/copy/live manifests at
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
empty diffs and independent fixture hash
9897b0a85b8ecaa24366839706743ce14c7a3c58644505c4933c0271f882398f.
Current API-unit and remaining compatibility receipts are still pending.

### Private invitation split preservation review

The READY invitation-extraction-candidate has requirements static PASS and
quality/security static PASS. I independently compared all 32 whole items
under visibility-only normalization and all 28 standalone function bodies as
exact bytes. The retained before-file exactly matches current live source;
the complete MailConfig impl, attributes/constants and absence of comment
changes are preserved. Its 74-line facade and ten cohesive private children
keep issuing, resend and acceptance transaction owners intact.

MailConfig and from_env use the required crate-only child bridges while their
facade and lib.rs construction access stay unchanged; fields remain private.
All five handler exports retain their original parent scope and route paths.
Current authorisation, household/invitation/account locks, token handling,
replay, permission-version effects, audit and sync retain their exact order.
Actual SMTP delivery occurs in resend, with its unchanged pre-completion
position and failure rollback. A minor README correction was requested so it
does not imply create sends mail; no product-body correction is needed.

This source ruling accepts no installed extraction or runtime behaviour.
Serial prior-split acceptance, both meaningful invitation targets and the
isolated configured SMTP-failure case remain mandatory before/after checks.
The upcoming OAuth candidate has not been reviewed or released by this ruling.

### Current unsplit unit baseline

The corrected G-DOSE-UNIT-GREEN-001 command has bounded requirements and
quality/security acceptance as a before-split unit baseline. I independently
read 36 passing API library units, including all four dose tests and the one
occurrence DST test, plus twelve passing medtracker-web units. There are no
failed, ignored or filtered units; empty binary/doc-test groups add no cases.
The corrected exit receipt is zero, and task.corrected.raw.log hashes to
23ffdd74a280837f1585038b04d5c3f84a1fc3ded1e52db4eb4f01446cd7ba5a.
The retained pre-input manifest matches
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722.

The initial task-d invocation failed before tests because its working directory
made the relative rust/api/Cargo.toml path invalid. It is a command setup
failure, not a failed unit test. This local unit packet retains a premanifest;
I add no original runtime-copy or postmanifest equality claim. The source was
held frozen by the coordinator. The three remaining compatibility targets and
all actual after-split gates still need their own receipts.

### Private OAuth split preservation review

The READY OAuth-extraction-candidate has requirements static PASS and
quality/security static PASS. All 59 whole items and 44 standalone function
bodies match independently. Complete impls, attributes and constants match
after removing only declared visibility prefixes. The retained before-file
equals current live source; the existing module attribute is unchanged and
no comment lines were added or removed.

The 100-line facade and seven cohesive private children preserve whole login,
consent, code-redemption, refresh, logout and revocation transaction routines.
Cookie signing/bytes, CSRF and origin checks, current account/MFA checks,
password-worker bounds, PKCE, redirect/scope checks, single-use codes, rotation,
session renewal and response ordering are unchanged. Public OAuthState,
from_env and both router exports retain their scopes. The occurrence-key method
keeps its effective crate access, browser-session/cookie helper facades retain
their original scopes, and the signing secret remains private. Only required
sibling configuration/session field bridges are added.

The private invitation README now accurately describes resend-only SMTP
delivery; I verified the correction without any product-body change. All four
private production proposals and the dose test split now pass bounded source
preservation review. This is not installation or runtime acceptance. Separate
authentication targets and the real login browser fixture must pass before
and after the serial OAuth move, following the earlier split checkpoints.

### Corrected management sync-event baseline

G-MANAGEMENT-SYNC-EVENTS-FIX-GREEN-001 has bounded requirements and
quality/security PASS. All three unfiltered HTTP cases and six route checks
passed, with runner exit zero. The cases prove only committed management
writes emit attributed updates, a tracked dose emits take/person/stock events
once, and replayed tracked removal adds no option/parent events. The genuine
earlier missing-default SQL setup failures remain recorded; the narrow helper
repair supplies valid required defaults without changing event assertions.

I verified raw SHA-256
8cee720bb484c33571e9a7b6629cabe65d2eb06a42ae8a1bea32f90d04d9257f,
all original 402-path pre/copy/live manifests at
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
empty diffs and the independent fixture hash
176d4850cf2d5ff7fcc88e8b8258b7e98ed7a5d73f633a46fdf6dd8374a9c154.
The two remaining OpenAPI target receipts and actual after-split checks remain
pending; this result does not accept all G work.

### Corrected OpenAPI medication acceptance

G-OPENAPI-MEDICATIONS-FIX-GREEN-001 has bounded requirements PASS and
quality/security PASS. All nine unfiltered HTTP tests and six route checks
passed, with no ignored or failed cases and runner exit zero. Executed controls
cover rate limits, strict list/filter validation, medication/take shapes,
required wrappers, numeric/decimal types, unknown fields and rejected-write
nonmutation. The corrected Medication helper retains every original required
key and unknown-key rejection, permitting only documented nullable optional
friendly_name, barcode and warnings strings.

I independently verified raw SHA-256
e03e752780d13f1a74bc0ff871d36fc26c2502c7d064cc62deb716c35562ba9f,
all original 402-path pre/copy/live manifests at
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
empty diffs and independent fixture hash
5a671eb705829c23570c04041f1417b0ea0a7288a6831de2beb4b6d362b1fc15.
The earlier stale optional-field failures remain recorded. Read-completion is
the remaining corrected compatibility receipt before dose installation can be
released. No after-split runtime or overall G readiness is accepted here.

### Corrected read-completion and dose baseline readiness

G-OPENAPI-READ-COMPLETION-FIX-GREEN-001 has bounded requirements PASS and
quality/security PASS. All three unfiltered HTTP cases and six route checks
passed, with runner exit zero and no ignored/failed cases. Mandatory no-store
is preserved: the helper reads all Cache-Control values/directives and accepts
only no-store with optional private. Extra and duplicate directives do not pass.
The oversized People query executes its bounded 200/meta100 response, valid
person shape, authorised total and hidden-person exclusion. Invalid-query,
detail, viewer, hidden and foreign access assertions now execute beyond the
former failed control. App-token and mobile household scope checks also pass.

I independently verified raw SHA-256
28a56bf4cce3c54a355d348654d0324f2501c6f8e8b3a26b957c6c32cb813700,
all original 402-path pre/copy/live manifests at
4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722,
empty diffs and independent fixture hash
c3e420d0ebdee1a426bc59ce25b962c71d3c22eb2259f354b09d5acb0c9032b0.
The earlier two header failures and obsolete People-size expectation remain
recorded; their correction does not change production access or shared header
contracts.

All planned dose before-split checks now have accepted current-source receipts:
doses, direct writes, mode transitions, source capabilities, full sync, full
replay, direct/sync source errors, preserved API units and the corrected
management-sync/Medication/read-completion contracts. The original single
ignored dose timestamp test remains explicit. This permits the coordinator to
release only the statically reviewed dose production and same-target test split
first. Actual after-split compiler, source/name comparisons and meaningful
regressions are still required; occurrence, invitation and OAuth installation
remains serial behind its own checkpoints. No complete G acceptance is added.

### Installed dose split before formatting

The coordinator released only the reviewed dose production and same-target test
split. I captured its live comparison before formatting: all 22 installed Rust
files exactly match both the private candidate bytes and the 22-path
installed-preformat.sha256 receipt. All 55 production and 40 helper/test
whole-item comparisons still match under the previously declared production
visibility normalization. The four-unit block is exact, root include
declarations match, all 18 source test names match and the original ignored
attribute is preserved.

This accepts the installation's source identity and structural scope. Later
formatting must be qualified separately rather than described as exact raw-byte
preservation. Compiler checks, actual registered-name discovery and meaningful
after-split runtime are still pending. Occurrence, invitation and OAuth remain
private and are not installed or accepted by this receipt.

### First dose-split compiler and lint result

The installed split compiles, but warning-denying Clippy stops on seven unused
facade imports. I read the actual G-DOSE-SPLIT-FAST-001/api-clippy.raw.log.
Two private imports, valid_numeric_10_2 and cycle_bounds_in_zone, are used only
by the preserved unit tests. Five warnings are deliberately retained original
facade exports: Pagination, quantity, same_dosage_signature,
selected_tracked_dosage and sufficient_stock. Their removal would violate the
agreed exact interface preservation.

A narrow structural repair is appropriate: gate the two unit-only imports with
cfg(test) to preserve the entire unit block, or record direct test imports while
keeping its bodies intact. Only a statement-level unused_imports attribute on
the five intentionally preserved compatibility exports is justified. No blanket
module allowance, dummy use or unrelated interface change is approved. The
writer's exact proposal remains to be reviewed before applying it. This is an
extraction lint failure, not evidence of changed medication behaviour; no
after-split runtime acceptance follows yet.

### Structural dose lint repair proposal

The writer found a cleaner repair that preserves the interface without lint
allowances. Static requirements PASS and quality/security PASS apply to the
private dose-lint-candidate proposal. It returns the original Pagination type
and four small shared stock helper definitions to the facade, removes their
redundant aliases, and gates only the two private unit-only bindings with
cfg(test). The facade remains small at 217 lines. Child modules continue to
use the shared definitions through their existing parent imports.

I independently compared Pagination, including its attribute and private
fields, and the whole definitions of quantity, same_dosage_signature,
sufficient_stock and selected_tracked_dosage with dose-before.rs. All five
match exactly, including original visibility. The entire four-test unit block
also matches exactly. The current and proposed bytes for dose.rs, history.rs
and stock.rs match every before/after hash in source-receipt.tsv. The diff
contains no comment, helper body, transaction, authentication, replay, stock
selection or mutation change. There are no new allowances or dummy uses.

This structural proposal supersedes the earlier possible statement-level lint
allowance. Installation may proceed under the coordinator's release. Compiler,
Clippy and meaningful after-split runtime acceptance remain pending; a static
comparison does not certify those checks.

### Installed lint repair and next verification preparation

The installed three-file correction matches the reviewed proposal exactly.
The independently calculated after hashes are b60760d95a546be922e58271f83e7c0bce1837adf34df3d9b7a20ae45518de51
for dose.rs, b9a0f57459fbca08618cd3d51e24e6dc917675832b26d1e6f22f9babdd0cb0de
for history.rs and 7955440f6ca7d4fe9825e41f653e291598debeaa2579629c46c2df7241f0509e
for stock.rs. Each equals its reviewed source-receipt.tsv after value. This
check is separate from the earlier extraction formatter changes; there is no
additional raw-byte difference between these reviewed proposal files and
their current installed counterparts.

The invitation verification checklist uses existing dispatch and requires no
new wiring:

- Run api:openapi-invitations-acceptance for the complete openapi_invitations
  target. Its child Task explicitly starts the owned rust-api-mail-fail service
  with readiness waiting before running the test container without dependencies.
- Keep the runner's exact CONTRACT_MAIL_FAILURE_BASE_URL at
  http://rust-api-mail-fail:39999 and CONTRACT_MAILPIT_URL at
  http://mail-test:8025. The failure API shares the owned database and disposable
  authentication key; its SMTP endpoint is deliberately 127.0.0.1:1.
- Require the named smtp_failure_rolls_back_invitation_rotation_without_receipt_or_audit
  test to execute and pass. Its environment lookup is mandatory. Both attempts
  must return 503 without replay, mail receipt, token rotation, version changes
  or resend audit changes. Ordinary successful SMTP coverage cannot replace it.
- Run api:openapi-invitations-legacy-acceptance separately for invitations.
  Preserve the actual six-case OpenAPI and eight-case legacy counts in receipts,
  with original source manifests and fixture hash retained before owned cleanup.
  A generic selected-Cargo dispatch alone does not start the failure service.

Successful post-dose doses, full sync, full replay, dose_source_errors and API
unit receipts can also serve as before-occurrence baselines if their accepted
source remains unchanged until the occurrence move. This avoids repeating
those same tests merely under a new packet name. The occurrence-specific
openapi_dose_occurrences and openapi_sync_reads targets and the isolated
fixed-clock dashboard/Leptodon regression still need their own before checks.
The fixed projection clock must not enter real-time recording fixtures. At
this review checkpoint, after-dose runtime receipts are not yet in the queue;
the reuse rule is preparation, not acceptance of unexecuted checks.

### First after-split dose behaviour receipt

G-DOSE-SPLIT-DOSES-001 has bounded requirements PASS and quality/security
PASS. The complete unfiltered doses target passes all 17 active tests, with
zero failures and the single original ignored timestamp case. Its exact
18-name before/after lists match. The route smoke reports six passing tests,
including its parent and five nested checks; wrapper exit is zero.

I independently read the raw assertions and verified runner.raw.log SHA-256
f8d279fbcb48c06331843333522eb24c93ecb317097a60e2f0f7b62df39b174f.
Original copied and live manifests match all 422 paths at
258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952;
every retained live path also matches current file bytes. Both 18-name lists
hash to 1b98b1498776a8117c3186fa003c13b3c53b298e74de608cd2471fe26224780c.
The fixture SHA-256 is runner-emitted only:
3ed7c4b5cbb649d6cb08e5128a9a3d53348752cb466b5071ada02652d8ffa430.
Cleanup preceded its independent rehash. No independently retained fixture
equality or metadata-only rerun is claimed.

Installed source preservation also passes: all 22 production/test extraction
files match the reviewed candidates exactly after accounting for two inspected
formatter changes. Only lock_row and lock_client_uuid signatures were wrapped
and gained ordinary trailing parameter commas in locking.rs and replay.rs.
Their bodies and comments are unchanged. The three later lint-repair files
remain exact reviewed bytes. Original facade exports, the complete four-unit
block, root includes, active names and ignored attribute therefore retain the
accepted structural contract.

I read the successful format, API check, warning-denying Clippy and selected
doses/direct-write/source-error compilation receipts. The API unit receipt
passes all 36 library tests, including the four preserved dose units and the
occurrence unit; all 12 web units also pass. The earlier seven-import Clippy
failure remains recorded alongside its reviewed structural repair.

This accepts the first after-split behaviour checkpoint. The six remaining
contract targets still need their after-split receipts before whole dose-split
acceptance. No later occurrence, invitation or OAuth move is accepted here.

### Accepted dose production and same-target test split

The dose recording code is now separated into smaller responsibilities without
changing its behaviour or the existing test names. Whole-split requirements
PASS and quality/security PASS follow the source preservation review and all
seven meaningful before/after contract checks. Authentication, current grants,
transaction locks, exact replay, source selection, stock updates and audit/sync
ordering retain their reviewed bodies. No allowance, dummy use, comment removal
or external interface removal was introduced to satisfy lint.

I independently read all seven after-split raw receipts. Each complete target
is unfiltered, passes its expected active count and then passes six route
checks, with wrapper exit zero. The seven runs use distinct owned projects.

| Packet suffix under /private/tmp/household-g-20261002/G-DOSE-SPLIT- | HTTP result | runner.raw.log SHA-256 |
| --- | --- | --- |
| DOSES-001 | 17 passed, original 1 ignored | f8d279fbcb48c06331843333522eb24c93ecb317097a60e2f0f7b62df39b174f |
| SOURCE-ERRORS-001 | 2 passed | 15653a1b2822a07c1ecf91f145f97c800893bbf2340894e6faa51e07516c23b2 |
| WRITE-API-001 | 6 passed | 099be401f5f605dbdc79f288a6e58f715d4d38c673340a9e6ba8a630fde9a346 |
| MODE-TRANSITION-001 | 2 passed | 16c4d791d96e9d463e3eee95681ded5bada04e3aaab763238facf1f6d63e2e46 |
| SOURCE-CAPABILITIES-001 | 2 passed | e6019f1b25426dafa490a05a84315401690bd8bb9da6202457c37774229b3834 |
| SYNC-001 | 23 passed | c4f8dacd798f52522f13db39c3cd1da14c0a65101bf50e3958419d09aad561cb |
| REPLAY-001 | 8 passed | be6aafd0f13ccce32306f5a0d0b17342906efc34a403ef6d81c8a7e809d3b1ce |

Every runner emits the same application source digest,
6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5.
All seven retained copied/live manifests are identical, contain 422 paths and
hash to 258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952.
Their diffs are empty; I also verified every retained live path against current
source bytes. The emitted source digest and full manifest-file hash describe
different receipt encodings and are not presented as interchangeable.

The first doses fixture hash remains runner-emitted only, as recorded above.
For the other six runs, each retained independent fixture hash matches its
runner's value: source errors c33cf6305efc6347be5b0b87cac5cdb848d4833a95fe8385ca71a90ecf107dc2;
direct writes d87667a2c0b094b3220c7f7e693793c8e68db70db13f248e4e5fbf31a8bb361c;
mode 5f9e6a3afc90580b191036aa25f2e58a84d5357005331e3ca80e2946701da60f;
source capabilities 57a4d81e671232b297a9d5e905af069a18f50232a055e9e56f6fb6787afaa63c;
sync 72a904ec1fea23b34d47bf0c2bb0d40be2a1e5129583d2084bece0c22c7dff71;
replay 135113167fedac82161117346421cd10b9f82dd800c26fb650d0f8ddf104df7b.

The exact 18 registered dose test names and original ignored attribute remain
unchanged. The accepted fast compiler/Clippy/unit receipts and 22-file source
comparison complete this mechanical checkpoint. These same unchanged-source
doses, sync, replay, source-error and unit results can serve as before-occurrence
checks; occurrence-specific targets and isolated dashboard checks still remain.
Occurrence, invitation and OAuth installation/runtime acceptance and overall G
readiness are outside this verdict.

### Private occurrence lint refinement

Static requirements PASS and quality/security PASS apply to the source
refinement, with a small receipt correction requested before handoff. The exact
original RangeQuery definition, derive attribute, private fields and pub(super)
visibility now remain in the 117-line facade. The reading child consumes that
parent definition. Only the facade's private unit-only scheduled_time_in_zone
binding gains cfg(test); calendar's production helper and call remain present.
The entire original unit block is byte-identical. No allowance, dummy use,
policy change or route-interface removal is introduced.

I independently verified both lint-refinement-receipt.tsv before/after hashes
against actual source, reconstructing each before file by reversing the exact
two-file diff. The retained dose_occurrences-before.rs still equals current
installed unsplit source. This refinement therefore retains the earlier
whole-candidate preservation ruling and adds no behaviour change.

At inspection, preservation.tsv still maps RangeQuery to reading.rs and
visibility.tsv still records its obsolete pub(crate) bridge. The README's
earlier table also retains the previous reading responsibility/line count.
The writer has the exact stale entries to correct; no source defect follows
from them. Actual installation remains held for the additional before-occurrence
OpenAPI and isolated fixed-clock dashboard checks. The accepted unchanged
post-dose results may supply the overlapping baselines, but a private source
review does not certify occurrence runtime or overall G readiness.

### Corrected occurrence receipts and first additional baseline

The private candidate's receipt corrections are verified. RangeQuery now maps
to the facade, its obsolete widening entry is removed and the README reflects
the 117-line facade and 83-line reading child. All 63 actual destination files
contain their original whole items after visibility-only normalization. The
two refined source hashes remain unchanged. Candidate static requirements and
quality/security PASS therefore have no remaining receipt blocker.

G-BEFORE-OCCURRENCE-HTTP-001 has bounded requirements PASS and quality/security
PASS: all ten unfiltered openapi_dose_occurrences tests and six route checks
pass, with wrapper exit zero. The executed cases include signed-key tampering,
household/source visibility, exact keyed replay, rate-limit enforcement,
projection boundaries, simultaneous takes and read-side event absence.

I independently verified runner.raw.log SHA-256
03bfefa5d83022f79b6a567beb5ad5e52e341b27ed41c822becebddd0eacc5fa.
The emitted application digest remains the accepted post-dose value
6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5.
All 422 original copied/live manifest entries match each other and current
source, with manifest-file SHA-256
258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952.
The independently retained fixture hash matches the emitted value
59230aa293025b7e848191f469b96eedbb75eb5dd236d5aa2c631f4ea709bd66.
Additional sync-read and isolated fixed-clock dashboard baselines remain pending
before installation. This does not accept an occurrence move or its after tests.

### Additional occurrence sync-read baseline

G-BEFORE-OCCURRENCE-SYNC-READS-001 has bounded requirements PASS and
quality/security PASS. Both complete openapi_sync_reads cases pass, including
closed snapshot/change envelopes and structured absent-household denial.
All six route checks also pass; wrapper exit is zero, without failed, ignored
or filtered HTTP cases.

I independently verified runner.raw.log SHA-256
2fbaba44448ef16292da14feaf17cd5b8d2678f9c464899f3004cd7e69558b19.
Its emitted source digest remains the unchanged post-dose
6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5.
All 422 original copied/live entries match each other and current bytes, with
manifest-file SHA-256
258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952.
The retained independent fixture hash matches the emitted value
3f59e5d64e14d8820be93416214642d8c47e097e2d9690412eee3f09abc7eafe.
Only the additional isolated fixed-clock dashboard baseline remains before
occurrence installation can be released. Its presentation clock remains
separate from real-time recording fixtures; no after-move result is inferred.

### Final before-occurrence dashboard baseline

G-BEFORE-OCCURRENCE-DASHBOARD-001 has bounded requirements PASS and
quality/security PASS. The existing fixed-clock Task passes all 35 dashboard
and Leptodon tests, with zero failures/skips and runner exit zero. The existing
runner dispatch sets 2026-03-29T00:30:00Z only for browser-dashboard-rust, keeping
this projection fixture separate from real-time dose recording. The complete
output includes the timezone/day-boundary, local schedule dates, taper/gap,
private access, patient-free failure, no-store/PWA, keyboard and mobile checks.

The outer runner log truncates the last browser assertions. I independently
read its authoritative full RTK tee at
/Users/damacus/Library/Application Support/rtk/tee/1790918882_task_api_206bd2.log.
It contains all 35 passing names and the exact summary. The verifier has been
asked to retain that tee in the packet without rerunning anything. Outer log
SHA-256 is 0254eabb44ca381ce89e34de802da75552ce99e566e12361b0f22e73e42c9a11;
full browser tee SHA-256 is fe3fa17d1433948aeb86c151149c3cf54a674b5db997d58e926896ff19814f37.
All 422 original copied/live entries match each other and current post-dose
source, retaining manifest-file hash
258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952
and emitted application digest
6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5.

The fixture hash f62e0e75a5d39fd5ff621400cb3eff9d0d82df1fd0f31343768f09a7648681ac
is runner-emitted only; no independent fixture rehash is claimed. I verified
all ten privately preserved generated screenshot checksums and all seven
pre-existing image checksums against their restored top-level files. Their
manifest hashes are 76c68bcc182749a678305519ed3a1f29015f83ea43907e8dfbfeb8b68848dce5
and e45a5c7ce320f132b92cec0f0930d51188410156d3318fd750ff479f78525f62.
This is preservation evidence, not a new visual-design review claim.

All mapped occurrence before checks are now covered by the additional
occurrence HTTP, sync-read and isolated dashboard packets together with the
accepted unchanged-source post-dose targets and units. The coordinator may
release only the statically accepted occurrence installation next. Actual
installed preservation, compiler/lint and equivalent after-move tests remain
required; invitation/OAuth moves and overall G acceptance remain outside this
checkpoint.

### Installed occurrence source preservation

Static installation requirements PASS and quality/security PASS. The retained
installed-preformat.sha256 receipt matches all fourteen reviewed refined
candidate files. Its hash is
827e7cf99d2e92699a15cc6e9d87ec288fc4c41546de2c8fff5562b8f88afd47.
Formatting had already begun before this independent inspection, so I do not
claim that I captured the original live pre-format bytes myself.

I inspected every current difference from the reviewed candidate. Formatter
changes are limited to wrapping attributes, key and decode_key signatures with
trailing parameter commas in input.rs/keys.rs, plus ordinary module/import
ordering in the facade. All other installed files match exactly. No routine
body or comment changed. Original RangeQuery, its derive/private fields and
visibility, the whole unit block and every facade export scope remain exact.
All 63 original visibility-normalized whole-item candidate comparisons still
hold. These checks preserve the transaction/authentication/replay/source/key
and projection contract already reviewed, without accepting unexecuted tests.

Compiler, warning-denying Clippy and meaningful after-occurrence runtime remain
with the verifier. Invitation and OAuth remain private; no other installation
or overall G acceptance follows from this source comparison.

### Occurrence after checks and named schedules gap

The six completed API suites and isolated fixed-clock dashboard packet pass
bounded requirements and quality/security review. Whole occurrence acceptance
remains pending the explicitly named schedules target in final-writer-brief.md.
The earlier statement that all mapped before checks were covered followed the
private map, which omitted this named requirement. That limited conclusion is
corrected here; schedule behaviour inside other passing suites does not prove
the schedules Cargo target passed.

The brief requires openapi_dose_occurrences, doses, schedules, affected sync and
replay, and fixed-clock dashboard before and after the move. The completed
packets establish the first two, full sync/replay and dashboard, plus sync-read
and source-error coverage. Earlier openapi_schedule_writes checks are a different
target and source. The coordinator preserved the requirement and assigned the
exact schedules suite against retained pre-occurrence and current source in
separate fixtures. No omitted suite is inferred green, and invitation release
remains held for that checkpoint.

I independently verified these complete, unfiltered after-split API receipts;
each passes six route checks and exits zero:

| Packet under /private/tmp/household-g-20261002/ | HTTP result | runner.raw.log SHA-256 |
| --- | --- | --- |
| G-OCC-SPLIT-OCCURRENCES-001 | 10 passed | 075118adb910c3a4df5c92f8021c689513a9b45a74ffe6dfbe202a1414465d6c |
| G-OCC-SPLIT-DOSES-001 | 17 passed, original 1 ignored | fed0d9833bf5da39e5c3fb0af365471d600867b108b7b682c54691c8c2007bdf |
| G-OCC-SPLIT-SYNC-001 | 23 passed | e669ca9e0e671175875576d254c8e2ddc4eee41e7aa9c3f2217b11adb2608d27 |
| G-OCC-SPLIT-REPLAY-001 | 8 passed | 073e82414422813f3b721ab3bb3936a040e50bddd2527d28541cd343d74d27d5 |
| G-OCC-SPLIT-SYNC-READS-001 | 2 passed | 3993b266e7eb08501dcf1f29b36e22c2e903a221db64c8b7691a3cfe1d70eb60 |
| G-OCC-SPLIT-SOURCE-ERRORS-001 | 2 passed | f92eb7bd52bdda85aec0fd1fc0040352087ebecb4445a4a5f8b6658e5d92799f |

All six use distinct projects and emit source digest
a5fedf06dbef8ac386f0c40eb741ed9f03cb8ff9294f6356a82ec9c2705b957e.
Every original copied/live manifest matches all 435 paths, with file hash
d31251ad535b03d110a108ef9dbf3c731cba02aa4c1bf9f0687279769246d58f;
I also matched every retained live path against current bytes. Independent
fixture receipts match each emitted hash in all six packets.

G-OCC-SPLIT-DASHBOARD-001 passes all 35 dashboard/Leptodon tests, with no failures
or skips and wrapper exit zero. Its copied/live/current 435 paths match the
same source. The independent fixture hash is
12ee98d81e6782e2ea347bfa94f3d7d657f6b4bc1c941dc15013ee915058a999.
Outer raw hash is 7381039d718ac0b65dc75e9545428d386d707df9f523fdf03242dcdb8c2b7fb0.
The initially named browser.full.log contains Docker build output, so I used
the actual complete assertion tee at
/Users/damacus/Library/Application Support/rtk/tee/1790920473_task_api_ef30ce.log,
SHA-256 2ce1b87e1e8669b117964bc4c43fc6012b8e2b2beca187687675f3bf5f9fd08e.
The verifier has been asked to preserve and label that output accurately;
no runtime repeat is needed for this receipt correction.

The fast format, API check, warning-denying Clippy and unit tasks all have
zero exits. Units pass 36 API and 12 web cases, including the preserved four
dose units and occurrence DST unit. The fourteen-file source preservation,
original exports/RangeQuery/unit block and formatter-only qualification remain
accepted. No additional source change was found by the current manifest check.
These passing checks do not waive the schedules requirement or accept any
other private split.

### Remaining baseline checklist reconciliation

The exact auth Cargo suite is required before and after OAuth. The final brief
names it explicitly; the private map's oauth/session/mobile coverage does not
replace that requirement. Its twelve source-declared cases cover public
capabilities, operational household/session scope, invalid/locked/expired
bearers, selected revocation, logout, deactivated users, membership loss and
permission-version invalidation. No documented equivalence waives those checks.

The complete verifier checklist is auth, oauth, web_session_api,
medication_mobile_oauth_api and openapi_auth_sessions, plus
tests/login.smoke.test.mjs. Declared counts are respectively 12, 11, 9, 7 and
8 HTTP cases; the login file declares seven viewport/keyboard cases. These
counts are planning, not execution proof. Each authentication target needs
its own fresh real-clock fixture because account/session/revocation mutations
must not affect another target. The existing api:api-legacy-auth-acceptance
dispatches exactly auth without filtering; other precise existing target
wrappers must retain their original assertions. Actual failure receipts must
precede any compatibility repair. Source preservation and format/check/Clippy/
unit gates remain separate requirements.

For the missed schedules before check, the coordinator's proposed private
reconstruction is valid evidence preparation: copy current application, remove
only the thirteen new occurrence children, restore the exact retained original
facade, and require every original 422 manifest path/hash to match before
building. This must be described as reconstructed pre-split source, never as
equality with a retained executable copy that no longer exists. No main-checkout
change or retrospective suite substitution is approved. Whole occurrence
acceptance remains pending the actual schedules before/after results.

### Reconstructed schedules before-source proof

I independently verified G-OCC-SCHEDULES-BEFORE-001's private reconstructed
source. reconstructed-source.manifest is byte-for-byte equal to the original
retained 422-path pre-occurrence copy manifest, with SHA-256
258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952.
Every actual file in both the private repo and its reconstructed-source
directory matches its original path hash. Both have the exact retained before
facade and no dose_occurrences child directory.

This accepts reconstructed original source identity for the missing before
baseline. It does not claim an original retained executable copy, an earlier
successful schedules run or a live main-checkout change. Actual schedules
before/after compilation and behavioural results remain pending before whole
occurrence acceptance.

### Exact schedules baseline failures and compatibility ruling

The reconstructed before suite executed all fifteen declared schedules cases:
three passed, eight failed and four retained their existing ignored attributes.
I independently read schedules-before.raw.log and verified SHA-256
136c9f64cb08bca8211c110902d3671e30b3c8cfbd38c8a571ef8a2f41652550.
These results do not establish harmlessness of the occurrence move or success
of later assertions blocked by each first failure. Whole occurrence acceptance
remains pending meaningful repaired before/after checks.

The failure at schedules.rs:233 is a real compatibility bug, not an obsolete
test merely because Rust's typed query currently returns 400. Rails
SchedulesController#index delegates to BaseController#paginate, whose
lines 225–227 convert page/per_page with to_i and then normalize/clamp. Thus
page=bogus&per_page=0 returns page 1, size 1 and 200. The OpenAPI integer
parameter describes standard inputs but does not establish the proposed
400 replacement for this observed legacy extension. The coordinator retained
the original normalization assertion and authorised a schedule-only correction,
with valid pagination and access/filter checks preserved. Other resources'
query parsers are outside scope.

The other source-supported corrections are bounded:

- The omission assertions at lines 422/557 conflict with the documented optional
  nullable current_pause_period. Rails source_preloads includes pause periods,
  and ScheduleSerializer#pause_data emits the key with null when none is open.
  The correction should validate the null/period shape without weakening history.
- The successful full PUT at line 677 supplies a different visible person.
  Authoritative ScheduleAttributes explicitly rejects a changed person, and
  Rust input.rs:224 enforces that rule. Use the original person for the success
  case and separately require 422 with complete unchanged source/version/sync
  evidence for a transfer. Rails excludes person_id during update; that difference
  does not override the explicit documented restriction already accepted here.
- Medication creation at lines 309/605 omits required reorder_threshold.
  Medication validation.rs:125–150 requires a non-null valid threshold. The
  accepted value 3 repairs only those fixture helpers, preserving clinical input.
- Two pause cases stop in the length-20 timestamp helper despite valid fractional
  timestamps. Use the already accepted RFC3339 UTC/microsecond checks and preserve
  all fractional values and ETags; no production truncation is approved.

The coordinator's proposed repaired before mirror must remain labelled
repaired/reconstructed evidence, with its exact new source/test hashes. The
original 422-path RED reconstruction stays immutable. Applying the same reviewed
schedule/test correction to private unsplit and current sources permits a
meaningful mechanical comparison without claiming their new hashes equal the
original RED source. Actual patches and execution are still pending review.

### Reviewed and installed schedule correction

The schedule correction passes static requirements and quality/security review.
It preserves the original bogus-page success assertion and fixes only schedule
query parsing. Authentication and adult schedule permission run before string
normalization; existing filters, timestamp parsing, scope and auditing remain
in their original order. Other resources retain their original extractors.
The helper implements signed ASCII decimal prefixes and saturation for the
proven malformed, zero, negative, oversized and prefix controls. This is a
bounded compatibility rule, not a claim of complete Ruby String#to_i equivalence.

The test corrections preserve the four ignored cases and all existing behaviour
assertions outside the identified mismatches. Nullable pause values are checked
without dropping history assertions. Required medication threshold 3 repairs
the two helpers. The renamed assert_utc_timestamp checks RFC3339, UTC, a Z
suffix and microsecond representability without truncating timestamps or ETags.
Full PUT succeeds with the original person; a separate attempted transfer
requires 422 and exact unchanged source and ETag readback. Existing unrelated
strict-pagination controls preserve the resource boundary.

I checked the retained installation receipt against both reviewed private files.
The pre-format production and test hashes are respectively
9dc10e211f13541b12f9c3cc11486112d1fc581c8b7f43ca9c2f0c8c0b2124a0
and ea43821bb12734ec66b1bd880a6825d106c88fbc0f4da801de56b34b703cff19.
The current test file still matches the candidate exactly. The current production
file differs only by import ordering and formatter wrapping of the integer
helper; its hash is
5676101d9f115f576bd77f089e656e60b27704458a2216432213d9306a66fa86.
I did not independently observe live pre-format bytes before formatting started;
the retained installed-preformat.sha256 and installation.txt provide that receipt.

Actual repaired/reconstructed before and current schedule results remain pending.
Whole occurrence acceptance stays held for those checks. The separately reviewed
thirteenth CI row promotes this complete schedules target with a fresh real-clock
fixture and read-only route checks; its actual local policy RED/GREEN is recorded
in review-final-ci-report.md. No other private split is accepted for runtime or
installation by this static ruling.

### Repaired reconstruction and regenerated UI assets

The repaired private source is correctly scoped. I independently compared its
422-path manifest with the original reconstruction and verified every actual
staged file hash. The path sets are identical. Exactly six hashes differ: the
two approved schedule source/test files and medtracker_ui_preview.js, its .d.ts,
medtracker_ui_preview_bg.wasm and its .d.ts under rust/ui-preview/public/pkg.
All authored web/UI sources and captured build inputs remain unchanged. The
retained original occurrence facade is exact and its child directory is absent.

The source-snapshot log records the existing UI build followed by wasm-bindgen
generation into public/pkg. contract-source-snapshot already depends on that
build, and its Task copies those generated outputs. This supports treating the
four asset differences as regeneration from unchanged UI sources, without
restoring stale bytes or changing dispatch. The actual repaired manifest has
SHA-256 2b99afbf336a2a8a83951f110546e0df8adad1ae91d9b6332db0deaed2f512b7.

This is reconstructed repaired source with regenerated UI assets. It is not
byte-identical to the original full manifest or an original retained executable
copy. Actual repaired-before and current schedule results remain pending; the
accepted six-suite/calendar evidence is not being repeated or weakened.

### Three later schedule assertions and narrow follow-on

The repaired unsplit run reached eight passing cases, three failures and the
four unchanged ignored cases. I independently read its schedules-before.raw.log:
the new locations control failed with actual 422 versus expected 400, and the
existing create/full-PUT numeric dose controls received must be a decimal string
instead of must be a string. Later assertions blocked by those failures remain
unverified. Whole occurrence acceptance is still pending.

The locations assertion was a new setup mistake missed in my earlier static
PASS. locations_index explicitly handles QueryRejection after authentication,
returning an audited invalid_pagination response at 422. Correcting that control
to 422 preserves strict rejection and proves schedule normalization does not
change this unrelated endpoint. It is not a location product fix.

The two numeric messages are real response compatibility mismatches. Rails
SchedulesController#schedule_params calls reject_numeric_contract_values! for
dose_amount, and BaseController#render_invalid_contract_value returns the exact
must be a string field error with validation_failed/422. The authoritative
OpenAPI DecimalValue requires a string; it does not justify replacing the
existing exact Rails-derived message assertion. Numeric rejection and both
existing assertions must remain.

The private schedule-followon-proposal/proposal.diff passes static requirements
and quality/security review. It adds only value.is_number() rejection at the
existing parse_amount validation position, using the existing InputFailure
envelope. Null, invalid decimal strings, range and precision failures retain
their current handling. Earlier shape/identity/permission checks, lock and
mutation order are unchanged; no global ApiError or parser change is introduced.
The only test change repairs the new locations expected status. Meaningful
repaired/reconstructed before and current execution remains required before
acceptance; no product files were edited by this reviewer.

The installed follow-on matches the approved private files byte-for-byte at
this review. The numeric-input file has SHA-256
8689173ed481439b427a3baa41ffb562118d4cc5f805a40a6129a952ae591524;
the schedules test file has
5a25313cc5899c3f13a5d64cc6fcea5d45405f667c1d9a9d9c2cd7b903f1ced2.
Both match installed-preformat.sha256 and the private candidate. No formatter
difference is present in these two files. The repaired baseline has exactly
three approved authored paths: schedule reader, schedule input and schedules
target; generated UI differences remain separately qualified above. Issue 2368
tracks the numeric-message bug. Whole occurrence acceptance remains pending
the actual repaired unsplit and current schedule receipts.

### Meaningful repaired unsplit schedules baseline

The repaired unsplit schedules baseline passes bounded requirements and
quality/security review. Despite CURRENT in its folder name,
G-SCHEDULE-FIX-CURRENT-BEFORE-002 has the reconstructed pre-occurrence facade,
no occurrence children and 422 source paths. Its expected repaired manifest and
actual runtime copy manifest are equal, with SHA-256
c34349b401efc507404191c3f22200fd446dcbbc480a3bc9aa89c2098a4a6a4e.
Comparison with the original reconstruction shows only the three approved
authored repairs and the four separately qualified generated UI artifacts.
This remains repaired/reconstructed evidence, not an unchanged original baseline.

I independently read schedules-before.raw.log, SHA-256
ee8f566e4c8ab33882631a40df64d5c9940f5b4cbb63e8149556a8b9d4ae395d:
all eleven active cases pass, none fail, and the original four are still ignored.
The complete suite therefore reaches pagination, numeric rejection, original-
person updates, rejected transfers, pause/history and recurrence/link assertions.
The independent fixture receipt agrees with the emitted hash
4b84126769a79e7966a8690c5b0438fa5411714bd7f537d7dac8b14afced4621.

The outer log truncates browser output. I read the authoritative RTK tee
1790924056_task_api_6f8601.log: one route parent and five nested assertions pass,
tests 6, pass 6, fail 0. I requested preservation of that tee and the actual
wrapper-exit receipt; root reports wrapper zero, but the original retained
folder did not yet contain its exit file at this review. No metadata-only rerun
is requested.

The follow-on api:check, warning-denying Clippy and schedules selected-compile
logs show successful completion. Their raw SHA-256 values are respectively
139821310a7d93b06805e38f0b56f1836b808494bd041b3d5996a9edc937dd9c,
3fe4d6d16c55e605ff783d14783a8b036fd1e63e755a85050487f88ad4b16709
and f0bceea3ce66253e7581b26cad77797f3e81457f7030dba862ae14132614523f.
Formatting is silent and coordinator-reported successful; installed source
preservation was independently checked above. Whole occurrence acceptance
still requires the separate current split-source schedule result. No previously
accepted six-suite or calendar checks have been repeated or substituted.

### Final schedules result and whole occurrence split verdict

The current split-source schedules packet passes: eleven active tests, zero
failures and the four unchanged ignored cases. I independently read
G-SCHEDULE-FIX-CURRENT-AFTER-001/schedules-after.raw.log and verified SHA-256
d3c992d2d3de27022354a7625d365fc0034a4c4216c88af80ecbb5555e9af3fc.
Its six exact route checks are inline near the end: one parent plus five nested
assertions, tests 6, pass 6, fail 0. outer.exit records zero. The earlier nested
RTK path is Docker build output and is not needed to establish these assertions.

The runner reports source digest
3ddaf3e03f370082def8fcef6d55f8e43ba83d9feb516998bad0e5e371e53600,
project mtcontract-a2962babab0a47eb and fixture SHA-256
c79ab408a96d1cd0208f28870ed7eaf25773f9dab9feccd2f4610826650a8374.
At this review the original runtime directory had been removed and the packet
contained raw output and outer.exit, without an independent fixture rehash or
original copy/live manifests. I requested any already-retained originals from
the verifier, without a rerun. Unless subsequently supplied, those identities
are runner-emitted only; no original per-file equality is claimed for this run.

Requirements verdict: PASS for the whole occurrence split and its bounded
schedule compatibility corrections. The independently reviewed mechanical
preservation, fast checks/units, six actual API suites, fixed-clock dashboard
and now meaningful repaired unsplit/current schedules satisfy the explicit
before/after checklist. The failed original and intermediate baselines remain
preserved; none were retrospectively labelled passing. The four old ignored
schedule cases remain explicit limits rather than new accepted behaviours.

Quality/security verdict: PASS. Original occurrence exports, RangeQuery, unit
block, transaction ownership and typed dose calls are preserved. Schedule-only
normalization retains authentication/scope and numeric rejection retains its
original validation position; no unrelated resource parser or shared error
contract changed. Earlier accepted API/calendar source evidence retains its
original manifest qualification, while this final schedule run retains the
emitted-only provenance limitation above. Passing checks are not being repeated
solely to recover deleted metadata.

The next invitation before-baseline work may proceed under coordinator release:
legacy invitations, OpenAPI invitations including the owned SMTP-failure case,
with separate fixtures and actual failure-service configuration as previously
reviewed. This does not authorise invitation installation or accept overall G,
OAuth or remote CI readiness.

### Invitation OpenAPI before baseline

The OpenAPI invitation before baseline passes bounded requirements and
quality/security review. G-INVITATIONS-OPENAPI-001/invitations.raw.log records
all six tests passing, no failures/ignores/filtering, and outer.exit is zero.
I verified raw SHA-256
120b889f80fbc7316d394dd8d8e2dff7b85a15d465a47c1817e5dc7f1d3195ba.
The log shows the owned rust-api-mail-fail service starts and becomes healthy
before the contract test. smtp_failure_rolls_back_invitation_rotation_without_receipt_or_audit
actually passes; its source requires the exact failure-service URL rather than
skipping when configuration is absent. Two failed sends require 503, no replay
receipt, no mail, unchanged invitation state and unchanged version/audit counts.

The emitted source digest is
3ddaf3e03f370082def8fcef6d55f8e43ba83d9feb516998bad0e5e371e53600
and emitted fixture hash is
9049142bcbf7752129b4ed9cfe17f05b4702b9c636c6dfd74c9cb155e714a2ef.
The retained packet currently has raw output and exit receipt, without independent
source/copy or fixture manifests; no original per-file equality is claimed.

Current invitations.rs still matches the reviewed private candidate's before
file exactly, SHA-256
3ccbb07d6446c1b2dba1769d4d8e310159e0e8b88a33cc4c3190b245eb9e3d57.
Its whole transaction owners, original facade exports and resend-only SMTP
position retain the earlier static acceptance. The separate legacy log shows
eight passing assertions, but its completed exit receipt remains pending at this
review. Invitation installation is held for that receipt; overall G remains
unaccepted.

The completed legacy invitation baseline also passes independent review:
eight tests, zero failures/ignores/filtering, and retained outer.exit zero.
G-INVITATIONS-LEGACY-001/invitations-legacy.raw.log has SHA-256
2a02033d55903908633fcc783ed8fb9265b611255cb8cd3829f79731074d7ed0.
It includes current-manager checks before cached resend replay, token rotation,
revocation, matching-identity acceptance and public authentication readback.
The source digest is the same emitted 3ddaf3e03f370082def8fcef6d55f8e43ba83d9feb516998bad0e5e371e53600;
its separate fixture hash is emitted as
835fc82dcad808605314018c45557bcdc816470a52ca418f7f00f20775ec0b9e.
No independent original manifests or fixture rehash are claimed for this packet.

All mapped invitation before checks now pass, including the mandatory SMTP
failure rollback. Requirements and quality/security verdicts: PASS for the
before-baseline checkpoint and reviewed private candidate readiness. The
coordinator may release only the invitation split installation next, preserving
its exact original exports, whole transaction/auth/replay routines and existing
resend SMTP ordering. Actual installed preservation, fast gates and both
after-baseline suites remain required before accepting that split. Overall G
and OAuth are still pending.

### Installed invitation source preservation

The installed invitation split passes static requirements and quality/security
review. All eleven retained pre-format hashes match the accepted private
candidate; installed-preformat.sha256 has SHA-256
0adcb1afd0274adf1bf95af8bfc391b6e853118d36c865aa2e1a91dd45579bf4.
Formatting had already begun when I compared live files, so live pre-format
identity is supported by that retained receipt rather than independently
observed earlier bytes.

Nine current files still match the candidate byte-for-byte. The facade differs
only by module/import/re-export ordering; responses.rs differs only by wrapping
the acceptance_error signature. No function body, comment, attribute, export or
visibility changed. The earlier independently verified 32 whole items and
28 standalone bodies therefore remain applicable. Original facade scopes for
MailConfig and index/create/destroy/resend/accept are preserved; child crate-only
bridges permit the existing caller paths, and mail configuration fields remain
private. MailConfig::from_env retains its effective caller access.

Whole create/destroy/accept/resend transaction routines are unchanged. In
particular, SMTP remains at its existing resend position, with rollback on
failure before successful completion; create has no new delivery call. Current
facade and responses hashes are respectively
1131d900769fc38111ce83959df509181d96566452194251c92f879fbf830a4a
and 54622d421559996dee50a3a3a6be57cc628c899dc831895bf1c78a96610d6079.
Fast gates and both actual after suites remain pending before whole invitation
acceptance. OAuth remains private and overall G remains unaccepted.

### Whole invitation split acceptance

The invitation split passes final bounded requirements and quality/security
review. The after OpenAPI suite executes six passing cases, including the
configured SMTP rollback, with no ignored or filtered cases and outer.exit zero.
Its raw SHA-256 is
e7aedfd6bd5264f0527ac30d1d509e542c5d4725dd0e014e570e42a410c7754e.
The dedicated failure service starts and becomes healthy before assertions.
The separate after legacy suite executes all eight passing cases, no ignores
or filtering, and outer.exit zero; its raw SHA-256 is
62df6298f57a4f7f132774a305ffdca464bc732644b67129572933e162e3c92b.
I independently read both logs and exit receipts.

Both runners emit source digest
0b8f532a35103a9774bbf2fa1f860e456b97c8e06dcba66f36d569c2dff4692c.
Their separate emitted fixture hashes are
c22a750e1b6658843f109ab0d916f60b7c8c772f3f85f3007df78a238c3531eb
and 0d24eb5df746588172a1cfe6ec28ba395c1de580f18a245da8f976a5b11e3a26.
Original independent runtime-copy and fixture manifests are not retained in
these packets; provenance remains emitted-only, without a per-file equality
claim. The reviewed source preservation and frozen installed files support
the behavioural comparison; no metadata-only rerun is required.

All fast exit receipts are zero: format, API check, warning-denying Clippy,
both selected compilations and units. Units execute 36 API and 12 web cases
with no failures; preserved dose and occurrence units pass. I verified the
eleven live files against the retained post-format hashes, with zero differences.
That manifest has SHA-256
5a6c7729010b04967940984c496e237bfe1977bec5187ed30157655633234f3d;
the reviewed formatter-only diff has
b45124980b0aa032cc729887760c0f0d7183a89f9cd10c6bab4db32f0e048550.
The unit raw log has SHA-256
eea11feb60dcd987826c7a89895cac4fce1b2062b401affc30efec2d5d44dff6.

The original 32 whole-item and 28 function-body proof, exact exports/private
fields, current authorisation before replay, transaction ownership and resend
SMTP ordering remain accepted. Both meaningful before and after suites now
pass. OAuth before-baseline work may proceed under coordinator release using
the complete explicit checklist: auth, oauth, web_session_api,
medication_mobile_oauth_api, openapi_auth_sessions and login browser, with
separate fresh fixtures. OAuth installation and overall G acceptance remain
pending.

### OAuth HTTP before result and separate login failures

The full OAuth HTTP before target passes all eleven cases with zero failures,
ignores or filtering. Its extra same-fixture login browser file passes five
cases and fails two, not five. The outer Task exits 201. These are separate
outcomes: the HTTP target passes, but this combined invocation does not.
G-OAUTH-BEFORE-OAUTH-001/oauth.raw.log has SHA-256
a52d3b0c67fece9d104a312a1b049704623acffe96cd4a5600883b082f820fab.
The authoritative Node tee 1790926463_task_api_1a27d7.log has
09e8e8bd4981a1f96ccf08db1ddd5cb45322485ea208c01d370408d328b5c953.

Only the desktop and mobile standalone-login cases fail. Both reach the
dashboard URL, then time out waiting for an exact h1 matching
fixture.household_name at login.smoke.test.mjs:88. Current dashboard renderer
uses page.greeting as its h1, with the API constructing that greeting separately
from household_name. Static login pages, failed-password usability and both
mobile consent cases pass. The source therefore supports an obsolete heading
expectation as the narrow cause candidate rather than a failed login assertion.

I inspected the full OAuth target's mutation paths. Its database helpers read
grant IDs/activity; its writes issue/rotate/revoke OAuth grants and create
browser sessions through public auth routes. It does not update household or
person names, add/remove management grants or mutate the primary account's
identity. Current fixture interference is not established by this source or
failure output. No actor reset, permission undo or test weakening is approved.
The separate fresh standalone-login run remains the required before proof and
will determine whether the same exact heading failure occurs. All remaining
OAuth before suites and installation remain pending.

The separate fresh standalone-login run reproduces the same result: five
passing browser cases and two failed household-name heading waits. I verified
the authoritative tee 1790926747_task_api_512961.log, SHA-256
fe96d2f2f81e4979e65caa2f7f544c8ddc5f948ea8cb0c3e399830c4e4cd25ec.
This disproves the initial fixture-interference hypothesis for these failures.
No OAuth actor reset or permission undo is needed or authorised.

Current UI intent is the greeting built from the profile-linked visible account
person's first name and local morning/afternoon/evening, rather than the
household name. A repair must assert that identity-specific greeting and the
actual authorised owner household, not any generic heading or merely the URL.
The old logout selector also differs from current controls: desktop Sign Out;
mobile Logout inside the keyboard-opened Navigation menu dialog. Existing
dashboard tests already exercise those exact accessible paths. The writer has
been asked to retain CSRF, keyboard focus, post-logout session revocation,
wrong-password and mobile consent/PKCE checks. The exact private test-only
proposal remains pending review; no new production change is implied.

### Private login selector correction review

The private one-file proposal correctly reads memberships, household /me,
profile and people through page.request, which shares the authenticated browser
context's cookies. It supplies no separate bearer token. Exact fixture account,
email, owner role, household ID/slug/name/membership and current navigation href
are checked alongside the dashboard schedule/metrics. These fields match the
current auth and household response shapes. Desktop Sign Out and mobile Logout
inside the opened navigation drawer match existing dashboard controls. Keyboard
logout, CSRF rotation, private redirect and a new post-logout API 401 preserve
meaningful session-revocation proof. Wrong-password, mobile consent and PKCE
blocks are unchanged.

One required correction was identified before setup approval: the optional
ownPerson name lookup with a there fallback permits a generic greeting if the
profile-linked visible person is absent. This owner fixture should require a
nonempty profile person ID, matching visible person and nonempty name, then
derive that person's first name. The three allowed day-period greetings remain
appropriate at a real-clock boundary. The writer has been notified; candidate
static approval remains pending that exact tightening. No production edit or
runtime was performed by this reviewer.

The refreshed private login proposal now passes static requirements and
quality/security review. It requires a nonempty string profile.person_id,
the matching visible ownPerson and a nonempty name; the generic there fallback
is removed. The exact greeting therefore derives only from this proved owner
identity. All other cookie-authenticated account/household/membership checks,
current accessible logout paths and security assertions remain intact. The
unchanged before test hash is
9c4fdbe589ff247eca9216fccfb1ed6dae8af86dd89ed791aa4bc8c4b737811d;
the reviewed private candidate hash is
af29a9fd7864861e441299a68e3424d355474692bef958876297f99adeba4400.
The coordinator may release only this test-file installation between fixtures.
Actual fresh browser GREEN, remaining OAuth before targets and eventual
installed-source review remain required; no authentication product changes
are introduced. The isolated fourteen-row CI policy/wiring acceptance is
recorded separately in review-final-ci-report.md.

### Complete OAuth before checkpoint and login correction acceptance

The complete required OAuth before set passes bounded requirements and
quality/security review. I independently read all five full HTTP assertion
results and retained exits. Counts are auth 12, oauth 11, web_session_api 9,
medication_mobile_oauth_api 7 and openapi_auth_sessions 8, each with zero HTTP
failures, ignores or filtering. Auth, web session, mobile and auth sessions have
outer.exit zero. The original OAuth combined wrapper retains outer.exit 201
because its obsolete extra login assertions failed; its eleven passing HTTP
cases are accepted separately, without relabelling that wrapper successful.
Mobile's read-only route checks also pass all six.

| Packet | Verified raw SHA-256 |
| --- | --- |
| G-OAUTH-BEFORE-AUTH-001/auth.raw.log | 139e1746423fc062705ca62193152fee726c1a2338ef40dc0dd85d5c5d20a3c5 |
| G-OAUTH-BEFORE-OAUTH-001/oauth.raw.log | a52d3b0c67fece9d104a312a1b049704623acffe96cd4a5600883b082f820fab |
| G-OAUTH-BEFORE-WEB-SESSION-001/web-session.raw.log | d4703a0c0f8808962bb067e6ea161078a875b181c413303cf2624feecc999186 |
| G-OAUTH-BEFORE-MOBILE-001/mobile.raw.log | b4e6e513a0a9b005339fc205f507213af4b7fcccd427b806b593ebbd5684b685 |
| G-OAUTH-BEFORE-SESSIONS-001/sessions.raw.log | 3964d21d037ba57c3422ea46d7b429d708f623536e98e1e5c671d4e84f2af3ef |

All five emit unchanged app digest
0b8f532a35103a9774bbf2fa1f860e456b97c8e06dcba66f36d569c2dff4692c.
Separate fixture hashes are emitted in their retained raw logs. Original
independent per-file copy manifests and independent fixture rehashes are not
claimed. The meaningful HTTP assertions include current-account/membership
loss, cookie and bearer precedence, CSRF/origin, PKCE, refresh rotation,
namespace collision, concurrency and revocation; the exact required auth
target is included rather than replaced by another suite.

The one installed login test matches the approved private candidate and retained
installed-preformat receipt exactly, hash
af29a9fd7864861e441299a68e3424d355474692bef958876297f99adeba4400.
Its fresh standalone run passes all seven cases and outer.exit zero, raw
G-OAUTH-BEFORE-LOGIN-SMOKE-FIXED-001/login.raw.log SHA-256
d36ffd3b0c3bf8603fe3fd9249d1fa695aabf6d782066eae1a7e320ab7cde9e4.
Actual desktop/mobile owner greeting and logout, CSRF rotation, API/session
revocation, wrong-password usability and mobile consent all execute. Its emitted
source digest is
310de470b49cd66e4f73e4720afe6bd74c05d90a46b67012a3e6084d5985b078;
emitted fixture hash is
4faa7ee4b3e6628d27c953894eb21e29549227e69800ed0de193f33f246569bb.
This source difference is the accepted test-only correction; OAuth product
source is unchanged. Original browser failures remain preserved as real REDs.

Current oauth.rs matches the reviewed private before file exactly, hash
948c109fadac0048ca94dbfcb8bb671fb22e8a231cc49ea2cd8b80b9510f472e.
The private candidate retains the earlier accepted 59 whole-item/44-body proof,
original route owners/exports, effective scopes, private secret and complete
authentication/transaction routines. Candidate readiness and complete before
checkpoint verdicts: PASS. The coordinator may now release only OAuth split
installation. Installed preservation, fast gates/units, all five after HTTP
targets and fresh standalone login remain required; read-only routes can follow
the OAuth HTTP fixture separately from required fresh login. Overall G remains
pending.

## Installed OAuth and first-option test comparison

The stock race assertion extension is installed and matches the approved
private candidate and its installation hash exactly. All original race, draft,
stock, version and sync checks remain, with the additional zero-to-one option
counts and explicit stock comparisons. Its extended runtime remains pending.

All eight OAuth candidate hashes match the writer's retained installed
pre-format receipt. By the time of independent current-file inspection,
formatting had occurred: configuration, discovery, helpers and login match
the candidate byte-for-byte. The facade differs only in module/import order;
authorization, sessions and tokens differ only in five function-signature
line wraps. No function body, export scope, attribute, comment, authentication
check or transaction order changed. This is a receipt-backed pre-format proof
plus a directly inspected current delta, rather than a claim that the reviewer
observed every file before formatting. The verifier's retained formatter delta
and after gates are still awaited before closing installed-source acceptance.
All OAuth after runtime checks and overall publication remain pending.

The verifier retained the formatter delta, which independently confirms only
the ordering and signature wraps described above. The first API check then
failed at three existing tokens.rs accesses to authorization::Client.id
(lines 64, 111 and 212). The earlier candidate review missed this necessary
sibling field bridge; source preservation alone did not establish compilation.
The failed check is retained in G-OAUTH-SPLIT-FAST-001/api-check.raw.log,
SHA-256 1a7ac4012688e1cf2de0415b9b7c5a34985989bebdc9e4133841d1136730f647.
Clippy and runtime had not started.

Reviewed private repair: oauth-visibility-proposal/proposal.diff changes only
Client.id to pub(super). The other four fields remain private; all bodies and
external exports are unchanged. Its visibility is confined to the OAuth parent
module and descendants, restoring the original internal token accesses.
Requirements and quality/security verdicts for this mechanical repair: PASS;
installation is released. Successful compilation and all after checks remain
required before whole OAuth acceptance.
