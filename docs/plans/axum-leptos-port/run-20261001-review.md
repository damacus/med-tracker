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
