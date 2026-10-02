# Persistent writer report

## D dosage management

Read the writer brief, plan, existing OpenSpec documents, scout report and Rust
project/behaviour/persistence/browser/verification guidance. Serena initial
instructions completed; active language server supports Ruby, so Rust navigation
uses rg. Context7 Axum documentation confirms body-consuming Form must be the
last handler extractor and existing Router merge retains shared AppState.

New HTTP regression file: `rust/contract-tests/tests/household_dosage_options.rs`.
Four cases cover
browser create/edit persistence with exact custom units, tracked zero versus
nullable supply and threshold; retained missing/blank/stale edit tokens; rejected
precision, CSRF and option/parent mismatch. Test-only medication renderer
regression requires safe localised unknown errors for all five locales.
The fourth case proves first-option confirmation and adult-default conflict
retain the draft and leave parent/option state unchanged. Rejected forms compare
all submitted native fields, checkboxes and cycle. A captured valid edit ETag
is made stale by a real API update, then retained on the rejected browser POST.
Login helpers use unique reserved 198.18 addresses. Invalid precision is minimum
hours 0.55; option amount 1.234 itself is valid.

Requested verifier jobs: existing `task api:openapi-dosages-acceptance` baseline;
existing household Rust fixture wrapper with
`HOUSEHOLD_TEST_FILE=household_dosage_options` for browser HTTP RED. Additional
web unit RED requested for `unknown_medication_errors_use_safe_localised_text`.

Proposed browser writes dispatch through existing WebApi and original If-Match.
No standalone create retry/idempotency guarantee: the API has none. Explicit
first-option confirmation will explain that scalar dose is cleared and tracked
option quantities become medication inventory totals. Null stock is untracked;
zero is empty tracked stock. Option ownership must match the path parent before
editing. Existing API optional ETag remains unchanged; browser edits require it.

After a passing 14/14 API baseline, extracted dosage API into validation, context,
responses, persistence, inventory, reads, create, update and sync responsibility
modules, retaining existing bodies/comments and public sibling imports.
Original source is `/private/tmp/dosage-options-D-baseline.rs`, SHA256
`f515bde959cc66f76f1026b27f31b657cc718357ae89c5e9041d1e73aa6e4850`.

Recorded RED: all four HTTP cases reach missing add route 404 versus 200.
The renderer unit failed by printing the supplied private diagnostic instead
of the generic English form message. Focused `TEST_FILE=dosage_locale` integration
failed because the non-English rendered dialog printed that same diagnostic.
Raw receipts are under `/private/tmp/household-d-20261001/`.

Implemented new browser adapter in `web_pages/dosage_options.rs`, typed native
renderer/list in `rust/web/src/dosage_options.rs`, new route wiring and entry
links. Writes reuse shared WebApi and retain submitted original If-Match;
browser missing/blank tokens preserve draft with 428. Medication errors use
safe translated fallback and useful explicit conflict/confirmation/reload maps.
Five locale trees gain equivalent dosage management and dose-dialog nodes.
Dose dialog JS reads the translated calculated-dose label from SSR data.
No dependency or Rails code changed.

First GREEN request: existing API/web formatting Tasks, web unit/integration
tests, matching-source existing dosage baseline plus new dosage HTTP/browser
jobs. Browser file covers all five locales at desktop/mobile, keyboard focus,
actual persistence, rejected drafts, escaped notes and viewport width.

Fast GREEN passed API all-target checking, selected contract compilation and
the full web unit/integration suite. Fixed one Leptos moved-title compile error
by cloning the heading value. Removed the genuinely unused Pagination root
re-export after the fast batch. Post-extraction API baseline passed 14/14 on
its earlier captured input; that receipt does not certify later test additions.

Final D additions are now authored: the four dosage HTTP tests assert create
version/sync linkage, unchanged parent and version/sync counts for rejected
tokens/precision/CSRF/forged related and foreign-parent POST, both adult and
child default uniqueness, and repeated edit rejection without another write.
`household_dosage_permissions.rs` is a separate single-case job: owner gives
the existing ordinary viewer a manage grant to a different permitted person;
medication creation becomes available but dosage GET/forged POST stays denied.
Run this grant-mutating file last in its own disposable fixture/token lifetime.

The browser dosage file now contains 20 tests: ten native CRUD cases plus ten
JS-enabled immediate-administration cases, all five locales at desktop/mobile.
The latter create tracked dosage through the browser, assign its snapshot through
the existing API, use the real translated source/dose dialogs, check focus return
and viewport width, prove exact stock decrement in both option and parent, and
replay the original dose UUID without another dose or decrement. Context7
Playwright 1.61 confirms shared browser/request cookies and URL-encoded form
requests with redirect following disabled for the replay response assertion.

Final runtime capture requested with stable source: API/web formatting and fast
checks, then `HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_dosage_options`
and `BROWSER_TEST_FILES=tests/household-dosage-options.test.mjs` under existing
`task api:browser-rust`. Separate final permission wrapper uses
`HOUSEHOLD_TEST_FILE=household_dosage_permissions`, same isolated browser wrapper.
No subsequent production edit is planned unless a concrete failure requires it.

Remaining: matching-source final runtime and screenshot receipts, independent
review and acceptance. Do not begin E before root accepts D.

D-CORE-GREEN-001 passed all four HTTP cases and 26/34 browser assertions.
All ten locale/viewport CRUD cases, both English immediate-administration cases
and prior household workflows passed. Eight later non-English immediate cases
failed when the test searched only the default first 20 dosage options; the
API collection defaults to page 1 / 20, and the helper discarded pagination
metadata. Independent source diagnosis agrees this is truncated test read-back.
The successful browser save already verified the created option through member
GET before redirecting. Raw assertions are in RTK tee
`1790874466_task_api_0b3d04.log`; the captured runtime remains separate from the
subsequent test correction.

Changed only the browser `read()` helper to follow metadata-checked collection
pages at 100 records per page, bounded to the existing 500-record limit. Member
reads and all exact option, parent-stock, translated-dialog and replay assertions
remain unchanged. This also prevents truncation of medication-take read-back.
Production source is unchanged. Corrected browser GREEN and the separate final
permission fixture remain pending; sources are stable for a fresh capture.

D final outcome: independently accepted by root after the four HTTP cases passed,
the isolated ordinary-member permission case passed 1/1, and the corrected
fresh browser input passed all 20 dosage cases. The full `ci:rust-port` gate
completed with exit 0. The earlier 26/34 browser receipt remains recorded above
as evidence of the test read-back defect and its bounded correction.

Product, test and report sources are frozen for D publication. E starts only
after root publishes D and provides the branch handoff. E will include the
nonblocking existing `med-primary` submit class reuse and localisation of the
legacy inventory detail literal `Household inventory`, with its relevant
desktop/mobile verification; neither requires another D capture.
