# Household completion execution packet — 1 October 2026

## Starting point and assessment

At the start, the household programme had 12/20 OpenSpec tasks complete. PR #2346
contains the accepted initial household slice and maintainability refactor at
`eccf62aace3aab81bf871b380c964ff0ee28e7b6`. Its acceptance is not programme completion.
Issue #2345 records remaining journey gaps; #2347 records baseline-only audit
failures (56/16, 7/2 and 21/2). Keep those RED until resolved on their merits.

Run starts 14:44 BST (13:44 UTC); assess at 16:44 BST (15:44 UTC).
Native two-hour assessment stops new feature dispatch, then finishes verification
and owned cleanup. Waypoints: 15 minutes, 45 minutes, 90 minutes, 120 minutes.

Assessment delivered at 16:44 BST: 14/20 requirements accepted after full
five-locale People and Locations journeys. Bounded A/B/C review passed with
28 HTTP and 70 browser cases on identical captured source across three fixtures.
No D/E/F work was dispatched. Final documentation and dependent publication
close this run; a further feature tranche requires a new bounded instruction.
Starting resources: 156 GiB available, 21% weekly allowance, three reset credits.
Reset use requires explicit per-use confirmation. No exposed 1.5x setting exists.

Use a dedicated continuation branch based on the accepted PR head; publish as a
lower-first dependent PR, preserving the existing PR and unrelated Rust skill,
stashes and other worktrees. Coordinator alone changes Git or ownership.

## Team and ownership

The existing team charter explicitly permits parallel disjoint product/test
writers. It overrides generic single-writer skill guidance. Up to eight seats:

| Owner | Model / effort | Concrete scope |
| --- | --- | --- |
| Coordinator | Current Sol 6.1; Medium normally, High for consequential decisions | Plans, Git, integration, public OpenAPI, manifests/locks, routing/module wiring, Taskfiles and acceptance |
| Localisation/forms | Sol 6.1 Medium | A: locale adapter, five catalogues, People/Locations renderers |
| Medication | Sol 6.1 High | B: stale medication browser submission; later D |
| Dashboard | Sol 6.1 High initially | C: dashboard handler and pure projections |
| Behaviour tests | Sol 6.1 High for deterministic interleavings; Medium otherwise | Disjoint named new contract/browser tests and agreed fixture changes |
| Build/verification | Luna Medium | Sole compiled/runtime lane, queue, snapshots, logs, screenshots and owned cleanup |
| Independent review | Sol 6.1 High | Read-only requirements and quality/security reviews; report only |
| Scout | Sol 6.1 Medium | Bounded read-only D/E/F and baseline-audit discovery; report only, retire when useful |

Every owner has a brief/report pair in `next-slices/`. Never edit another owner's
files. Material findings return to the original owner. Coordinator resolves
interfaces and priority; do not silently inherit every implementation fix.

## First run: independent completion slices A/B/C

A completes People/Locations localisation and forms: authorised list/detail/add/
edit, success and invalid drafts in en/cy/ga/es/pt, retained values, associated
useful field errors, escaping, keyboard access and mobile width. Unknown API
errors use a translated safe generic message, never unexplained English or
invented clinical advice. Do not weaken validation. Independent review and real
UI evidence precede checking People/Locations acceptance boxes. Medication save
button styling needs an explicit tiny path transfer from its renderer owner.

B makes scalar medication edits safe when the first dosage option is inserted
before the medication read or between medication/options reads. Deterministic
RED precedes product code. Reject stale forms with 409, retain every draft value
(including decimals/warnings), no persisted write. Cover missing/blank ETag 428,
stale 409, malicious scalar override denial and medication/options/stock read-back.
Re-read/check parent version if needed, but forward ORIGINAL submitted If-Match
to the final API mutation; never silently adopt the current version. Retain locks,
idempotency and real session/CSRF/permission checks.

C makes a valid active limited-view member dashboard usable without granting
own-person access. Reproduce 503 with the realistic fixture first. Distinguish
optional personal preferences from required authorised data; no policy shortcut
or fixture grant to manufacture a pass. Cover applicable owner/admin/self/carer/
parent, limited-view, inactive/revoked, anonymous and foreign-household cases.
No hidden person/stock/history or mutation control leaks. Profile endpoint changes
require coordinator security approval. Preserve dose/history/mobile regressions.

The test writer queues actual RED against unchanged product, then releases that
behaviour to its product owner. Product discovery/design continues meanwhile.
Tests keep real HTTP persistence and browser assertions; never weaken selectors,
width, stock, dose, replay or permission checks.

## Ordered subsequent slices

D finishes medication dosage-option add/edit/management: positive decimal values,
units, parent version updates, permissions, stock safety and identity/scalar modes.
Prefer focused private dosage-options adapter/renderer modules. Coordinator wires
routes; A supplies catalogue keys. Prove immediate dose recording, invalid/stale
forms, multiple options, replay and tracked/untracked handling. Accept medication
management before implementing stock.

E delivers stock adjustment/removal with reason, ordered and received states,
visible quantity/status refresh. Scout verifies Rails/API semantics before forms.
Cover exact decimals, zero/negative policy, role/resource access, audit trail,
repeated/concurrent writes, partial receipt and immediate read-back. Preserve
existing dose consumption. Route existence alone is not acceptance.

F first accepts direct assignment add/edit/remove with person/medicine visibility,
exact dose and usable dashboard/dose result. Then deliver scheduled assignments
in schedule-family slices until all seven REAL Rails/contract types pass, including
taper, effective dates, overlap, timezone/DST and pause/resume where applicable.
One product owner retains coupled editor/backend semantics. Do not invent types
or mark assignments complete after routine schedules alone.

G re-evaluates the entire original checklist, five locales, dose/history/stock,
authorisation, concurrency, private caches and coexistence. Authentication parity
retains its separate programme; synthetic OTP helpers do not constitute login.
Finder, reviews, reports and administration remain subsequent scoped work.

## Exclusive build lane and visible queue

Only Luna invokes dependency installation, UI/Tailwind/WASM builds, Cargo build/
check/test/Clippy, Docker builds, runner self-tests, fixture/bootstrap and HTTP/
browser acceptance, including all dependent Task commands. Coordinator submits
jobs rather than running gates. Product/test writing, read-only discovery/review,
docs lookup and narrow formatting/static syntax work can proceed without mutating
shared outputs. Luna alone measures caches/disk and cleans owned resources.

Queue: `next-slices/build-queue.md`, sole Luna writer. Requests arrive by agent
message and contain job ID, requester, slice/reason, approved HEAD + digest and
file manifest including new source, exact Tasks/selectors, disposable fixture/
subnet, expected outcome/log paths, blocking status and Sol failure owner.
Record submitted/start/end time, state (queued/running/blocked/passed/expected-RED/
failed/superseded), digest, exits/results/evidence, failure owner and priority reason.
FIFO unless coordinator records bounded acceptance/publication priority. Combine
only identical snapshot/commands/fixture-state/evidence requests. Preserve stale
jobs as superseded after confirmation. Expected RED and bootstrap success are
not acceptance. Mutable mutation fixtures must be fresh or uniquely identified.

Before snapshot capture coordinator freezes relevant paths. Luna records and
validates the existing immutable snapshot manifest/digest before releasing writers.
Evidence certifies that snapshot; change during execution invalidates it. Use
focused checks first, then one combined stable acceptance/publication gate.
One running job at a time; no concurrent npm/build directory activity.

Unexpected failure report includes exact command, assertion/HTTP response,
snapshot and likely infrastructure, fixture, product or baseline category. Luna
never patches product/tests or guesses policy; Sol diagnoses. Retry mechanically
only with evidence. Two failed evidence-led fixes trigger coordinator diagnosis.
UI failure evidence includes URL, screenshot, viewport/page width, element bounds/
accessible name and failed response before teardown. Inspect raw RTK output promptly.
Fresh screenshot names carry job ID/digest and desktop/mobile viewport.

Existing gates: `task ci:rust-port`, `task api:household-browser-syntax`,
`task api:browser-rust` with explicit BROWSER_TEST_FILES and HOUSEHOLD_ACCEPTANCE,
`task api:openapi-stock-workflows-acceptance`, `task api:openapi-dosages-acceptance`,
`task api:openapi-person-medication-writes-acceptance`,
`task api:openapi-schedule-writes-acceptance`. Inspect task requirements first.
Coordinator owns any scoped Task selectors/wiring. Check locale structure across
all five catalogues, docs build and diff whitespace. Rails preflight/suite only
if Rails executable code changes. Scout classifies #2347 without changing policy
or old assertions. Use checked disposable subnets and ownership-checked cleanup.
No global prune, blanket retained databases/images or silent deletion after the
disk incident. BuildKit cache survives ordinary image cleanup; measure reuse.
Log disk start/end and build waits; threatening growth stops new builds.

## Waypoints and completion

T+0 confirms concrete paths, freeze rules, owners, no second build, headroom and
first RED queue. T+15 reports active writers/review/run, queue length/current job
and exact dependency/blocker. T+45 seeks the first independently accepted outcome,
review rework and queue/build wait; dispatch D only after its preceding criteria.
T+90 freezes the nearest complete slice and stops speculative writing that delays
acceptance. T+120 stops NEW feature dispatch, reports implemented/reviewed/verified/
published separately, publishes accepted work and finishes in-flight cleanup.
Assess the next bounded run against the still-open programme.

Done requires RED→GREEN, real persistence/UI evidence, relevant locale coverage,
independent requirements AND quality verdicts, resolved material findings and
coordinator commit/push with honest PR/issue evidence. Keep denominators as actual
OpenSpec requirements; queue jobs/lines/agent counts are not completion.
Keep final broad review and verified Dan author/committer/signing identity.
No reset redemption, scanner bypass, weakened assertions, merge, deployment,
production migration or Rails cutover is authorised. Rails remains operational.
