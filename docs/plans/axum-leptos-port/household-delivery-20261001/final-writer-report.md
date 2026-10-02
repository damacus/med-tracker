# Final household delivery

People can manage medications and dosage options, adjust or remove stock, assign
treatments, edit all seven schedule types, and pause or resume treatment with
history. The earlier journeys are published, including assignment delivery in
[PR #2362](https://github.com/damacus/med-tracker/pull/2362).

The remaining household fixes work in local checks: large authorised collections
load completely, permitted person pages remain readable for minors, and long
stock-adjustment reasons save in full. The portable import and dose recording
refactors are accepted. The occurrence refactor is also accepted, including the
local schedule pagination and numeric-error fixes. The invitation split is now accepted too. Four of the five module refactors
are accepted. Sign-in is installed after five HTTP baseline suites and all
seven login browser checks passed. Its first compile found a missing internal
field visibility bridge; that reviewed repair is installed and awaits rerun. Final
combined checks, publication and CI remain before this stage is complete.

## Current result

Large authorised medication and treatment collections now load beyond 500 rows.
Stock can be removed from a finite parent balance when every dosage option is
untracked; zero stock still counts as tracked. People with minor status can open
an otherwise authorised person page without a forbidden schedule subsection
blocking the whole page. The page explains when schedules are unavailable.
Assignment editing explains that a blank option keeps the existing dose/link,
and missing-version errors tell the person to reopen the form.

Long stock-adjustment reasons now save in full without overflowing the indexed
audit description. Stock choices use database location-name order. Positive
stock remains selectable, while an actual dose rejects insufficient stock or a
paused treatment without changing clinical records. People page sizes above 100 are capped
at 100; malformed queries retain their existing rejection behavior.

Dose checks also exposed incorrect occurrence audit labels, lost paused/empty
stock error codes, numeric amount field errors and empty-stock selection
priority. Those fixes are installed and the full corrected dose target passed
all 17 executable cases. Its pre-existing ignored timestamp case is unchanged.
Direct and sync error responses retain their reviewed boundaries.

## Checks completed

- Focused stock completion, assignment persistence and large People/treatment
  checks passed. The earlier combined inventory run passed seven HTTP checks
  and ten desktop/mobile browser cases.
- Minor access passed its HTTP case and ten browser cases, including disposable
  viewer preparation and restoration. Five-language guidance and restricted
  empty-state renderer checks passed.
- Six adjustment-reason cases and the existing short audit-description
  regression passed, including ASCII/multibyte boundaries and an 8,192-character
  reason with complete audit metadata.
- The source capability and People pagination compatibility checks passed.
  Paused and insufficient-dose requests preserve clinical rows, stock, versions
  and sync records.
- All required unsplit dose baselines and four API unit tests passed, including
  dose writes, mode transitions, source capabilities, direct/sync source errors,
  sync and replay. The corrected management sync, medication projection and
  read-completion targets also passed.
- The import split passed independent whole-body review and its two write plus
  three portability checks after installation.
- The dose production and same-target test split passed requirements and
  quality/security review. All seven API suites passed after the move, with
  six route checks each; the complete test names and ignored attribute remain
  unchanged.

Detailed commands, selected counts, raw results and source-capture qualifications
are in [the verification queue](verification-queue.md). Independent findings and
accepted boundaries are in [the final follow-up review](review-final-followups.md).

## Installed refactors and remaining work

The import split is installed and checked. The dose production split and its
same-target test includes are installed and accepted after their post-split
checks. The dose facade is 217 lines after the reviewed structural lint
correction, with no warning allowances; its transaction owners stay whole in cohesive
private files. The test facade includes unchanged root-level test functions,
preserving names, filters and the ignored attribute. All 22 installed files
matched the reviewed private candidate before formatting. Exact body, visibility
and installation receipts are in
`/private/tmp/household-g-20261002/dose-extraction-candidate/`.

The occurrence split passed its post-split checks and independent review.
Schedule pagination normalisation (#2367) and numeric amount error wording
(#2368) were repaired locally with their preserved failing assertions. The
accepted unsplit file matched its before copy, and all
fourteen installed files matched the refined private candidate before formatting.
Its original type definitions, unit block, routine bodies and exports remain
preserved. Movement and installation receipts are in
`/private/tmp/household-g-20261002/occurrence-extraction-candidate/`.

The invitation split is accepted after source review and fast gates, six OpenAPI
checks including configured SMTP-failure rollback, eight legacy checks and
36 API plus 12 web unit checks. Its runtime provenance is runner-emitted only,
with that limitation retained in the verification queue. All eleven installed files matched
the reviewed candidate before formatting; receipts remain in
`invitation-extraction-candidate/`.

OAuth is installed after all five HTTP baseline selections and seven login
browser checks passed. Its retained before file matched current source exactly,
and all eight installed files matched the reviewed candidate before formatting.
The first compile exposed a missing `Client.id` sibling visibility bridge. Only
that field now has parent-module visibility, restoring the previous internal
access. All other fields, bodies and security ordering remain unchanged. The
corrected compile and post-split runtime checks remain pending; receipts are in
`oauth-extraction-candidate/` and `oauth-visibility-proposal/`.

The existing deterministic first-option stock race passed all four concurrency
checks before an assertion-only extension was installed. The extension makes
zero-to-one option creation and unchanged parent/option stock explicit, alongside
the existing rejection, retained-draft and version/sync proofs. Its original gate
and test names are unchanged. The extended checks still await runtime.

Before publication, finish the final sign-in split, then run
the final composed household checks and browser groups,
affected compatibility checks, Rust/documentation gates and independent final
review. The final composition includes the added audit and limited-member page
coverage; those additions still require the final combined runtime.

## Verification limits

Collection reads reauthorise every page and check complete IDs and consistent
totals. The API has no collection snapshot token, so this does not promise a
transactional snapshot across simultaneous edits. Fixture-only bulk rows,
identity changes and restoration are confined to disposable test projects.
The successful minor run's deleted source manifests were later reconstructed
to match its recorded digest; this is not an original-copy comparison.

The final current schedule source provenance is runner-emitted only; an
independent copy comparison is not claimed for that run. Occurrence acceptance
does not establish overall final-stage acceptance.

The first post-split doses fixture hash is runner-emitted only: independent
rehashing was unavailable after cleanup. The later six fixtures were independently
hashed before cleanup. This does not change the retained actual test results.

The existing ignored fractional timestamp test remains disclosed. No Rails code,
recurrence policy, new authentication feature or production cutover is part of
these changes. Source-only preservation proofs and private review approvals are
not runtime acceptance. Earlier milestone notes below retain failure/setup
classifications; their historical pending statements are superseded by this
current result and the verification queue.

## Ownership

The existing writer owns product code, regression tests and same-owner review
fixes. The verifier owns formatting, compile, runtime checks and screenshots.
The independent reviewer checks setup and final requirements. Root owns Task,
runner, CI, Git and publication. No runtime is performed by the writer.

Initial product paths are `rust/api/src/web_pages/api_client.rs`,
`rust/api/src/web_pages/inventory.rs`, `rust/api/src/web_pages/stock.rs` and
`rust/web/src/stock.rs`, with related translated guidance where needed. Shared
pagination integration remains coordinated with root. The canonical loss API
and public query contract remain authoritative.

Later mechanical splits are limited to `portable_imports.rs`, `dose.rs`,
`dose_occurrences.rs`, `invitations.rs` and `oauth.rs`, with cohesive child
modules, exact existing bodies/comments and facade exports preserved. They stay
held until their actual named baselines are green. No authentication feature,
delivery-policy change or production cutover is part of this stage.

## Initial failing-test request

`rust/contract-tests/tests/household_inventory_completion.rs` contains four
cases:

- `finite_parent_stock_with_only_null_options_removes_replays_and_rejects_without_changes`
  removes 2 ml from finite 20 ml parent stock with a real null-stock option,
  checks 18 ml and full history, exact replay, changed-payload conflict,
  insufficient-stock drafts and parent/option/version/sync/history preservation.
- `null_parent_and_null_options_cannot_remove_stock_or_write_history` retains
  rejected entries and proves no finite stock or history is invented.
- `tracked_zero_and_mixed_options_require_option_stock_even_with_finite_parent`
  proves zero is tracked, and a null option does not enable parent fallback.
  Its disposable public parent-adjustment stimulus deliberately makes parent
  stock finite without changing option stock; rejection remains authoritative.
- `visible_option_collections_above_five_hundred_keep_every_medication_page_complete`
  distributes 501 new visible options across two medications (200 and 301),
  alongside a scalar medication. It reads every authoritative API page, checks
  stable totals, unique IDs and foreign-option exclusion, then checks medication
  pages, dosage forms/list links, stock screens and complete assignment choices.

The large collection uses two real API-created null-stock dosage seeds. Only
additional rows in the disposable fixture are inserted from those seeds, with
required cycle, maximum, minimum interval and frequency copied. Stock remains
null; no production mutation shortcut is introduced. The large case runs last
in the serial target so its intentional page failure cannot spoil earlier login
setup. Fixtures use unique reserved client addresses and no grant/profile changes.
Canonical medication reads format integer stock as `20.0`/`18.0`; removal history
uses its separate normalized `20`/`18`/`2` representation. Option audit counts use
the actual `MedicationDosageOption` type.

Setup review is requested before the verifier's compile and actual RED:

```fish
rtk proxy task api:contract-selected-compile TEST_TARGET=household_inventory_completion
begin
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_inventory_completion
    set -lx API_TIME_ZONE UTC
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-routes.test.mjs
end
```

The requested HTTP count is four. Actual target selection and assertions must
be read from the raw log. The expected positive failures are parent fallback
removal returning 422 instead of 303 and a medication page returning 503 instead
of 200 above the option cap. The two negative controls may already pass. No
runtime outcome is claimed yet. Product code remains unchanged pending failures.

The first actual run selected all four tests, but each stopped at medication
creation with 422 instead of 201. None reached the intended stock or pagination
assertion. The helper supplied a null medication reorder threshold, which the
canonical medication validator rejects as blank. The corrected fixture uses
the accepted `"3"` threshold and includes JSON response context in medication
and dosage setup assertions. Intended null parent and option stock values are
unchanged; dosage thresholds separately permit null. This is a test-only setup
repair. The same four-case compile/runtime request is frozen for retry, with
the original failed setup evidence retained by the verifier.

The corrected run selected four cases: the null-parent and tracked-zero/mixed
controls passed, while finite parent fallback returned 422 instead of 303 and
the large-option medication page returned 503 instead of 200. These are the
actual product REDs. No browser smoke ran. Raw assertions are in
`/private/tmp/household-g-20261002/G-INITIAL-RED-002/runner.raw.log`, SHA
`40485820728b23bd6a4eabd37a977235e38a1fd3357e8c351cad35e70dc6aa12`.
Project `mtcontract-493bcdcbb3a54234` used matching copied source and fixture SHA
`6fda8987b4c1b19c8fb2a91494ce740b243d7b26dc29f51222f6e0d738b325bb`.
The initial setup-only failure remains in the verifier's task transcript; a
persisted raw assertion file was not located, so no build tee is labelled as
that missing raw evidence.

The narrow candidate removes the five-page collection cap. Every page still
uses the canonical authorised API, and the reader requires the reported page
and size, a stable authoritative total, unique IDs and exact total coverage.
An empty, duplicate or changed collection fails closed rather than establishing
that a medicine has no options. These are per-call authorised reads with
consistency checks, not a transactional snapshot guarantee; no new query,
snapshot or permission-version field is invented. Existing edit ETags remain
unchanged.

The loss adapter now accepts blank dosage identity only if every option has
null stock and the parent has finite stock. The renderer omits the empty option
selector in that case and shows the existing truthful parent balance. A tracked
zero still requires an option. The canonical locked loss API remains the final
stock and permission authority. No public API validation is relaxed.

The changed product paths are `web_pages/api_client.rs`, `web_pages/stock.rs`
and web `stock.rs`. Tests remain frozen for the same four-case focused GREEN,
preceded by API/web format, API check/Clippy and the selected test compile.
Minor People-page and linked/unlinked guidance fixes are separately pending
their own failures; no production changes for them have been made.

## Remaining verification plan

The focused stock candidate passed all four HTTP cases and six existing route
browser cases, with formatting, selected compile and warning-denying API Clippy.
The raw result is
`/private/tmp/household-g-20261002/G-FOCUSED-GREEN-001/runner.raw.log`, SHA
`1aab6a8511a56bffd636df287632910947195180e3bf9d990a3b7bc46824da3a`.
The three stock product paths remain unchanged while additional checks are added.

The next bounded test-only request is `readiness_locale` (ten rendering cases:
five missing-version messages and five truthful assignment-edit placeholders
and help), plus `household_minor_readiness` (one isolated minor-viewer case).
The minor helper copies the accepted temporary-age restoration and Drop bodies
from `web_reads_api.rs`; it queries the actual viewer account email, checks
authorised person and assignment reads, retains schedule API 403, then requires
the People page and readable assignment history in five languages. Hidden and
foreign person reads remain denied. This temporary-age fixture runs separately.
No production changes for these failures are present yet. Setup review precedes:

```fish
rtk proxy task -d rust/web test TEST_FILE=readiness_locale
rtk proxy task api:contract-selected-compile TEST_TARGET=household_minor_readiness
begin
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_minor_readiness
    set -lx API_TIME_ZONE UTC
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-routes.test.mjs
end
```

After these failures and narrow fixes, add five-language desktop/mobile loss
journeys. The ten rendering cases reached their intended failures: the known
missing-version error has no reopen guidance, and blank option editing still
uses the creation placeholder. Raw assertions are
`/private/tmp/household-g-20261002/G-READINESS-RED-001/readiness-locale.raw.log`,
SHA `50ea68f4716e0579e9fd879100554b511c5569a00e512d6d6ab644376501bb37`.
The isolated minor case compiled; its actual runtime remains pending. After its
application copy was validated, the already-failing guidance candidate was
applied. The browser maps only the known `If-Match is required` error to existing
translated reopen guidance. Editing gets a truthful keep-current placeholder
and explicit-replacement help in all five languages; creation retains its
default-dose placeholder. Unknown-error privacy and public APIs are unchanged.
The minor runtime then reached its intended failure: mandatory person and
assignment API reads returned 200, schedules returned 403, and the native People
page returned 403 instead of 200. Raw assertions are
`/private/tmp/household-g-20261002/G-MINOR-RED-001/runner.raw.log`.
The candidate now retains mandatory person and assignment reads, catches only
schedule-specific Forbidden, and passes an explicit unavailable flag to the
preloaded renderer. Five-language guidance explains the omitted schedules.
Other failures propagate, existing renderer entry points remain available, and
the public schedule policy is unchanged. The candidate is frozen for independent
review and the focused one-case GREEN. The verifier subsequently confirmed that
one actual minor HTTP case passes on the validated source copy. The ten known
missing-version and dose-choice renderer checks also pass at
`/private/tmp/household-g-20261002/G-READINESS-GREEN-002/readiness-locale.raw.log`.

Independent review then confirmed one remaining empty-state branch: empty
assignments plus unavailable schedules must not claim that no schedules exist.
The reviewed `overview_access_locale` target contains two renderer cases across
all five languages: unavailable-empty guidance without the false empty claim,
and the available-empty legacy control. It is installed after the minor copy
release for actual RED via
`rtk proxy task -d rust/web test TEST_FILE=overview_access_locale`.
The unavailable-empty test reached its intended failure while the
available-empty control passed. Raw assertions are
`/private/tmp/household-g-20261002/G-OVERVIEW-LOCALE-RED-001/overview-locale.raw.log`,
SHA `d5924146800895318cb8a7ea1585bfba3d9712bde985b2c3b14d43672e21fa16`.
The minimum repair now shows the legacy empty text only when schedules are
available. Restricted schedules retain their truthful guidance. This changes
presentation only. Both renderer cases then passed (2/2, exit 0):
`/private/tmp/household-g-20261002/G-OVERVIEW-LOCALE-GREEN-001/overview-locale.raw.log`,
SHA `5d2be08c02c3c3b4e834fb47467ee9a07c5c1267bbf892689d75f04742e1fa7c`.
The focused source manifest omitted the changed overview renderer, so its hash
is not claimed as part of that metadata receipt. The actual executed assertions
and the independently reviewed single-condition diff remain explicit.

The minor edits were applied immediately after the actual failure and before a
later coordinator notice that locale GREEN was active arrived. The verifier was
notified of the exact overlap (People/overview renderers and one extra key per
locale) and must distinguish the actual captured source from later live inputs.
No writer runtime or formatting was performed.

The separate `household_assignment_readiness` target contains two independently
reviewed existing-behaviour controls. A linked assignment keeps its stored link
when the choice is blank, rejects a mismatched changed dose without clinical,
version or sync writes, then replaces the link and dose through an explicitly
selected matching option. An unlinked assignment keeps manual dosing and does
not acquire a link. Scoped read-only persistence establishes identity because
the public response intentionally omits that field. Neither case changes grants
or profiles. Its exact requested compile is
`rtk proxy task api:contract-selected-compile TEST_TARGET=household_assignment_readiness`,
followed by an explicit `HOUSEHOLD_TEST_FILE=household_assignment_readiness`
HTTP-only inner Task selection only if a verifier-owned disposable runtime is
already active. The standalone outer command uses the existing household
wrapper and its bounded six route smoke cases after HTTP success:

```fish
begin
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_assignment_readiness
    set -lx API_TIME_ZONE UTC
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-routes.test.mjs
end
```

This outer wrapper provisions its own project; it does not need a caller-supplied
`CONTRACT_PROJECT`. Root selects fixture reuse or the standalone wrapper. Two actual
passing cases are required; setup review alone is not acceptance.

The assignment controls subsequently passed both actual HTTP cases and all six
existing route browser checks. Both portable baselines passed on separate fresh
fixtures: two portable-write cases and three portability cases. Their copied
application inputs matched. The People collection target subsequently passed
its one HTTP case and all six route browser checks on its validated copy.

The reviewed one-case `household_people_collections` target is now installed
after the prior copy release. It creates a canonical medication, assignment and
daily schedule, then clones 500 additional valid rows in the disposable fixture.
Every medication, assignment and schedule page is checked against stable totals
and unique IDs. The People page must include every authorised source history,
and native new/edit treatment controls must include the complete medication ID
set while retaining original edit tokens. This is caller coverage of the already
fixed shared collection reader, without a new API or snapshot guarantee. It runs
in its own fixture, without minor or grant changes. Exact compile request:
`rtk proxy task api:contract-selected-compile TEST_TARGET=household_people_collections`.
Exact runtime selection is `HOUSEHOLD_TEST_FILE=household_people_collections`
with `HOUSEHOLD_ACCEPTANCE=true` and UTC, using the existing household wrapper.
Its standalone result is now green; root controls the remaining serial queue.

The final acceptance facade is now installed separately at
`rust/contract-tests/tests/household_final_acceptance.rs`. It composes the
unchanged assignment (two), inventory (four) and People collection (one) modules:
seven HTTP cases, with assignments before inventory and large People data last
in the serial runner. Installation followed the standalone People pass; the
single combined run is still pending. The minor and permission-mutating fixtures remain
separate. The final native inventory browser draft has ten locale/viewport
cases; its actual screenshots and interaction results are still pending.

The first mechanical split divides portable imports into a facade and nine
modules: contracts, validation, identity, access, preflight, mapping, writing,
pauses and links. The original orchestration, transaction and authorisation
order remain in the facade. Each original implementation stays together in one
child module, with narrow internal visibility for shared helpers. No comments
were added or removed. The preserved before-file is
`/private/tmp/household-g-20261002/portable-imports-before.rs`, SHA-256
`d24868946b11011dd9fd100f01f56479abc7a59e9653e0cf05c0277bd05c64d7`.
The adjacent `portable-imports-function-bodies.tsv` records 41 moved function
and method bodies and zero original comment lines. Independent comparison and
post-split verification are pending; no later module split has started.

The frozen split request is API formatting, `api:check`, `api:clippy`, then
`api:contract-selected-compile TEST_TARGET=openapi_portable_writes` and
`api:contract-selected-compile TEST_TARGET=portability` as separate invocations.
After clean fast checks, rerun `api:openapi-portable-writes-acceptance` (two)
and `api:openapi-portability-legacy-acceptance` (three), each in its own fresh
fixture. Every command uses the `rtk proxy task` prefix. The seven-case
composition installation is a separate test-only change from this split.

The first formatter completed, then compilation identified two internal access
errors: the access module reads `ExistingIndex.ids` and preflight reads
`ExistingIndex.names`. Only these two fields gained `pub(super)` visibility;
the external interface, bodies and comments remain unchanged. This is a
mechanical compile repair, not a behaviour regression. Formatting changes are
separate from the initial move receipt. The renewed fast checks and post-split
five-case runtime results remain pending.

Independent import review passed all 49 original routine bodies, eight
constants/types, attributes and comments. Two formatter-only signatures were
identified separately. API compilation now passes; warning-denying Clippy and
the post-split runtime checks remain in progress. Import source stays frozen
through both runtime-copy validations.

The corrected private inventory browser draft passed setup review, including
browser teardown after registered tests and the existing exact generic
validation message in all five languages. It keeps the full stock, replay,
draft and no-write assertions. Its ten actual browser results remain pending.

Prepared-minor browser orchestration is private and has not run. The proposed
interface is `fish browser_minor_wrapper.fish <CONTRACT_PROJECT>
'<BROWSER_TEST_FILES>' <FIXTURE_PATH>`. Ordinary browser selections call
`api:contract-browser-node` only. The exact minor browser file must run alone
and uses its own disposable project's fixture and ownership marker. The helper
validates IDs and generated viewer identity, then calls
`api:contract-minor-viewer-sql CONTRACT_PROJECT=<id> PHASE=prepare
FIXTURE_PATH=<absolute fixture>` before the existing node Task. It always calls
the matching restore phase after browser success or failure. A preparation
failure prevents browser execution; a restoration failure fails the run.
The SQL pair uses atomic `psql -1 -v ON_ERROR_STOP=1` phases, safely quoted
literal parameters and exact membership/household/account/person predicates.
It changes and restores only the original person type, birth date and capacity.
No grants or public access policies change. Root owns Task installation.

The private fake-Task test contains nine cases: ordinary selection, successful
minor run, browser failure, preparation failure, restoration failure, mixed
selection, invalid project, ownership mismatch and invalid identity. The helper,
fake Task and test are under `/private/tmp/household-g-20261002/` as
`browser_minor_wrapper.fish`, `browser_minor_fake_task.fish` and
`browser_minor_wrapper_test.fish`. Review and execution remain pending.
The test accepts an optional helper-path argument. The private
`browser_minor_legacy.fish` reproduces the current unconditional node-only
dispatch, so the same nine cases can first demonstrate missing preparation,
restoration and isolation, then verify the proposed wrapper. The fake Task
also checks exact Task names and forwarded project/files/fixture arguments.
Root will provide private Task wiring for both variants; neither has run yet.
Independent setup review passed the prepared-minor SQL/browser pair and the
wrapper's returned-status paths. The wrapper restores after browser success or
failure and gives restoration errors precedence. It has no interruption
handler; forced termination relies on the unchanged outer owned-project
cleanup. Root's Task/SQL wiring remains separately owned and reviewed.
Installation waits for both import runtime-copy validations and the nine-case
wrapper RED/GREEN results. No further module split is authorised yet.
The first wrapper-test attempt ran the real Task command and stopped with
`No Taskfile found`; it did not establish a wrapper behaviour failure. The
private test now starts helpers with `fish --no-config`, preserving the injected
fake-Task PATH. The verifier must confirm actual fake-call receipts before
classifying the legacy RED or running candidate GREEN. Production wrapper
behaviour is unchanged by this test setup repair.

The legacy retry reached actual fake Task calls and produced the intended
failure: eight cases expose missing preparation/restoration or isolation.
Candidate attempts then identified two bounded helper/test issues. The email
literal dot now uses `[.]` to avoid Fish/jq escape layering. The fake test
expects the wrapper's canonical `realpath` for an existing fixture; missing
normal-path expectations are unchanged. These repairs do not weaken ownership
or identity checks. Candidate GREEN remains pending; minor installation waits.

Both post-split import copies are validated. The independently reviewed
inventory browser was installed unchanged at
`rust/web/tests/household-inventory-completion.test.mjs` and frozen separately
from the minor helper. Requested compile target is
`api:contract-selected-compile TEST_TARGET=household_final_acceptance`.
The runtime request uses the following exact selection, with the dashboard
clock unset:

```fish
begin
    set -lx HOUSEHOLD_ACCEPTANCE true
    set -lx HOUSEHOLD_TEST_FILE household_final_acceptance
    set -lx API_TIME_ZONE UTC
    rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-inventory-completion.test.mjs
end
```

Expected execution is seven unchanged HTTP cases (assignments two, inventory
four, People one) plus ten stock locale/viewport browser cases. Current results
remain pending. A read-only dose extraction map is prepared privately at
`/private/tmp/household-g-20261002/dose-extraction-map.md`; no dose source or
baseline facade has changed, and its actual baselines have not started.

The corrected wrapper candidate passed all nine fake-Task cases, Task exit
zero, with raw evidence at
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-004/green.raw.log`.
The reviewed minor packet is now installed and frozen:

- `rust/web/tests/household-minor-readiness.test.mjs`
- `rust/contract-tests/browser_minor_wrapper.fish`
- `rust/contract-tests/browser_minor_wrapper_test.fish`
- `rust/contract-tests/fixtures/minor-viewer-prepare.sql`
- `rust/contract-tests/fixtures/minor-viewer-restore.sql`
- `rust/contract-tests/fixtures/browser_minor_fake_task.fish`
- `rust/contract-tests/fixtures/browser_minor_legacy.fish`

The only installation adaptation is the test's fake Task fixture path.
Root owns Task wiring and its separate review. Intended isolated execution is
one `household_minor_readiness` HTTP case followed by the explicit minor browser
file's ten locale/viewport cases, with UTC and no dashboard clock. Actual minor
browser and SQL restoration evidence remains pending; the prior one-case HTTP
pass does not stand in for browser acceptance.

The installed wrapper's nine checks, runner lint and selector checks passed.
The separate minor runtime is underway; its copied-source validation and actual
browser/restoration outcome remain pending. Installed application and
orchestration inputs are held through that capture.

Two audit-reason probes are prepared privately at
`/private/tmp/household-g-20261002/household_adjustment_reason.rs`. Current API
validation accepts a reason string without a maximum; the audit event is an
unbounded `character varying` column with an event index. Neither source nor
the root OpenAPI supports the alleged 255-character limit. The tests inspect
the live column/index and check a 300-character reason plus a deterministic
varied 8,192-byte printable string. Both require complete reason preservation
in the existing event or structured audit context, saved stock, exact quantity
changes, one version/sync write and actor/request linkage. On non-success they
collect the public response, persisted stock, version/sync counts and actual
index metadata before failing. The larger input is a robustness probe, not a
confirmed defect or established index limit. Setup review and actual execution
are pending; no audit production change has been made.

The audit probe's two-case private setup passed independent review. It is not
installed and no actual long-reason result is claimed. The separate minor run
passed its one HTTP case, then stopped at the SQL Task's project/phase
precondition before browser assertions. This is orchestration setup failure,
not a minor browser result; root owns its correction.

A private combined dose baseline facade and shared-fixture safety note are at
`/private/tmp/household-g-20261002/dose_recording_baselines.rs` and
`dose-baseline-sharing.md`. The proposed serial modules retain all original
test bodies: doses 15, direct writes six, mode transitions two, sync 23,
replay eight, source capabilities two. The 56 total is source-derived only.
The note flags the missing required cycle in the existing mode fixture,
overlapping UUID prefixes whose tail values need prepared-fixture confirmation,
the replay actor's permanent grant revocation and last-suite temporary
restoration limits. It does not change tests or reset clinical state to make
sharing pass. Independent composition approval and actual baselines remain
required; no facade installation or dose source edit has occurred.

The corrected minor run passed one HTTP case and all ten browser cases,
including SQL preparation/restoration. The verifier reconstructed the frozen
400-file snapshot and matched the successful runner digest `9bae6448...`;
this is reconstructed-source evidence, not a claim about the original deleted
copy manifests. No metadata-only acceptance rerun was needed.

After that source release, the reviewed audit probe was installed unchanged at
`rust/contract-tests/tests/household_adjustment_reason.rs` and frozen. Compile
request: `rtk proxy task api:contract-selected-compile
TEST_TARGET=household_adjustment_reason`. Runtime uses the existing disposable
household wrapper with `HOUSEHOLD_ACCEPTANCE=true`,
`HOUSEHOLD_TEST_FILE=household_adjustment_reason`, UTC, no dashboard clock and
`BROWSER_TEST_FILES=tests/household-routes.test.mjs`. Expected execution is two
HTTP probes, followed by six route checks only if HTTP succeeds. Actual results
remain pending; no audit production change is authorised by setup alone.

Independent review did not approve six-suite dose sharing. The mode fixture
omits all three non-null/no-default option fields: cycle, maximum doses and
minimum hours. Prepared-fixture UUID inequality and default-page membership
also remain unproved. The facade stays private. The next clinical baseline is
standalone `source_capabilities_api`; preserve its tests until actual runtime
identifies the bounded compatibility or setup failure.

The audit probe executed both cases. The 300-character reason passed and
recorded its full reason and stock/audit write. The varied 8,192-character case
returned HTTP 500 `internal_error`; stock stayed `20.0` and medication
version/sync counts stayed `(1, 1)`. Raw evidence is
`/private/tmp/household-g-20261002/G-AUDIT-PROBE-001/runner.raw.log`, failed
request `850bfe44-ee85-4a12-b18d-7a71955006e8`. The test observed an unbounded
varchar column and the B-tree event index, but no retained server/SQL error
message establishes the exact internal failure. The cleaned fixture will not
be recreated solely for that missing diagnostic. This establishes a valid
large-input 500 without partial stock/version/sync effects, not a 255-character
limit. Source inspection finds the same full-event construction in Rails;
Ruby runtime has not been reproduced.

The agreed repair preserves complete legacy event text through 1,024 UTF-8
bytes and uses a quantity-only label above that internal display budget. Every
adjustment keeps its exact supplied string reason and normalised quantity in
`audit_context.inventory_adjustment`, without replacing trusted actor/request
fields. Missing, blank and non-string reason validation stays unchanged;
whitespace-only strings remain absent from the short label. This is not a
public input restriction or a claimed PostgreSQL limit. Stock-removal history uses a separate typed event;
Rust audit listing reads security events. Existing Rails admin renders raw
version event labels, while source payload verification retains the full audit
context. No history/public field or Rails code change has been made.

Four focused boundary tests reached their intended failures: ASCII and emoji
events at 1,024 bytes lacked structured reason metadata; at 1,025 bytes the
event still included the reason. Selected compile passed; actual run was 0/4
with two original probe cases filtered. Evidence is
`/private/tmp/household-g-20261002/G-AUDIT-BUDGET-RED-001/runner.raw.log`.
The production candidate changes only `medication_management/inventory.rs`
and `persistence.rs`. Existing version callers retain their interface and
metadata; a private version payload shares insertion without adding another
wide argument list. Audit insertion and stock/sync writes remain in the same
transaction. Two unused test helpers were removed after source release.

The stable verification request is API formatting, check and warning-denying
Clippy, then selected compile with `TEST_TARGET=household_adjustment_reason`.
Run the existing self-provisioning `api:browser-rust` wrapper with exported
`HOUSEHOLD_ACCEPTANCE=true`, `HOUSEHOLD_TEST_FILE=household_adjustment_reason`,
no filter, UTC and no dashboard clock: all six audit cases must pass. Use
`BROWSER_TEST_FILES=tests/household-routes.test.mjs` for its six cheap route
checks. Follow with the existing stock audit contract, including the exact
short reason/event assertion. Runtime and independent patch review remain
pending; the source-capability baseline stays held.

The audit repair passed independent source review, API format/check/Clippy and
selected compile. All six audit HTTP cases and six route checks passed. The
existing exact short-event stock test also passed (one HTTP case and six
routes), preserving common history text. Long reasons now save fully without
losing actor, request, stock-version or sync linkage.

Two test-only additions are now frozen for selected compile and test-list
inspection: `household_final_acceptance.rs` adds the unchanged six audit cases
as `stock_adjustment_audit`, bringing the composition to thirteen cases. Its
name sorts after `people` under serial libtest execution; declaration order
alone is not an ordering guarantee. Audit tests create unique medication rows,
use scoped counts and revoke only their own freshly minted tokens. They do not
change grants or profiles. The existing People test now checks a fresh viewer
cookie against complete permitted medication, assignment and schedule pages,
their exact ID sets, hidden/foreign exclusions, all native history links and
absent management controls. It reuses the existing managed-person view grant
and five hundred clones without adding grants or changing person identity.
This remains one People case; it makes no second-page People claim.

Next verification selects `TEST_TARGET=household_final_acceptance` for compile
and inspects the actual thirteen-case test list. The broad composed HTTP and
inventory browser repeat is reserved for the final G source freeze after
compatibility and the remaining refactors. The unchanged standalone
`source_capabilities_api` baseline follows this bounded checkpoint.

The unchanged standalone capability baseline reached both known compatibility
failures: moved stock remained in medication-ID order, and the paused-source
test incorrectly required an empty stock list. The bounded candidate changes
only `read_resources/source_stock.rs` and `source_capabilities_api.rs`.
Already-authorised candidates receive database ordering by location name then
medication ID, preserving database collation and same-name ties. Every ID is
bound and household-scoped; incomplete ranks fail closed. Existing scope,
person links, signature and supply predicates remain unchanged.

The paused-source test retains permission-derived `can_record`, expects its
scoped medication choice and submits an actual current-time dose. It requires
422 with the canonical paused error, unchanged full medication/source/dosage
rows, and unchanged take, version and sync counts. This is Rust browser
compatibility, not a new Rails JSON guarantee or pause projection policy.
Verification selects API formatting/check/Clippy and
`TEST_TARGET=source_capabilities_api`, then the same exported standalone
`HOUSEHOLD_ACCEPTANCE=true`, `HOUSEHOLD_TEST_FILE=source_capabilities_api`
wrapper with UTC, no dashboard clock and the six route checks. Its two HTTP
cases and independent exact-patch review remain pending. Dose refactoring
stays held until this checkpoint passes.

The first capability candidate passed its permission/paused-dose case, including
the actual rejection and no-write assertions. The second case passed moved
location ordering, then reached another obsolete expectation: positive stock
of 1.00 was offered for a 1.25 dose. Authoritative OpenAPI defines eligible
stock as untracked or positive; Rails out-of-stock and Rust locked submission
checks distinguish this choice projection from a sufficient full dose.
Production eligibility therefore remains unchanged. The later tracked-stock
empty-list expectation was corrected on the same rule before its next run.

The bounded test-only follow-up expects both positive choices and submits
actual scalar and tracked insufficient doses with distinct UUIDs. Both must
reject 422 with the exact out-of-stock error. Snapshots cover the distinct
original-source and selected medications, both dosage sets, source row and
household take/version/sync counts. Same-suite selected compile and two-case
runtime remain pending; no People or dose-refactor work has begun.

The existing People pagination baseline reached its intended failure:
`per_page=999` returned 422 rather than 200 with size 100. The narrow repair
clamps only People typed page sizes above 100 before its unchanged strict
parser. Zero/negative values, invalid pages, blank/invalid timestamps and
malformed integer queries remain rejected. No shared parser changes were made.
The same test adds 100/101/i64 maximum boundaries with identical permitted
data and totals, preserves strict oversized-query rejection for Locations,
Medications and Assignments, and verifies Schedules' existing lenient clamp
and blank-timestamp behavior.

The frozen patch is `read_resources/people.rs` and `web_reads_api.rs`.
Verification selects API format/check/Clippy, `TEST_TARGET=web_reads_api`,
then exported `HOUSEHOLD_TEST_FILE=web_reads_api` and
`HOUSEHOLD_TEST_FILTER=shared_collections_filter_before_stable_pagination`
with the existing household flag, UTC, no dashboard clock and six route
checks. One selected HTTP case is expected. Exact-patch review and runtime
remain pending; dose refactoring stays held.

People pagination passed its focused existing HTTP case and all six route
checks; source review and fast checks passed. Oversized positive sizes now
produce pages of 100 while the new invalid-query and unrelated-resource
controls pass. The next unchanged baseline is `dose_mode_transition_api`,
with two source-declared cases. Its known raw INSERT default omissions remain
untouched until an actual failure. The dose extraction map stays private and
read-only; no production split or shared baseline facade is installed.

The unchanged dose-mode target executed both tests and stopped during fixture
insertion: PostgreSQL 23502 rejected a missing `default_dose_cycle`. No
behaviour assertion ran. Its sole test-only repair adds all three current
non-null defaults to that INSERT: daily cycle 0, maximum 4 and minimum hours
0. Existing amount, unit and frequency and all behavioural assertions remain
unchanged. Selected compile and the same two-case runtime are requested;
production dose source remains unsplit.

The unchanged `doses` baseline declared fifteen cases: fourteen executed and
the existing timestamp case stayed ignored. Two passed, eleven stopped at the
shared medication create helper, and one reached an unknown-source mismatch
(422 instead of expected 404). The helper omitted the currently required
reorder threshold. Its test-only repair adds accepted threshold `3` and
captures public error JSON before the success assertion; behaviour assertions
and the ignored case stay unchanged.

Unknown-source compatibility is a distinct proposed ruling. Rails direct and
sync resolvers raise RecordNotFound for unknown kinds; Rust's scoped source
lookup already returns 404, but its early enum validation returns 422 first.
OpenAPI lists both statuses without defining that distinction. A bounded
proposal returns 404 for a supplied nonblank unknown string with a valid source
identifier, while retaining current 422 behavior for missing/non-string/blank
kinds and malformed identifiers. Direct, sync and occurrence callers were
inspected; occurrence builds only canonical kinds. No production handling has
changed pending root/reviewer agreement and focused regression controls.

Root agreed shared direct/sync 404 for a valid identifier plus a supplied
nonblank unsupported string kind. Missing, blank, non-string and malformed
references remain 422; other field priorities, auth and replay stay unchanged.
The reviewed two-case `dose_source_errors.rs` target is installed and frozen
for selected compile and actual failure evidence on unchanged production.
Each case covers twenty rejected requests, including exact unknown-field
priority and full clinical stock/source/options/takes/version/sync snapshots.
Sync attempts use distinct valid retry keys through the existing helper;
client UUIDs are distinct between direct and sync. Snapshots run before status
assertions, so an intended classification failure can still establish no
clinical mutation. The conflicting existing direct-write assertion remains
untouched until its unchanged baseline is recorded. No production handling
has changed.

The focused source-error target reached the intended failures in both direct
and sync: eighteen malformed-reference controls and the unknown-field priority
control passed with complete clinical snapshots unchanged; the final valid
identifier plus unsupported string kind returned 422 instead of 404. Evidence
is `/private/tmp/household-g-20261002/G-DOSE-SOURCE-ERRORS-RED-001/runner.raw.log`.
The candidate now separates malformed reference validation from unsupported
kind classification within the existing shared pre-validation position.
Unknown kinds return 404; all earlier unknown-field and later field/auth/lock/
replay/prepare/write bodies stay unchanged. The documented `dose_write_api`
unknown-kind expectation is the only changed behaviour assertion. API fast
checks, the focused two-case target and six-case direct-write rerun are
requested before other unsplit baselines. No extraction has begun.

Private read-only maps for occurrences, invitations and OAuth are prepared in
`/private/tmp/household-g-20261002/`. They preserve existing facade exports,
whole routines/impls/comments and transaction/security ownership; each is a
separate later checkpoint after meaningful baseline acceptance.

The remaining browser checks include five-language desktop/mobile loss
journeys, exact replay/history/stock assertions, keyboard access and screenshots.
A separate final permission fixture must cover grant loss and pagination-time
permission/version consistency. Partial pages cannot establish absence of dosage
options. It also checks hidden and foreign rows without broadening access.

Check 300-character adjustment reasons at the real audit boundary. A larger
varied reason is a robustness probe, not an established 255-character limit;
only actual indexed-field failure can authorise a storage remedy. Preserve the
full public reason contract. Add focused five-language missing-version guidance
tests, and verify mobile navigation by actual horizontal scrolling.

For compatibility, retain the approved People integer page-size clamp,
location-name/medication-ID stock order and paused-source stock projection with
actual paused-write no-write proof. Repair only demonstrated fixture/default,
optional-field, decimal, timestamp and semantic cache assertion defects. Preserve
microsecond ETags and all current permissions.

Before each module split, the verifier must run the exact named targets from
`final-writer-brief.md`. Known existing selectors cover portable writes,
portability, occurrences, invitations, auth sessions, auth and sync/replay.
Missing dedicated wrappers can use the existing
`api:contract-household-web-test CONTRACT_PROJECT=<owned runtime> HOUSEHOLD_TEST_FILE=<exact target>`
on the verifier's disposable runtime, without a browser run. Root owns any
bounded orchestration wiring. Compile success or a broader aggregate does not
replace an actual named baseline. Preserve before-files and exact-body receipts
for independent review, then rerun the same observable tests after each split.

Final acceptance requires combined accepted journeys, fixed-clock dashboard,
affected compatibility tests, five-language desktop/mobile evidence, Rust and
documentation gates, independent written acceptance and publication checks.

## Dose compatibility test installation

Reviewed test/spec changes are installed and frozen. Dose product changes remain
private; no occurrence out-of-stock mapping is installed. The existing dose run
reported eight passes, six failures and one pre-existing ignored case. Those
failures distinguish audit labels, occurrence paused codes, numeric amount
validation and an obsolete pagination expectation.

Installed paths are `rust/contract-tests/tests/doses.rs`,
`management_sync_events_api.rs`, `openapi_medications.rs`,
`openapi_read_completion.rs`, and `docs/api/openapi.v1.yaml`. The OpenAPI edit is
limited to the capability cache-header enum. The private proposals received
independent setup approval before installation.

The doses target retains its original ignored case, corrects invalid pagination
to 422 with a separate valid filtered page, strengthens numeric field errors,
and adds these two cases:

- `zero_stock_occurrence_retains_domain_error_and_direct_take_remains_generic_without_writes`
- `numeric_direct_dose_keeps_earlier_validation_priorities_and_sync_error_without_writes`

Next request: selected compile for `doses`, then the unchanged-product zero-stock
case through `api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=doses
HOUSEHOLD_TEST_FILTER=zero_stock_occurrence_retains_domain_error_and_direct_take_remains_generic_without_writes`.
Use the accepted explicit browser route-smoke selection if the HTTP case passes;
the expected occurrence error failure stops the wrapper before browser. Clinical
stock, takes, source, occurrence, versions and sync snapshots are checked before
the error assertion. The direct generic error is an unchanged control.

The first focused attempt was interrupted before Cargo assertions and supplies
no behavior result. The reviewed two-case refinement is now installed and
frozen: automatic selection omits `taken_from_medication_id`, while the separate
`zero_stock_explicit_selection_retains_empty_stock_priority_without_writes`
case selects its fresh zero-stock medication. Rails checks empty available stock
before selected-location resolution, so the explicit case intentionally checks
that priority. Both independently baseline after the successful stock adjustment
and compare clinical records before response assertions. Distinct UUIDs and
fresh sources avoid shared state. The target now has 17 executable cases plus
one pre-existing ignored case; the `zero_stock_` filter selects exactly two.
Dose product files remain unchanged. Next verifier request is selected compile
for `doses`, followed by the same wrapper with `HOUSEHOLD_TEST_FILTER=zero_stock_`.

Both independent zero-stock cases subsequently reached their intended failures
on unchanged product: explicit stock selection reported unavailable location,
while automatic occurrence reported the generic error code. Clinical snapshots
passed before the error assertions. Independent review accepted the private
correction, now installed only in `rust/api/src/dose.rs` and
`rust/api/src/dose_occurrences.rs`.

The dose-local typed cause preserves the existing shared ApiError facade for
sync, adapts only numeric direct responses and paused/empty-stock occurrence
responses, and leaves all earlier validation, access, locks and replay in order.
Empty authorised matching stock is checked before location selection; positive
stock selection is unchanged. Occurrence audit labels identify their actual
controller. Later locked decrement insufficiency remains unchanged and untyped.
The reviewed exact private diffs remain under
`/private/tmp/household-g-20261002/dose-compatibility-proposal/`.

Source is frozen for verifier formatting, API check/Clippy and selected target
compilation, then meaningful dose runtime checks. No module extraction begins
before corrected unsplit checks pass. All other installed test/spec and product
paths remain unchanged in this installation.

The dose production and same-target test split is now installed after the
accepted unsplit checks. Its first compile passed, but Clippy found seven unused
facade imports. The independently reviewed correction is installed in only
`dose.rs`, `dose/history.rs` and `dose/stock.rs`: five original definitions return
to the small facade, while two private unit-only bindings use `cfg(test)`.
The facade is 217 lines. Routine bodies, exports, comments and the complete
unit-test block remain unchanged; there are no warning allowances.

These three files are frozen for the verifier to rerun Clippy and continue the
post-split checks. Exact movement and installed-byte receipts are in
`/private/tmp/household-g-20261002/dose-lint-candidate/mapping.tsv` and
`/private/tmp/household-g-20261002/dose-lint-candidate/installed-preformat.sha256`.
Occurrence, invitation and OAuth splits remain private proposals.

Schedule baseline checks exposed a real read compatibility bug, tracked in
[#2367](https://github.com/damacus/med-tracker/issues/2367): malformed page values
were rejected before the schedule normaliser could apply its defaults. The
reviewed correction is installed only in the schedule reader and its contract
tests. It parses a signed decimal prefix locally and retains the existing page
and size normalisation, authorised filtering and timestamp handling. This is a
bounded schedule rule, not a claim of complete Ruby `to_i` equivalence; other
resources retain their current query parsing.

The same test file now uses valid medication setup defaults, nullable pause
projection comparisons, a precision-preserving RFC3339 timestamp helper, and
original-person successful updates with rejected transfer readback. All four
existing ignored cases remain unchanged. Both before files matched the reviewed
receipts; installed bytes match the candidate. These files are frozen for
formatting, compilation and actual schedule checks. Exact receipts are in
`/private/tmp/household-g-20261002/schedule-compatibility-proposal/`.
