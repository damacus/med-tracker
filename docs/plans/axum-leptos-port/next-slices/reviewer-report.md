# reviewer report

Status: final bounded A/B/C requirements PASS; independent code quality/security
PASS, with no material unresolved finding. Source and tooling gates PASS.
Actual final evidence comprises 28 HTTP cases, 35 scoped browser cases and
35 fixed-clock dashboard browser cases on the same frozen authored inputs,
using three separate disposable fixtures. Visual evidence inspected. A supports
People/Locations acceptance 2.3/3.3 (14/20 total); no D/4.3 or broader-audit
acceptance. Earlier pending statuses below are historical review records.

Baseline: eccf62aace3aab81bf871b380c964ff0ee28e7b6.

## Initial requirements verdict

A's proposed fallible form-error adapter is acceptable. Preserve the existing
`api_error` Option contract and useful recognised errors, including exact known
translations already produced by the People API adapter. Unknown field or base
messages must become a translated safe generic message in every locale. The
proposed English copy, "This value could not be saved.", makes no clinical or
retry promise. Preserve field association, escaping and all draft values.
Real authorised list/detail/add/edit, invalid/success, keyboard and mobile
evidence is still required before accepting People or Locations completion.

C's browser-only optional-profile fallback is acceptable after mandatory `/me`:
403 and 404 can use absent personal preferences. Profile 5xx and every required
authorised clinical collection must still fail closed. Keep own-person access
and profile API policy unchanged. The current profile requires a live grant to
the account-linked person (`profile.rs::self_person`), which explains why a
valid limited-view membership can currently fail at the mandatory profile read.
Visible-person-first selection, generic greeting, existing timezone and shortcut
defaults do not require widening visibility. Test the realistic fixture without
an own-person grant; hidden people, stock, history, counts and mutation controls
remain explicit acceptance criteria.

B's proposed parent-version re-read after options discovery is acceptable for
the stale precheck. The final PATCH must retain the exact original submitted
If-Match. Every 409 must render the original submitted scalar/identity mode and
preserve decimal and warning drafts. A fixture-only advisory lock on the
browser medication-show audit can establish the interleaving without product
hooks, provided the unique session, actor, household and controller/action are
scoped, the blocked read is positively observed, option insertion uses the real
API, and teardown always releases the lock/removes the trigger. Existing insert
before POST covers insertion before medication read; coordinator confirms any
additional in-request boundary requirement. No sleep-based race proof.

## Initial quality and security verdict

No material objection to these proposed contracts. Preserve browser session,
CSRF, origin, permission, transaction locking and idempotency boundaries. The
second parent version is only a comparison input; silently adopting it for a
write would defeat concurrency safety. The C change must not catch arbitrary
profile failures or swallow `/me`/clinical failures. A must not pass unknown
messages through merely because they have already been translated elsewhere.

## Evidence and cannot-verify items

Read the saved execution packet, team charter, owner briefs/reports and relevant
Rust project, behaviour, persistence, browser and verification references.
Serena initialised successfully but only Ruby symbolic support is active;
Rust reads use narrow rg/sed. Inspected current dashboard handler, projections,
API client, profile access, medication browser save path and existing household
contract assertions. No compiled, runtime, dependency or Docker commands run.

New RED/GREEN, deterministic race execution, real persisted medication/options/
stock read-back, five-locale browser journeys, desktop/mobile screenshots,
privacy/role regressions and final combined stable-input review are pending.
Policy acceptance is not requirements acceptance or a passing quality verdict
on implementation that has not yet landed.

## Initial test and coordinator wiring review

Read the new A renderer tests and C limited-view HTTP contract. They target
observable output without changing profile access or granting own-person
visibility. A's renderer tests cover both field/base unknown errors, useful
translated known errors, retained values and escaping. C verifies real login,
profile denial before/after, permitted people, private HTML and absence of
clinical mutation forms. Wider journey/privacy/role/mobile evidence is pending.

Read B's deterministic test before capture. The audit gate positively observes
an ungranted advisory lock after medication body/ETag construction; real bearer
option creation bypasses the exact browser-session gate. Drop releases the lock
and removes its trigger/function. Initial retained-value assertions incorrectly
expected inputs for native location/unit selects; the test owner repaired them
to assert selected option values. Description/barcode drafts and proof that the
first option changes parent ETag were added after review. Coordinator explicitly
approved preserving drafts on missing/blank 428 responses as a bounded extension;
the test/product owners are aligning that behaviour before capture.

The coordinator's optional Task selectors preserve the original default three
household targets and serial execution. Current Task documentation confirms
exported environment variables are template inputs. The Fish runner preserves
those exports for its child Task. CLI Task variable assignments alone do not
export into that child; use exported selectors and unset them for the final
broad acceptance job. No runtime selector result is claimed by this review.

Read the new A and C browser suites. A exercises all five locales at desktop and
mobile widths with real add/edit POSTs, API persistence read-back, detail reload,
invalid 422 drafts, exact translated labels/errors, keyboard access and escaped
textarea content. C keeps profile 403 before/after, compares authorised people
and medicines against owner-only names as privacy markers, denies hidden-person
selection and foreign/anonymous access, checks mutation controls and mobile width.
No material selector defect found statically. Screenshots need job/digest
provenance from the build lane.

C's required-read 503 case invokes the existing debug/CONTRACT_PROJECT server
query flag. It is not browser interception. Its exact limit: the handler returns
the error before calling the people collection, so it proves server failure
rendering rather than propagation of a failing internal API read. Wider roles,
inactive/revoked membership, profile 404/5xx and actual internal read failure
coverage remain cannot-verify items until named evidence establishes them.

## A product review

Reviewed the eight owned A product files after recorded A-RED-001. Bounded
requirements implementation: PASS for unknown-error sanitisation and preserved
known/pretranslated messages. `api_error` is unchanged; `form_error` accepts only
exact recognised English/current-locale values and otherwise returns the generic
catalogue message. People and all Locations field/base paths use it and propagate
translation failures. The new key exists in all five catalogues. Submitted
values, escaping, field association and validation rules are unchanged.

Code-quality/security static verdict: PASS; no material finding. No raw message
fallback, access or API policy change, comment edits or expanded product scope.
Full A requirements verdict remains PENDING actual GREEN and real browser
persistence/locale/keyboard/mobile evidence. This static review is not runtime
acceptance.

A-GREEN-001 independently audited: raw `evidence/A-GREEN-001-completion.log`
shows 2/2 new renderer tests and Task exit 0; `A-GREEN-001-regression.log` shows
11/11 legacy i18n tests and Task exit 0. Compared the 37-path pre/post manifests:
identical, with full digest
`efcfa6653edb78f425a5e0f7f2693c5aab324764fbd448d88041a61f367aad02`.
The bounded safe-adapter behaviour has RED→GREEN and static review PASS. Full
People/Locations journey acceptance remains open until real five-locale browser,
persistence and mobile evidence arrives.

## C fixture failure and browser snapshot tooling

C-RED-001's first runtime attempt did not reach the dashboard assertion: the
reference hidden-person GET expected 200 with the primary owner's token, whose
explicit grants do not include that person. This is a fixture setup failure,
not product RED. The existing feed token has an explicit manage grant to the
hidden person in `contract_provision.rb`; using it only for hidden reference
read-back preserves the viewer's unchanged access and strict privacy assertions.
The test owner must capture a real dashboard RED after this narrow correction.

The coordinator identified that the API/test context was copied but the browser
image still built from live `rust/web`. No browser pass is certified from that
split context. Reviewed the narrow tooling tests: runner shim verifies the
inherited captured browser path plus Dockerfile/selected test files; the new
context test renders the actual merged Compose model and extracts only Rust and
Rails build contexts with jq. It checks unset, empty, explicit default and
captured paths while preserving Rails context. Static quality verdict: PASS;
Luna tooling RED, root's bounded configuration fix and GREEN are still required.

Reviewed expanded C contract cases. Temporary viewer status (`suspended` and
`revoked`) and same-household membership-person mismatch restore original values
through RAII and verify restored people/profile access. The managed fixture
person has no membership, so the temporary pointer does not conflict with the
current unique membership-person index. Profile 404 preserves `/me` success and
unchanged authorised people rather than adding grants. Actual people/profile
read failures use uniquely scoped audit exceptions and a nontransactional
sequence: hit 1 proves real API 500; hit 2 after dashboard 503 proves SSR executes
the failing internal read. This closes the earlier debug-flag evidence gap once
actually executed. Trigger/function/sequence removal attempts are separate.
Static quality verdict: PASS; runtime results are still pending.

Reviewed B's desktop/mobile browser regression: real API first-option insertion,
parent ETag change, keyboard native POST, exact 409 draft/token/select/Decimal
retention, escaped warning text, real medication/option/stock read-back and page
width checks. Static quality verdict: PASS; deterministic interleaving remains
the contract test's responsibility and requires actual execution.

## Coordinator tooling review

Audited TOOLING-RED-001 retry and TOOLING-RED-002 raw logs and compared their
pre/post manifests. The retry reached the inherited-context assertion; the
Compose test reached the actual captured-context mismatch. Both are valid
behavioural tooling REDs. Reviewed the subsequent runner export, optional Rust
browser context and fixture digest output: correct bounded changes with Rails
browser context and owned cleanup preserved. The fixture digest logs only its
hash. Found a missing fake-provisioned fixture at the new hash boundary; the
test owner repaired `test:exec` to create valid probe JSON only after checking
the exact disposable run path, owner file and single matching project argument.
Static quality/security verdict: PASS after that repair.

Reviewed the completion selector test and Task change. Actual Task dry rendering
checks default/unset/false targets, exact true completion additions and explicit
file/filter precedence. The first silent-output failure was setup; verbose
rendering repaired it without weakening assertions. Final raw RED reaches the
true-case target mismatch; final raw GREEN exits 0. The exact-string true branch
preserves defaults and explicit file selection. Runner callers must export the
flag; this direct-Task test deliberately uses CLI variables.

Read final runner/context GREEN logs, selector GREEN, runner lint, three new
browser syntax checks and locale-tree synchronisation logs: all Task exits 0.
Final stable-input metadata for those tooling checks is still awaited before
certifying their exact snapshot. These checks do not prove A/B/C browser or
clinical persistence completion.

Subsequent metadata audit: runner/context GREEN pre/post manifests differ only
in `rust/api/Taskfile.yml`; the coordinator added a separately approved selector
Task during the run. Other listed runtime inputs match. These are actual passing
checks with qualified provenance, not stable-snapshot certification under the
execution packet's freeze rule. A final frozen rerun is recommended. Lint,
browser syntax, locale structure and selector GREEN pre/post manifests match.
Their recorded digests are respectively `3f6147fc…`, `a9b9f3ff…`, `f3903596…`
and `27b77168…`. These scoped checks do not establish clinical/UI acceptance.

## B-RED-001 classification

Independently read the actual contract RTK tee
`1790864926_task_api_browser-rust.log`; the earlier supplied nested tee contains
only image build output. The pre-read stale control and current malicious scalar
override control passed. The between-read gate was observed, but both browser
POST and bearer first-option creation timed out. This is a deterministic fixture
lock conflict: the AFTER INSERT audit gate holds the household foreign-key KEY
SHARE while dosage-option creation requires household FOR UPDATE. It is not
stale-window product RED. A BEFORE INSERT gate still runs after medication
body/ETag computation and avoids the audit FK lock; actual retry is required.
Recommended issue labels if follow-up is needed: testing/tooling.

The missing/blank precondition test checks persisted medication equality and
successfully asserts status 428 before calling `assert_retained`; its missing
native input then fails because the current 428 response has no draft form.
This is valid RED for the coordinator-approved 428 draft-retention extension,
not fixture setup. Stronger named-field diagnostics are useful without weakening
assertions. Recommended product labels: rust + bug. Race production edits remain
held until the corrected deterministic test reaches its intended assertion.

The test owner repaired B's gate to BEFORE INSERT with the same exact scope,
positive wait observation, real insertion and persistence assertions. Missing
input diagnostics name the field and HTML byte length without printing CSRF or
medical drafts. Static verdict: PASS; retry remains necessary.

Reviewed the authorised 428-only product fix independently. `trim` classifies
whitespace-only tokens as blank but does not modify the submitted draft token.
Missing/blank 428 joins the original-mode form rendering used for 409, retaining
all submitted values and using the existing translated reload copy from
`stock_removals.errors.invalid_submission` in every supported catalogue. Lookup
failure remains a 500. Session, origin, CSRF, access and final If-Match are
unchanged. Bounded requirements implementation and quality/security static
verdict: PASS. No second medication GET/race correction is included; race RED
and runtime acceptance are still pending.

## C actual RED and product review

Independently read `evidence/C-RED-002-full.log`: corrected fixture assertions
reach the intended dashboard status failure, 503 instead of 200 at line 88.
This is valid product RED. Source pre/post manifests are unchanged; all 296
overlapping copied/source paths match. One generated `.tailwind` asset and two
authored files (`browser_context_test.fish` and the medication browser suite)
are outside that source-path comparison; the coordinator checked contemporaneous
hashes and confirmed they were not inputs executed by this C job. The full
logged source digest is
`7034ac78e59d9cdadf221fa6396c0ab179daf9f07b09c31f1979f3780bed3838`;
fixture SHA is recorded before owned cleanup.

Reviewed the minimal `dashboard.rs` implementation. Bounded requirements
implementation: PASS. Mandatory `/me` is read before profile; only explicit
profile status 403/404 becomes `Value::Null`. Other profile and every required
clinical read failure still returns 503. Existing generic greeting, timezone,
shortcuts, authorised visible-first selection and projections are preserved.
No `/me` person fallback, profile API or grants change was added.

Quality/security static verdict: PASS; no material finding. All five C contract
cases, privacy/role regressions and desktop/mobile browser evidence still need
actual stable-input GREEN before full C requirements acceptance.

## B corrected deterministic RED

Independently read `evidence/B-RED-002-full.log`: the corrected BEFORE INSERT
gate reaches the intended race failure, 400 instead of 409 at line 280, without
timeout. Positive gate observation, real API first-option insertion, changed
parent ETag and medication/option/stock no-write read-backs all precede this
assertion. This is valid deterministic race product RED, source
`79c04b1cf2a291ddfd6ec434d73b06c0fcde7ef3adf5d018ee759ebadb69f0a1`,
fixture `47f32fc5fc42246c61cb81faf7aca73cca2722251ef99db5ea4cf61201bf93f2`.
Only this selector ran; the other three cases were filtered, so this does not
establish 428 GREEN. Coordinator released the agreed second-read correction.

Reviewed the full B minimal correction. Bounded requirements implementation and
quality/security static verdicts: PASS; no material finding. The second authorised
parent `get_reply` follows options discovery and its ETag is used only for the
stale precheck; the helper already rejects non-success/login responses. The
final PATCH retains exact `draft.etag` from submitted fields. Precheck 409/428
and final API 409 render the original submitted scalar/identity mode and drafts.
Origin, CSRF, session, capabilities, locking and idempotency are untouched.
All 298 copied paths in the frozen source comparison match for B-RED-002; only
generated `.tailwind` is outside that comparison. Actual all-four-case GREEN
and desktop/mobile browser acceptance remain open.

## Final acceptance coverage and tooling provenance

Found a material acceptance coverage gap in the initial five-file browser
selection: it omitted existing dose, stock, history, CSRF/replay and dashboard
metrics/private-cache regressions. Coordinator expanded the combined selection
to include both medication-journey suites and `dashboard.test.mjs`. Named
owner/self, administrator, manage/carer and parent dashboard cases also needed
explicit coverage; existing target counts did not prove that matrix.

Reviewed the new isolated role-matrix test design. Found and returned two public
grant API setup defects: no grant show endpoint exists, and grant creation has
no ETag/If-Match contract. Writer repaired them to strict collection-ID read-back
and actual public DELETE. Also identified that real minor creation increments
the creator's permissions version and invalidates its bearer credential; the
test must use real refreshed authentication and protect the shared primary
fixture token used by subsequent regressions. Implementation/runtime acceptance
of the matrix remains pending. Recommended follow-up labels: testing.

FINAL-TOOLING-001 runner raw log passes, but its broad manifest pair is invalid:
Luna disclosed that an old background per-file hash loop overwrote the pre-path
after the correct capture. Do not certify that pair. Seven focused executable
plumbing paths match immutable B-RED-002 hashes; independently expanded the
comparison to the real UI build dependency chain invoked by source-snapshot.
All 22 additional UI source/build/package/style and included Taskfile paths match,
with no missing paths. UI source contains no include-file assets. This supports
the passing runner result for unchanged executable inputs, with the invalid
broad-manifest qualification retained. Coordinator ordered the owned hash loop
stopped and unique exclusive capture paths for subsequent jobs.

Independently compared the subsequently saved FINAL-TOOLING focused runner,
expanded UI-build, context, selector, lint, syntax and locale manifest pairs:
every focused pair matches. Expanded UI-build includes actual build dependencies,
web assets/catalogues and separates generated `.tailwind`. The executable-input
comparison supports tooling acceptance without another rerun; the invalid broad
pair remains unusable. Test-owner repairs now use existing care-owner parent
creation and current cookie/CSRF authentication, protecting primary seed
credentials; exact updated helper review and actual matrix results remain pending.

Final authentication helper review: static requirements and quality/security
PASS. Two newly authorised Target methods keep existing methods/comments intact
and enforce local writes, checked same-origin URL, actual Origin and current
CSRF. New B helper performs real owner login and role/account proof, mints a
public app token, proves its distinct Bearer `/me` response, replaces only the
in-memory fixture token and revokes through the real public API with nonpanic
RAII plus required normal-path 204. Deterministic gate and all original
draft/persistence assertions remain intact. Parent setup uses the existing
care owner, real cookie/Origin/CSRF writes and fresh authentication after minor
creation; main viewer grants/profile denial are unchanged. Exact grant collection
read-back and public revoke restore delegated visibility. New B/C browser
helpers use current cookie sessions; B API writes include actual Origin+CSRF.
No SQL credential/version bypass or token output was added. All role cases and
combined HTTP/browser execution still require actual GREEN.

FINAL-SOURCE-001 independently audited: full raw includes API 36/36 and the named
unknown/known i18n, dashboard rendering/projection and existing form/navigation
suites passing. Luna reports Task exit 0 for the complete format, lint/Clippy,
test, build and contract all-target check chain. Compared authored 353-path
pre/post manifests: identical at
`bcac1978e6d36c95ed0493235304b096e912bd291c5fea3379113d5d3b2326ac`.
The explicit nested role/authentication helpers and Target additions are included.
Generated six-path pre/post pair also matches at
`074272ec08cf2fb8d9a309e59473274d75e426b88749b314d3b586d3af0dbeb3`.
Source gate: PASS. Clinical HTTP/browser acceptance and screenshot inspection
remain pending ABC-GREEN-001.

Final selector integration check identified missing synthetic dashboard clock:
`dashboard.test.mjs` requires `2026-03-29T00:30:00Z`, but the combined
`browser-journey-rust` path does not set it automatically. Luna confirmed the
approved ABC-GREEN-001 environment omitted `CONTRACT_DASHBOARD_NOW` and will not
mutate the running job. Any affected date/task assertions are setup/config
evidence, not product defects; that job cannot establish full dashboard
regression acceptance. Scoped actual passing cases can still be recorded, but
a corrected frozen job is needed for the combined verdict. Recommended labels:
testing/tooling.

## Final A verdict and scoped browser evidence

A requirements: PASS. A code quality/security: PASS, no material finding.
Recorded renderer RED→GREEN and unchanged recognised-error tests establish safe
unknown-error localisation without changing `api_error`'s Option contract.
Five locale trees match, relevant source suites pass, and actual browser journeys
prove authorised People/Locations list/detail/add/edit, successful persistence
with API read-back/reload, 422 no-write draft retention, associated translated
field errors, labels, Tab/Enter keyboard access, escaping, no-store and mobile
width across all five locales at desktop/mobile. A supports People/Locations
acceptance tasks 2.3 and 3.3; it does not establish Medicine task 4.3 or D.

ABC-BROWSER-GREEN-004 raw independently audited: 35/35 PASS, zero failed,
cancelled or skipped. Includes ten locale/viewport journeys, desktop/mobile
stale scalar drafts, limited-view privacy dashboards, immediate location use,
manual medication persistence, decimal dose/stock/history, CSRF and replay,
Escape/focus and foreign/invalid-auth regressions. This uses a fresh fixture,
not the HTTP job fixture: SHA
`d4b7a6054bc329b4af44923059ece712a62a1b0b508905bd2f49019e09309ec3`.
Independently compared both job pre/post manifests and the HTTP/browser
premanifests: all identical at
`4fea2556d999ab6dce08587bfad2cd8513fa53b0c3db1389838a5c63f4189cb4`.
Immutable captured source is the same `52eca56e...` set. These are separate
successful HTTP/browser evidence groups, not one successful ABC3 outer job.

Independent visual review: PASS for bounded changes. Inspected EN People and
Locations desktop/mobile; CY and GA People mobile; ES and PT Locations mobile;
stale medication and limited dashboard desktop/mobile. All five locales are
represented. Labels/errors are readable, forms remain usable, long copy wraps,
script drafts display literally, and scalar decimals/warnings remain retained.
Dashboard greeting stays generic and mutation controls disabled. The 40-image
archive is `docs/screenshots/ABC-BROWSER-GREEN-004-52eca56`; screenshot manifest
SHA `da08d621d31d5c15ce0b925493799d303fb348f195dd2f02a0ef6e9e5d75fd7a`.
Desktop images are 1400px wide, mobile 390px. Metadata discloses and corrects
the initial local-BST start timestamp mislabeled UTC; start time is approximate.
Final B/C broader requirements verdict still awaits the explicitly separated
fixed-clock dashboard metrics/private regression.

ABC-GREEN-001 full raw independently audited: eight of nine C contract cases
pass, including realistic profile 403/404, mandatory-read/server-failure denial,
suspended/revoked sessions, owner/self, administrator and manage/carer scopes.
The parent case fails during its first care-owner HTML CSRF read, before minor,
grant or dashboard assertions. Its helper incorrectly used `/inventory`;
the actual inventory route is `/households/{slug}/medications`. This is test
setup failure, not a C product failure. Cargo stops at that target, so no B,
original household targets or browser cases ran. No full combined PASS claimed.

Found the same wrong route in the two new B authentication/browser helpers.
Writer corrected only these three URLs and applied narrow Rust formatting.
Independently reversed each URL and the Rust line wrapping in memory: all three
hashes exactly recover ABC-GREEN-001 premanifest entries. Strict 200, CSRF,
permission, draft and persistence assertions and comments are unchanged. The
care-owner `/me` proof and inventory handler's permitted collection/capability
reads support this setup. Static requirements and quality verdict for correction:
PASS. Corrected frozen runtime and screenshots remain required.

ABC-GREEN-002 full raw independently audited: C 9/9, B 4/4, lifecycle 2/2 and
navigation 2/2 pass. B includes the positively gated real-API interleaving and
all missing/empty/whitespace precondition draft cases. C now includes actual
public parent grant/source/stock visibility and revocation. The old household
web target passes 10/11, then its delegated capabilities bearer gets 401 at
line 278 instead of expected 200. Both old target and People API are unchanged.
The new parent grant creation/revocation legitimately increments the delegated
member's permissions version; restoring visibility does not restore its seeded
token. This is new shared-fixture interference, not a Rust product defect or
an unchanged-baseline failure. Preserve all capability assertions using real
fresh authentication or an isolated fixture/job; never reset SQL versions or
accept 401. Browser execution was not reached. Recommended labels: testing.

ABC-GREEN-002 source pre/post independently compared identical at
`8d7d258233be5e7f7aa657c89b89208f40f28f3093e584e4a09ce4c56d583c7b`.
Copied source SHA `a73a37679986879e9db631828957e1b1018ba48242add4b73a9df653e335c618`;
fixture SHA `db967f59ee1504dcfd8b02631f7d8f7dc72e2dbdb99589932bdfc8d87ae51115`.
Eight earlier C cases plus the parent now have actual GREEN; full browser and
old capability regression acceptance remain open.

Coordinator approved fixture isolation through narrow Cargo invocation ordering,
leaving old tests intact. Actual TOOLING-ORDER-RED002 reports exacttrue expected
two commands, actual one; its pre/post manifests match. Reviewed new selector
assertions and minimal Task correction: original three targets plus B execute
first, C executes second only for exacttrue without explicit file selection.
Default/false/explicit selection, filters, project, manifest, all cases and
single-thread settings remain intact. All seven scoped browser files use fresh
cookies or unaffected viewer/feed tokens; no delegated or primary seeded bearer
use remains. Static requirements and quality verdict: PASS. Actual ordering
GREEN and recaptured combined browser execution are still required.

TOOLING-ORDER-GREEN003 independently audited: selector's exact default,
explicit selection and isolation-order assertions PASS, Task exit 0. Its
pre/post manifests are identical. Ordering repair now has actual RED→GREEN
evidence; frozen ABC-GREEN-003 is running the seven scoped browser suites.

ABC-GREEN-003 independently audited: all 28 HTTP cases PASS (B 4, lifecycle 2,
navigation 2, original household web 11, C 9). The ordering correction preserves
the old delegated capability assertion at 200. Compared captured-copy manifests:
all 301 copied inputs match. Source premanifest SHA
`4fea2556d999ab6dce08587bfad2cd8513fa53b0c3db1389838a5c63f4189cb4`;
copy SHA `52eca56e5adfd378c891d042055fa840be8d21e90eba8c1379bbc970b3d25b32`;
fixture SHA `3682d6bd8af84185d7d7cbdfcc8ea708a8d086d472aea8c16a7c913c03768e50`.
Browser startup fails because the supplied seven filenames are comma-joined
and Node treats them as one path. Existing Task expects whitespace-separated
filenames. This is job configuration failure; zero browser assertions or fresh
screenshots, and no browser acceptance claimed. Correct job arguments on the
same frozen sources; no product/Task modification required. Recommended labels:
testing/tooling.

## Final combined independent verdict

| Scope | Requirements | Code quality/security |
| --- | --- | --- |
| A: People/Locations safe form errors and five-locale journeys | PASS | PASS |
| B: medication concurrency/preconditions and retained drafts | PASS | PASS |
| C: optional profile fallback with authorised dashboard data | PASS | PASS |

No material unresolved finding in the approved A/B/C implementation. A's
known-error contract and fallible generic translation are preserved. B's second
authorised parent read is comparison-only; actual gated interleaving returns
409 with the original token/mode/draft and no unintended medication, option or
stock writes. Missing/empty/whitespace preconditions return 428 with retained
drafts. C uses absent profile preferences only for 403/404 after mandatory `/me`;
real required-collection/profile server failures still return private 503. API
grants, household/session, CSRF/origin, locking and idempotency policies remain
intact. Actual owner/self, administrator, manage/carer, public parent and denied
membership/foreign/hidden cases cover the approved matrix; no broader policy
change is claimed.

ABC-CLOCK-002 raw independently audited: 35/35 PASS, zero failed, cancelled or
skipped. Includes exact visible-only metrics, timezone/day boundaries, history
beyond 500 rows, equivalent stock scope, PRN/taper limits, private failure/empty
states, PWA patient-free offline/cache behaviour and keyboard/focus/hydration.
Clock is `2026-03-29T00:30:00Z`; fresh fixture SHA
`17bb31070d5e3438e8ee12d5075b6b1514db8088d1d53feadc398ee2246dddc4`.
Independently compared its pre/post and captured-copy pairs: identical; its
authored inputs equal the HTTP and scoped-browser jobs at `4fea2556...`, copied
source `52eca56e...`. Inspected fresh fixed-clock desktop/mobile dashboards and
validated all ten archived screenshot checksums. Archive:
`docs/screenshots/ABC-CLOCK-002-52eca56`; manifest SHA
`f6e57986a77a53f790ad1de4e1b872399cff12f010c875844bd4dab2a6efbabd`.

Combined acceptance uses three separately authenticated disposable fixtures on
the same frozen source: ABC-GREEN-003 28 HTTP cases; ABC-BROWSER-GREEN-004 35
scoped browser cases; ABC-CLOCK-002 35 fixed-clock browser cases. ABC-GREEN-003
outer Task failure remains disclosed as an invalid comma-separated browser
invocation; it is not reclassified as a successful combined job. The invalid
earlier broad tooling manifest also remains unusable, with executable-input
certification supported by the independently matching focused pairs.

Cannot verify/accepted scope limits: this review does not accept D, Medicine
task 4.3, later stock/audit tranches, the entire original checklist, publication
or deployment. Named broader audit contracts remain with the coordinator. The
execution packet and existing OpenSpec incomplete tasks remain authoritative;
only People/Locations 2.3/3.3 gain new checklist acceptance here. Final docs
verification and publication are coordinator/Luna responsibilities. Reviewer
source/test/Git edits or runtime execution: none; only this report was written.
