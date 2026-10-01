# Independent household delivery review

Reviewer owns this report only. Source/tests are read-only; no build, dependency,
Docker, fixture, Git mutation or external review/comment actions were performed.
Read the delivery plan and writer/reviewer briefs. Serena initialised; narrow
Rust source reads use the established rg fallback. Existing Rust behaviour,
persistence, browser and verification guidance remains applicable.

## Separate review of current PR #2349 comments

Inspected live unresolved threads and review submission against PR head
`5a7c984a354fc7cd3ad07a21e2d3c0cf320fb17e`, then checked current local source.
The two comments are from Devin's review submitted 1 October 2026 at 15:53:20Z.
CI completion is coordinator evidence; this bounded job did not poll CI.

1. [Unknown medication errors remain untranslated](https://github.com/damacus/med-tracker/pull/2349#discussion_r4157509908).
   The observation is factual: `rust/web/src/medication_management.rs:32`
   maps recognised messages with `api_error` and otherwise clones raw text.
   The general future scenario at
   `openspec/changes/add-rust-household-medication-workflows/specs/rust-household-management/spec.md:87`
   requires safe generic unknown errors for rejected forms. Completed A was
   explicitly People/Locations only; tasks 2.3/3.3 are accepted while 4.3 remains
   unchecked, and tasks.md:60 describes the bounded evidence accurately.
   Requirements verdict: remaining medication requirement, not a regression
   invalidating A. Quality verdict: no new blocker in the accepted change;
   recommended P3 documentation clarification of accepted scope in design.md:33
   or a scoped PR explanation. Preserve the broader future requirement; do not
   weaken it merely to close the comment. D/full medication acceptance must
   address or explicitly retain this gap with actual evidence. Labels if tracked:
   `rust,bug` for the remaining medication behaviour; documentation for scope text.

2. [Known validation errors can lose detail](https://github.com/damacus/med-tracker/pull/2349#discussion_r4157510133).
   Inspected `household_i18n.rs:111`/`:128`, People/Locations validation and public
   create/update response call sites. Current form-facing field messages are
   `can't be blank`, `is invalid` and Locations' `has already been taken`; each
   has a catalogue mapping. People parser/person-valid failures already fall
   back to `person: is invalid` in `people/responses.rs:38`. No additional
   actionable exact message lost by the new adapter was demonstrated.
   Requirements and quality verdict: no actionable defect in this comment.
   Do not invent a broader raw-message allowlist or change the stable `api_error`
   Option contract without a real failing case. Existing known/pretranslated and
   five-locale unknown-error tests remain the evidence for A.

No material unresolved source finding in #2349's accepted A/B/C scope from the
current two comments. Root owns any PR replies or document clarification; this
review did not post or resolve threads. This does not accept medication task
4.3, D or subsequent journeys.

## D requirements and quality/security review

### Final D acceptance

**Requirements verdict: PASS for D and medication task 4.3 within the assigned
household journey scope. Quality/security verdict: PASS with the agreed
nonblocking P3 polish follow-up.** No material unresolved authorisation,
precondition, persistence, concurrency, privacy or new-flow mobile finding.
The earlier pending statements below describe intermediate review states.

Independently audited actual raw results: the first core's four HTTP cases pass
in `D-CORE-GREEN-001.raw.log:273–281`; its previously misnamed build-only sublog
was corrected to `D-contract-runner-build.raw.log`, with actual assertions in
`D-HTTP-4-results.txt`. Corrected `D-CORE-BROWSER-002/D-browser-20.raw.log` passes
20/20. The isolated `D-PERMISSION-003/D-permission-http-results.txt` and outer
log prove the real manage-granted ordinary-member case passes 1/1; its fresh
`D-browser-20.raw.log` also passes 20/20. These are separate disposable fixtures,
not one combined-job or same-fixture claim. Earlier post-extraction API baseline
14/14 and compiled web tests support the unchanged extraction; full final Rust
CI remains a separate coordinator gate.

Compared manifests directly: browser-002 and permission-003 copied 316-path
premanifests are identical; each equals its actual runtime copy and postmanifest
at `9fe5a02741c6822b55c2f8fb65b19dfe133752cf740e53fd7caa467e4d4e2f19`.
Each full 319-path source/Taskfile pre/post pair matches
`f861b50977624ac515f05d547b1ebe752ab2682552ccfd6b57febbecbb68ea16`.
First-core versus corrected copy differs only in the browser test pagination
helper; product and four HTTP test inputs are unchanged. Source/fixture receipts
are explicit: core fixture `0ba267d97500f31c6150fc5d996fdaeb6cf2ef3fa893c883783c56322f74781c`,
browser-002 fixture `013dcbee1baff1e5333cd2a7794f537946bc209c05aa779f0d09c7667104baa3`,
permission-003 fixture `b62e724c65daed04b616e0ba74352c4e6d2ac2209e368ccbcf1e1f260d134208`.

The twenty corrected screenshots cover form and immediate-administration cases
in all five locales at 1400px desktop and 390px mobile. Visually inspected and
hash-matched representative English desktop, Welsh desktop/mobile, Irish
desktop/mobile, Spanish mobile and Portuguese desktop forms/dialogs from the
browser-002 and permission-003 archives. Labels/errors remain readable, escaped
notes and drafts remain literal, the chosen dose is 1.25 ml, stock/source controls
are present and mobile dialogs fit the viewport. Actual tests prove focus return,
exact option/parent decrement and original-UUID dose replay without duplication.
Archive locations are `docs/screenshots/journey-medication-rust/` and
`/private/tmp/household-d-20261001/{D-CORE-BROWSER-002,D-PERMISSION-003}/generated-screenshots/`;
checksummed screenshot manifests accompany each private archive.

Acceptance preserves the existing owner/admin API policy, path-parent ownership,
current Origin/CSRF boundaries and original supplied tokens. Missing/blank tokens,
real stale writes, invalid/default conflicts and forged related/foreign POSTs
retain drafts or deny access without product/version/sync writes. First-option
confirmation and null-versus-zero stock remain truthful; standalone option-create
replay safety is not claimed. New option forms, dialogs and notices are localised
and unknown medication diagnostics now use the safe five-locale fallback.

Two nonblocking integration details remain coordinator-owned for E/shared forms:
the dosage submit action should use existing `med-primary` styling, and the
legacy background inventory heading is still literal `Household inventory`
(`web_pages/inventory.rs:168`). This acceptance covers new forms/dialogs/notices,
not every legacy detail string. It does not accept E/F/G, stock-reason audit parity,
global authentication/cutover or publication/CI disposition.

### Partial API extraction verdict

Requirements-preservation verdict: **PASS, extraction only**. Quality/security
verdict: **PASS, source inspection only**. Compiled and post-extraction runtime
proof is explicitly pending; this does not accept the browser journey.

Compared the retained published baseline at
`/private/tmp/dosage-options-D-baseline.rs`, independently verified SHA-256
`f515bde959cc66f76f1026b27f31b657cc718357ae89c5e9041d1e73aa6e4850`,
with `rust/api/src/dosage_options.rs` and its nine private child modules. All 32
top-level declarations match after visibility-only normalisation: 28 functions,
two structs, the `TransposeOption` trait and its implementation. The 21-line
original import header is identical. Pagination's Deserialize derive is retained;
no source comments existed in the baseline or were introduced in the extraction.
The widened child visibility allows the parent facade and sibling helpers to use
the moved items; the facade retains the existing root-visible exports.

Checked API route exports and sibling consumers in dose, occurrence, assignment,
schedule, pause, health, portable projection and sync-batch code. No obvious
missing export, private field or conflicting import issue found. Checked the
scoped policy/context helpers, household-before-medication lock order, reloaded
exclusive dosage lock and original optional API If-Match branch, savepoint
rollback/commit boundaries, nullable aggregate supply/baseline arithmetic, parent
scalar-dose clearing, version records, audit finish and sync change/replay
authorisation. These bodies and ordering are preserved from the baseline.

The coordinator reports 14/14 pre-extraction baseline runtime passes. This review
subsequently audited the raw pre/post-extraction receipts in
`/private/tmp/household-d-20261001/D-baseline.raw.log` and
`D-GREEN-baseline.raw.log`: each actually executes and passes 14 tests. The latter
includes permission, atomic parent/sync, invalid/default rollback and same-ETag
concurrent-update cases. Expected/actual post-extraction copy manifests compare
equal, with hashes `33536065ab758e95baea99c9947f643573f61e5fe27a0158f175ad3925a30b49`
(copy) and `ccb2bdd6f8fa305e5aee5e2ea9a15103bf6df5885ad1dd9564e76dbf3bc52e7d`
(314-path premanifest), matching the verifier's queue. Pre-extraction source
pre/post manifests also compare equal. These are earlier captured inputs;
they do not certify the later strengthened D test additions or final journey.
No unstable UI source was inspected as part of the partial extraction verdict.

### Early journey test-design review

Early test-design review completed; production implementation and actual runtime
acceptance remain pending. Inspected the four current public-HTTP cases in
`rust/contract-tests/tests/household_dosage_options.rs`. Initial findings sent to
the writer concerned invalid forwarded client addresses, missing whitespace-only
tokens and the absence of a genuinely changed record behind a captured token.
The writer corrected these with reserved `198.18` addresses, whitespace coverage
and a real distinct bearer API update before the stale browser submission. The
test now checks the original captured token and full supplied draft. Arbitrary
option amount precision is valid; the invalid precision stimulus is constrained
minimum-hours `0.55`, whose API storage limit has scale 1. First-option transition
confirmation and duplicate-default draft retention are also covered statically.
No material harness blocker found in this bounded recheck. This is not a RED,
GREEN, permission or persistence acceptance claim. Related/foreign POST denial,
permission boundaries, aggregate/audit/sync, repeat submissions and actual locale/
desktop/mobile evidence remain to be assessed at full handoff.

Review original If-Match, reauthorisation, exact decimal
drafts, parent/related-ID ownership, nullable stock, defaults, repeat submissions,
audit/sync and safe localised errors. A module extraction must preserve existing
contracts and comments. Static approval alone cannot accept the journey: inspect
matching-source actual API/browser receipts and representative desktop/mobile
screenshots before separate final requirements and quality/security verdicts.

### First stable browser implementation review

Requirements verdict: **PENDING** actual full journey evidence. Quality/security
verdict: **no material source defect found in this bounded first review**;
compiled/runtime and regression proof remain pending.

Inspected the stable adapter `web_pages/dosage_options.rs`, typed form/list
renderer, medication entry links, shared error helpers and localised dose dialog.
The context uses real authenticated household/medication reads and the existing
medication-update capability. The list hides add/edit controls when false;
new/edit and POST enforce it, while the real dosage API reauthorises every write
as owner/administrator. An option GET must match the route's scoped medication
before edit or POST. Submitted related IDs cannot replace the path parent.
Origin and current CSRF checks precede writes. Missing/whitespace edit tokens
render 428 with the draft; nonblank tokens reach PATCH unchanged, retaining the
API's fenced conflict check. 422 and 409 render original fields/token, not a
freshly adopted version. Nullable stock fields remain strings or JSON null;
zero remains tracked empty stock. Native first-option confirmation and truthful
repeat-add guidance preserve the API's existing non-idempotent create boundary.

The shared medication error helper now uses safe translated `form_error` for
unknown diagnostics, with explicit conflict/confirmation/reload messages. Native
form labels, error references, select/checkbox state and escaped textarea content
are present. Five locale/desktop/mobile browser cases statically cover real option
CRUD, an invalid precision draft, keyboard focus and overflow checks. They do not
yet demonstrate a JS-enabled dose dialog or immediate use of the created option.

Remaining evidence was sent to the same writer: foreign/mismatched-parent forged
POST and ordinary-member permissions; rejected-write parent/aggregate/audit/sync
proof; repeated POST semantics; and immediate option dose/stock use with the
actual interactive localised dialog. Matching-source GREEN receipts and
representative screenshots are necessary before accepting D or medication 4.3.
The API extraction verdict above remains separate from this pending journey.

### Strengthened final test-design review

Static test-design verdict: **PASS**; matching final runtime/visual evidence is
still pending. Rechecked the four dosage HTTP cases, isolated one-case permission
file and all twenty browser cases. The HTTP additions use only read-only
disposable-database queries to link the option create version with exactly one
parent version and two sync changes under its request ID. Rejected missing,
blank, whitespace and real stale tokens preserve original drafts, parent/option
records and version/sync counts; invalid precision/CSRF and forged same-household
or foreign-parent POSTs likewise leave state unchanged. Repeating a successful
edit with its old token gives 409 without another version/sync change. Adult and
child default conflicts retain the submitted draft and parent state.

The separate grant-mutating case must remain last in an isolated disposable
fixture. It uses a real owner grant and fresh member login, proves the member
role and manage-person capability, then distinguishes medication-create=true
from medication-update=false. Dosage add/edit controls are absent, add/edit GET
and forged create/update POST return 403, and parent/options stay unchanged.
This supports the Rails policy source ruling below; no legacy token refresh or
policy reset was added.

Ten new JS-enabled browser cases create a tracked option through the actual
browser, assign its snapshot through the real API, then open the real source and
dose dialogs in all five locales at both viewports. They check translated labels,
selected source/stock, 1.25 ml display, Escape focus return, viewport width, exact
parent and option stock reduction from 12.25 to 11.0, and one medication take.
Reposting the original dose UUID/time gives 303 with unchanged option stock and
one take. This is dose replay evidence, not a standalone option-create idempotency
claim. No blocking harness defect found. Final screenshots and raw result counts
remain required before journey acceptance.

### First core runtime failure classification

D-CORE-GREEN-001's verifier reports copied source digest `6ffb4e19...` with all
316 runtime-input paths matching its premanifest. The HTTP four cases pass. This
review independently read the raw browser receipt at
`/Users/damacus/Library/Application Support/rtk/tee/1790874466_task_api_0b3d04.log`:
34 tests ran, 26 passed, eight failed. All ten native dosage CRUD cases and both
English immediate-administration cases passed, as did the fourteen existing
household browser checks. The eight non-English administration cases failed only
at `assert.ok(option)` after their successful browser save. No complete D or
non-English dose-dialog acceptance is claimed from this run; the isolated
permission job remains held.

Source-derived diagnosis sent to writer/root: the test's bare `/dosage_options`
read returns only the first page's data and discards metadata. The API defaults
to twenty rows in ascending ID order (`dosage_options/reads.rs:22–47`), while the
product adapter uses the paginated `WebApi.collection` (`api_client.rs:177`) and
rechecks a saved option by member GET before issuing success 303. Accumulated
fixture, HTTP and CRUD records crossing the first page therefore explains the
late-case lookup failures. This is a strong harness diagnosis, not direct
runtime inspection of the failed response metadata. Correct collection read-back
pagination with strict total-count coverage, retaining all locale, ownership,
stock, dialog and replay assertions; medication-take collection lookups need the
same completeness. The cleaned immutable browser source was unavailable at this
review; the raw stack's captured line 153 is not presented as the current
formatted line 169. Matching corrected-input rerun remains required.

The verifier's later durable receipt explicitly distinguishes the executed
immutable copy from one subsequent live browser-test change. Captured test hash
was `45ccbc5b830610a97ffc30a2ea5ac3574d300384b56eb3a90e34067654f53406`;
the live post-run hash was `47f7b91b25703c44ad8974c4006fd471581626ec214d619fa58f46ca6e755af5`.
The matched copy/pre-manifest remains valid evidence for the earlier input,
not for that later test. Fixture SHA was
`0ba267d97500f31c6150fc5d996fdaeb6cf2ef3fa893c883783c56322f74781c`.
Twelve feature screenshots exist, covering ten forms and two English dialogs;
the eight missing translated-dialog screenshots cannot be inferred as passes.

The writer's bounded test-helper correction has now passed static review:
member replies retain their data; incomplete arrays require integer total-count
metadata and fetch 100-row pages while preserving existing URL query values.
It fails on an empty page before total coverage or at the existing five-page /
500-record bound. Option, parent, locale, dialog and UUID assertions are retained.
This changes test collection completeness, not product/API page limits. Actual
corrected-input rerun remains pending.

Partial visual review of the checksummed first-core archive inspected English
desktop dosage-error form, Welsh mobile dosage-error form and English mobile
administration dialog. Fields/errors are readable, submitted drafts and literal
escaped textarea text remain visible, Welsh labels are localised and the dose
dialog fits the 390px viewport. This is limited to those produced images; the
eight untranslated-run failures have no modal screenshot acceptance.
Nonblocking P3 pattern/polish finding sent to writer/root:
`rust/web/src/dosage_options.rs:234` omits the existing `med-primary` class on the
submit button, producing a small browser-default desktop action instead of the
established primary treatment. Mobile CSS still provides a 44px minimum target.
It is not a permission, persistence or mobile-blocking defect.

### Bounded ordinary-member dosage permission compatibility

Source parity verdict: **PASS for ordinary household members with a current
person manage grant**. Such a grant does not permit dosage-option create/update
in Rails. This is the existing medication-manager boundary, not a new Rust-only
restriction introduced by D. No product permission change is recommended.

`app/policies/medication_dosage_option_policy.rb:3` inherits `DosagePolicy`.
`dosage_policy.rb:5` delegates dosage create to `medication_policy.update?`, and
`:7` delegates update to that same policy. `medication_policy.rb:25` requires
household-manager authority and the same household; it has no person-grant
alternative. The API controller calls `authorize dosage_option` before create
and update in `app/controllers/api/v1/dosage_options_controller.rb:21` and `:30`.
Rust's extracted `dosage_options/context.rs:11` uses the existing
`medication_management/context.rs:39` owner/administrator rule.

Do not confuse dosage create with medication create: the latter can be allowed
for a manage-granted member, while dosage create deliberately follows medication
**update** authority. Adjacent source tests are
`spec/policies/dosage_policy_spec.rb:33` (owner dosage writes and ordinary member
update denial) and `spec/policies/medication_policy_spec.rb:43` (a manage grant
permits medication creation). They are source evidence here, not freshly executed
exact manage-granted dosage request receipts; D's real member-denial coverage
remains required. Rails `policy_helpers.rb:36` also recognises an active platform
support session as a manager. That exceptional staff path is outside this bounded
ordinary-member check and does not accept global authentication/cutover parity.

## Bounded baseline parity recommendations

The separately assigned [parity-rulings.md](parity-rulings.md) records only the
three #2347 disagreements. Source-derived recommendations are Rails-compatible
People oversized-integer pagination clamping; location-name then medication-ID
stock selection order; and matching inventory retained for paused origins while
actual paused dose submission is separately rejected without writes. The
coordinator owns final rulings and matching runtime acceptance. No other #2347
baseline failure was assessed in this job.

## E/F and combined acceptance

Pending sequential authorised handoff. Preserve real API semantics for order
receipt and stock adjustment, all seven schedule types and effective taper/date/
timezone behaviour. No broader checklist or cutover acceptance is implied.

The separately assigned [stock-ruling.md](stock-ruling.md) records parent-only
absolute adjustment versus supported per-option PATCH aggregation, concurrency
and audit semantics. It also identifies the existing Rust adjustment-reason audit
parity gap for coordinator follow-up (`rust`, `bug`). This readiness source review
does not accept E or implement a new endpoint.
