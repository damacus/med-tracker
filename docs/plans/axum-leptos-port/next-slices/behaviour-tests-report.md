# behaviour-tests report

Status: A focused renderer RED/GREEN, both browser-context tooling REDs, B's
428 draft and corrected race REDs, and corrected C dashboard RED are recorded.
ABC-GREEN-003 passed all 28 HTTP cases. The separate corrected browser run
ABC-BROWSER-GREEN-004 passed all 35 cases and produced 40 desktop/mobile
screenshots. Both jobs used matching source hashes, with different disposable
fixtures. The existing fixed-clock dashboard browser regression remains queued
separately. Luna owns runtime execution and final source/evidence gates.

Baseline: eccf62aace3aab81bf871b380c964ff0ee28e7b6.

## Prepared sources

- `rust/web/tests/household_completion_i18n.rs`: unknown People/Locations field
  and base errors use the agreed safe generic message in all five locales;
  known English and pretranslated blank errors retain their translations.
  Draft values, escaping, document language and native error association remain
  asserted.
- `rust/contract-tests/tests/household_completion_dashboard.rs`: realistic
  active limited-view member signs in with the existing password/CSRF flow,
  retains profile 403 and the sole managed-person view grant, and expects a
  usable private dashboard without hidden people or mutation controls.
- `rust/contract-tests/tests/household_completion_medication.rs`: real HTTP
  first-option insertion before the medication read and between medication and
  options reads; stale scalar draft 409, exact retained values, unchanged
  medication/option/supply and stock-removal read-back. Missing/empty/whitespace
  ETag 428 and malicious scalar override denial are separate tests.
- `rust/web/tests/household-completion-locales.test.mjs`: People/Locations
  create/edit/detail and invalid persisted read-back in en/cy/ga/es/pt at
  desktop/mobile sizes, exact labels, native keyboard use, associated localised
  errors, escaped drafts, width and screenshot capture.
- `rust/web/tests/household-completion-dashboard.test.mjs`: desktop/mobile
  limited-view dashboard, profile 403, authorised data, hidden people/medicine
  markers, disabled clinical actions, no-store, hidden selection/foreign
  household/anonymous denial and required people-read failure 503.
- `rust/web/tests/household-completion-medication.test.mjs`: desktop/mobile
  stale scalar form after first-option insertion, exact retained description,
  barcode, warnings, decimals, selections and original ETag, with real
  medication/option/supply and stock-removal read-back.

Coordinator transferred only the required runner test paths:
`rust/contract-tests/runner_compose_test.fish`,
`rust/contract-tests/test_support/rtk` and the new
`rust/contract-tests/browser_context_test.fish`. They assert the inherited
captured browser path, existing copied Dockerfile/selected browser tests, actual
merged Compose contexts for unset/empty/default/captured input, and unchanged
Rails context. Existing runner sequence and cleanup assertions remain. Only
narrow Fish syntax parsing and whitespace checks ran here; Luna owns execution.
After the runner added a real fixture hash, the mock gained an approved minimal
`test:exec` provision stub. It requires the generated run-directory pattern and
matching project-owner sentinel, then writes probe JSON before the hash. It adds
no trace marker and invokes no Rails process. The preceding context RED remains
valid; the later missing-fixture result is a separate setup failure.

Coordinator also approved `rust/contract-tests/household_selector_test.fish`.
It dry-renders the actual household Cargo Task and compares exact target/filter
arguments after whitespace normalisation: unset/false preserve the original
three targets, true adds both completion targets, and explicit file/filter
selection takes precedence. Only Fish syntax and whitespace checks have run;
Luna must establish its RED before coordinator changes the production selector.
Its first Luna attempt stopped in the unset control because the root Taskfile's
`silent: true` suppressed dry-run command output. Luna's separate
`evidence/TOOLING-SELECTOR-render.log` proved `--dry --verbose` emits the exact
command. The bounded correction adds verbose output while preserving dry-run
and strict target/filter assertions; the first result remains a setup failure.
The corrected `evidence/TOOLING-SELECTOR-RED-final.log` reached the intended true
case: the original three targets were rendered without both completion targets.
`evidence/TOOLING-SELECTOR-GREEN.log` subsequently passed all four cases. Earlier
tooling runs with Taskfile manifest drift remain historical; the coordinator
requires a final serial stable-input rerun.

## Deterministic B boundary

Coordinator approved a disposable audit trigger gate with no product seam.
The test reads the unique browser session reference from the preceding edit GET.
Its trigger matches the actor, household, browser session, medication show GET
and 200 audit. Medication show computes its body/ETag before that audit insertion
and commits after it, so the gate fixes the gap before dosage-option discovery.
After the first runtime exposed an audit foreign-key locking deadlock, the
coordinator approved moving the fixture gate to BEFORE INSERT. Body and ETag
remain computed before this boundary; household FK and existing audit-ledger
AFTER triggers have not run when the advisory gate blocks.
A separate bearer session inserts and reads back the first option through the
real API. `pg_locks` must show the blocked advisory lock within a bounded five
seconds; no timing sleep establishes the interleaving. Teardown releases the
owned lock and removes the unique trigger/function even on assertion failure.

Static review corrected selected native control assertions and added description,
barcode and changed-parent-ETag proof. Coordinator explicitly approved retaining
drafts on missing/blank 428 as well as stale 409. The submitted ETag remains exact;
tests never adopt the new version.

## Exact runtime requests

Initial C selector:
`--test household_completion_dashboard active_limited_view_member_without_own_person_access_can_use_dashboard -- --exact --test-threads=1`.

Initial A selector: `--test household_completion_i18n`.

B-RED-001 is queued for all four cases in
`--test household_completion_medication -- --test-threads=1`.
The initial single-case proposal was superseded by this explicit coordinator job.

Coordinator owns Task selectors. Existing household runner requires disposable
fixture/bootstrap and `CONTRACT_AUDIT_DATABASE_URL`; no original fixture/helper
file was changed. A/C first RED jobs were approved for snapshot capture. All test
source writes were frozen at the coordinator's capture request.

No compiled/runtime/dependency/Docker command was run by this writer.

## Recorded focused results

Luna's A-RED-001 ran `rtk proxy task -d rust/web test
TEST_FILE=household_completion_i18n` against unchanged product code. The unknown
diagnostic test failed at line 48 because raw API text appeared; the known and
pretranslated test passed. Its 34-path pre/post digest was
`fc39f2f4f22b42118a7b29741c7ae9845038f1de45ab04c1bba8a87c7adbef67`.
Evidence: `evidence/A-RED-001.log`; Task exit 201, Cargo exit 101.

A-GREEN-001 passed the same two new tests and the eleven existing
`household_i18n` tests using the two exact focused Tasks. Its 37-path pre/post
digest was `efcfa6653edb78f425a5e0f7f2693c5aab324764fbd448d88041a61f367aad02`.
Evidence: `evidence/A-GREEN-001-completion.log` and
`evidence/A-GREEN-001-regression.log`; both Tasks exited 0. This proves renderer
behaviour and the existing locale regressions; browser journeys remain pending.

TOOLING-RED-001's first attempt stopped at crates.io DNS before assertions. The
approved retry of `rtk task api:contract-runner-test` reached the intended
`Rust browser build did not inherit its captured source context` assertion.
Its frozen digest was
`bb694f4c0cf2239778d5120ea124886fce43b10c7a582e728eb77789fafde814`;
`evidence/TOOLING-RED-001-retry.log` records Task exit 201.

TOOLING-RED-002 ran `rtk task api:contract-browser-context-test`, with no
containers, and reached `Rust browser context differs in captured: .../rust/web`.
Its pre/post digest was
`4490c092412cfb2e680e0fbf569fbfe9ff44cc62038151467497abb61af52802`;
`evidence/TOOLING-RED-002.log` records Task exit 201. Both tooling results are
expected context REDs, not application failures or passes.

## B-RED-001 mixed result

The pre-read stale-form and current identity malicious-scalar controls passed.
The between-read race reached its advisory gate, then browser and option-create
requests timed out. Source inspection identified the fixture's AFTER INSERT
audit household FK lock conflicting with the option write's household lock;
this race result is setup failure, not product RED. The approved BEFORE INSERT
correction is statically reviewed and formatted, retaining exact scope, positive
lock proof, actual insertion/read-back and teardown.

The missing-token case read back the unchanged medication and checked status
428 before failing its exact native draft-value assertion. That independent
failure is a genuine draft-retention RED. It stopped on the missing-token
iteration; empty and whitespace tokens remain unexecuted in this run.
`evidence/B-RED-001-http.log` preserves the actual HTTP result section from
RTK tee `1790864926_task_api_browser-rust.log`. The similarly named
`1790864905_task_api_edaf7e.log` contains build output only. The durable extract's
first line is that nested build-output pointer, not the HTTP evidence origin.
The input helper now reports the missing field and response length without
printing CSRF or draft HTML.

## C-RED-001 setup failure

Luna verified the actual copied input at `tmp/contract-tests/run.moUPe9/source`,
digest `422566e6fd3c795720472b87754c007e9b1b66673068643f506306822cc4e4c9`.
The test established `/me` 200, real viewer login, profile 403 and authorised
people 200. It then failed at initial source line 39: the primary reference
token received the correct hidden-person 404. The dashboard request had not
executed, so this result is a test setup failure, not dashboard RED.
Raw evidence: `/tmp/medtracker-C-RED-001.log`.

Coordinator approved the exact correction to the existing `feed_access_token`,
which already has an explicit hidden-person grant. The reference still must
return 200; the actual viewer must return hidden-person 404. Main profile 403,
dashboard 200 and privacy assertions remain. Browser hidden markers use this
same existing permitted reference. No grants, accounts or policies changed.
The corrected C result follows below; no GREEN or acceptance is claimed here.

## Corrected B/C product REDs

B-RED-002's focused between-read test returned 400 where 409 was required,
after the BEFORE INSERT gate allowed the real first-option creation and read-back.
Its source-copy digest was
`79c04b1cf2a291ddfd6ec434d73b06c0fcde7ef3adf5d018ee759ebadb69f0a1`,
project `mtcontract-6de4bff2c6ab463a`, fixture digest
`47f32fc5fc42246c61cb81faf7aca73cca2722251ef99db5ea4cf61201bf93f2`.
`evidence/B-RED-002-full.log` records the actual assertion at line 280 and Task
exit 201. This is genuine race RED; the coordinator released the bounded fix.

C-RED-002 reached the intended limited-view dashboard response: 503 where 200
was required, with reference lookup and real authentication now established.
Its source-copy digest was
`7034ac78e59d9cdadf221fa6396c0ab179daf9f07b09c31f1979f3780bed3838`,
project `mtcontract-39a668d2b4294294`, fixture digest
`b46e575cd41915f5c1fd91fe60f7e7fda2404ff786cfda25a60ed441b4116427`.
`evidence/C-RED-002-full.log` records line 88 and Task exit 201. This is genuine
dashboard RED; the coordinator released the optional-profile browser fallback.

## Applicable C roles

Coordinator approved four additional HTML cases in private nested
`tests/household_completion_dashboard/role_matrix.rs`, explicitly included by
the existing C target. Existing legitimate actors establish owner plus its
authorised self profile, administrator without person grants, and member with
a manage/carer grant. Household role and person relationship remain distinct.
Each checks real login, profile status, exact API-to-HTML choices, privacy,
no-store and household administration presence.

The parent case uses real owner API creation of a minor with no capacity,
public manage/parent access grant to the existing delegated member, medication
and direct as-needed assignment. It signs in freshly after the permissions
change, checks the actual selected child's source/stock and no-write read-backs,
and retains the original viewer's forbidden profile and child access. Public
grant revocation uses its exact created resource ID and non-panicking RAII;
the existing public grant endpoint has no ETag/If-Match contract. Read-backs use
the real collection and strict created-ID selection because no item GET exists.
Normal cleanup must return 204 and persist revoked state, then a fresh member login must lose
child selection with 404. Owned new records exist only in the disposable fixture.
These matrix cases are formatted and independently reviewed. All nine C cases,
including the four applicable role cases, passed in ABC-GREEN-003 below.

## ABC-GREEN-001 setup failure

Luna verified the immutable source at `tmp/contract-tests/run.CM7dFN/source`,
digest `d14ac174b2f5149fc63a1de037edad76e405bc27550332c2c105192e8d9a9c89`,
with 301 copied inputs matching the frozen source. The C target passed eight
of nine cases, including the existing owner/self, administrator and carer cases.
The parent case authenticated its delegated member and care owner, then failed
at its first CSRF setup HTML request: `/households/{slug}/inventory` returned
404 where 200 was required. No child or parent grant had yet been created.
Browser acceptance was not reached; this result is a test setup failure and
does not establish final A/B/C acceptance.

The Rust router exposes the inventory page at `/households/{slug}/medications`.
Coordinator approved correcting that URL in only the three new CSRF helpers:
the C role module, B current-bearer module and B browser writer setup. Required
200 responses, actual meta CSRF, authentication, permissions, draft and clinical
read-back assertions remain unchanged. Both nested Rust modules were narrowly
formatted. Independent review recovered the prior captured hashes by reversing
only those URL and wrapping changes, confirming no other source edits.
Corrected code is frozen for Luna's next immutable acceptance job.

Evidence: `evidence/ABC-GREEN-001.log` and full RTK tee
`1790867852_task_api_browser-rust.log` (durable full copy owned by Luna).

## ABC-GREEN-002 fixture ordering failure

Luna validated the next immutable copy at `tmp/contract-tests/run.8W8vcT/source`,
digest `a73a37679986879e9db631828957e1b1018ba48242add4b73a9df653e335c618`.
All nine dashboard cases and all four medication cases passed, including the
public parent grant, correct revoke/denial behaviour, deterministic between-read
race and missing/empty/whitespace draft retention. The original lifecycle and
navigation targets also passed. Browser acceptance was not reached.

The later original `household_web` capability case received delegated bearer
401 at line 278. The parent test had legitimately created and revoked a grant
for that member, invalidating the original seeded bearer through permission
version changes. Restored visibility does not revive the old credential. This
is shared-fixture execution ordering, not an authorization failure or reason
to reset permission versions or weaken the original assertions.

Coordinator approved a narrow selector regression requiring two ordered Cargo
commands for combined acceptance: the original three targets plus medication
first, then dashboard alone on the same immutable fixture. Default and explicit
single-target commands retain their exact existing selection. The real dry-run
test now checks exact command count, ordered targets and filters; Luna must
establish RED before coordinator changes the Task. The browser sources do not
use the delegated seeded bearer. Evidence: `evidence/ABC-GREEN-002-full.log`.

## Final HTTP and separate browser acceptance

ABC-GREEN-003 invoked `rtk proxy task api:browser-rust` with
`HOUSEHOLD_ACCEPTANCE=true`, `HOUSEHOLD_COMPLETION_ACCEPTANCE=true` and
`HOUSEHOLD_TEST_FILE`/`HOUSEHOLD_TEST_FILTER` unset. The ordered HTTP commands
passed all 28 cases: medication 4, lifecycle 2, navigation 2, household web 11
and dashboard 9. Its immutable copy at `tmp/contract-tests/run.Tf8ykg/source`
had SHA-256
`52eca56e5adfd378c891d042055fa840be8d21e90eba8c1379bbc970b3d25b32`;
its fixture SHA-256 was
`3682d6bd8af84185d7d7cbdfcc8ea708a8d086d472aea8c16a7c913c03768e50`.
Evidence: `evidence/ABC-GREEN-003-full.log`.

That job did not pass browser acceptance: `BROWSER_TEST_FILES` was incorrectly
comma-joined, so Node treated all seven paths as one filename and reported
`Could not find ...`. No browser assertions ran and the Task exited 201. This
invocation failure is separate from the successful HTTP result.

ABC-BROWSER-GREEN-004 reran `rtk proxy task api:browser-rust` with both household
acceptance flags and file/filter selectors unset, and the corrected single
`BROWSER_TEST_FILES` argument containing these space-separated paths:

```text
tests/household-routes.test.mjs tests/household-workflows.test.mjs tests/household-completion-locales.test.mjs tests/household-completion-medication.test.mjs tests/household-completion-dashboard.test.mjs tests/medication-journey.test.mjs tests/medication-journey-form.test.mjs
```

All 35 browser cases passed with no failures, skips or cancellations; Task exit
was 0. This includes all five locale create/edit/error and persisted read-back
journeys at desktop/mobile sizes, exact stale-draft retention and stock
read-backs, permitted-only limited-view dashboards, and the existing decimal
dose/history journeys. Evidence:
`evidence/ABC-BROWSER-GREEN-004-api-final-full.log`.

The browser job used the same immutable copy digest and a fresh separate
fixture with SHA-256
`d4b7a6054bc329b4af44923059ece712a62a1b0b508905bd2f49019e09309ec3`.
Both jobs' 361-path pre/post source manifests matched SHA-256
`4fea2556d999ab6dce08587bfad2cd8513fa53b0c3db1389838a5c63f4189cb4`.
The 40 screenshots are archived under
`docs/screenshots/ABC-BROWSER-GREEN-004-52eca56`, with their exact paths and
hashes in `evidence/ABC-BROWSER-GREEN-004-screenshot-paths.txt` and
`evidence/ABC-BROWSER-GREEN-004-screenshots.manifest`.

The eighth selected regression file, `tests/dashboard.test.mjs`, is excluded
from this seven-file browser result and queued separately as ABC-CLOCK-002.
These results establish HTTP and desktop/mobile acceptance in separate jobs;
they do not claim one combined job or a shared fixture passed both phases.

Source review found People creation increments its creator's permission version,
invalidating seeded bearer credentials. The parent creator therefore uses the
existing care actor, proven by `/me` to be an owner in this household, and fresh
public cookie login plus API CSRF after creation. The owner/self case likewise
uses a fresh primary login. Neither uses a permission-version or credential SQL
reset. The main viewer grants remain unchanged.

Owned B HTTP cases obtain a current owner bearer by real login and public
`admin/app_tokens` creation. The one-time token stays in the test's local Fixture
value; the disk fixture is untouched. Real public token revocation has both
failure-safe non-panicking RAII and normal successful cleanup assertions. This
retains a distinct bearer for the deterministic concurrent option insertion.
The private helper is `tests/household_completion_medication/authentication.rs`.

Owned B/C browser setup uses fresh owner cookie sessions rather than stale seed
tokens. Unsafe cookie APIs carry actual Origin plus current CSRF. Coordinator
transferred only new `Target::post_browser_json` and `delete_browser_json`
insertions; they retain local-write/origin guards and JSON headers without
changing existing helper semantics. Public cookie creation/revocation in the
final suite must verify those headers. No server guard was relaxed.

## Reused coverage and remaining work

Existing `web_session_api::inactive_primary_user_denies_cookie_web_and_api_even_with_active_secondary_user`
covers inactive primary-user dashboard denial. Existing `web_reads_api` tests
cover hidden/foreign related identifiers, authorised counts and live grant
revocation. Those need current stable-snapshot execution before reuse as evidence.
Coordinator approved and this writer added test-local suspended/revoked membership
denial with restoration. The valid membership enum uses suspended for the inactive
case. Optional profile 404 uses an isolated same-household membership/person
association mismatch, with authenticated `/me` and unchanged authorised people
IDs before/after; role/account/grants remain unchanged and association restores.
Actual people/profile server errors use scoped disposable audit exceptions.
A sequence increment survives rollback and proves both the direct API and SSR
requests reached the exact gate; each trigger/function/sequence removes on
teardown. These added cases remain runtime-unverified.

The existing server-side `contract_fail_dashboard_read=people` debug query in
the browser test proves handler error-page behaviour, not an actual upstream
collection failure. The new HTTP audit failure case supplies the latter proof
when executed. No browser interception or new product failure hook was added.
The five-locale browser journeys and C privacy checks require real desktop/mobile
acceptance. B runtime may expose fixture privilege or locking issues; classify
those before treating a failure as product RED.
