# E stock writer report

Stock actions are now available from the medicine list and detail page. Managers
can adjust scalar stock, edit actual dosage stock, and record removals. Visible
medicines retain the existing permission boundary for placing and receiving an
order. Receiving an order changes its status without increasing stock.

A stock adjustment could overwrite stock from a dosage added at the same time.
The browser adjustment now rejects that change while keeping the user's entries.
It preserves the original form token and rejects missing/stale tokens before
quantity validation. The public stock API retains its existing behaviour.

Dosage stock displays each option's own quantity and unit, including in the dose
selector. Untracked options explain the supported parent stock fallback, with
an absent parent balance distinct from zero. Parent threshold edits are omitted
and rejected in option mode. The selected dosage and consumption rules are
unchanged. Adjustment history includes the supplied quantity and reason.

Focused verification passed. The core HTTP packet has 13 cases, including
actual null-option dose consumption and empty-stock rejection, concurrent dosage
insertion, original-token retention, removal replay, unrelated/foreign removal
rejection, audit/sync read-back and public API compatibility. One permission
case passed in a separate fixture, including ordinary-member order/receipt
and grant revocation. The browser packet passed all five languages at desktop
and mobile, actual dosage editing, stale drafts, removal replay, the mixed-unit
selector and prior dosage administration journeys. The full Rust gate passed.
Root owns final written acceptance and publication;
F remains held until E is published.

## Earlier work and failing checks

Root handed off `codex/rust-household-stock-20261001` after D publication as
PR #2351. E keeps the same sole writer, independent reviewer and runtime owner.
Rails and the authoritative OpenAPI document remain unchanged.

The initial RED packet contained four `household_stock` HTTP cases covering
scalar adjustment audit reason/quantity/request linkage, retained invalid draft
and rejected CSRF/foreign writes, order details/timestamp and receipt-only status,
and the real dosage editor route instead of parent adjustment for option stock.
Ten `household-stock.test.mjs` cases cover five languages at desktop/mobile,
list/detail action entry, adjustment, order, receipt, keyboard and viewport.

The existing Rust baseline selector is `api:openapi-stock-workflows-acceptance`.
HTTP RED uses `api:browser-rust HOUSEHOLD_ACCEPTANCE=true
HOUSEHOLD_TEST_FILE=household_stock
BROWSER_TEST_FILES=tests/household-stock.test.mjs`. Browser RED uses the same
browser file with household acceptance unset, since an HTTP failure stops the
first wrapper before browser execution. No fixture grants are changed by this
packet. Runtime and formatting belong to completion_build. Production behaviour
was held until relevant assertion failures were recorded.

Later E acceptance additions covered removals/replay, option edit concurrency,
permission loss in a final separate fixture, exact audit/sync and dose regression.
The accepted stock ruling reserves absolute parent adjustment for scalar mode;
option quantities use existing dosage edits with original preconditions. Receipt
does not increase stock. The two agreed presentation fixes reuse `med-primary`
and translate the existing inventory location literal. F remains held until E
acceptance and publication.

Initial runtime receipts: the existing `openapi_stock_workflows` baseline passed
3/3; seven of nine `medication_stock` cases stopped at the pre-existing timestamp
length assertion (27 versus 20), initially retained under #2347/G. Root later
accepted its shared timestamp assertion repair into E so the stock baseline
can run: valid UTC RFC3339 timestamps at documented precision, without changing
production precision or ETags. Independent static review passed this repair.
HTTP RED reached two missing stock forms (404 versus 200) and #2350's
actual audit event mismatch. Its option case had a setup failure: mandatory
frequency was absent. This was repaired in the test, with no API change.
Browser RED reached all ten missing stock-action link assertions.

The #2350 production fix now includes normalised quantity and supplied nonblank
reason in the version event, retaining the enclosing transaction and request
linkage. Its matching-source GREEN was pending at this stage. Integer API stock read-backs
use the existing decimal-string convention (`20.0`/`11.0`); new test expectations
were corrected accordingly without changing precision or values.

Root accepted #2352 into E: truthful all-null option scalar fallback, individual
option quantities with their own units, and omitted/read-only parent threshold
in option mode with forged edit rejection. Four new `household_stock_boundaries`
tests cover these boundaries and tracked-option removal/replay. Independent
setup review passed, including the selected card/detail DOM text assertion for
mixed units. Its actual RED and the repaired focused option-route RED reached
their assertions before implementation began.
Order/receipt preserve the existing API-visible medication permission boundary,
including ordinary managed-member success, while quantity/removal/option edits
remain manager-only. Existing API null-option stock semantics remain unchanged.

The first implementation passed focused compile and web checks. Later actual
failures proved missing/stale adjustment preconditions and both deterministic
dosage-insertion races; those failures stopped before later no-write assertions,
so the final run must establish the state outcomes. A renderer check proved the
absent-stock copy gap. The focused browser check then displayed a combined
15.75 ml instead of separate tablet/capsule balances; the selector fix followed
that failure. A display syntax error was corrected before the browser check.
Exact inputs, commands and results remain in the verification queue.

## Final verification request

The runtime owner formats the final inputs, then captures a fresh manifest.
All product, locale, test and Task inputs remain frozen through verification.
The commands below are the final request; earlier single-file RED commands
above are historical evidence only.

```fish
rtk proxy task api:fmt:write
rtk proxy task -d rust/web format
for file in household_stock household_stock_boundaries household_stock_concurrency household_stock_permissions medication_stock
    rtk proxy task api:fmt-file FILE=rust/contract-tests/tests/$file.rs
end
rtk proxy task api:check
for target in household_stock household_stock_boundaries household_stock_concurrency household_stock_permissions medication_stock
    rtk proxy task api:contract-selected-compile TEST_TARGET=$target
end
rtk proxy task -d rust/web test
rtk proxy task api:openapi-stock-workflows-acceptance
```

The full web test command includes the one `stock_locale` case, which checks
absent stock versus zero in all five languages. The existing stock baseline
runs the Rust `openapi_stock_workflows`, `medication_stock` and
`medication_management_security_api` targets, including the repaired shared
timestamp assertion. Formatting and compilation do not establish runtime passes.

Validate the new selection before capture: `HOUSEHOLD_STOCK_ACCEPTANCE=true`
with `HOUSEHOLD_TEST_FILE` unset selects exactly `household_stock` (four cases),
`household_stock_boundaries` (five cases) and `household_stock_concurrency`
(four cases), with no second dashboard command. Explicit file selection still
takes precedence; completion and default selections retain their prior behaviour.

Core request, with `HOUSEHOLD_TEST_FILE` and completion flags unset:

```fish
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_STOCK_ACCEPTANCE=true BROWSER_TEST_FILES='tests/household-stock.test.mjs tests/household-dosage-options.test.mjs tests/household-stock-selector.test.mjs'
```

This requests 13 HTTP cases, 20 stock browser cases, 20 accepted dosage browser
regressions and one mixed-unit selector regression, for 41 browser cases total.
An explicit `BROWSER_TEST_FILES` replaces the wrapper's default file selection;
it does not add implicit route/workflow cases.
The stock journeys capture desktop/mobile screens in all five languages.

Run the permission case last in a separate fresh fixture, with the stock and
completion flags unset:

```fish
rtk proxy task api:browser-rust HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_stock_permissions BROWSER_TEST_FILES=tests/household-stock-selector.test.mjs
```

This requests one HTTP permission case, followed by one selector browser
regression. It is the only E job that creates a
person/access grant and revokes that grant. Its mutations never share a fixture
with the core or existing stock baseline.

Root additionally requested the two existing medication browser files explicitly
to verify shared inventory and dose-dialog behaviour. These contain seven cases.
They may join the permission fixture's browser selection before that job starts;
otherwise run them once in a fresh browser-only fixture, with household flags
unset:

```fish
rtk proxy task api:browser-rust BROWSER_TEST_FILES='tests/medication-journey.test.mjs tests/medication-journey-form.test.mjs'
```

This request does not repeat the healthy stock browser cases.

After the targeted results and independent review, the relevant full Rust gate is:

```fish
rtk proxy task ci:rust-port
```

Root owns the gate, acceptance, Git and publication. Final results and copied-input
provenance are recorded in the verification queue; E is not yet accepted.

Final fast checks passed: formatting, API check, all five selected target
compiles, full web tests (52 passed) and Task selection validation. The repaired
existing stock baseline passed all 17 cases: stock workflows 3/3, medication
stock 9/9 and medication management security 5/5.

The first core run passed three stock-action cases and stopped on the fourth
case's audit expectation. The stored initial quantity is `20.00`; the test
incorrectly expected `20`. Root approved correcting that one expected string,
keeping the exact ordering, reason, request and sync assertions. No production
serialization changed. Remaining core HTTP and browser checks did not execute
in that run and still need their actual results.

The corrected core run passed all 13 HTTP cases, including the later audit/sync
linkage, rejected-draft and concurrent-change no-write assertions. Independent
review confirmed that the captured runs differed only in the audit test's
expected starting decimal string; all product inputs were unchanged. Browser
and separate permission results were pending at that receipt; their verified
outcomes follow below.

Final focused outcomes are green on the reviewed source: all 13 core HTTP cases
and all 41 selected browser cases passed. The separate permission HTTP case and
its selector browser regression passed. The two existing medication browser
files also passed all seven cases, covering prior inventory, dose-dialog,
stock/history, CSRF/replay and taper behaviour. Desktop/mobile stock journeys
ran in all five languages and captured screenshots for review.

The first full `ci:rust-port` gate stopped on three Clippy style findings in the
new stock browser route. Four small corrections followed: an explicit UUID
if/else, the default empty error map, an owned redirect string and an explicit
error-map type required by inference. No warnings were suppressed. Independent
review reconstructed the captured file by reversing exactly these changes and
confirmed their behavioural equivalence, so the focused runtime evidence remains
applicable.

The final full `ci:rust-port` gate passed. All requested stock verification is
green. Product, test, Task and this report are now frozen for the documentation
gate and final written acceptance. Publication remains with root; no F
implementation has started.
