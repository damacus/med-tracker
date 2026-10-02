# Independent E review

The stock journey passes its focused tests, visual review and full Rust gate.
Final requirements verdict: PASS. Final quality/security verdict: PASS.
Managers can adjust
scalar stock, edit dosage stock and record removals; visible medication retains
its existing order and receipt permissions. Receipt leaves quantity unchanged.
The dose dialog now shows separate quantities and units, and concurrent changes
reject an old adjustment while keeping the user's entries. All bounded E
acceptance requirements are fulfilled; the coordinator owns programme acceptance.
The sections below retain the earlier findings and their evidence chronologically.

1 October 2026. Source and evidence review only. D's final review report remains
immutable. The coordinator owns acceptance; the sole verifier owns runtime.
This review uses the accepted [stock ruling](stock-ruling.md).

## Initial RED design

Requirements: pending. The four HTTP cases and ten locale/viewport browser cases
are an initial packet, not final stock-journey acceptance. They cover scalar
quantity/reason/version/request/sync linkage, rejected adjustment drafts and
CSRF/foreign no-write checks, persisted order fields and receipt-only status,
and tracked-option routing to its real editor with scalar adjustment denied.
Permissions, repeat/concurrency behaviour, actual option edit/reconciliation,
and matching-source runtime/screenshots remain pending.

Quality/setup: one initial compile blocker was notified to the writer and coordinator.
`rust/contract-tests/tests/household_stock.rs:8` includes the shared
`household_completion_medication/authentication.rs`, whose first line imports
`super::fixture` and `super::login`. Neither is defined/imported in the initial
stock target. Correct the test setup before treating a runtime failure as RED
product evidence. The writer then imported fixture and added the compatible
public CSRF login helper; source reinspection confirms these missing names are
resolved. Actual compilation remains verifier-owned. The shared helper retains
fresh public login, bearer mint/read-back and revocation; no credential/version
bypass was introduced.

The remainder of the initial design agrees with the stock ruling. Receipt checks
correctly preserve stock, and the option case makes no false parent-total
redistribution or standalone replay promise. The browser negative-number case
needs a form that permits server validation (`novalidate`); an invalid date
draft must survive rendering rather than disappear through a native date
control. These are implementation checks for the stable handoff, not observed
product findings. No runtime was performed by this reviewer.

## Strengthened boundary packet and audit fix

The four initial `household_stock_boundaries.rs` cases use fresh public cookie
login and current API CSRF, include the required option frequency, and read back
through the actual API. They cover omitted/forged parent threshold, per-option
mixed-unit display, preserved all-null scalar fallback with a warning, and
tracked removal draft/replay/payload-conflict handling. Removal replay and
changed-payload 422 agree with `rust/api/src/stock_removals.rs:384`.

One assertion defect was returned to the writer before capture: the mixed-unit
case checks absence of raw HTML `15.75 ml`, but current detail stock splits that
quantity/unit across strong/span (`rust/web/src/medication.rs:272`). An incorrect
aggregate could remain alongside correct per-option rows without failing that
negative assertion. Preserve the positive per-option checks and inspect scoped
rendered stock text or explicit aggregate absence. The fallback case currently
establishes preservation and warning only; actual finite/empty fallback dose
behaviour and no-write evidence remain required for acceptance.

The writer's renewed packet repairs this assertion: it selects the named
medication's card or detail stock container, joins DOM text nodes and normalises
whitespace before the same negative check. Per-option positive checks remain.
Required option frequency is present in both helpers, and stock assertions now
retain the API's `20.0`/`11.0` decimal representation. Bounded static repair
verdict: PASS, with actual compiled RED still verifier-owned.

The narrow #2350 fix at `rust/api/src/medication_management/inventory.rs:177`
formats the normalised quantity and appends the supplied nonblank reason to the
version event. It preserves original reason text, skips whitespace-only reason,
and retains the same encompassing transaction, request ID, before/after
snapshots and linked sync change. This matches source-defined Rails event
formatting in `app/services/adjust_medication_inventory_service.rb:30` and
`app/models/medication_stock_quantity_formatter.rb:13`. Static quality verdict
for this narrow fix: PASS. Actual audit RED/GREEN linkage must still be checked
from the verifier's detailed receipt before requirements acceptance.

## First E product handoff: fast-check scope

Requirements: PENDING. The stable first product implements native stock hub,
scalar adjustment/order/receipt/removal through WebApi, per-option inventory
units, read-only parent option threshold, safe translated errors and five-locale
copy. Final fallback-dose, permissions, concurrency and JS option/removal proof
were explicitly not part of this first fast-check handoff.

Quality/security: material race returned to the writer and coordinator.
`rust/api/src/web_pages/stock.rs` reads options in context, rejects option-mode
adjustment in permitted, then calls the parent adjustment API in save. A first
tracked option can be created after that read/check but before the PATCH writes.
`medication_management/inventory.rs` reauthorises and locks the parent but does
not check option mode or If-Match, so the scalar browser operation can replace
an option aggregate. A second preflight GET cannot close the final interleaving.
An explicit compatible atomic ruling and deterministic RED are required before
claiming scalar-only browser adjustment under concurrency. No API change is
implicitly authorised by this finding.

The timestamp-only baseline helper repair at
`rust/contract-tests/tests/medication_stock.rs:55` statically passes. It parses
RFC3339, requires zero UTC offset and Z suffix, and rejects non-microsecond
nanosecond precision without assuming a fixed string length. Existing callsites
use the renamed helper. It changes neither production precision nor ETags;
meaningful actual stock baseline runtime remains verifier-owned.

Other bounded source observations returned to the writer: the new
`inventory_quantities` helper renders blank parent supply plus its unit when all
options and the parent are untracked. The fallback warning should distinguish
parent null (no finite fallback) from zero (finite empty stock), and render a
truthful untracked label rather than an empty quantity. Base stock error summaries
currently link to nonexistent field nodes; render those as plain list items as
the dosage form does. The accepted G 501-option limitation now additionally
applies to inventory list/detail via their complete option_inventory read;
preserve that expanded impact in G verification.

The threshold browser repair preserves precondition priority: original missing
or stale tokens are rejected first, forged threshold gets draft-preserving 422,
and the legitimate option-mode payload omits the parent threshold. Public API
policy remains unchanged. Native stock forms use novalidate/text date with a
format hint, so invalid numeric/date drafts can reach server validation. Ordinary
visible-member order/receipt affordances are retained separately from manager
quantity/removal/option-edit permissions; final role evidence is still pending.

## Atomic adjustment and final evidence packet

The dose dialog still shows dosage stock with the medication's old unit. It
needs to show the selected dosage's quantity and unit. The list/detail stock
regions now use individual option quantities, but the administration selector
still receives aggregate MedicationCard values from
`rust/api/src/web_pages/inventory.rs:184` and renders supply plus scalar unit at
`rust/web/src/medication.rs:364`. This is a remaining #2352 correctness finding,
recommended labels `rust,bug`. The writer was asked for an exact real-option
browser regression before repair; the existing scoped stock-region assertions
do not establish correct selector text. The focused browser test now uses one
eligible medication container holding tablet and capsule balances. Its actual
assignment and displayed dose remain tablet-only. Describing both container
balances does not imply that the capsule option can supply the tablet dose.
Bounded selector stimulus/setup verdict: PASS. The actual browser RED displayed
`15.75 ml` for the mixed option container, independently confirmed in
`/private/tmp/household-e-20261001/E-STOCK-SELECTOR-RED-002/browser-node.raw.log`.
The stable fix supplies the existing authorised request-boundary option map to
the detail renderer for every eligible container. Labels list each option's
quantity and own unit, with translated null/fallback copy; Leptos escapes the
text. Source IDs, eligibility, dose summary and recording rules are unchanged.
Existing renderer entry points delegate through the new wrapper. Static
quality/security verdict for the selector fix: PASS; matching GREEN is pending.

The stock Task branch selects exactly household_stock, household_stock_boundaries
and household_stock_concurrency when HOUSEHOLD_STOCK_ACCEPTANCE is exactly true.
Explicit file selection retains precedence; unset/false retains the original
three targets, and completion selection retains its two ordered commands. The
permission target remains intentionally separate and last. Added same-household
unrelated and foreign removal sources retain the valid removal/replay assertions
and require unchanged selected/unrelated option stock, parent/version/sync and
empty removal history after denial. Bounded static setup verdict: PASS.

The scalar browser adjustment race has a compatible implementation ready for
verification. A crate-private ScalarAdjustment extension is inserted only by
WebApi.adjust_scalar_stock, then the shared API handler checks the original
captured ETag and existence of any option under its existing household/parent
locks before quantity parsing or mutation. Public JSON/headers cannot create
the typed extension. Requests without it retain the public adjustment contract.
Missing/blank browser tokens are rejected with 428 before quantity validation;
conflict rendering retains the original token and draft. Static quality/security
verdict for this bounded guard: PASS, with runtime acceptance pending.

The race RED receipt at
`/private/tmp/household-e-20261001/E-RACE-RED-001/runtime.raw.log` independently
shows four product failures: tracked and synthetic null-option interleavings
returned 303 instead of 409; missing/blank returned 422 instead of 428; stale
original token returned 422 instead of 409. Later draft/no-write assertions did
not execute because status failed first. The null-option test explicitly restores
only its synthetic parent's captured timestamp after real option creation; it
proves the independent option-mode guard rather than claiming a normal API write
leaves the ETag unchanged.

The final public compatibility assertion creates a tracked option, supplies a
stale public If-Match, and expects the existing parent-only adjustment to succeed
without redistributing option stock. The actual fallback-dose case assigns a
real null option, records a dose that changes parent 20 to 18.75 while the option
remains null, then empties the parent through the public API and expects a second
dose rejection with unchanged takes, parent/option data, versions and sync.
The separate permission case uses a real ordinary-member manage grant, proves
order/receipt access versus manager-only adjustment/removal, then revokes the
grant and rejects captured actions without writes. These fixtures pass static
setup review; run the grant/person-mutating case last or in its own fixture.

The null renderer now distinguishes absent parent stock from finite zero, and
base stock summary errors no longer link to nonexistent inputs. Requirements
remain PENDING until matching GREEN, the selector remedy and representative
five-locale desktop/mobile screenshots are reviewed. No runtime was performed
by this reviewer.

## Current requirements and quality verdicts

Requirements: PASS. Matching-source focused GREEN, visual evidence and the final
full Rust gate fulfil the bounded E journey. The coordinator owns acceptance.

Quality/security: PASS for the reviewed stock changes, with no unresolved
blocking E finding. The browser uses existing session, Origin, CSRF and API
authorisation boundaries. Scalar adjustment checks the original captured token
and absence of any option under the existing household/parent locks. Forged
related stock, revoked permissions, invalid drafts, stale tokens, controlled
interleavings and replay conflicts have actual no-write evidence. Public parent
adjustment semantics remain intact. Safe error translation and escaped draft
rendering remain in the existing adapter. No new endpoint, policy, dependency,
public guard field or clinical unit conversion was introduced.

### Exact verified evidence

- Existing stock baseline: 17/17 HTTP cases, comprising openapi_stock_workflows
  3, medication_stock 9 and medication_management_security_api 5. The raw
  `/private/tmp/household-e-20261001/E-FINAL-BASELINE-001/runner.raw.log` includes
  stock concurrency, replay/permission rechecks and audit behaviour. Its captured
  pre/post manifests match. The timestamp helper preserves UTC RFC3339 and
  microsecond representability without a fixed string length or production
  precision change.
- Core E: 13/13 HTTP cases, comprising household_stock 4, boundaries 5 and
  concurrency 4; 41/41 explicitly selected browser cases, comprising stock 20,
  dosage/edit/immediate administration 20 and mixed-unit selector 1. Detailed raw
  results are in `/private/tmp/household-e-20261001/E-CORE-GREEN-002/runner.raw.log`
  and `rtk-task-full.raw.log`. No default wrapper browser suites are included in
  this count. The final races execute their later draft and parent/option/version/
  sync no-write assertions. The synthetic null-option timestamp restore is
  expressly a test-only stimulus for the independent option-mode guard.
- The core audit correction changes the exact persisted before value from
  `20` to `20.00`, matching medication_snapshot's stored Decimal.to_string().
  It retains exact after value 15.12, reason, request linkage and one sync event;
  independent API decimal formatting remains separate. CORE001/002 captured
  manifests differ only in household_stock.rs; product inputs are unchanged.
- Separate permission fixture: 1/1 HTTP and 1/1 selector browser case in
  `/private/tmp/household-e-20261001/E-PERMISSION-GREEN-001/runner.raw.log`.
  Ordinary manage-member order/receipt succeeds; manager-only quantity/removal
  and revoked captured actions cannot write. The premanifest equals the core
  premanifest, and copied hashes match the captured application hashes. The
  runner emitted fixture SHA before cleanup at line 175:
  `0c6011584a753f2994e358150c3ca9e1ebda233399418ae92adef2368ec461e2`.
  Independent rehash of the removed fixture is unavailable; this is a preserved
  runner receipt, not a reconstructed checksum.
- Existing medication journeys: 7/7 browser cases in
  `/private/tmp/household-e-20261001/E-JOURNEY-REGRESSION-001/runner.raw.log`,
  including decimal dose/stock/history, CSRF and UUID replay, taper effective
  dose, Escape/focus return at both viewports, and invalid/foreign access.
  There are no HTTP or default browser extras in this count. Its captured
  pre/post manifests match the core source set.

Core source SHA is
`4ae39a39e47bb6d62ef8cae99345d81ddaebf135fbe4eaea1c9472050dacae57`.
All 325 copied application paths match captured inputs; copy manifest SHA is
`5d715a559a32e6ab09d81363e222dfe502ea8297fa2505e4f8c0c860e9427aea`.
The 326-path live pre/post manifests are independently compared equal, with SHA
`9c141b93fb0b0e7d47f35c7665544afcf6c575cc460179607babb36aa31d5adf`.
Core fixture SHA emitted before cleanup is
`432c503459ed3146d6676a570ae2b87451928a2643993d512a24725c61a1e282`.
Different jobs use separate disposable fixtures; no same-fixture or single-job
combined acceptance claim is made.

### Visual evidence and limits

All 40 E archive files independently match the manifest with SHA
`ebd7d72da54451bc9d79828dd734b4dc7ff9fd57ba7219e49cae40b85e1b52c6`.
The reviewer viewed twelve representative images across all five locales:
scalar hubs (EN desktop, CY mobile, GA desktop, ES mobile, PT desktop), option
hubs (EN mobile, CY desktop, GA mobile, ES desktop, PT mobile), and dose dialogs
(EN desktop, CY mobile). Controls and translated copy are readable, per-option
units are clear, receipt-only guidance is visible and E content fits the mobile
viewport. The selected mixed-unit label is proved by the actual browser
assertion; no dedicated mixed-unit screenshot was generated.

The six regression archive images also match their checksum manifest and were
all viewed: desktop/mobile dose dialogs, saved stock and dashboard history.
They show the decimal dose and resulting 18.75 ml balance with readable mobile
layouts. Images supplement actual persistence and keyboard assertions; they
do not establish untested roles or a full accessibility audit.

The existing 501-option collection limit remains tracked in #2353 for G, with
inventory list/detail impact retained. Remaining module extraction is recorded
separately in [remaining-module-review.md](remaining-module-review.md). Neither
authentication cutover nor all broader baseline audits are accepted here. The
reviewer performed only source/evidence reads, checksum comparisons and report
edits; no runtime or product/source changes.

### Final gate lint repair

The first full Rust gate stopped on three Clippy errors in the new browser stock
adapter. The writer replaced removal UUID then/default with equivalent if/else,
error-map constructor fallback with unwrap_or_default, and borrowed redirect
formatting with an owned String. The compiler additionally required the explicit
local BTreeMap<String, Vec<String>> annotation. The redirect helper accepts
AsRef<str>; the resulting location is identical. UUID creation still occurs once
and only for removal; default error state remains the same empty map.

Independent reverse-text checksum comparison reconstructs the passed runtime
stock.rs byte-for-byte at
`64ba30e584f33b8e3957819311a26e8c224ae76018baea5f1db9e9a36d1df108`.
These are exactly four mechanical expressions/type annotations with no warning
suppression, comment or behaviour change. Prior focused HTTP/browser/permission
and regression evidence remains applicable; repeating runtime solely for these
edits is unnecessary. The final full Rust gate on the repaired source remains
required and is still PENDING at this update.

### Final successful gate and frozen verdict

The first gate remains preserved at
`/private/tmp/household-e-20261001/E-RUST-GATE-001/runner.raw.log`: three Clippy
errors stopped acceptance. The final `ci:rust-port` gate on the repaired source
passed, with actual detailed output independently inspected at
`/private/tmp/household-e-20261001/E-RUST-GATE-002/runner.raw.log`.
It includes formatting, warning-denying Clippy, UI preview build, API 36 unit
and 12 authentication compatibility tests, web 52 tests including the five-locale
null/zero stock distinction, and the contract compile check. There are no failed
tests or remaining Clippy errors in this successful receipt; verifier exit is 0.
Raw log SHA is
`cd2a0ada9da3d78bc36fc250c71fd6e54b2681e95fd7ec788df3f78967d1b893`.
The 326-path pre/post manifests compare equal at SHA
`6ac0208380cd549bf697d7ad6111384880a87f86d2992e2c63c6927c5c705c8e`.

Comparing the passed core inputs with this final gate shows only stock.rs changed.
Its exact four equivalent lint/type-annotation edits are independently established
above; the focused runtime evidence remains applicable without claiming identical
bytes across that repair. The final gate covers the repaired implementation.

Final requirements verdict: PASS for scalar adjustment, truthful option stock,
real option editing, original preconditions and draft retention, finite/null/zero
fallback, actual dose consumption/rejection, removal replay and forged sources,
order/receipt persistence, current permissions/revocation, audit/sync linkage,
controlled concurrency and public API compatibility, five locales, keyboard
journeys and desktop/mobile presentation. Final quality/security verdict: PASS;
no unresolved E blocker remains. No broader authentication or production cutover
acceptance is implied.

Remaining G work is the existing #2353 collection limit, the outstanding #2347
baseline/parity rulings and associated meaningful regressions, portable-import
and accepted remaining-module responsibility extraction, and final combined
acceptance/CI promotion. Assignment/schedule/pause extraction and editors stay
in their approved preceding journey. These are tracked programme tasks rather
than unfinished E acceptance. This Markdown report is final and frozen for the
coordinator's documentation gate.
