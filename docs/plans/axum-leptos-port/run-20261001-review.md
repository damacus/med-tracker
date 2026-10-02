# Independent review — 1 October 2026

## Scope and evidence

Initial review at 08:45 UTC covers the run plan/ledger, household and authentication
OpenSpec changes, shared household shell/CSS/i18n, household rendering/browser/API
test sources, and synthetic OTP test requirements. New journey modules remain in
implementation; missing routes before candidate delivery are not review findings.
The reviewer edits only this report and performs no source/test mutations or Git
publication. Serena instructions were read; the active language server is Ruby,
so Rust source inspection uses repository tools. Context7 is available for API
details that need verification.

## Requirements verdict

Pending journey candidates and runner evidence. Native create/edit drafts,
session/CSRF/origin/foreign-record denial, private caching, policy-derived
affordances and locale rendering must pass before accepting each journey.
Stock actions, seven treatment types, taper boundaries and pause/resume are
separate unfinished requirements until independently demonstrated.

Authentication review distinguishes pure OTP compatibility from enabled login.
The design explicitly preserves credentials, exact legacy derivation, replay
state and assurance, and gates passkey import independently. Factor login,
recovery races, OIDC and lifecycle mutations are not delivered by a pure helper.

## Code-quality verdict

Shared household foundation: no blocking source findings identified.

- Household/title labels pass through Leptos escaping; document locale is
  allowlisted and household navigation path segments are encoded.
- Native page bodies are supplied by trusted renderers; no browser draft or
  clinical cache is introduced by the shell.
- Translation catalogues are the five embedded authoritative Rails files.
  Locale is request-local, parsed catalogues are immutable, fallback is explicit,
  interpolation does not reprocess user argument content, and missing keys or
  arguments return errors.
- Exact zero/one/other plural selection is explicitly inherited Rails behaviour;
  fuller Welsh/Irish plural rules remain a disclosed separate gap.
- Cached deprecated serde_yaml is disclosed as a transitional dependency;
  leptos_i18n adoption/build support is not claimed.

Test source review: the workflows use JavaScript-disabled native submissions,
desktop/mobile widths, API read-back, invalid-draft retention and associated
errors. The API tests check missing sessions, foreign household/person reads,
invalid CSRF, dependent capacity normalisation and owner/viewer/delegate
capabilities. These are meaningful observable checks. Initial rendering RED and
i18n 7/7 GREEN are recorded by their owners; this reviewer has not rerun them.

## Outstanding verification

Journey candidates, broad build/lint, origin/expired-session mutations, restricted
member browser affordances, complete locale form/error rendering, cache contents,
stock/treatment behaviour, OTP implementation/vectors, and final integrated
candidate review are pending. No acceptance or full authentication parity claim
is made by this initial report.

## OTP candidate review at 08:50 UTC

Reviewed source SHA256: helper
`3cf66ea2f8987d082448b365b77480b5d5c8dacc952d6cd8890f83a0e8ebd58f`;
tests `7d31123af22e91e044ab23d73a6900dde5e9c811ef0138f779c91fa964bcd0a8`.

Requirements verdict: accepted for the bounded pure synthetic compatibility
slice. `auth_compatibility.rs` and twelve tests agree with the recorded locked
Rodauth 2.48.0/ROTP 6.3.0 oracle for 16/32-character seeds, current/old HMAC
derivation, explicitly disabled HMAC, fixed-time codes, one-step drift, strict
integer interval/last-use bounds, ASCII whitespace and latest matching step in a
synthetic OTP collision. The owner records the host Ruby 4.0.7 versus repository
Ruby 4.0.6 difference and reproducible oracle bodies.

Code-quality verdict: no blocking finding in the helper or tests. Seed checks
precede the import-only noncompliant builder, whose algorithm/digits/step are
fixed valid values. Timestamp conversion fails closed, arithmetic remains
within the accepted i64 timestamp range, old-secret use is explicit, and token
comparison uses the crate's constant-time token equality. There are no
environment/database reads or route changes in the helper. Context7 and cached
totp-rs 6.0.0 source confirm matched-step and builder semantics.

Independent check: `rtk proxy env CARGO_NET_OFFLINE=true task api:test
TEST_FILE=auth_compatibility` passed 12/12. The initial attempted task name
`rust-api:test` did not exist; rerunning through the actual `api` include passed.
Existing proc-macro-error2 future-compatibility and sandbox cache warnings do not
change that successful result. Broader lint remains the runner's responsibility.

Authentication caveats: full-precision database timestamps, atomic last-use and
failure-state updates, concurrent OTP/recovery one-use behaviour, session
assurance, passkey import, OIDC and account lifecycle are not proved by this
helper. Rodauth stores database time rather than matched step. The report
correctly records these gates and does not enable factors or claim full
authentication parity. Existing accounts and credentials remain unchanged.

## Household integration findings at 08:56 UTC

The following were sent directly to the coordinator while adapters were still
in progress. Final journey verdicts await completed candidates.

- Location success copy is selected solely by the caller's `saved` query
  parameter (`rust/api/src/web_pages/locations.rs:166`). A read-only request with
  `?saved=created` can therefore claim a successful save. Remove that unverified
  notice or derive it from trusted one-use state before accepting the design's
  verified-success-notice requirement. Resolved in candidate source: the caller
  query no longer selects a notice and successful POST redirects are read back.
- Location creation currently uses `locations.index.add_location` for its title
  (`rust/api/src/web_pages/locations.rs:282`), rendering “Add Location”; browser
  tests expect “New Location”. Reconcile with the actual Rails heading before
  recording runtime GREEN. Resolved in candidate source by selecting the
  authoritative `forms.locations.new_title` key.
- People/location renderers use `.form-field` and several panel/button classes
  absent from initial household/medication styles. Shared styling must cover the
  delivered markup before desktop/mobile accessibility acceptance. Resolved in
  source at 08:59 UTC: shared CSS now covers form-field, med-primary, med-panel,
  household heading/grid/details and focus states. Visual acceptance is pending.
- The initial translation adapter maps only blank/invalid/taken API messages.
  Numeric/unit failures can therefore retain English in non-English forms.
  Existing catalogue messages can cover several cases; full locale validation
  parity needs bounded mappings/tests or must remain explicitly unaccepted.

## Assembled source candidate review at 08:59 UTC

Code-quality verdict: no blocking security or data-loss finding in the assembled
People, Locations and Medication handlers/views, UI capabilities, shared
WebApi bridge or medication read-field additions. Source acceptance is bounded
to the delivered create/edit journeys; runtime acceptance awaits the runner.

The browser boundary remains cookie-only and rejects Authorization headers.
Each mutation checks the real Origin/Referer and CSRF; internal API requests
forward those actual headers. Extra mutation headers are limited to If-Match
and idempotency-key. The bridge retains ETag and session-renewal cookies.
Current household membership/person/API policies determine accessible reads
and writes, and capabilities call the existing people/medication permission
helpers. Private HTML, validation responses and redirects remain no-store;
rendered private pages keep the existing restrictive CSP.

The unchanged service worker caches only its explicit public asset/offline
allowlist. Household navigation is network-only with the public offline screen
as the failure fallback; the new pages add no browser medical draft/API storage.
This is source evidence, not a new runtime cache-content acceptance result.

SSR form values remain explicit native attributes/content. Person type and
capacity retain canonical API values. Decimal drafts remain strings. Textarea
content is encoded before trusted HTML insertion, including malicious closing
textarea input. Existing dosage-option medications omit scalar dose/supply/unit
fields, explain that those options remain unchanged, and reject forged scalar
fields instead of changing the medication's dosage mode. This preserves options;
it does not deliver a dosage-option editor.

Requirements verdict: source design is suitable for bounded runtime acceptance.
Independent `rtk proxy env CARGO_NET_OFFLINE=true task -d rust/web test
TEST_FILE=household_rendering` passed 9/9, including native SSR state, text
escaping, five form locales and options-mode exclusion. `git diff --check`
passed. Runner API/browser/security/screenshot results are pending.

Full translated numeric/conflict validations, all non-English browser journeys,
stock actions, assignments, seven schedule types, pause/resume and taper
boundaries remain unaccepted. Exact catalogue maintenance checks must cover the
new five-locale dosage-options explanation. Auth factor/lifecycle gates remain
as recorded above.

Catalogue diff review confirms identical new key paths in all five catalogues:
`forms.locations.new_title` and `forms.medications.dosage_options_read_only`.
Spanish/Portuguese wording preserves the read-only-options meaning; native
Welsh/Irish linguistic review is not claimed.

Non-blocking maintenance note: Location capabilities currently use the
medication household-manager helper; its owner/administrator rule agrees with
`locations.rs::manager`, but sharing the actual Location permission helper
would prevent future policy drift.

Reviewed SHA256:

| Source | SHA256 |
| --- | --- |
| `rust/api/src/web_pages.rs` | `09d04ac6a2b8864993caf680aaa40ab7d2bf9ff187ad4ffbb95f6a322eca248b` |
| `rust/api/src/ui_capabilities.rs` | `5d66bd9786bcf447339654bd58e8dd7ea5f5049afbf90622eb7a4f356f94f314` |
| `rust/api/src/web_pages/people.rs` | `7d2c89c62e09120a11f93618eae6509ecfb227331d61e6832ffb2603011b3084` |
| `rust/api/src/web_pages/locations.rs` | `a1e8ccce88da1a902f38a8782a6bc1102a42edaa701f36aa3544f48ee268378f` |
| `rust/api/src/web_pages/medications.rs` | `ebeb708d6b4178eb2af665e65b30caa2f82f4511b9d1f856b8756738045c74f9` |
| `rust/web/src/people.rs` | `c011b9961ba464531e185e9a6684797e4f4ab5aa91b25d9ea3310b4c227fbd9b` |
| `rust/web/src/locations.rs` | `d45afd73b52f38b611f2a68c3121a94bd697ce8d2c8e73dc14f1d4314eaedec8` |
| `rust/web/src/medication_management.rs` | `57b8256e9a1dea45494f24ceb6a519802a337976f1388baedeed82783cf31e0e` |

Runner integrated acceptance started 08:59:27 UTC on immutable product/test
digest `ee1f625dd4357de7dfef110076e6d47f92ec0f4797b42407dc0443fbc9831f74`,
project `mtcontract-dfaa00f2263c488a`. The report records household API contracts
before household routes/workflows and prior medication browser regressions.
Actual runtime assertions remain in progress at 09:04 UTC.

## Follow-up correctness finding at 09:07 UTC

P2: scalar Dose is optional, so blank-dose creation stores null without creating
dosage options. The subsequent editor treats null as options mode and removes
scalar dose/unit/supply edits. The new medication cannot later acquire its
standard dose through the delivered editor. The medication owner independently
confirmed this lifecycle edge; root was notified. Accepting this full lifecycle
requires a tested bounded fix or an explicit unsupported-path decision.

The same edge affects scalar editing: clearing an existing Dose submits null,
then the next edit becomes options mode without options creation. A create-only
fix is insufficient. Keep scalar create/edit from silently changing dosage mode
until the separate mode/options editor is delivered; retain existing options
medicines without scalar writes.

References: rust/web/src/medication_management.rs:178;
rust/api/src/web_pages/medications.rs:98 and :260.

Locale mapping follow-up: inclusion, numeric and fixed-zero greater-than messages
now map to catalogue keys; owner records 10/10 focused GREEN. Unknown
decimal/precision/type and conflict details remain full-translation gaps.

Full locale parity is also unaccepted across the prior inventory/detail pages:
their new action labels receive locale, but existing Dashboard/Inventory/
Medications/remaining/View-medication copy and the medication document language
remain English (`rust/web/src/lib.rs:267`). A non-English form save can therefore
return to mixed/English inventory. Dashboard/search hydration remains outside
this bounded adapter. Five-language form rendering must not be described as
five-language end-to-end journey completion.

## Follow-up checkpoint at 09:17 UTC

The coordinator selected actual scoped dosage-option presence as the mode
source, preserving the API's optional-dose contract. A null dose with no options
must remain scalar editable; existing options must remain untouched. The new
`household_lifecycle.rs` has meaningful API read-back checks for first scalar
dose editing and a stale scalar form after options creation, including retained
drafts and unchanged medication/option records. Runtime RED and the production
fix remain pending, so the P2 finding is still open.

The existing parent medication ETag serialises timestamps to whole seconds.
An untracked first-option insert on a null-dose medication can otherwise leave
its serialised parent values unchanged. An adapter-only options read cannot
prove atomic mode preservation if an option appears between that read and the
API PATCH. This existing API concurrency limit was sent to the coordinator for
the fix's acceptance bounds; the regular stale-form test does not prove that
race. Supplied form ETags must remain unchanged through forwarding.

The location capability now calls the actual location manager helper, resolving
the earlier nonblocking policy-maintenance concern. New navigation tests check
labelled People/Locations/create/edit reachability and viewer mutation-link
absence; their runtime result remains pending. The integrated runner has not
yet produced product assertions, and its last report records build/disk
diagnostics rather than a journey result.

Report checks: `git diff --check` passed. `task docs:build` passed with
`UV_OFFLINE=true UV_CACHE_DIR=/tmp/medtracker-review-uv-cache` (no issues,
1.31 seconds). The initial default UV cache invocation was sandbox-denied;
the writable temporary cache resolved that prerequisite without escalation.

Inventory locale follow-up source review: the new catalogue title/sidebar/View
and `stock_remaining` interpolation preserve exact decimal/unit strings and
escape record content. The document locale replacement uses the allowlisted
locale and the existing title-escaping helper. No blocking source finding in
that candidate; its focused GREEN result is pending. Medication detail and
dashboard remain outside that correction. Null supply still renders an empty
amount followed by the unit/remaining text (`web_pages.rs:654`), so tracked and
untracked stock display completion is not established by scalar form tests.

## Recovery source review at 10:20 UTC

Reviewed the recovered household source and the coordinator's frozen medication
precondition candidate. No new authorization, credential-disclosure or
write-integrity finding was identified in this bounded review. Serena discovery
in this review session did not expose `initial_instructions`; direct Rust source
inspection was used. No source or test files were changed by this reviewer.

The earlier null-dose lifecycle finding is resolved in source: options mode now
depends on the medication's actual scoped dosage-option collection. A null dose
without options retains scalar controls. The bounded collection loader fails
closed on incomplete or oversized collections. Existing option medications
continue to reject scalar dose/unit/supply submissions.

Medication serialization now retains six fractional timestamp digits. Existing
option creation updates the parent timestamp inside the same household and
medication locks used by API PATCH. The original submitted ETag is still
forwarded as If-Match to the authoritative API mutation. These source properties
protect a PATCH from overwriting a concurrent option-mode change; a full HTTP
race test has not been run by this reviewer.

The ordinary stale native form now compares its submitted ETag before the new
options-mode scalar-field guard, returns 409, and renders the submitted draft's
original scalar/options controls. The lifecycle test checks retained scalar and
warning fields and reads back unchanged parent and dosage-option records. The
coordinator captured a focused unit RED result before this helper correction;
the final focused GREEN result is not yet independently recorded here.

One remaining P2 draft-retention race was sent to the coordinator: `save` reads
the medication ETag before loading options. A first option inserted between
those reads can make options mode true while the captured ETag still matches the
submitted ETag. The scalar-field guard then returns 400 without retaining the
draft. Refreshing the medication ETag after loading options would detect that
newly observed mode change; the submitted If-Match must still reach the API for
changes after that refresh. This is an error-response/draft-retention finding;
the branch does not write medication data.

The WebApi bridge continues to use the real browser Origin/Referer, native CSRF
and cookie session, forwards only the established mutation headers, and keeps
the existing API authorization boundary. Private rendered pages and failures
remain no-store. The location capability now calls its actual permission helper.
Native SSR values and textarea escaping remain explicit. Inventory and the
medication management heading/details now use the five catalogues; the existing
dose modal still contains English copy and full locale journey parity remains
unaccepted.

`git diff --check` and `task docs:build` passed in this review session. The docs
build used the writable temporary UV cache and finished with no issues in 1.31
seconds. Application build, lint, fresh Rust-listener
HTTP acceptance, browser submissions, security denial cases and desktop/mobile
screenshots remain the runner's responsibility. The disk/build incident has not
produced completed browser acceptance evidence. Source review alone does not
establish a completed household journey or full authentication parity.

## Navigation test setup review at 10:43 UTC

Reviewed the narrow `household_navigation.rs` change after rebase. Removing the
dashboard GET from the sign-in helper is appropriate for these inventory/detail
contracts: the viewer dashboard's 503 response prevented the test from reaching
its intended assertions. The helper still requires login HTML 200, extracts the
native CSRF token and requires login POST 302. Each owner/viewer inventory and
medication-detail request still explicitly requires 200 before checking labelled
owner links and absent viewer mutation links. No intended navigation or
authorization assertion was weakened by this change.

Viewer dashboard 503 remains a separate behaviour requiring investigation; this
test adjustment does not establish dashboard acceptance. The coordinator reports
the rebased `ci:rust-port` gate GREEN, but this reviewer did not independently
rerun that application gate. The generated `preview.css` update from main's
Tailwind dependency is outside this narrow test-source review. The between-read
option-mode draft-retention P2 above remains open. Fresh HTTP/browser acceptance
results must be recorded by the runner before claiming completed journeys.

## Viewer dashboard dependency investigation at 10:45 UTC

Read-only source investigation identifies `/api/v1/households/{id}/profile` as
the likely failing dependency. Dashboard unconditionally reads the profile after
its visible-people collection, then converts any profile API failure into the
generic dashboard 503 page (`web_pages.rs:1166`). Profile reads require an active
view/record/manage grant on the member's own account-linked person
(`profile.rs:189–222`). The navigation viewer fixture creates its own person and
member but grants view access only to the managed person
(`scripts/contract_provision.rb:570–580`). HouseholdMembership has no callback
that creates a self grant. In contrast, the separate profile-view fixture
explicitly adds its own person view grant (`contract_provision.rb:252–254`).

This predicts profile 403 followed by dashboard 503 for this navigation fixture;
it is a source-derived explanation, not a completed HTTP dependency diagnosis.
The recovered acceptance log confirms only dashboard 503 at the old helper
assertion. No individual profile API response was captured in that run. The
grant must not be widened solely to make this test pass without establishing the
intended fixture and profile policy.

Comparison with current origin/main confirms that WebApi GET already rejected
non-success API replies and dashboard already mapped profile failures to 503.
The new `get_reply` delegation retains those semantics. No evidence in this
review points to the generic bridge change causing the observed failure.

Inventory reads household membership, medications and UI capabilities; it does
not read the profile and therefore does not share this likely failure dependency
(`web_pages.rs:665–714`). Medication detail also avoids profile, medication takes
and dashboard occurrence reads. It does share visible people, assignments and
schedules with dashboard. The rerun's explicit inventory/detail 200 assertions
remain necessary to establish their actual behaviour for this viewer.

## Household API test credential investigation at 10:50 UTC

The rerun log records both lifecycle cases and both owner/viewer inventory-detail
navigation cases GREEN. `household_web` then records five passes and five failures.
Four failures are 401 responses on bearer API reads at test lines 287, 210, 459
and 435. The fifth is the known viewer dashboard 503 in this suite's separate
sign-in helper. These results do not establish full household acceptance.

The four 401s match fixture credential invalidation after the capacity test's
first successful native person creation. `people::create` adds a manage grant and
increments the owner's `household_memberships.permissions_version`
(`people.rs:680–683`). Bearer authentication rejects an ApiSession whose recorded
permissions version differs (`lib.rs:755–756`). The capacity test and subsequent
owner API checks reuse the initially provisioned `fixture.access_token`, so they
expect a now-stale credential to remain valid. The initial creation redirects
successfully, then its read-back fails 401 and later bearer checks fail in turn.

This is a source-supported test credential explanation; no live database version
snapshot was captured by this reviewer. It is inconsistent with login throttling:
the owner helpers use distinct `198.18.24.x` client addresses, their login POSTs
return 302, and the failures occur on API reads rather than login. The API rate
limiter responds 429 to exceeded limits.

The narrow correction is to retain the signed-in browser context for owner API
read-backs, add sign-in to owner API-only cases, and preserve separate viewer,
delegate, foreign and anonymous authorization assertions. Target's cookie jar is
enabled, so `get(path, None)` can use the actual signed-in browser session and
current membership. No production permission-version rule should be weakened.
This suite's dashboard helper should use its intended inventory starting page,
retaining 200 and native CSRF assertions.

The same stale-bearer setup occurs in browser `household-workflows.test.mjs`:
`apiRows` forces `fixture.access_token` despite using the signed-in page's request
context, then first person creation changes the membership version. Cookie API
read-backs preserve the intended native-save verification. `household-routes`
already uses that cookie context for API checks. These narrow test changes were
sent to the coordinator; this reviewer made no test or production edits.

## Narrow credential test fix review at 10:51 UTC

Reviewed the coordinator's test-only cookie read-back correction. Owner API-only
cases now sign in, owner reads use the cookie context, and the anonymous 401
assertion uses a separate Target without cookies. Viewer, delegate and foreign
bearer checks remain intact; explicit Authorization still chooses that credential
instead of the owner's cookie. Browser `apiRows` now uses the signed-in page's
cookie context. These changes preserve the intended state and permission checks.

One immediate setup finding was sent to the coordinator: replacing dashboard
with the People index while still calling `csrf` cannot work. `render_people`
accepts `_csrf` but does not render it, and `household_document` has no CSRF meta
element. The helper would panic after a successful People 200 response. Inventory
is the suitable shared owner/viewer starting page: `medication_document` calls
`authenticated_document`, which emits its native `meta[name='csrf-token']`.
Retain inventory 200 and token extraction before accepting this helper correction.
The fresh API/browser rerun must use the final corrected test snapshot.

Resolved in the coordinator's subsequent source correction: the helper now GETs
inventory, requires 200 and extracts its native CSRF meta. No additional blocking
finding was identified in the final narrow test changes. Runtime assertions remain
pending in the new immutable acceptance run.

## Existing dose journey investigation at 11:00 UTC

The latest inventory-CSRF run records all fourteen household HTTP contracts GREEN.
The untruncated browser log records private household routes and the new People,
Locations and scalar Medication create/edit/error journeys GREEN at desktop and
mobile. The overall browser result is fifteen passes and six failures. All six
existing dose-journey failures timeout waiting for the exact `Log` link before
their administration, CSRF/replay, taper or focus-restoration assertions run.

Source comparison rules out a translated-label mismatch: current and origin/main
renderers both use the exact `Log` link, `#administration` target and
`data-open-administration` hook. The corresponding dialog and dose controls are
unchanged, and the native medication doses POST route remains present.

The producer/consumer mismatch is explicit. The browser `detail` adapter reads
`can_record` and `eligible_stock_medication_ids` from assignments and schedules,
defaulting to false and an empty collection when absent. The API's
`serialize_assignments` and `serialize_schedules` emit neither field. Consequently
the renderer's existing `has_recordable_source` guard suppresses the Log link.
Origin/main API serializers also omit these fields, so this is an existing
mismatch exposed by the required regression run, rather than the new locale copy.

The coordinator was notified that the correction belongs in the existing API
permission/stock projection, preserving the renderer guard. `source_context`
already fetches active record/manage grants, but retains only manage IDs; record
permission must include both levels. Eligible stock must retain the authoritative
dose-write rules: scoped visible candidates, matching name/dose/unit signature,
null or positive supply, and for members only medications linked to that source
person. Owner/administrator stock eligibility differs from member eligibility.
The unchanged mutation endpoint must remain authoritative at submission time.

No product edits were made by this reviewer. All six existing dose journeys remain
unaccepted until a corrected source snapshot completes their runtime assertions;
the household journey GREEN results do not replace those regressions.

## Dose projection candidate review at 11:08 UTC

Reviewed the frozen `read_resources.rs` correction, its seven new tests, the
coordinator's HTTP contract and optional OpenAPI fields. No blocking source
finding was identified. The API now emits `can_record` from active record/manage
grants, independently of manage permission. Viewer-only sources do not gain
recording permission or stock IDs. Stock candidates use the existing household
and medication visibility query; member results are additionally filtered by the
specific source person's assignment/schedule links. Matching name/dose/unit and
untracked or positive supply mirror the dose-write eligibility rules. Foreign,
cross-person, mismatched and exhausted candidates are excluded.

Permission, source links and stock reads are batched at the request boundary;
there are no new per-source database queries. Schedule serialization retains one
captured reference date for the collection. The renderer and authoritative dose
POST permission checks remain unchanged. The optional OpenAPI projection fields
were added by the coordinator with this repair; they were not present on
origin/main.

The implementation owner reports the three missing-field projection tests RED
before the fix, followed by thirty-six library tests and twelve auth integration
tests GREEN. Seven new pure cases cover emitted fields, record/manage/view
distinctions, owner and delegate stock filtering and empty projections. The new
HTTP case checks owner fields/stock, foreign stock exclusion and viewer recording
denial on both assignment and schedule reads. Those HTTP assertions and the old
dose browser journeys remain pending in the fresh runner snapshot. This reviewer
performed source review and diff checks rather than rerunning the owner's warm
unit suite. The coordinator subsequently confirmed the owner's clippy gate GREEN
with warnings denied. The full rebased `ci:rust-port` gate and combined HTTP/browser
acceptance are running; runtime acceptance remains pending.

Reviewed source SHA256: `read_resources.rs`
`5af40b0514a749a518414191e3b9d69569fdcfd38a6347c93ebb252da548deda`;
HTTP contract `household_web.rs`
`fdf59c5b83a6748e748db88936f1aa7015768d76bf3b36a050c1938e88e5b56f`.

At 11:09 UTC, the coordinator corrected the API crate's import ordering with its
edition-2021 `api:fmt:write` task after the full gate rejected edition-2024 ordering.
The current `read_resources.rs` SHA256 is
`d75dfcd40bf491c516a9a75fddb97ce43669f93a8083adf6eb29216677060bbe`;
the HTTP contract hash above is unchanged. The earlier digest identifies the
pre-format source review, not the final formatted file. The coordinator is
repeating the full gate. No production behaviour change is reported by that
formatting correction.

## Dose history assertion diagnosis after the projection rerun

The fresh projection run records fifteen household HTTP contracts GREEN: two
lifecycle, two navigation and eleven household-web cases. Browser results improve
to seventeen passes and four failures. The Log control is restored and both
desktop/mobile Escape focus-restoration cases pass. The remaining four tests reach
successful dose submission, the expected stock decrement and the expected history
medication-row count before failing their exact amount/unit text selector. The
untruncated evidence is `/tmp/medtracker-runner-dose-projection-browser-raw.log`.

Source review identifies a test/markup mismatch. Both current source and
origin/main render one history `<small>` containing the person name, ` · ` and
the dose together (`rust/web/src/dashboard.rs:506`); this renderer has no diff
against origin/main. There is no element with the isolated exact text `1.25 ml`
or `0.75 ml`, as the browser tests currently demand. The API dose serializer
normalises decimal strings and retains the dose unit (`dose.rs:552`), and the
dashboard adapter joins these fields (`web_pages.rs:996`). Its person and local-day
filters have already produced the expected medication row in each failed test.
There is no evidence of a new producer, normalisation or date-filter failure.

The smallest correction is confined to these assertions: retain the medication
row scope/count, inspect its `<small>` and require the exact amount/unit after the
person separator, or assert the complete person-and-dose text. Keep successful
submission, stock, CSRF, replay and taper checks intact. This reviewer made no
product or test edits. Full dose browser acceptance remains pending until the
corrected assertions complete in the fresh runner.

## History test correction source acceptance

Independently reviewed the final test-only change in both medication journey
files. The three changed assertion sites select the medication row's parent
span and its `<small>`, require visibility and match the trimmed complete text
against an anchored expression requiring a nonempty person plus exactly
` · 1.25 ml` or ` · 0.75 ml`. The fix matches the existing combined history
markup without accepting another dose value or a hidden summary.

Medication row counts, successful redirect/message, invalid-dose rejection,
CSRF failures, rejected draft preservation, fresh UUID generation, unchanged
stock/history after replay, exact stock decrements and selected-date taper
assertions remain unchanged. No product, permission or mutation code changes
are included in this correction. No blocking source finding was identified.
The coordinator confirms the repeated full `ci:rust-port` gate GREEN in session
53529. Final live browser acceptance is still pending at this report update.

The runner's first attempt with the corrected history selectors failed during
shared UI npm/Tailwind setup because `lines-and-columns` was missing. It stopped
before the HTTP/browser assertions and produced no fresh history screenshots;
owned resource cleanup succeeded. The runner suspects overlapping dependency
installation but has not confirmed the cause. This is an infrastructure/setup
failure, not an assertion result for the corrected tests. The coordinator is
serialising the shared build before retrying. The preceding fifteen HTTP passes
and seventeen browser passes remain the latest completed runtime evidence.

## Serial history run and mobile layout follow-up

After the serial runner self-test passed, the released retry completed all fifteen
HTTP cases GREEN. Browser results were twenty passes and one failure. CSRF/replay,
taper, both Escape cases, desktop decimal-dose journey and exact history dose
assertions now pass. The mobile decimal-dose case passes its visible `1.25 ml`
history assertion before failing the existing global page-width assertion at
`medication-journey.test.mjs:164`. This is an observed responsive layout RED,
not a remaining amount/unit selector problem. The desktop history screenshot was
freshly captured at 12:31 London and independently inspected: medication, person,
dose and time are readable. Mobile history was not captured after the failing
assertion; the older image cannot establish the current layout.

Source review identifies an unconstrained history flex child: its medication and
person metadata include long tokens but the span retains the automatic minimum
width and strong/small lack wrapping rules. The mobile grid's `1fr` track also
retains an automatic minimum. The raw failure does not include geometry of the
offending element, so the precise overflow source is inferred from CSS rather
than proven by a captured DOM measurement.

Independently reviewed the coordinator's minimal CSS correction. History copy
now has `min-width:0;flex:1`, strong/small use `overflow-wrap:anywhere`, and time
has `flex-shrink:0`. It reduces the row's intrinsic width without clipping or
hiding medication, dose or time. No DOM, value, permission or assertion change
is included. No blocking source finding was identified. Keep the existing global
page-width assertion and require a fresh mobile history screenshot in the next
browser run before accepting the responsive fix.

The subsequent history-layout runtime retains fifteen HTTP passes and twenty
browser passes but fails the same mobile page-width assertion. Exact dose history
still passes, and no fresh mobile history screenshot is available because capture
follows the assertion. The history-only correction is therefore source-reviewed
but does not establish the actual overflow cause or resolve the observed RED.
The next diagnostic must record element bounds and a screenshot before the
unchanged assertion. Candidate remaining contributors include the mobile grid's
automatic minimum, native select intrinsic width and stock's nowrap quantity.
These are source hypotheses, not confirmed runtime findings; avoid masking the
failure by clipping page overflow or weakening the width assertion.

## Measured mobile grid correction

The coordinator's diagnostic run records viewport width 390 versus document
scroll width 471. History and Insights grid sections measure width 455 with
automatic minimum widths; a stock descendant measures width 407 extending to
right 431. The captured mobile history is clipped. This establishes grid-wide
overflow rather than proving history text alone was responsible.

Independently reviewed the targeted follow-up: at widths up to 1023px the single
column changes from `1fr` to `minmax(0,1fr)`, and direct grid children receive
`min-width:0`. These rules remove the track/item intrinsic floors responsible for
the measured sections exceeding the viewport. The desktop two-column rule stays
unchanged; history wrapping and the non-shrinking timestamp remain. The correction
does not clip or hide content. Diagnostics and screenshot capture now precede the
unchanged strict document-width assertion. No blocking source finding was
identified; final runtime GREEN and fresh readable mobile history remain required.

For publication, the delivered People, Locations and scalar Medication scope has
no other unresolved blocking source finding. The existing between-read option
insertion draft-retention race is P2 and has no write-integrity breach. The
coordinator records that race, localisation/options, stock/assignments and broader
authentication gaps under issue #2345. A scoped PR is suitable only after the
remaining responsive regression and all applicable local gates pass. This does
not establish completion of the broader journey or authentication requirements.

## Final independent acceptance and publication disposition

The final immutable source snapshot is
`453d76d198e59dd96a22373c8953a0555feeda615951f87e33a11e2f0d5e96dc`,
project `mtcontract-1c213d1948a54acc`. Combined acceptance exited 0. The completed
HTTP log records fifteen passes: two lifecycle, two navigation and eleven
household-web cases. This reviewer independently checked the untruncated browser
log, `/tmp/medtracker-runner-grid-repair-browser-raw.log`: all twenty-one tests
pass with no failures, skips or cancellations. The strict mobile page-width
assertion remains enabled and passes. The new People, Locations and scalar
Medication desktop/mobile workflows and preserved dose, stock, history, CSRF,
replay, taper, focus and foreign-record regressions are accepted for this snapshot.

The fresh mobile history screenshot was independently inspected. Medication and
person text wrap visibly, exact `1.25 ml` summaries remain readable and timestamps
and cards stay within the 390px viewport. The runner records all sixteen images
fresh and ownership cleanup successful. Earlier failed runs remain historical
RED/setup evidence, superseded by this completed final runtime.

The coordinator confirms full `ci:rust-port` gate session 41999 passed after the
history wrapping change and before the final CSS-only grid correction. No Rust
production code changed afterward. The final combined acceptance rebuilt and
tested that grid correction. These checks have distinct source provenance; the
earlier full gate is not represented as a run against the final CSS snapshot.

No unresolved source or runtime finding now blocks publication of the bounded
People, Locations and scalar Medication PR, with ordinary publication checks
owned by the coordinator. The accepted scope includes the narrow existing dose
projection and responsive regressions needed to preserve working journeys.
Full localisation, dosage-option editing, stock/assignment workflows and broader
authentication parity remain unaccepted. The between-read option-insertion
draft-retention P2 remains recorded with those follow-ups under issue #2345.
This disposition does not mark the broader tranche requirements complete. The
reviewer changed only this report and performed no Git publication or product/
test mutations.

## Published PR GitGuardian finding disposition

At PR #2346 head `d5a3dd59ddfc0ccaa7ab6cfeab82faddc9b71bc6`, GitGuardian
Security Checks run `110357918541` completed with FAILURE and title
“3 secrets uncovered!”. Its output lists three Generic High Entropy Secret
incidents: `37785603`, `37785604` and `37785602`. The finding links point to
commit `80b9fb66aaaf1eb66ad8748f96a6ba5939bba360`,
`rust/api/tests/auth_compatibility.rs` lines 14, 21 and 15 respectively.
This is a completed scanner finding, not a scanner-availability failure.

Independent source inspection confirms the flagged literals are expected outputs
in `legacy_hmac_derivation_matches_locked_rodauth_vectors`. Both HMAC input
constants explicitly contain synthetic labels; the fixed 16/32-character seed
inputs and these constants are confined to the test file. The test compares
fixed derived outputs from the pure compatibility helper against the recorded
locked Rodauth oracle. No deployment configuration, account credential or live
provider key is used by these vectors. Classification: synthetic test-vector
false positives. No flagged value is reproduced in this report or tool output.

No tracked GitGuardian, ggshield or gitleaks ignore/configuration was found. The
repository's recorded prior GitGuardian response regenerated a disposable APNs
fixture key rather than adding scanner suppression. No authenticated GitGuardian
incident-disposition capability is available in this session. The check therefore
remains externally failed, with no suppression, test-vector change, history
rewrite or scanner bypass performed. An authorised GitGuardian account owner
must classify these exact incidents as false positives and have the integration
re-evaluate the check. Keep the PR unmerged while that external check remains
failed; the local/source acceptance above does not claim remote CI completion.
