# Migration progress

Updated 15 October 2026. Complete every slice in [the plan](plan.md).
Foundation is accepted. The verified baseline PR #2451 is merged; all remaining
migration areas stay in scope. No live deployment has occurred.

## Current delivery

### Capabilities local stack propagation, 15 October 2026

The coordinator integrates published predecessor `1a2e59bcbb528d15b0e5ce9937b18fc3d0d07484` into
PR [#2639](https://github.com/damacus/med-tracker/pull/2639), retaining
previous head `022854c80a921d2601970b5bad03ec8e950d6fd9` and all existing proof ancestors.
This ordinary branch merge preserves the immutable capability proof and all
existing behaviour. Source `8f2d3d70b1bcec64b1a09c1f22c4d6c63ea4ad73` passed
focused `48052` (six cases), inventory `87017` (55 passed, one ignored),
runner `57537` (47/47) and strict lint `78646`. Dispatcher `28021` passed
55 request and 524 care cases, then rejected only the 117 incomplete operations.
Resumed review `endurable-continent` accepted this candidate in `46973`
(SWE-2 Max, 15 steps), with no required corrections. Existing capability 503
metadata remains tracked in #2638; no owner decision or new credit is implied.
Full CI `37168` passed 115 units, 55 requests (one ignored), 508 care cases
(two ignored, 16 filtered) and all 744 browser cases in 19.6 minutes.
The coordinator independently verified and reversed only 41 generated PNG
changes; all 1,093 baseline hashes pass and the tested source is clean.
Final documentation/review, publication and fresh exact-head hosted checks
remain pending. Earlier results retain their exact scope; no whole-API
acceptance, PR merge or deployment is claimed.

Request-ID predecessor `1a2e59bc` is now hosted accepted: run `38691350791`
finished all 42 checks (26 successful, 16 skipped), aggregate `116282371577`
passed, including every browser and integration shard. Review threads are empty.
Coverage artifact `11953410231` has verified digest `cb39792c6e6c23eed76223f097c5ee6443ab75f9faf5717197d367ef0fe1be11`;
complete clean source `e5a40888` has the published head's exact tree.
Its 16 added executable runtime lines are covered. Wider general coverage is
64.05% including inline tests, report-only and below the 80% target.

### Request-ID successor passes cumulative CI, 15 October 2026

The coordinator integrates published account `1178c11491e434e9cbfba5d739ebd9a6d8d0cf93`
into PR [#2637](https://github.com/damacus/med-tracker/pull/2637), preserving
previous head `adfe373fa79bd575d53db6b25f17e76f7600352a` and all proof ancestors.
Both production files and the complete rate-limit test module are unchanged.
Resumed Devin session `smart-hallway` accepted source `3000e9f2` in review
`68995` (SWE-2 Max, 15 steps); coordinator owns integration and publication.
The verifier's first rerun `93506` stopped at linker ENOSPC before any tests.
No cache was deleted: scoped Task settings moved this lane to an owned internal
build directory. Rate `81309` passed 9/9, account `15905` passed 10/10,
HTTP units `30016` passed 3/3, inventory `16343` passed 47 (one ignored),
runner `87371` passed 47/47 and strict lint `49313` passed.
Full CI `92048` then passed on exact `3000e9f20aae75aeb16cc5bcf0938b6dae9a4e1a`:
113 units, 47 requests (one ignored), 508 care cases (two ignored, 14 filtered)
and all 744 desktop/mobile browser cases in 19.4 minutes. The HEAD/GET shared
rate-bucket test also passed. Root independently verified and reversed only
39 generated PNG changes; all 1,093 baseline hashes pass and source is clean.
Final documentation checks/review, publication and exact-head hosted checks
remain pending. Earlier results retain their exact scope; no new operation
credit, whole-API acceptance, PR merge or deployment is claimed.

Subsequent final docs `20967` passed, and same-session review `92487`
accepted the progress-only commit (SWE-2 Max, 17 steps). Request-ID head
`1a2e59bcbb528d15b0e5ce9937b18fc3d0d07484` is published, 427 changed
lines against account `1178c114`; fresh hosted checks run in `38691350791`.
This updates publication only; hosted acceptance remains pending.

Account predecessor `1178c114` is hosted accepted: run `38684411332`
finished all 44 checks (30 succeeded, 14 skipped), aggregate `116259564959`
passed. Root verified coverage artifact `11949803598`, its digest and source
`0ef19dd4`; its tree matches the published head. Added runtime lines cover
27/27; retained account paths and general coverage gaps remain explicit.

### Account stack propagation, 15 October 2026

The coordinator integrates published predecessor `80f34a50fb6973624f40530523d71482907147ea`
into PR [#2636](https://github.com/damacus/med-tracker/pull/2636), preserving
the tested account correction `294f38d93e29309825456c4efdddfc075233f8f4` and
immutable proof source `0893879794d009bbb94cc72a0dd5df11aa7f6dfa` as ancestors.
The original three account overlap tests, helper, oracle and four evidence rows
are unchanged. The published prerequisite `faf0cc2f` passed every applicable
hosted check; its former browser failure is corrected on that exact head.
Execution-proof runner, inventory and full CI passed on `3004fb7d`; resumed
review and final documentation review accepted. Its final head `80f34a50`
is published and hosted accepted: run `38678068738` passed 25 checks with
16 skipped, including CI Success `116239828257`. Root verified coverage
artifact `11946842546` and source `83188ec9`; its tree exactly matches `80f34a50`.
Same-session account review `68306` accepted candidate `8b94ed4c` with no required
corrections (seventeen SWE-2 Max steps). Account code, tests and proof rows are
unchanged from the earlier reviewed correction. Frozen candidate
`9ec67c71af5725ef4fcb517a4cc2a34fb544271b` passed inventory `25929`
(45 passed, one ignored), runner `67022` (47/47) and strict lint `51810`.
Dispatcher `48028` executed all thirteen credited account identities: nine
request cases and four care cases. Its request phase passed 45 with one ignore,
care passed 522 with two ignores, and only final acceptance failed for 118
incomplete operations. Full `task ci` job `81494` then exited zero: 112 Rust
units, all integrations including 508 care cases (two ignored, fourteen filtered),
and all 744 browser cases. Root independently verified/restored only generated
PNG changes and checked all 1,093 baseline hashes; Serena work remains preserved.
Log: `/private/tmp/s05-i01-evidence/account-propagation-full-ci-9ec67c71.log`.
Final documentation checks/review, publication and exact-head hosted checks remain
pending. Earlier CI and review below retain their exact candidate scope. No new
operation credit, PR merge or deployment is claimed.

### s05-i01 execution-proof propagation, 15 October 2026

Coordinator-owned PR [#2633](https://github.com/damacus/med-tracker/pull/2633)
now integrates its verified predecessor `faf0cc2f58707555b794829bb05ec4574e44698e`.
The previous execution-proof head `630a7f016d364becfb682b855a7da0c57c0b7f7c`
is preserved as an ancestor. The parent passed full local CI and independent
review, then all applicable hosted checks (27 succeeded, 16 skipped). Its
hosted merge tree exactly matches the published source; coverage covers all
five added runtime lines, with wider migration gaps retained separately.

The three execution-proof JavaScript files are unchanged from `630a7f01`:
477 additions and 21 deletions against the predecessor, before this progress
update. Only the verified lower browser correction and progress are inherited.
Existing proof commits are preserved through an ordinary branch merge.
Frozen candidate `3004fb7d520a9000639612861ed43d58f915db17` passed runner
job `11623` (46/46) and inventory job `95949` (20 passed, one ignored).
Same-session `fate-button` SWE-2 Max review `91629` accepted the integration
with no required corrections (nineteen exported steps). Full `task ci` job
`43141` exited zero: 111 Rust units, all integrations including 508 care cases
(two ignored, fourteen filtered), and all 744 browser cases (19.4m).
Log: `/private/tmp/s05-i01-evidence/execution-propagation-full-ci-3004fb7d.log`.
Root independently verified and restored only forty generated PNG changes;
all 1,093 baseline hashes pass and unrelated Serena work remains preserved.
Final documentation checks and review, publication and exact-head hosted
checks remain pending. No operation acceptance, PR merge or deployment is
credited; account and later API work continues in its existing order.

### s05-i01 prerequisite browser correction, 15 October 2026

Published settings PR #2644 (`eb9ac904`) failed its hosted NHS dm+d import
browser journey in both desktop and mobile jobs (`116197321693` and
`116197321724`). The broken ZIP reached Failed and passed its ZIP alert. The
failure was the first `Completed` poll reload after the valid ZIP upload:
`Page.reload` reported `Not attached to an active page`. Trace shows the valid
upload POST returned 303 and its redirected GET returned 200; the reload began
about 1.4 ms after the desktop click returned and 1.1 ms after mobile. The
exact Chromium attachment cause remains unproved; the logs do not show an
importer or storage failure.

The previously reviewed dm+d prerequisite source is `923d5b63098a61ed4a9df0dfd4f27ae348d0b26a`.
A browser-test-only correction is applied locally in its owning checkout:
both upload clicks now wait for the exact navigation POST and page load,
then verify the 303 redirect before the existing status poll. The test file
is SHA-256 `ff19b1e6a9859312cc350071d538d4dc772d2326fe49039a72d80e92e93b19b2`.
The existing Failed, ZIP alert, Completed count and barcode checks remain.
Focused desktop/mobile browser job `22215` passed both cases (2/2, 6.8s)
and exited zero with normal cleanup. All 1,093 tracked PNG hashes were unchanged.
The frozen candidate is `a96876bfcb1613f6da6a4a7e9f404cfeeb9d6144`, based on
main `1800d858ef99e71edcf5e00cf5a0530616673dc6`; owner is the API coordinator,
PR [#2632](https://github.com/damacus/med-tracker/pull/2632). Same-session
`successful-dollar` SWE-2 Max review `51331` accepted with no required code
corrections (fourteen exported steps). Full CI `95472` exited zero on this exact
candidate: 111 Rust units, all integrations including 508 care tests (two
ignored, fourteen filtered), and all 744 browser cases (19.4m). Both NHS
import viewports executed. Root independently verified and restored only the
generated PNG delta and checked all 1,093 baseline hashes. The unrelated Serena
change remains preserved. Full log is
`/private/tmp/s05-i01-evidence/nhs-browser-full-ci-a968.log`.
The reviewed prerequisite diff is 542 additions plus deletions against main;
this final progress-only update remains below the 2,000-line limit. Final
documentation checks, push and fresh exact-head hosted checks are still pending.
Dependent propagation remains held until required gates pass; no operation
acceptance, merge or deployment is credited.

### s05-i01 — API contract completion, 15 October 2026

### Account overlap evidence correction, 15 October

The coordinator owns this correction to PR #2636 on
`codex/s05-i01-account-sessions`, base `630a7f016d364becfb682b855a7da0c57c0b7f7c`.
Its previously published head is `451c357c4f71539bf62056c5ac0459d235528a63`.
The later settings review exposed that client barriers alone do not prove
handler overlap. This audit also applies to account revocation and logout;
their old complete labels require stronger evidence before whole-API acceptance.
The test-only correction holds each target credential row and observes two
distinct restricted-role UPDATE waiters, tied to the controlled holder, before
release. A bounded condition check releases on timeout, and assertions follow
fixture cleanup. No production race or runtime correction is claimed.

Immutable tested source `0893879794d009bbb94cc72a0dd5df11aa7f6dfa` passed focused
`87259` (one case), all ten account cases `45760`, and strict lint `79310`.
Nine of those ten cases are credited request proofs; the tenth checks metadata.
The independent guard now positively validates all four account rows before its
tamper negatives. RED `46302` executed one case and rejected stale revision
`2ae0b911`; after exactly four inventory-row updates, GREEN `73360` passed.
The proof labels now explicitly require two target-UPDATE waiters before release.
Inventory `3820` passed 45 cases with one intentional completion ignore;
runner `75633` passed 47/47 and strict lint `92718` passed. Dispatcher `36151`
passed 45 inventory cases, 522 care cases with two ignored, then 45 request
cases before rejecting only final acceptance for 118 incomplete operations.
All 13 credited identities executed: nine request proofs and four care proofs.
The fourth care proof identifies separate same-application mobile grants in
their audits; it must not be omitted from the executed evidence count.
This earlier stack layer lacks the later capability credit, so its remaining
count is 118 while the published upper stack reports 117. Neither is completion.
Same-session `important-platypus` SWE-2 Max review `27798` accepted the exact
corrective candidate `294f38d93e29309825456c4efdddfc075233f8f4` with no
required correction. Full local `task ci` job `29691` exited zero on that same
candidate: 112 Rust units; 45 API request cases with one intentional ignore;
508 broad care cases with two ignored and 14 filtered; 17 OAuth cases; and
744 browser cases in 19.4 minutes. Root preserved and verified exactly 39
generated PNG changes, restored only those files, and confirmed all 1,093
baseline PNG hashes. The unrelated Serena configuration remained unchanged.
The four account rows still pin immutable source `0893879794d009bbb94cc72a0dd5df11aa7f6dfa`
and 13 executed proof identities (nine request, four care). No new operation
acceptance is inferred. Publication and dependent stack propagation remain
held by the separate hosted NHS dm+d browser failure; this local correction
has not been published, merged or deployed.
Native pins and production code are unchanged. The unrelated
Serena configuration is preserved and excluded from commits. Later People and
profile/avatar proof drafts remain unexecuted; membership execution stays held
until lower corrections are published and propagated. Older entries below retain
their original revisions and results as history.

The API coordinator owns integration, contracts and evidence; one API writer
owns each active area's implementation/tests and one verifier owns the serial heavy lane.
Rate-limit [PR #2623](https://github.com/damacus/med-tracker/pull/2623) was merged
externally as main `1800d858ef99e71edcf5e00cf5a0530616673dc6`; this coordinator
did not merge it. Its 1,499-line candidate and historical failures remain below.

| Published stack layer | Base | Exact head | Changed lines | State |
| --- | --- | --- | ---: | --- |
| [#2632](https://github.com/damacus/med-tracker/pull/2632): storage transport and reminder clock | main `1800` | `923d5b63098a61ed4a9df0dfd4f27ae348d0b26a` | 496 | Reviewed; local application and exact-head hosted gates pass |
| [#2633](https://github.com/damacus/med-tracker/pull/2633): executed acceptance proof | `codex/s05-i01-dmd-storage` | `630a7f016d364becfb682b855a7da0c57c0b7f7c` | 498 | Reviewed; local and exact-head hosted gates pass |
| [#2636](https://github.com/damacus/med-tracker/pull/2636): account session contracts | `codex/s05-i01-contract-execution` | `451c357c4f71539bf62056c5ac0459d235528a63` | 1,711 | Reviewed; local and exact-head hosted gates pass |
| [#2637](https://github.com/damacus/med-tracker/pull/2637): request IDs on rate-limit responses | `codex/s05-i01-account-sessions` | `adfe373fa79bd575d53db6b25f17e76f7600352a` | 399 | Reviewed; local CI and exact-head hosted checks pass |
| [#2639](https://github.com/damacus/med-tracker/pull/2639): capability discovery | `codex/s05-i01-rate-request-ids` | `022854c80a921d2601970b5bad03ec8e950d6fd9` | 1,101 | Reviewed; local CI and exact-head hosted checks pass |
| [#2640](https://github.com/damacus/med-tracker/pull/2640): household errors and UI privacy | `codex/s05-i01-capabilities` | `012e8cfe9b2e2555fedd3001213a877795aac4a4` | 990 | Reviewed; local CI and all 45 exact-head hosted checks completed successfully or skipped |
| [#2641](https://github.com/damacus/med-tracker/pull/2641): People collection and writes | `codex/s05-i01-me-ui` | `4229289bdf729defa3b21bf5589a296c14c6924d` | 1,551 | Reviewed; local CI and all 45 exact-head hosted checks pass or skip |
| [#2643](https://github.com/damacus/med-tracker/pull/2643): profile privacy and avatar acceptance | `codex/s05-i01-people-write` | `9eb729ddb9f7aecb4229899ebdcdeab82b002013` | 1,768 | Independent review, full local CI and all 45 exact-head hosted checks pass or skip |

Stack `2634` is published; no layer is merged or deployed. Full local
CI `61813` passed 111 units, 20 request cases, 508 care cases and all 744 browser
cases. Hosted coverage and clean source manifests are retained below. The storage
defect has controlled RED/GREEN; its relationship to the original intermittent
failure remains an inference, not a captured historical root cause.

Published account branch `codex/s05-i01-account-sessions` follows `630a`. Malformed-ID
and mobile-login target corrections each have genuine behavioural RED/GREEN.
Existing care contracts pass 12 cases; pure deadline boundaries pass one table;
the full request inventory passes 44 cases with final acceptance intentionally
ignored. Final foreign-account collection case `75396` and full inventory rerun
`72412` both pass; owned resources cleaned up normally. Tested source candidate
is `2ae0b911576912bccf22f10c5db65856bd8cdc60`, with request SHA `abeb3c38`.
Exact executed evidence now credits these four account operations locally;
PR #2636 now carries that evidence. Earlier review
`78959` accepted saved source `9219c243`; final review `66814` checked the later
production and evidence changes and required only a negative-test lookup fix.
That fix passes; same-session correction review `59730` accepted the final source
and exact evidence, retaining full CI and published-head gates. The final
source includes seeded foreign households and API sessions whose exclusion is
asserted through the booted router.

Cumulative CI `11105` on `b846a681` stopped in the fake-Cargo runner tests:
40 passed and six failed because their explicit transcripts still lacked the
newly credited account names and multi-binary call sequence. No Rust, database
or browser phase ran and no screenshots changed. Publication and dependent work
are held for the writer's test-fixture correction, same-review follow-up and a
fresh cumulative run. Log: `/private/tmp/s05-i01-evidence/account-full-ci-b846.log`.
The correction changes only explicit fake transcripts and their assertions:
requests, care API, then final requests. Missing-care evidence now has a negative
case; summary-count mutation remains effective as the credited set grows.
Intermediate `90084` passed 44/46 before two stale call-count assertions were
fixed. Final `86436` passed all 47 runner cases, with fixture SHA `599a20d6`.
`account-runner-fixture-green-b846.log` beside the CI log is explicitly a
reconstructed summary of tool output, not a captured raw transcript. Same
independent review `25548` accepted this test-only delta; runtime and evidence
hashes remain unchanged. No real database or browser job ran in these unit tests.
Fresh cumulative CI `51506` passed on exact code candidate
`d0781fb84e2fd3f73454d8899157986517b4089f`: 112 units, 45 request cases (one
intentional completion ignore), 508 care cases (two ignored, 14 filtered) and
all 744 browser cases in 19.3 minutes. Full Task exit was zero with normal
owned-resource cleanup. Log: `/private/tmp/s05-i01-evidence/account-full-ci-d078.log`.
The 39 generated screenshots are preserved in `account-full-ci-d078-screenshots.patch`
beside that log, SHA-256 `db5c08c171f9b9be2ba35c0d38e2424a2a9d5cfd9184cf1b2eadc92d55a6907b`;
only those verified paths were restored. Unrelated Serena drift remains excluded.
This final evidence update changes documentation only; tested code, schemas,
runner fixtures and request assertions remain unchanged. Hosted acceptance,
publication, merge and deployment remain separate.

| Final account evidence | Job | Result | Retained log under `/private/tmp/s05-i01-evidence/` |
| --- | --- | --- | --- |
| Mobile login target regression | `82913` / `75274` | Behavioural RED / GREEN | `account-mobile-target-lifetime-{red,green}-630a.log` |
| Pure checked deadline boundaries | `3355` | One table passed after equivalent helper refactor | `account-mobile-deadline-unit-630a.log` |
| Existing account schema/transactions | `20402` | 12 passed; two people-route activity tests excluded from account credit | `account-existing-care-api-lifetime-630a.log` |
| Final seeded foreign collection | `75396` | One passed | `account-foreign-collections-focused-7d2.log` |
| Final request inventory | `72412` | 44 passed; one intentional completion ignore | `account-foreign-full-inventory-7d2.log` |
| Strict evidence registration | `2733` | Expected RED: old validator rejects non-null candidate revision | `account-evidence-metadata-red-2ae0.log` |
| Negative-test lookup correction | `16340` / `69593` | Test-only lookup failure; corrected baseline passes 10/10 | `account-evidence-metadata-green{-v2,}-2ae0.log` |
| Credited execution dispatcher | `58221` | Requests 45 pass; care 522 pass; final gate correctly rejects 118 remaining operations | `account-evidence-dispatcher-2ae0.log` |

The authoritative inventory remains 122 operations, 121 registered routes,
four complete, two partial and 116 unverified. The missing route is the existing
notification test operation. All 13 unique credited account test names executed
and passed before the final completion rejection, with normal owned cleanup.
Final request and inventory hashes are `17190df2` and `656073df`; candidate
revision pins remain `2ae0b911`. Hosted root run `38599154874` passed with CI Success
job `115980431882`; no active or failed checks remain. Exact-head Android checks,
generated-client determinism, Swift compilation/tests and Kotlin compilation/tests
already pass. Published-head Devin review has no inline threads. The account
branch matches origin; unrelated Serena changes were preserved byte-for-byte
during upstream synchronisation. The app cannot currently load its artifact
attachment tool, so no successful attachment of PR #2636 is claimed.

Hosted coverage artifact `11913521606` is retained as
`/private/tmp/s05-i01-evidence/loco-rust-coverage-2636-451c.zip`, SHA-256
`eeefb064c7030365d7ddc88e91cdb9fc0ba95eea2417c5d02b2634b4c50a3f08`.
Its clean manifest identifies source `8b6abd35023ac78875bdf757ba374b9529911185`,
parents `630a` and `451c`; Git comparison proves an identical source tree to
the published head. Rust 1.99.0/cargo-llvm-cov 0.9.1 ran lib, request API and care
selectors. Account controller production coverage is 129/142 (90.85%); account
model production coverage is 330/336 (98.21%). All added instrumented production
lines are covered (controller 4/4, model 23/23), including the critical deadline
calculation 12/12. Remaining uncovered lines are pre-existing adapter/error paths,
not newly added business logic. API controllers total 6,881/7,451 (92.35%);
application total is report-only 39,779/62,657 (63.49%). That broader gap remains
explicit; percentages do not substitute for the 118 outstanding operations.

The API writer now owns isolated `codex/s05-i01-capabilities`, based on published
account head `451c357c`; root retains contracts, progress and integration.
Genuine TestServer RED `31340` executed two cases: the aggregate discovery
projection advertises implemented features as unavailable, and profile/avatar
metadata is absent. Both fixtures cleaned up normally. Log:
`/private/tmp/s05-i01-evidence/capabilities-flags-profile-red-451c.log`.
GREEN `10395` then passed both cases through the booted router, with normal
cleanup. A green-phase refactor removed an unnecessary rendering panic and kept
the shared avatar limit crate-visible. Subsequent seeded client RED `52477`
passed both earlier cases and failed only because registered mobile client
`name` was absent despite being required by the authoritative schema. The name
assertion runs before any unsupported URI-format helper. Logs:
`capabilities-flags-profile-green-451c.log` and
`capabilities-mobile-client-name-red-451c.log` beside the initial RED log.
The writer now also owns only the `mobile_clients` projection in the identity
model, its typed client response and the narrow shared URI-schema helper tests.
Other OAuth/security handlers and generated entities remain unchanged. No
OpenAPI or native pin change is authorised for this correction. The separate
getPerson conditional-response omission proposal awaits the owner's decision
before the later people area. Broader slice 05 is not complete.
Mobile-client projection GREEN `18390` passed all three capability requests.
The shared schema helper previously ignored URI formats: genuine helper RED
`71150` accepted an invalid URI unexpectedly. The test-only maintained
`iri-string` dependency now validates RFC URI syntax; helper GREEN `69892`
passed 16 cases, and actual capability schema GREEN `29978` passed all three
requests. The lock change adds only that package and its root dependency edge.
Configured lifetime unit RED `18040` now proves that maximum age zero is
incorrectly rejected, although OpenAPI explicitly defines it as disabling the
absolute login deadline. Request RED `95817` compiled and executed but failed
during real application boot with unavailable browser session configuration;
it did not reach its HTTP assertion and receives no HTTP RED credit. The pure
unit failure establishes the parser correction; the next GREEN must prove
both successful boot and the capability projection. YAML null continues to mean an absent setting;
numeric configuration strings retain existing parsing. Capability acceptance
remains uncredited and unpublished pending its remaining checks and review.
Lifetime GREEN `67033` passed both pure setting tables after the parser allowed
zero only for maximum age. Request GREEN `28389` passed all four capability
cases, including real successful application boot and HTTP 200 projections for
custom 3/14 and disabled 3/0 day windows. Default 7/30 projection is explicitly
asserted. Logs are `capabilities-lifetimes-unit-green-451c.log` and
`capabilities-lifetimes-request-green-451c.log` in the same evidence directory.
Direct capability 429 and controlled database lookup failure remain next.
Focused six-case run `9320` passed five cases, including controlled database
lookup failure with the existing 503 envelope. The rate-limit case consumed 300
successful requests, then stopped because the next response lacked
`x-request-id`; its later status/schema assertions did not execute. The fixture
cleaned up normally. Loco 1.2 applies its default request-ID middleware before
`after_routes`, while the application adds the rate limiter in `after_routes`,
allowing early rejections to bypass request-ID generation. Capability source is
frozen while the same API writer owns a bounded shared-HTTP prerequisite on
`codex/s05-i01-rate-request-ids`, based on `451c`, in
`/private/tmp/s05-i01-rate-request-ids/med-tracker`. Root retains contracts,
integration and publication; no capability acceptance is credited yet.
Prerequisite RED `40206` executed nine rate-limit request cases: six passed,
two proved missing generated/supplied IDs on actual 429 responses after status,
recursive schema, rate headers and persisted-effect assertions passed. A third
failure exposed a test assumption: a downstream 401 handler emits its own ID
even when Loco's request-ID middleware is disabled. That handler behaviour is
preserved; only the early 429 is required to remain ID-free when the framework
layer is disabled. The correction moves Loco's existing middleware once to the
outer boundary; no custom ID generator or second middleware implementation is
introduced. RED log: `request-id-prerequisite-red-451c.log` in the evidence directory.
Prerequisite GREEN `58540` passed all nine rate-limit cases; downstream account
regression `4298` passed all ten cases with existing audit/header semantics.
Logs are `request-id-prerequisite-green-451c.log` and
`request-id-account-regression-green-451c.log`. Read-only review then found that
the inner HTTP completion logger reads the response ID before the outer Loco
middleware adds it. A scoped subscriber test will prove that diagnostic gap
before a narrow logging correction; the framework middleware and request tests
remain frozen. Independent review and cumulative CI still precede publication.
Completion-log RED `27536` compiled and executed one scoped-subscriber unit
test: both inner completion events recorded unavailable while final response
headers held a generated UUID and sanitised `client42`. The collector also
encoded string quotes, so its string visitor is being corrected before the
same behavioural RED is rerun. Production logging remains unchanged at this
point; no passing diagnostic claim is inferred from the test scaffold.
Corrected collector RED `66711` confirmed plain unavailable versus UUID/client42.
The minimal logging correction then passed two existing HTTP unit cases, but
candidate GREEN `73639` captured no events in the new third case. This is not
accepted as a passing logging check; subsequent rate/account reruns did not run.
The test is being aligned with the existing scoped, thread-local subscriber
pattern before another module run, without global tracing state or blind sleeps.
Final scoped HTTP module GREEN `7781` passed three cases. Final rate-limit
regression `20845` passed nine and account regression `85598` passed ten, all
with Task exit zero and normal fixture cleanup on frozen source. Final HTTP
source SHA-256 is `fcd2f805e9ef5db19dea135186024f7ac849a165c8e958c043a4a4c813762583`;
app and request source remain `d8119f44` and `2b053c86`. Logs are
`request-id-completion-log-scoped-green-451c.log`,
`request-id-final-rate-green-451c.log` and `request-id-final-account-green-451c.log`.
Independent SWE-2 Max review is running inline from the existing trusted account
checkout; the new temporary checkout is not added to the review tool's trust
list. Production source remains frozen. Cumulative CI and publication are pending.
Independent SWE-2 Max review `smart-hallway` (process `6657`) then exited zero
with ACCEPT and no required corrections on that same frozen production/test
source. Cumulative Task CI `72554` passed on clean-start code candidate
`0b76739935810b2890ef4efe0356661ea1bd6ebc`: 113 Rust units, 47 request cases
(one intentional completion ignore), 508 care cases (two ignored, fourteen
filtered) and all 744 desktop/mobile browser cases. Other integration shards
passed; browser execution took 19.3 minutes and owned fixtures cleaned normally.
The original verified-archive dm+d failure case passed again. Full log:
`/private/tmp/s05-i01-evidence/request-id-full-ci-0b767.log`.
Only 41 generated PNGs changed during CI. They are preserved in
`request-id-full-ci-0b767-screenshots.patch` beside the log: 8,213,657 bytes,
SHA-256 `e8cde1cf8688a973dbb67502b72550c8e5e5a16ac5a4a39e35ca62cdca546c8d`.
The exact path list SHA-256 is
`62cd5b92b2b8fe6b2abe43f9d1dffd01d6869c480e66db0c2d17ea7f08d7eb97`;
reverse-checking and applying that verified patch restored only those generated
outputs. The checkout was clean afterwards. This final record changes only
documentation; reviewed/tested application code, contracts and native pins
remain identical. Publication, exact-head hosted acceptance, merge and deployment
remain distinct, and no capability operation credit is added by this prerequisite.
Request-ID PR #2637 is now published in stack `2634` at
`adfe373fa79bd575d53db6b25f17e76f7600352a`, targeting account head `451c`.
Its final documentation-only record passed docs build `11841`; runtime/tests
remain identical to fully tested `0b767399`. The branch was pulled/rebased and
pushed, and is synchronised with origin. Exact-head hosted run `38612969250`
is active with no reported failure; published-head Devin Review passed and its
actual inline review thread list is empty. Coverage job `116020860508` remains
active. Codex artifact attachment remains unavailable; no attachment is claimed.

The capability source has been preserved and integrated separately onto that
published head. Automatic approval review rejected an initial reset-containing
integration command before execution. The accepted alternative stashed every
change, including progress notes, in owned stash
`d88b1df9e33250c9c83eb7bbcefbd890a0f2b572`; all notes were retained while resolving
the shared helper conflict. Eleven unaffected source hashes matched the backup.
The shared boot helper keeps configurable settings and the inherited request-ID
enable test; no duplicate runner was added. Integrated request source SHA-256 is
`91d76d2567046029eac010164772e105ef79c93279b8493e18ede3d4ad8c7812`.
Capability request SHA-256 is
`e1795daf7fac11b29852ad6b9ceba605ab7ceaba9878bfda02e60a3528dbcb90`:
its rate case now checks persisted effects and credential counts before/after
both 300 successful requests and the rejection. Focused `31351` passed all six
cases; full request inventory `62969` passed 55 with one intentional completion
ignore. Both Tasks exited zero and cleaned owned fixtures normally. Logs:
`capabilities-integrated-six-green-adfe.log` and
`capabilities-integrated-inventory-green-adfe.log` in the existing evidence directory.
The controlled 503 case preserves an existing failure envelope not declared in
OpenAPI; no schema or native pin was changed. Exact immutable capability evidence,
independent review and cumulative CI still precede its publication.
Capability source is now committed as `96d81719e900296f3b72ab5f7c86ad3aee22a00f`.
The independent inventory oracle pins `getCapabilities` to that immutable source
and six explicit executed request proofs; revision/proof tampering is rejected.
Runner fixtures `23204` passed 47/47. Dispatcher `30195` executed 55 request tests
(one completion ignore) and 524 care tests (two ignores). Its final request run
passed 55 tests and failed only `api_v1::acceptance`, correctly rejecting 117
incomplete operations. This credits five local complete operations, two partial
and 115 unverified; it does not pass whole-API acceptance. Evidence logs are
`capabilities-runner-fixture-green-96d817.log` and
`capabilities-evidence-dispatcher-96d817.log`. Capability review and full CI remain
pending. Request-ID head `adfe373f` has now passed every applicable hosted check,
including coverage job `116020860508` and roll-up `116027225928`; native/client
checks were skipped for this prerequisite. Coverage artifact inspection remains.
Capability candidate `ccf0c3522bad57b3835ad3c09894421a0afebe67` is accepted locally.
Independent Devin session `endurable-continent` exported model `SWE-2 Max` and
returned ACCEPT after nine steps; process `40156` exited zero. Its required
separate tracking action is fulfilled by [#2638](https://github.com/damacus/med-tracker/issues/2638),
which records the pre-existing undocumented 503 without changing this layer.
Optional observations required no source correction. Full CI `93895` exited
zero on that exact clean candidate: 115 units, 55 requests (one intentional
completion ignore), 508 care tests (two ignores, 16 filtered), all other applicable
integration suites and 744 browser cases passed. Browser duration was 19.6 minutes.
The 38 generated screenshots were preserved in a 7,476,761-byte binary patch,
SHA-256 `fad141b48c8e5f969adba0a3b8b1567451cb64726bd304f77f27a98d88cff8ba`;
the exact path-list SHA is `795446e6f6eef7c668598f616197b5b5191f1129b547089cca3aa066bb278cc1`.
Reverse-apply verification restored only those files; source hashes and clean
HEAD matched the candidate. Review and full CI logs remain under the existing
`/private/tmp/s05-i01-review` and evidence directory. Publication and exact-head
hosted acceptance follow the final documentation gate.

Request-ID hosted coverage artifact `11919385306` matches GitHub digest
`c195e7921d563d890d084193241042a9d6a1b33bde4edca2a4821e9997771f21`.
Its clean merge `c62557e6` has parents `451c` and published `adfe`, with identical
tree to `adfe`. Added runtime lines are covered 16/16; raw application coverage
is 63.05% and API controller coverage 91.29%. The report-only general baseline
remains below target. This does not pass final API acceptance.

The next isolated `codex/s05-i01-me-ui` branch contains seven tests only, based
on `ccf0`; no RED or production correction has run. It distinguishes actual
foreign households from absent ones, preserves integration-token 401 and mobile
insufficient-scope 403, and checks private caching, revocation, audits and rates.
The malformed household-path 400 is an executed existing behaviour omitted from
me/UI contracts; its proposal, capability 503 and getPerson 304 await owner
decisions. Independent work continues while those contract edits remain held.

Capability PR #2639 is now published at the table's exact head in stack `2634`.
Final documentation gate `59941` passed; its 38-line documentation-only commit
preserves all reviewed and CI-tested runtime/test files from `ccf0`. Pull/rebase
and push confirmed upstream synchronisation. Hosted run `38619565391`, coverage
job `116043331721` and unit compilation `116043332248` are active without a
reported failure; published-head Devin Review has passed. Root Taskfile changes
trigger the repository's broader hosted suite policy, including native clients.
Codex artifact attachment remains unavailable; no attachment is claimed.

The next test-only branch fast-forwarded to that published head with both owned
test hashes unchanged. First run `59408` stopped at four unsupported test-header
API calls before executing tests; it is not behavioural RED. The writer corrected
only those calls. Genuine RED `83825` then executed seven full-boot request cases:
one rate case passed, six failed. Own 200 and seeded foreign 403 were correct for
API sessions, app tokens and mobile OAuth; absent household 99999 incorrectly
returned 403 for both me/UI. Five other cases stopped at missing UI no-store
headers, so their later assertions were not reached. Owned fixtures cleaned
normally. The writer is correcting only typed household NotFound propagation
and route-level private caching; malformed-path behaviour and contract decisions
remain held. No new operation credit is claimed before GREEN and exact evidence.

Me/UI request GREEN `64282` passed all seven cases. Existing care me/UI
regressions passed 6/6 and 3/3 (`7934`, `18176`). Identity-resource initially
passed 6/7 because its unseeded household 72002 expected Forbidden; fixture
inspection confirmed absence. The corrected missing-household assertion passed
7/7 in `68673`, without changing production. Formatting check `52750` initially
failed; the writer applied only the six reported Rust files through
`task fmt:apply`. Strict lint `51161`, check `56312` and formatted request rerun
`10614` all passed. The two production changes preserve typed NotFound through
the household API's existing 404 renderer and reuse route-level no-store for UI.
Non-household exhaustive error matches preserve their previous behaviour.

The global contract already promises 404 for absent households, but eight
operations omitted it from their response lists. Initial metadata RED `12342`
also rejected AI's valid inline 404; the corrected semantic test in `50739`
reported exactly eight genuine omissions. Sixteen YAML lines add existing
NotFound references, eight inventory rows and the full-schema guard refresh;
AI's description/envelope and native pins remain intact. Full request inventory
`25083` passed 63 with one intentional completion ignore. Generated-client
verification `8732`, Swift `2192` and Kotlin `84603` all passed; outputs remained
ignored temporary SDKs. New source review and full CI still precede publication.
Final metadata lint `7664` passed. Format check `13539` reported two test-line
wraps only; the writer applied them with the project formatter and verified
`task fmt` plus diff check. Runtime and contract semantics remain unchanged from
the successful inventory/client runs. Immutable partial evidence follows the
source commit; neither me/UI operation is claimed complete while its additional
malformed-path/audit-failure contract gaps remain unreconciled.

Capability coverage artifact `11922576872` matches digest
`09a71fb52f838199432289e393100ee9f1f59a0f9637d3de8cf82946b3525938`.
Its clean merge `702eeb8c` has parents `adfe`/`022854` and exactly the published
tree. Added runtime lines are 182/183, lifetime parser 26/26, typed view 153/153.
The capability handler is 15/17: invalid-configuration and serialization-error
fallbacks remain uncovered; the handler alone is below the 90% target. Raw
application coverage is 63.16%, API controllers 91.29%. General coverage remains
below target; coverage does not replace the 117 incomplete operation records.
All 61 capability hosted checks have now passed on exact published `022854`;
roll-up job `116060644770` in run `38619565391` is green. Hosted generated Swift
`116043761157`, Kotlin `116043761698`, Android `116043332516` and coverage
`116043331721` passed, as did published-head Devin Review. Actual inline review
threads are empty. The latest Codex artifact attachment retry still failed to
load the tool; no successful attachment is claimed. No merge or deployment occurred.

Me/UI immutable source `e9b6475c6c30c2a9bdce77f082ece454995b2b13` now has
proposed partial evidence for both operations. Guard RED `49932` rejected the
new evidence before registration; GREEN `15873` rejects changed source pins,
proof lists, gaps and promotion to complete. Runner fixture `5939` passed 47/47.
Full dispatcher `53961` then stopped before final completeness acceptance:
64 request tests passed (one ignored), while care had 491 passes, 33 failures
and two ignores. All seven new request proofs and nine me/UI care cases passed.
The 33 failures expect foreign-household 403 but use absent households: 23 use
72002 and ten use 99999. Neither row exists in the loaded care fixture.
The global contract explicitly requires missing-household 404, so the writer
owns scoped foreign-household fixture corrections in those tests and the shared
fixture helper, without changing the common template or runtime policy.
Classification is retained in `me-ui-care-household-classification.md` beside
the dispatcher log. Review, full CI and publication remain held until the
dispatcher passes its actual suites. The existing Sol API writer retains
ownership: this is a bounded fixture correction with objective database and
HTTP assertions, not a change to the agreed permission rules.
The frozen correction adds one helper and exactly 33 scoped setup calls, without
changing expectations, comments or production. `task fmt:apply`, `task fmt` and
diff checks pass. Dispatcher rerun `26127` passed 64 request tests (one ignored)
and all 524 care tests (two ignored), including every previously failing case.
The final request phase passed 64 and failed only `api_v1::acceptance`, correctly
rejecting the 117 incomplete operations; Task exit 201 is that intentional
completion rejection, not an ordinary-suite failure. Owned resources cleaned
normally. Both partial-evidence and global-404 guards executed and passed.
Local evidence now records five complete, three partial and 114 unverified
operations. No new complete credit, merge or deployment is implied.
Candidate `efa9d167dc039caa1910df020877addd7ca2e405` is committed, 952 changed
lines against published capability `022854`. Inline independent SWE-2 Max review
`49067` is running on that source. Full CI `72794` passed 115 units but failed
strict lint: the shared fixture helper is unused when `oauth_server` imports
the fixture module. The correction moves the exact helper into the care test
binary, without changing SQL, assertions or runtime policy. Only owned downstream
integration/browser runners were interrupted through their normal handlers;
all owned processes exited. No full-CI success is claimed. Three generated PNGs
were preserved in `me-ui-ci-efa9d167-generated-screenshots.patch`, SHA-256
`f2e8889ee82a583ac7b90140f39dbc32716fd56f9c1b253a47a68d6fd0798501`.
Root verified the three exact paths and reverse-applied only that patch.
People test drafting in its separate branch remains paused until this required
failure is corrected. Publication remains held for lint, full CI and review.
The unchanged helper now lives in an inherent `impl Fixture` within
`tests/care_api.rs`, so the OAuth binary no longer compiles an unused method.
Formatting and diff checks pass; strict all-target lint `13425` passed in
18.14 seconds. Full CI will rerun on the committed correction; the existing
independent review will receive that exact placement delta in the same session.
Corrected candidate `9eb99ed02cfee6d63f20ae218a6d649b36a6d8f1` is now accepted
locally. Same Devin session `giddy-protocol` exported `SWE-2 Max`: initial review
`49067` identified only the helper placement; correction review `24809` exited
zero with ACCEPT after 11 cumulative steps. Full CI `53209` exited zero on the
clean candidate: 115 units, 64 requests (one completion ignore), 508 care cases
(two ignored, 16 helpers filtered), OAuth 17, identity 7, all other applicable
integration suites and all 744 browser cases in 19.4 minutes. Owned processes
exited normally. Forty generated PNGs were preserved in an 8,170,202-byte patch,
`me-ui-full-ci-9eb99ed0-generated-screenshots.patch`, SHA-256
`df6547e5244d56b4d4eabdabd7f5231cb0e7d8d535309499a683f69f2e84e8a4`.
Root verified all 40 current file hashes and the exact path list, then reverse-
applied only that patch; source and HEAD remained unchanged and clean.
The candidate has 972 changed lines against published capability `022854`.
This final evidence update is documentation only. Publication and exact-head
hosted acceptance remain required; no merge or deployment occurred. Whole-API
acceptance still rejects 117 incomplete operations. The separate People draft
will next demonstrate its own RED, including the missing replay metadata;
neither draft nor source inspection earns additional complete operation credit.

PR #2640 is now published and hosted-accepted: run `38632805964` completed with
31 successful and 14 skipped checks; Devin Review passed with no review threads.
Coverage artifact `11926499279` has verified SHA-256
`0d47cac978fcf7d0556942c44aafd4836f5e56d50bb29ce272fe34f8abffb6d0`.
Its merge revision has the same tree as the published head. Added runtime
coverage is 5/7, and critical `operation_error` coverage is 5/7, below targets.
Raw general source coverage is 63.63%; API source coverage is 92.37%.
These measurements do not replace operation acceptance. No merge or deployment
occurred; Codex artifact attachment failed because its tool could not load.

The coordinator owns the next isolated `codex/s05-i01-people-write` branch,
based on published `012e8cfe`. The API writer transferred its frozen tests,
four scoped OpenAPI operations and matching inventory metadata to the coordinator;
the verifier owns builds and disposable fixtures. This boundary covers People
list, create, PATCH and PUT. The person-detail operation remains held for its
304 contract decision. No People runtime code or native pins changed.

The initial draft jobs `1075` and `62069` failed compilation and executed zero
tests; neither is behavioural RED. Actual RED `14358` executed ten cases, with
three failures demonstrating missing idempotency-key and 409 metadata on all
three writes. Metadata-only corrections passed ten cases in GREEN `98150`.
Concurrency and audit-failure additions initially exposed overclaimed test
expectations: People does not promise no-store on every response, and an
overlapping create can invalidate the permission snapshot of its competitor.
Those expectations were corrected without changing runtime security. The POST
case requires exactly one creation, allows the competitor's existing 403 or
replay response, then requires successful same-key replay through a fresh
request. PATCH and PUT each require two successful replies and one replay.
Job `28202` then demonstrated the actual four 500 ErrorEnvelope replies and
atomic rollback before failing their missing OpenAPI responses. Adding the
shared response metadata produced GREEN `14361`, fourteen passing cases.
Inventory `16932` passed 78 cases with only final completion ignored.

Two further real-boundary tests passed in job `3023`: restricted-role household
read failure returns 500 without domain/audit writes; account closure during
the real mobile OAuth touch returns 401 before household access. The latter
uses a clone-local, account-scoped trigger with fixed search path, verifies its
execution and persisted account status, and retains restricted runtime access.
All sixteen People cases passed, followed by strict lint `69105`. Earlier
generated-client verification `82539`, Swift `11452` and Kotlin `67388` passed;
their inputs are unchanged by the two additional test-only cases. Logs are
retained under `/private/tmp/s05-i01-evidence/people-*`.
Malformed household-ID extraction remains an unresolved contract gap. This
candidate still needs immutable partial evidence, independent SWE-2 Max review,
full CI, publication and exact-head hosted acceptance. No new complete-operation
credit is claimed; whole-API acceptance still rejects 117 incomplete operations.
Immutable People evidence is now pinned to tested source
`7ba79ac8cbfdffa365c0e15646ea13075a1e6918`. Guard RED `35467` compiled and
executed one test, failing on unverified versus partial coverage; GREEN `23940`
passed after the four exact proof arrays, source revision and gaps were recorded.
Tampering with any of those fields is rejected. Evidence describes actual denial
audits, domain versions and pagination schemas without claiming untested effects.
Inventory `44715` passed 81 cases with completion intentionally ignored; runner
fixtures `46909` passed 47 cases. Full dispatcher `14472` executed requests
81/1 ignored, care 524/2 ignored, then requests 81 passed/one failed solely at
the final acceptance guard for 117 incomplete operations. Every credited People
test executed. The ledger is five complete, seven partial and 110 unverified;
this is partial verification, not whole-API acceptance. Independent review and
cumulative CI will run on the next frozen commit before publication.
Candidate `cdfa408bf1582b84d4def703ede2b7d251f48b90` has 1,525 changed lines.
Full CI `44296` stopped at the early CI policy suite: 137 cases passed and
the module-reachability case failed. Rust had compiled and executed the evidence
guard, but the repository scanner requires an explicit `#[path]` declaration
for this module, matching its siblings. The correction adds that declaration;
no application code changes. No database/browser jobs or screenshots started.
Publication and independent review remain held for the corrected candidate.
The corrected registration passed all 138 policy cases in `44749`, formatting
in `50670` and diff checks. Full CI must rerun on the committed correction.
Corrected candidate `164129cf0b6d0316701427c2236de455cd393f83` is locally accepted,
with 1,535 changed lines against published me/UI `012e8cfe`. Independent Devin
review `73581` exited zero with ACCEPT and no findings; its export verifies
session `bright-glove`, model `SWE-2 Max` and nine steps. The inline packet has
SHA-256 `26ead56a9d4868aa05f5ba90681af4b8db587339e9871e36d9d7bb4832b2b524`.
Full CI `91429` exited zero on that exact candidate: policy 138, runner 47,
Rust units 115, requests 81/one ignored, care 508/two ignored/16 helpers filtered,
all other applicable integrations, and all 744 browser cases in 19.4 minutes.
No live verification process remains. Thirty-nine generated PNGs were preserved
in `people-ci-164129cf-screenshots.patch`, SHA-256
`4329b2f85c1ca0b9a9d60f4db4d4060c56422d6ac9d65f8b6ce450ca7212d0d3`.
Root verified every current image hash, the exact path list and binary patch
against the working diff, then reverse-applied only that patch. HEAD stayed
`164129cf` and the checkout returned clean. This final progress update changes
documentation only. Publication and exact-head hosted acceptance remain required;
no merge, deployment or additional complete-operation credit is claimed.

People PR #2641 is published in stack `2634`, clean and pushed. Exact-head hosted
run `38642987849` completed with 31 successful and 14 skipped checks. CI Success
`116128762620`, strict lint, generated-client, Swift, Kotlin, Android and coverage
checks passed. Hosted Devin Review passed; its pagination comment was clarified
against the exact schema-only proof and the separately executed care pagination
test, then its exact thread was verified resolved. No code or acceptance credit
changed. Artifact attachment again failed because the app tool could not load.
Coverage artifact `11930924743` has verified SHA-256
`aefda01a051137142c7d105dbebb677e2a13ff37885b3bbbbdc7eafac639e37c`.
Its clean merge revision `ab979092` has the same tree as published `4229289b`.
The combined suite now covers 7/7 instrumented lines of `resource::operation_error`;
this is not all critical application logic. Retained me/UI additions remain 5/7,
with two defensive adapters uncovered. Raw API coverage is 92.41%, models 73.53%
and general source 63.65%, including inline tests; remaining targets are explicit.

The next isolated `codex/s05-i01-profile` branch follows published `4229289b`.
Its single API writer owned profile/avatar tests and their narrow implementation,
then explicitly transferred the frozen files to the coordinator for evidence and
integration. The verifier owns the serial build lane and disposable resources.
The six-operation profile/avatar journey fits one bounded PR; it retains existing
socket-level storage/retirement tests and adds full-boot TestServer acceptance.

Runner RED `92059` passed 47 cases and failed the new storage-provisioning case:
requests started PostgreSQL without the existing owned RustFS service. The narrow
correction selects existing storage for requests. Intermediate `79393` proved
that regression fixed but exposed two old cleanup-sequence expectations; updating
those expectations preserved unrelated-resource and cleanup assertions. Final
runner `61210` passed all 48 cases, including success and failing-Cargo cleanup.
No new application runner or database credential path was introduced.

Profile `85092` compiled ten cases: seven aborted on a test-only nonexistent
`api_idempotency_keys.path` column, so those are not behavioural RED. The fixture
now uses preserved `request_path`. Source-backed test corrections account for
recorded denial audits and required DOB; missing-DOB 422 remains an explicit case.
Avatar `8111` passed five and failed two raw-upload-byte assumptions: production
intentionally normalises PNG encoding. Independent format, dimensions and RGBA
checks now verify image preservation without deriving expectations from production.
Avatar `13691` passed all seven actual owned-storage request cases.

Accurate profile RED `98547` passed two and failed eight on missing private headers
and replay/internal-error metadata. The correction applies the existing no-store
response layer to the profile MethodRouter, covering authentication/permission
errors as well as successes. It adds existing idempotency-key/409 definitions to
PATCH/PUT and existing 500 ErrorEnvelope definitions to GET/PATCH/PUT, preserving
runtime statuses, permissions and audit behaviour. No shared header promise changed.
Final profile `81908` passed all ten cases; inventory `97568` passed 98 with final
completion ignored; strict all-target lint `81559` passed. Generated-client verify
`19154`, Swift `63166` and Kotlin `39832` passed, with native pins unchanged.
Logs remain under `/private/tmp/s05-i01-evidence/profile-*` and `avatar-*`.
Source commit `4bd788c1592a29b505217a4e92e4e2a6116bf0d8` pins the tested behaviour
before evidence promotion. On that committed source, fresh existing care-profile
job `37711` passed all 16 selected tests and care-avatar job `20183` passed all 15.
Both owned fixtures cleaned up. The source diff is 1,253 lines against published
People `4229289b`; no native pins changed. The API writer transferred source,
evidence and runner ownership to the coordinator after their frozen corrections.
Evidence guard RED `23814` compiled and executed its one selected test, rejecting
the proposed profile proof before inventory promotion. This was an assertion
failure, not a compiler or fixture failure. The guard pins the six operation rows
to source `4bd788c`; model-only Gravatar coverage is excluded from route credit.
Guard GREEN `20748` passed its one selected test. Full inventory `23533` passed
99 tests with one ignored and one filtered; runner fixtures `55614` passed 48/48.
The six new partial rows require 17 exact request and 30 route-relevant care test
identities. Their gaps retain malformed household-ID handling, profile write
foreign/absent targets, PUT malformed JSON and overlap, and JPEG/WebP acceptance.
The ledger is now five complete, 13 partial and 104 unverified; 117 operations
still fail final completion. Runtime and request cases remain identical to `4bd788c`.
Strict lint `79296` rejected only the new guard's complex tuple annotation and
unnecessary borrow. The test-only correction names the proof slice type and
passes the existing document reference directly; formatting passes. No runtime,
contract, proof assertions or inventory values changed for this correction.
Corrected all-target lint `90937` passes; the final focused guard `25342` passes
its one test. Dispatcher `44888` executed 99 passing request checks and 524 passing
care tests with two ignored. Its final selection passed 99 and failed only
`api_v1::acceptance`, correctly rejecting all 117 incomplete operations.

Frozen candidate `f4feb6224ccf9b0d9d029f52c219a23b2c9403f5` has 1,748 changed lines
against published People `4229289b`. Independent Devin job `42687`, session
`florentine-trunk`, model SWE-2 Max, accepted it in nine steps without required
corrections. Review packet SHA-256:
`bee743c38990850aac33b933879a039d3b4ce1ebb3417697aa650ad069656e77`.
Full `task ci` job `6669` passed on that exact candidate: policy 138, runner 48,
Rust units 115, requests 99 with two ignored, care 508 with two ignored and 16
filtered, all other integration binaries and 744 desktop/mobile browser cases
(19.3 minutes). Owned fixtures cleaned up and no Task job remains live.
The 40 generated screenshot changes were preserved as
`/private/tmp/s05-i01-evidence/profile-avatar-ci-f4feb622-screenshots.patch`, SHA-256
`c75a429ffab53e3b3be7ad5bed9b0f506bd28c954957775c16961f50d74648d6`.
The coordinator verified the exact paths, current hashes and binary patch before
reversing only that patch; all 1,141 pre-CI screenshot hashes match again.

The coordinator owns integration/publication, Sol owns implementation/tests, and
the verifier owns disposable fixtures and the serial build lane. Profile/avatar
PR #2643 is published at `9eb729dd`, 1,757 additions plus 11 deletions. Its final
commit changes only the verified progress document; docs job `66908` passed.
Pull/rebase and push succeeded, and the clean branch tracks origin in stack `2634`.
Hosted Devin review passed in 1m28s with no review threads; all 45 exact-head checks
completed, 31 successful and 14 skipped. Coverage artifact `11935463623`, job
`116154687913` in run `38652555229`, has a verified complete clean source manifest.
Its SHA-256 is `231d4bee02540576b7baf564113ed11463edf03207f0d2fd1250e3c4fe9abb13`.
Merge `f5698994` has parents `4229289b` and `9eb729dd`; its tree equals the published
head. All four added runtime lines and the named critical error mapping's seven
lines execute. Raw API coverage is 92.44%, general 63.75%; the called avatar
controller is 84.87%, and two retained defensive adapters remain uncovered.
These report-only figures do not establish all coverage targets or operation
acceptance. Full details remain in `profile-avatar-hosted-coverage-9eb729dd.md`
under `/private/tmp/s05-i01-evidence/`. All eight layers are unmerged and undeployed.
Whole-API acceptance still rejects 117 incomplete operations.

The next three-operation settings draft is isolated on
`codex/s05-i01-household-settings`, based on published `9eb729dd`. Propagation of
the final parent documentation preserved the exact two owned draft files through
a temporary autostash, which applied successfully. Formatter `9480` passed.
The first TestServer run `40624` compiled/executed 13 cases: 12 passed and one
failed on missing GET 500 metadata. Before its schema lookup, the failing case
proved GET/PATCH/PUT all return `500 internal_error`, JSON with coherent request
IDs and no data, with unchanged domain state after controlled audit failure.
It does not assert private cache headers for these responses. The API writer owns
the narrow three-operation 500 declarations, their exact inventory metadata and
full-schema hash; runtime, security, shared schemas and native pins stay unchanged.
The coordinator owns evidence promotion, progress and Git integration.
Initial settings GREEN `36695` passed all 13 cases; inventory `10527` passed 112,
strict lint `49766` passed, generated-client determinism `80617`, Swift `83532`
and Kotlin `19828` passed. Native pins stayed unchanged. Before source pinning,
the writer strengthened the same 13 cases with individual audit actor/provenance,
request IDs, exact version field changes, documented absence of Household sync
events and broader body/type validation. Enhanced focused `12338` passed all 13,
inventory `9556` passed 112 with one ignored and one filtered, and strict lint
`52079` passed. These extra assertions confirm existing runtime behaviour; no
additional behavioural RED or production correction is claimed. The three
operation evidence rows remain unverified until independent pinned promotion.
Source `86e81214` recorded that verified point; fresh care administration `94785`
then passed all 14 selected cases, of which six have settings-route assertions.
Before evidence promotion, the writer extended both keyed-write cases to prove
mobile replay and denial of the same saved key after role demotion, membership
withdrawal and API/app/mobile credential revocation, without reviving credentials.
Focused `73836` passed all 13, strict lint `78557` passed and inventory `46911`
passed 112 with one ignored and one filtered. These assertions confirm existing
behaviour; runtime, OpenAPI and native inputs stayed unchanged. SDK checks remain
valid for the unchanged API inputs. The stronger immutable test source is
`fed7aa3f503229fffa29b2baab45a856def6f6e2`, 1,049 changed lines against the
immediate profile base. Fresh care administration job `69348` passed all 14 on
that clean revision. Exactly six contain settings-route assertions; unrelated
administration cases earn no settings evidence. Independent evidence guard RED
`45064` compiled and executed one case, failing because the existing validator
rejected the proposed GET evidence before any inventory row was promoted. The
writer completed the narrow validator, three-row inventory and fake-runner
integration. Guard GREEN `13346` passed exactly one case, and inventory `77554`
passed 113 with one ignored and one filtered. The ledger now records five
complete, 16 partial and 101 unverified operations; 117 remain incomplete.
Runner fixtures `27536` passed all 48, and strict all-target lint `21355` passed.
Settings retain explicit malformed-ID/authentication-order, exact-route
lockout/expiry, in-flight membership withdrawal and mixed PUT attribute gaps.
Exact dispatcher `47127` passed 113 request inventory cases and 524 care cases
with two ignored, then passed 113 requests and failed only the final acceptance
case for 117 incomplete operations. All 13 new settings cases and the evidence
guard executed in both request phases, and all six relevant care identities
passed. This is a deliberately closed completion gate, not whole-API acceptance.
Clean evidence candidate `c40b44b9ad76f9d0f6b5aa12de3290e9d82ceccb` has 1,287
additions plus nine deletions against published profile base `9eb729dd`.
Independent Devin job `69596` accepted it without corrections: session
`gorgeous-layer`, SWE-2 Max, nine steps. Its immutable packet SHA-256 is
`b8bab6e5754c3c461afd45eb442b25741c6cb4d6808e78b3c936a469a87fcb34`.
Full CI `49704` passed on that exact clean revision: 115 Rust units, the 16
selected care cases, 508 broad care cases with two ignored and 16 filtered,
113 API requests with two ignored, remaining integration shards and all 744
desktop/mobile browser cases in 19.5 minutes. The original dm+d archive queue
regression passed again. No production changes were needed for settings.
CI generated 42 tracked PNG differences. The coordinator independently verified
the exact paths, current hashes, byte-identical binary patch and reverse
applicability before restoring only those differences. All 1,093 original PNG
hashes match again; the reviewed source is clean. The preserved patch SHA-256 is
`deef384c5c142cfb7328912a5974fe7e35f7f69daff559ead8b1551d9469cd42`.
The coordinator owns integration, Git and publication; Sol owns implementation
and tests; the verifier owns disposable resources and the serial test lane.
Final documentation checks and publication follow this verified point.

The first published settings head is `a49eaef9`, PR #2644 in stack `2634`,
1,319 changed lines. Final docs `41536`, pull/rebase and push passed. Hosted Devin
completed in 1m19s but left an actionable finding: the client barrier permits
sequential processing and therefore does not prove handler overlap. The
coordinator verified that gap and held dependent membership execution. The
writer strengthened only the two existing concurrency cases' shared helper:
a dedicated transaction holds the household row, API/app requests enter the
booted router, and both distinct runtime-role lock waiters must be observed
before release. The bounded database-condition check uses no blind sleeps or
production hooks. Existing write/replay/conflict assertions remain unchanged.
This is a stronger behavioural characterisation, not a claimed runtime defect
or production correction. Focused execution, refreshed evidence, same-session
review and full CI are required before the correction is published.
The stronger test source `63a87ceef59462334a1308c79ece5672030b3c101a33660f21a7e4e1c4d585d7`
passed focused `12085` (13/13, including both lock observations), fresh care
administration `34102` (14/14) and strict all-target lint `29095`. Runtime,
OpenAPI and native pins are unchanged. The independent evidence revision is
being refreshed from this tested source before the same review resumes.

The next membership family is isolated on `codex/s05-i01-household-memberships`,
with 17 test-only cases prepared for GET, PATCH, PUT and DELETE. Formatting and
diff checks pass; no runtime changes, contract edits or membership test jobs
have run. Its first full-module RED waits for the released verifier lane and
propagation of the final published settings parent. It earns no operation credit.

Entries below retain historical revisions, failed attempts and publication states.

Historical prerequisite failure, 14 October:

- Frozen execution-proof `8733febee` CI job `29550` passed tooling checks, 110
  Rust units, documentation, strict checking and preceding integration binaries.
  Care API then reported 507 passed, one failed, two ignored and fourteen
  filtered. `nhs_dmd_queue_imports_verified_archive_and_cleans_it_after_completion`
  failed at `tests/care_api/nhs_dmd.rs:603:38`: the extra `store.get(&key)` returned
  `Unavailable` after upload and its own verification had succeeded. This
  narrows the failing phase; it does not establish the provider error or cause.
- This test also failed in the original hosted merge coverage run, before its
  unchanged retry passed. The recurrence requires diagnosis rather than another
  unchanged retry. OpenDAL errors and timeouts are currently folded into
  `Unavailable`; the public Task surface has no owned storage-log command, so
  provider diagnostics were not captured. No raw Compose fallback was used.
- After the care failure, the verifier proved the owned Task/browser tree and
  sent SIGTERM only to browser runner PID 70536 at roughly 7/744. The existing
  handler/finally cleanup removed owned processes and fixtures without error.
  Outer exit 201 reflects the underlying Task failure; cancellation did not
  cause the already-recorded care failure. Full log:
  `/private/tmp/s05-i01-evidence/contract-execution-full-ci-8733.log`.
- Three generated screenshot diffs were preserved at
  `/private/tmp/s05-i01-evidence/contract-execution-ci-8733-screenshots-before-cancel.patch`,
  SHA-256 `a9504f966c1026ff21af5f0eaa7696cedd778fae0adad490155a538524765b7a`.
  Root verified its exact paths and restored only those files after exit;
  execution-proof source is clean again. No acceptance credit is added.
- The owner explicitly transferred this work: “we're now the only one working
  on this issue. Fix it.” The new managed prerequisite checkout is
  `/Users/damacus/.codex/worktrees/s05-i01-dmd-storage/med-tracker`, branch
  `codex/s05-i01-dmd-storage`, based on merged main `1800d858e`.
  The API writer owns the storage adapter and its tests; shared operator changes
  require diagnosis first. Root owns integration/Git/progress; the verifier owns
  the idle heavy lane and will run a frozen diagnostic before any unchanged
  broad retry. No production storage correction has been made.
  The account correction remains frozen and independently
  reviewed; its GREEN test and further acceptance expansion wait for this
  prerequisite. The four account operations can fit one coherent subsequent PR
  (projected 700–1,100 changed lines), rather than a separate partial fix PR.
- Temporary test-only storage diagnostics are frozen at base `1800d858e`,
  test SHA-256 `868c72c2a1407a13ce8ba052c052b193725f836b9e131cfbd4dab9560a78f072`.
  They replace only the extra test read with the same reader/stream/collection
  stages and timeouts, exposing safe underlying stage and error classification
  before fixture cleanup. No retries or production behaviour change. This is
  diagnosis, not adapter acceptance, and must be removed before final review.
  One full `care_api` Task run with two test threads is authorised because the
  smaller dm+d module previously passed while broader runs reproduced failure.
  There is no automatic retry or operation credit from this diagnostic.
- Diagnostic job `42467` stopped at compilation: the storage module is
  crate-private and byte-stream chunks expose `std::io::Error`, not the directly
  assumed OpenDAL error type. No care test executed, so it yields no provider
  evidence. Log:
  `/private/tmp/s05-i01-evidence/dmd-storage-full-care-diagnostic-1800.log`.
  Cleanup completed. The duplicate read helper is being removed; temporary safe
  instrumentation will instead expose the original adapter's error/timeout
  mappings without changing retries, visibility, configuration or assertions.
- Original-adapter diagnostic `64200` completed with 522 care cases passed,
  two ignored and exit zero in 326.72 seconds. It retained source SHA-256
  `848d58f37c80174258489b5573cd56beccaf3f99d13bae61d625dde8222a8276` and cleaned
  up normally. No classified read error appeared, so neither cause nor correction
  is established. Log:
  `/private/tmp/s05-i01-evidence/dmd-storage-full-care-diagnostic-1800-v2.log`.
  No automatic retry follows. The writer will remove temporary instrumentation
  and establish a controlled regression against the actual S3 operator factory:
  independently configured runtimes with a provider-response barrier, checking
  whether closing one runtime interrupts the other's active read. The global
  OpenDAL HTTP client is a source-backed hypothesis, not a confirmed explanation
  of the historical failures. Production behaviour remains unchanged.
- Controlled factory regression job `48333` compiled and executed one test,
  which failed at the second read with `Unexpected`, `temporary=true`.
  The first read succeeded; the second response body started on the same
  accepted socket; then the first runtime was closed. All three fixture threads
  joined before the final assertion. This is behavioural RED, not a setup or
  timeout failure. Source test SHA-256:
  `e221656ff168c9e0fe8b873f047302468ec6823fa37d6719a01cb6ae23127f96`.
  Log: `/private/tmp/s05-i01-evidence/dmd-storage-runtime-lifetime-focused-1800.log`.
  Temporary stderr instrumentation is removed exactly. The owner authorised
  a narrow correction in the actual shared S3 factory using maintained OpenDAL
  transport scoped to each operator, without retry masking or changes to API,
  timeouts, content types, storage names or data. The required transport package
  already exists in the lockfile; only a direct root dependency edge is added.
  Historical CI failure causality remains an inference until further evidence;
  the independently reproduced transport lifetime defect is established.
- The exact previously-red regression passed in locked public Task `62784`:
  one passed, 110 filtered, documentation phase passed and final exit zero.
  Frozen corrected hashes are storage source
  `fa9cda200debefea1fa93da4f37f4225af17aee340f084366fd24fd3e172a6c1`, manifest
  `b2aa82f771e6c87a1812abf96dd38c6aed17f14c06e807f6fdaf13723ddacba5`, lockfile
  `78def61ebd82aca3aeb9550b53fe9d5dc4f2864f500819dee19e7e247284d8ac`.
  Log: `/private/tmp/s05-i01-evidence/dmd-storage-runtime-lifetime-focused-green-1800.log`.
- The shared factory now uses a fresh maintained ReqwestTransport per operator,
  configured with OpenDAL's existing operation-context API. No custom HTTP/S3
  protocol, retries, scattered configuration, timeout or MIME changes were added.
  The direct dependency is an existing locked package; no package versions changed.
  Devin SWE-2 Max review `33298` is active; inline packet/output/session files
  are retained under `/private/tmp/s05-i01-review/dmd-storage-*`.
- To keep published heads verifiable, the already-reviewed 26-line reminder
  clock correction is included in this same bounded prerequisite PR: root
  fast-forwarded the storage branch to existing `caf79` while preserving the
  storage changes. The combined PR still targets main `1800`; the execution-proof
  tooling will follow it, then account acceptance. The unpublished separate clock
  checkout is retained for recovery. Its unchanged independent review and focused
  results remain valid; the storage review will also assess the combined scope.
- Actual original dm+d application/storage case verification `32036` and avatar
  lifecycle verification `89283` both passed sequentially: each one named case
  executed, 523 filtered and final Task exit zero. Logs:
  `/private/tmp/s05-i01-evidence/dmd-storage-real-archive-case-caf79.log` and
  `/private/tmp/s05-i01-evidence/dmd-storage-real-avatar-case-caf79.log`.
  Both owned runs cleaned up normally. These prove real AppContext storage,
  object content types and lifecycle boundaries remain functional; they do not
  establish the historical intermittent failure's exact cause. Source is frozen;
  unrelated generated `.serena/project.yml` drift is preserved and excluded from
  the candidate. No full cumulative success or publication is claimed yet.
- Combined prerequisite is committed as `923d5b63098a61ed4a9df0dfd4f27ae348d0b26a`
  on `codex/s05-i01-dmd-storage`, targeting main `1800`: 496 changed lines across
  five files, including both tested corrections and this existing progress record.
  Same independent SWE-2 Max session `successful-dollar` accepted code in `33298`
  and final combined scope in `14601`, both exit zero; no code corrections remain.
  The final review confirmed the sole production S3 factory and the real GREEN
  and provider proofs. Execution/hosted gates remain mandatory.
- Execution-proof tooling now follows at clean
  `630a7f016d364becfb682b855a7da0c57c0b7f7c`, based on `923d5b63`; its immediate
  diff is three reviewed JS files, 477 additions and 21 deletions. All three
  reviewed hashes remain unchanged. The sole rebase conflict was this coordinator
  progress file, resolved by retaining the newest preserved lower-layer record.
  Every Rust/application/config/asset file matches the prerequisite exactly.
  Fresh cumulative CI is job `61813`, log
  `/private/tmp/s05-i01-evidence/contract-execution-full-ci-630a.log`.
  Its 111 Rust units, request suite (20 passed, one intentionally ignored) and
  full care suite (508 passed, zero failed, two ignored, 14 filtered) passed.
  Both previously failing dm+d cases and avatar lifecycle passed. The 744-case
  browser stage passed all 744 cases in 25.4 minutes. The complete Task exited
  zero on this exact candidate. Hosted checks and publication remain separate.
  CI generated exactly 42 tracked screenshot changes and no other changed or
  untracked paths. Their binary patch is preserved at
  `/private/tmp/s05-i01-evidence/contract-execution-ci-630a-screenshots.patch`
  (SHA-256 `e87e3fbca5dd8afe7a3e1b0318aa66b90fb7e75e1dfc4bd0a8c8452246386919`);
  the exact paths are in `contract-execution-ci-630a-screenshot-paths.txt` beside it.
- Published through `gh stack link` as stack `2634`: prerequisite PR
  [#2632](https://github.com/damacus/med-tracker/pull/2632), base `main`, exact head
  `923d5b63098a61ed4a9df0dfd4f27ae348d0b26a`, 486 additions plus ten deletions;
  execution PR [#2633](https://github.com/damacus/med-tracker/pull/2633), base
  `codex/s05-i01-dmd-storage`, exact head `630a7f016d364becfb682b855a7da0c57c0b7f7c`,
  477 additions plus 21 deletions. Both are attached to this chat. No merge or
  deployment occurred. Exact-head hosted checks are active (root CI runs
  `38579974030` and `38579993324` respectively). Initial generated titles failed
  title validation; both titles were corrected to Conventional Commits and
  replacement title checks passed. Hosted acceptance is not claimed.
- Published-head Devin review found no issues in PR #2632. PR #2633's sole thread
  asked whether incomplete inventory should fail the final completion gate.
  Root checked the current runner and acceptance assertion, confirmed this is
  explicitly required by the plan and replied with the real dispatcher and
  cumulative CI evidence. Reply
  [4272091290](https://github.com/damacus/med-tracker/pull/2633#discussion_r4272091290)
  was created before resolving exact thread `PRRT_kwDOPAWvwc6sXMlM`; a fresh
  thread read confirmed `is_resolved: true`. No code correction or weaker gate
  was introduced. Hosted execution/coverage jobs remain active.
- PR #2632 hosted coverage job `115907098466` passed: 111 units, 20 request cases
  (one ignored), and all 522 care cases (two ignored). Artifact `11902762626`
  is preserved as `/private/tmp/s05-i01-evidence/loco-rust-coverage-2632-923d.zip`,
  SHA-256 `bf456b3aad1916f3cfd33a1178fe729cac7c48c3e6bc454bdc784afea78e6ee1`.
  Its clean manifest identifies merge-test source
  `f80b89423a1529aac4ea037b2df45b23fce4e20f`, parents `1800` and `923d`;
  Git comparison confirms no source difference against the published head.
  Extracted manifest and LCOV are under
  `/private/tmp/s05-i01-evidence/hosted-coverage-2632-923d/024c7554-2371-4c0e-bf31-5786fc15dca7/`.
  Report-only totals: application 39,616/62,582 lines (63.30%); API controllers
  6,877/7,448 (92.33%); storage production factory 100/101. Its sole uncovered
  line is pre-existing virtual-host configuration at `src/models/storage.rs:112`;
  every added transport/context runtime line is covered. Inline regression lines
  are counted separately, not credited as business logic. Whole-application and
  operation-level acceptance remain incomplete; hosted browser gates are active.
- Both published prerequisites now pass exact-head hosted acceptance: PR #2632
  head `923d`, CI roll-up job `115916215172` in run `38579974030`; PR #2633
  head `630a`, CI roll-up job `115914704499` in run `38579993324`. Latest title,
  lint, security, unit, integration, coverage and browser gates passed; older
  auto-generated-title failures were superseded by passing corrected-title runs.
  Devin review has no unresolved findings. Neither PR is merged or deployed.
  Highest-head coverage artifact `11903381637` is preserved as
  `/private/tmp/s05-i01-evidence/loco-rust-coverage-2633-630a.zip`, SHA-256
  `54009d32e20402ecea8b8675cefec360b687bd7179c4064f26b489fe1809eeb7`.
  Its clean source manifest is merge-test `80076e371dde06bcf9f4a16414d406768948b7e2`;
  Git comparison confirms no source difference from published head `630a`.
- Account correction and unrelated Serena drift were preserved in a named stash
  before propagation, then restored on `630a7f016`. Both controller and test hashes
  remain unchanged. Stash identity is retained at
  `/private/tmp/s05-i01-evidence/account-before-combined-prerequisite-stash.txt`.
  No old/unknown stashes were removed. Its GREEN request test subsequently passed
  in job `1890`; no complete API operation credit has changed.

Account regression update, 14 October:

- Attempts `90800` and `57428` failed test compilation and earn no behavioural
  evidence. After test-only type corrections, job `14758` compiled and executed
  `api_v1::account::malformed_delete_session_authenticates_before_identifier_validation`.
  Nonnumeric, overflowing and invalid UTF-8 identifiers returned 400 for both
  anonymous and authenticated requests, before authentication could decide the
  documented 401/404 outcomes. Zero and negative identifiers already returned
  401/404. The fixture cleaned up normally. Genuine RED log:
  `/private/tmp/s05-i01-evidence/account-malformed-delete-red-8733-v3.log`.
- The writer owns only the existing test and a narrow controller adapter that
  authenticates before handling rejected path extraction or integer parsing.
  Valid identifiers retain the existing model behaviour. Root retains OpenAPI,
  inventory and progress ownership. No complete operation credit is added.
- The verifier owns the sole heavy lane for fresh cumulative CI on clean
  `8733febee`, job `29550`, log
  `/private/tmp/s05-i01-evidence/contract-execution-full-ci-8733.log`.
  The account GREEN run waits for that lane. Rust/application/config
  sources are identical between the clock and execution-proof candidates.
  Each published PR still requires checks for its exact head.
- The narrow controller correction is frozen after `task fmt` and diff checking:
  controller SHA-256 `aec3cdafd963e7b18e894837835b339f9524fd04e787406f71c75d46e5fd54d9`,
  test SHA-256 `c2a61c45a0180f1a30dfb2e612b3741c2234eb8c94b6de957dc1a28dfc716748`.
  Independent Devin SWE-2 Max session `important-platypus`, job `63546`, exited
  zero and accepted with no required corrections. It retains real GREEN as a
  verification requirement and recommends actual positive-ID revocation proof
  in the already-planned account acceptance tranche. Packet, output and session
  export are retained under `/private/tmp/s05-i01-review/account-malformed-*`.
  Subsequent genuine GREEN job `1890` exited zero after compiling and executing
  this exact case: one passed, zero failed, 21 filtered, 2.80 seconds. Both frozen
  source hashes match and the owned fixture cleaned up normally. Log:
  `/private/tmp/s05-i01-evidence/account-malformed-delete-green-630a.log`.
  The writer now owns the full four-operation account test candidate on this
  branch; root retains OpenAPI, inventory and progress. New production corrections
  still require their own failing behavioural tests. No full operation is yet
  credited from this single boundary regression.
- Account metadata test-only candidate produced genuine RED in job `73953`:
  one named test compiled and failed because `GET /auth/households` had no 403
  response reference, rather than the required `AccountForbidden`. Log:
  `/private/tmp/s05-i01-evidence/account-scope-denial-metadata-red-630a.log`.
  Root then applied the user's approved missing 403 entries to the three
  authenticated account operations, using a narrow response with the existing
  `ErrorEnvelope` and exact OAuth challenge. No runtime security or native pin
  changed. OpenAPI SHA-256 is now
  `88749db9cdffb0aa8ca66d03c59d8487d25c1fadfaa05093e485495aab6b18dc`.
  Only the three corresponding inventory response/schema/header records changed;
  they remain unverified until actual acceptance execution. GREEN is queued
  after the writer updates the owned test pin and removes its unused import.
- Focused framework contract dispatcher job `34551` executed all eight account
  tests: six passed, two failed, 21 filtered. The missing-403 metadata test is
  GREEN. Neither failure established a runtime defect: one challenge expectation
  incorrectly inferred a comma where the actual library response uses a space;
  the SMART fixture omitted authority fields required by a PostgreSQL constraint
  and never reached its request. Log:
  `/private/tmp/s05-i01-evidence/account-focused-contracts-green-630a.log`.
  Root corrected only the new header example to the observed response, giving
  OpenAPI SHA-256
  `2e974738ac88e583452ac342690814e651226c93cf85571bf89908cd0a200758`.
  Writer owns test corrections and remaining independently aged mobile/credential
  boundary assertions before a fresh freeze. Controller remains unchanged;
  complete operation credit remains pending.
- Follow-up focused account job `32767` compiled and executed all eight cases:
  seven passed, one failed, 21 filtered. The observed challenge and SMART authority
  fixture corrections passed, as did independent mobile age/inactivity and stale
  credential boundaries. Remaining failure is test-only: SMART logout correctly
  revoked the grant, but the assertion queried the mobile `versions` audit table.
  Existing non-mobile revocation writes `security_audit_events`; writer is
  correcting that assertion and replay count. No production correction is
  justified. Log: `/private/tmp/s05-i01-evidence/account-focused-contracts-green-v2-630a.log`.
- Corrected account candidate, requests SHA-256
  `5ae2af17d81861fae2145be16fcf8fcd78d5a1fe38ccbe8486e2a841ff34b796`, passed
  framework dispatcher `92761` (eight account cases), inventory dispatcher
  `37623` (28 passed, one intentionally ignored final acceptance), and existing
  account schema/transaction cases `7699` (12 passed, 512 filtered), sequentially.
  Owned fixture cleanup completed normally. Logs are respectively
  `/private/tmp/s05-i01-evidence/account-focused-contracts-green-v3-630a.log`,
  `account-full-api-v1-inventory-630a.log`, and `account-existing-care-api-630a.log`.
  Historical simultaneous-revocation evidence belongs to the legacy runtime;
  writer is adding its missing Loco TestServer proof before full operation credit.
  The unchanged generic account projections are separately tracked apparent
  drift #2588, not an invented #2592 exception; this bounded correction preserves
  already-correct responses and introduces no new response component.
- Added the missing current-runtime concurrency proof without production changes:
  request source SHA-256
  `9219c243db5d7ac572ecc272a88d8f014fb93e44b322c4f20193dacd658a4d09`.
  TestServer case `concurrent_account_revocation_and_logout_commit_one_effect_each`
  uses a controlled Tokio barrier for stored and mobile session revocation and
  app-token logout; job `35496` passed one named case. Full inventory dispatcher
  `18199` then passed 29 cases with one intentionally ignored acceptance gate.
  Both owned runs cleaned up normally; route reconciliation remains 121/122.
  Logs: `/private/tmp/s05-i01-evidence/account-concurrent-revocation-630a.log` and
  `account-full-api-v1-inventory-concurrency-630a.log`.
  Same independent SWE-2 Max session `important-platypus` resumed as job `78959`
  for the full four-operation candidate, inline controller/domain/auth/audit,
  fixtures, schemas and tests. Packet and export are
  `/private/tmp/s05-i01-review/account-complete-review-*`. Review is active;
  complete evidence mapping and full cumulative gates remain pending.
- Further tests-only request source `dcec718f` reuses the recursive contract
  helper for actual account error bodies and checks mobile foreign-account and
  expired-login target isolation. Genuine HTTP RED job `82913` compiled and
  executed one case: foreign target returned 404, maximum-age and inactivity
  targets each returned 204 instead of 404. Listing had correctly excluded those
  targets. The Rails reference uses the same lifetime-filtered `sessions_for`
  lookup for listing and revocation, establishing the required parity behaviour.
  Log: `/private/tmp/s05-i01-evidence/account-mobile-target-lifetime-red-630a.log`.
  The fixture queried persistence/audits but failed before asserting or printing
  them; the RED log does not establish those values, and cleanup has completed.
  Writer now owns the narrow model/controller correction, reusing configured
  `AuthenticationLifetime` and preserving stored-session namespace, token-expiry
  distinctions, concurrency and transactional audits. No OpenAPI change is needed.
  Existing independent review uses saved source `9219c243`; the corrected source
  and actual RED/GREEN will require a same-session follow-up before publication.
- Earlier full account review `78959`, same SWE-2 Max `important-platypus`,
  exited zero and accepted saved source `9219c243` with no required corrections,
  retaining operation mapping, cumulative CI and hosted gates. Counts are
  unambiguous: `92761` ran all eight then-existing account tests including metadata;
  subsequent `35496` added the ninth concurrency case, which `18199` also executed.
  The later discovered lifetime correction is outside that accepted snapshot and
  will receive the required same-session follow-up, not inherit its acceptance.
- Minimum lifetime correction reuses one model deadline calculation for listing
  and revocation, with settings supplied by the controller's `AppContext`.
  Frozen model/controller/request SHA-256 values are respectively
  `800b7665a0e31799b8bff426ae215d311884d2e433b3252236392a83e0eb1785`,
  `5f9a7ce2b8bcbb5fc3d9a046bf6eee349f7e9e471b32f0948b00d129d85b2a69`,
  `6c577cbf44c82a949ec162df723d1eea19ea79f2e627052f1085fe1524139397`.
  Genuine GREEN `75274` passed its one named case: foreign/max-age/inactive targets
  returned 404, stayed active and produced no revocation audit; a token-expired
  but login-active target returned 204 and produced exactly one audit. Full
  inventory `4188` then passed 44 tests with one intentional acceptance ignore;
  imported recursive schema helper tests are compiled and executed too.
  Logs: `/private/tmp/s05-i01-evidence/account-mobile-target-lifetime-green-630a.log`
  and `account-full-api-v1-inventory-lifetime-630a.log`. Both owned runs cleaned
  up normally. Existing care-account verification `20402` is active; no operation
  credit, full account CI or publication is claimed yet.
- The shared report renderer rejected unsupported local-check states `complete`
  in `s07-i01` and `s07-i08`, and an unevidenced publication claim in `s07-i08`.
  Root preserved the prior JSON snapshot and conservatively recorded those
  unknown dimensions as pending/unpublished, retaining the exact repair reason
  in limitations. No item gains or loses a completed point: both snapshots
  contain 50 completed items among the same 77 IDs. HTML and derived Markdown
  now render from the authoritative JSON, including this storage prerequisite.
- Read-only next-area reconciliation identified two coherent follow-up PRs:
  account credential/session acceptance for the four authenticated or logout
  operations, then public capabilities acceptance. Existing `care_api`
  `account_sessions::*` and `oauth_server` contract tests retain their real
  transport assertions, but need observed exact-name dispatcher execution before
  credit. New Loco request assertions must cover account-specific denied/error
  envelopes, scope challenges, stale authority, SMART rejection, empty logout
  responses and shared 429 behaviour. Capabilities still needs truthful flags
  and complete nested schema/URI checks. The approved account 403 OpenAPI entries
  will accompany that contract evidence; native pins remain unchanged.

Retained execution-proof CI failure, 14 October:

- Frozen `90593aaf` full CI job `91033` passed 105 units, tooling, documentation
  tests and strict Rust checking, then care API reported 506 passed, one failed,
  two ignored and fourteen filtered. Existing
  `browser_reminder_obeys_retry_after_and_attempt_limit` expected `invalid` but
  observed `pending` at line 1027. No API operation acceptance was added.
- The test floors its controlled clock to a minute, but forced a retry with
  PostgreSQL's live `now() - interval '1 second'`, then selected candidates at
  controlled `now + one minute`. A boundary crossing can put the fixture's due
  timestamp after that cursor. The observed run did not capture timestamps, so
  this source inconsistency is not presented as a conclusively proven cause.
- After proving the owned process tree and existing SIGTERM/finally cleanup,
  the verifier stopped only the already-failed job's browser child at 134/744.
  The Task exited and owned processes disappeared without teardown error.
  Four screenshot changes were preserved in
  `/private/tmp/s05-i01-evidence/contract-execution-ci-90593-screenshots-before-cancel.patch`
  and restored. Source returned to clean `90593aaf`.
- The coordinator explicitly transferred only the reminder test path to the
  API writer in managed checkout
  `/Users/damacus/.codex/worktrees/s05-i01-reminder-clock/med-tracker`, branch
  `codex/s05-i01-reminder-clock`, based on current main `c284d03e0`. The completed
  care coordinator had merged #2617 and released the API hold; no active writer
  overlaps this test path. Its test-only correction binds and checks the due
  timestamp against the controlled clock; production reminder code is unchanged.
- Candidate diff: 25 additions, one deletion, hash
  `a28ec6e9c4e5a8b3a7e242937d7cfc0d88546460637a51507aadb90d58d10e35`.
  Independent SWE-2 Max review job `42221` accepted; focused real Task verification
  job `96528` passed, log `/private/tmp/s05-i01-evidence/reminder-clock-focused-c284.log`.
  The subsequent committed-head verification, propagation and current cumulative
  job are recorded above. Publication remains pending the cumulative gate.

Hosted verification update, 14 October:

- Job-only coverage retry `115832966637` in run `38555091097` passed on the same
  verified PR head and merge candidate: 110 unit tests, 20 request tests (one
  ignored) and 522 care API tests (two ignored). Both original dm+d failures
  passed. All applicable hosted checks, including the aggregate CI Success gate,
  now pass. This result does not establish a cause for the first failure.
- Hosted artifact `11890718641` has a complete manifest proving clean merge
  source `a68b371afce223d1a842a9fd37a8901a627e48ec`, fingerprint
  `14974951a73de0983562c49817c1fc7f4833e99f90ea36e8233ba8d486f22771`.
  Local artifact and assessment:
  `/private/tmp/s05-i01-evidence/hosted-coverage-c85-merge-a68/`.
  Affected runtime lines remain 176/176 in the limiter model, 46/46 in its
  controller and 16/16 in its view. Broader raw application lines are
  41,677/66,420 (62.75%); API controllers 7,236/7,929 (91.26%). Inline tests
  remain included in the broad totals; complete clinical logic and operation
  acceptance are not established by these report-only figures.
- Root owns this progress record, OpenAPI and the inventory. The API writer now
  owns account-session tests and, after genuine RED, its controller correction
  in `/Users/damacus/.codex/worktrees/s05-i01-account-sessions/med-tracker`.
  The verifier retains the sole heavy resource lane. The earlier execution-proof
  candidate `90593aaf` and account setup have since advanced as recorded above.

Parent verification update, 14 October:

- Coordinator owns Git, evidence, documentation, review and publication;
  `/root/api_writer` owns implementation and tests; `/root/verifier` owns builds,
  disposable resources and live test jobs.
- Full `task ci`, job `77107`, exited zero on frozen `c85ffa024`: 105 unit tests,
  20 request tests (one ignored), 17 OAuth tests, 507 selected care API tests
  (two ignored, fourteen filtered), remaining integration binaries and all 744
  browser cases passed. Log: `/private/tmp/s05-i01-evidence/rate-limit-full-ci-c85.log`.
  Generated screenshot changes were preserved separately and restored after exit.
- Independent Devin SWE-2 Max session `ambiguous-jupiter` accepted the correction
  in job `32066`. Public review threads are resolved on the published head.
- Coverage job `1360` exited zero: 105 unit, 20 request and 521 care API tests.
  Runtime-only affected lines are 176/176 in the limiter model, 46/46 in its
  controller and 16/16 in its view. Raw application coverage remains report-only:
  39,440/62,339 (63.27%); API controllers 6,877/7,448 (92.33%). These figures do
  not establish complete operation acceptance. Manifest and LCOV are under
  `/private/tmp/s05-i01-evidence/rate-coverage-c85/68b022cc-d88d-4b7d-b280-dc9377a271be/`.
- Hosted run `38555091097` passes all twelve browser shards, API/security/database
  shards, formatting/lint, unit compilation, documentation and generated
  Swift/Kotlin client checks and Android.
  Report-only coverage job `115822215155` failed with `Unavailable` in
  `nhs_dmd_queue_imports_verified_archive_and_cleans_it_after_completion` and
  `nhs_dmd_retries_terminal_cleanup_without_restarting_import`; 520 care tests
  passed, two failed and two were ignored. Local success does not override this
  hosted failure. Source reconciliation established that CI tested merge
  `a68b371afce223d1a842a9fd37a8901a627e48ec`, whose parents are newer main
  `c284d03e0968fe5a9399f3c0fcdc997b4a7ed4b7` and PR head `c85ffa024`.
  Main's merged PR #2620 replaces dm+d's AWS SDK boundary with OpenDAL and adds
  the extra test seen in hosted output. Therefore the local coverage run did
  not exercise the failing combined source. Unchanged c85 focused reproduction
  passed 18 dm+d tests (one ignored), job `36369`, but establishes no cause.
  The verifier's exact-merge reproduction is job `78353`, using managed checkout
  `/Users/damacus/.codex/worktrees/s05-i01-merge-storage-repro/med-tracker`.
  Exact-merge reproduction job `78353` subsequently passed 19 dm+d tests
  (one ignored), including both failures, with clean source and owned cleanup.
  The original failure remains unexplained. After re-verifying the same PR head
  and merge SHA, one job-only hosted coverage retry was started: `115832966637`
  in run `38555091097`. No storage correction has been made; a passing retry
  would not establish the cause.
- The execution-proof layer was initially isolated in managed checkout
  `/Users/damacus/.codex/worktrees/s05-i01-contract-execution/med-tracker`, branch
  `codex/s05-i01-contract-execution`, based on `c85ffa024`. Its fake-resource suite
  passed 46 cases after genuine RED/GREEN for transcript validation, exact
  completeness-gate execution, real should-panic output, writable stdout EPIPE
  and readable Cargo stdout EIO cleanup. The latter uses a scoped test-only Node
  preload at the external process boundary; production has no injected spawner.
  Three JS files add 477 lines and delete 21. Independent session `fate-button`
  accepted all corrections in final job `52653`, after job `28490` identified
  one test-only Linux temporary-path portability correction. Both error tests
  now use the existing portable temporary-directory helper; 46/46 checks pass
  in `contract-execution-portable-socket-green.log`. Real dispatcher verifier
  job `21634` completed with the intended rejection: first proof run 20 passed
  and one ignored, including all five manifest-credited identities; second
  include-ignored run 20 passed and the named `api_v1::acceptance` failed because
  all 122 operations still lack complete evidence. Cargo exited 101, Task exited
  one and its outer wrapper exited 201 solely for that expected gate failure.
  Log: `/private/tmp/s05-i01-evidence/contract-execution-proof-real-gate.log`.
  Owned fixture cleanup finished without errors. This proves real execution and
  rejection of incomplete coverage; no complete operation is credited by this
  tooling layer. Cumulative CI and publication remain pending.

The owner approved completing all 119 documented API v1 operations using
`rust-testing`, upstream Loco guidance and area-based `gh stack` PRs, each at most
2,000 added plus deleted lines against its immediate base. The detailed agreement
is in [the API slice](slices/05-api-integrations.md).

- Baseline: refreshed `main` at `714eaac7a2e981451265edbab1b295096ff12bab`,
  including the recent test-registration, isolated AI-configuration and shared
  fixture-helper fixes. The stack was rebased with `gh stack rebase`.
- Active branch: `codex/s05-i01-contract-harness`, stack rooted on `main`.
- Coordinator: current Codex chat; sole source writer: `/root/api_writer`;
  verification/resource owner: `/root/verifier`.
- Current work: exhaustive operation mapping and Loco-native request acceptance,
  preserving isolated PostgreSQL 18 fixtures and restricted runtime roles.
- Independent reviewer: Devin CLI `swe-2-max`, session `cool-actress`;
  packet and output retained under `/private/tmp/s05-i01-review/`. Its first
  attempt stopped at CLI read permissions without a verdict. Devin accepted the
  corrected source with framework source supplied directly. The same-session
  follow-up also accepted the fixture registration correction after rebasing.
- Focused checks: inventory/harness 12 passed, JavaScript dispatcher 16 passed,
  formatting, documentation and whitespace passed. The complete API group ran
  thirteen tests: twelve passed and its completeness gate failed as intended.
  All 119 operations still require
  complete behavioural evidence. Loco registers 118 documented operations; the
  missing route is `POST /api/v1/households/{household_id}/push_subscription/test`.
  Logs are under `/private/tmp/s05-i01-evidence/`. Coverage is recorded below.
- Root CI on signed source `5f12b5b86a8ad650e59246f52f08783f98184b3d`
  (job `62949`) failed with two transport errors: a five-second app-token GET
  timeout and a mobile recovery HTTP parse error. The other 483 care API and
  661 browser cases passed. Both failed cases passed individually without code
  changes; their causes remain unproven. Original logs and browser trace remain.
- Final root CI passed on signed source
  `11ea37b402fa1583dc35ec768b4e384e5d1bad47`: `task --concurrency 1 ci`, job
  `14255`, exit zero. All 662 browser cases passed; care API reported 486 passed,
  zero failed, one ignored and fourteen filtered. Documentation and whitespace
  checks also passed. Full log: `full-task-ci-11ea37b4.log` in the evidence folder.
  This records a successful serial run, not a proven cause or fix for the earlier
  transport failures. The final commit adds only this verification record.
- Published [PR #2600](https://github.com/damacus/med-tracker/pull/2600) against
  `main` at signed head `b56d4b486b85ced3bbe8e22ffd9bec265c722bb3`: 1,099 additions
  and nine deletions. Full API acceptance remains pending.
  No merge or deployment.
  The owner's GitHub update subsequently merged `main` into this PR at
  `2808f38ce508f2606b6d2d0787301961bffd85d7`, preserving our source and adding
  merged PR #2599's unit coverage and property tests. The local stack now
  preserves that merge. Earlier local CI is evidence for its recorded source;
  [Hosted CI run 38412852801](https://github.com/damacus/med-tracker/actions/runs/38412852801)
  passed on this exact updated revision. The hosted checksum finding was answered
  and resolved after independent same-session Devin adjudication accepted the
  full-document provenance guard; no source change was needed.
- Review corrections: duplicate schema/header substitutions and handlers from
  another route now fail explicit regression tests. Both tests failed before the
  guards were corrected. Final focused log: `green-review-metadata-guards.log`;
  final expected incomplete gate: `review-fixes-expected-incomplete.log`, both
  under `/private/tmp/s05-i01-evidence/`. Same-session re-review accepted both.
- Inventory now includes explicit schema/header/handler references and gaps for
  every operation. Middleware proof uses two requests and persisted source data,
  avoiding reliance on a timed request burst. The initial signed snapshot is
  `c7edacd480b103bfe551884b3d5fc68e61ccf568`; the corrections above were verified
  after that snapshot and remain on the same branch. Rebased source is
  `efb13f2473e9b6c2efa0d53fea914edd4ebcc397` before the fixture registration fix.
  The missing root helper first failed compilation; its two-line registration
  then passed all twelve focused cases, formatting and documentation checks.
  The publication gate recounts the final diff against the immediate base.
- Next action: extend report-only coverage on
  `codex/s05-i01-coverage-baseline`, based on PR #2600, coordinating with #2583.
  PR #2599 closed #2583's unit-coverage scope. This branch extends its existing
  Task/job to integration coverage and reproducible evidence; it adds no duplicate
  coverage pipeline. Overlapping local work was preserved before reconciliation.
  The owner approved correcting two OpenAPI omissions: capability rate-limit
  responses and account OAuth insufficient-scope responses. Apply each correction
  in its owning area, preserving enforcement and leaving native pins unchanged.

Main subsequently advanced to `60e7de5d` (PR #2604), adding pure unit tests.
The harness branch preserves that ancestry and corrects the new blanket
TestServer prohibition to match the approved full-router acceptance harness.
Devin session `cool-actress` accepted this guidance and withdrew an inaccurate
in-process transport suggestion after checking installed axum-test 17.3.0. Both
agent guides are synchronized at signed harness head `d8201065`; publication
and updated-head hosted checks remain pending. No upstream tests were removed.

Coverage extension on `codex/s05-i01-coverage-baseline`:

- Base: PR #2600 branch at `d8201065910fbe79f276b514970399919a67eb51`, including
  PR #2599's unit-coverage setup. Extend its `test:coverage` command and existing
  CI job to run unit, request-harness and care API tests using owned fixtures.
- Preserve the report-only policy. Test/tool failures still fail the selected
  CI job; no line-percentage threshold is introduced.
- Coordinator retains documentation and stack ownership; the existing writer
  owns implementation/tests, and the verifier owns provisioning and live jobs.
- Red evidence covers the missing Task boundary, CI aggregate failures,
  contaminated metadata environment and shared instrumentation paths. Corrected
  runner checks pass 24 cases; CI policy checks pass 137. Installed prerequisites
  are cargo-llvm-cov 0.9.1 and matching Rust 1.99 LLVM tools.
- Each run now owns its target, build intermediates and report directory. The
  report records source revision, dirty state and a source fingerprint, rejecting
  mutations during measurement. Failed runs cannot publish a complete manifest.
- Independent Devin SWE-2 Max session `living-pick` identified an ambient coverage
  directory override and verification gaps. Corrections passed focused checks
  and the same reviewer accepted the final source: tool overrides are stripped, version matching
  is exact, malformed or padded output paths fail before provisioning, failed-output checks
  cover the real run directory, and the redundant test Task alias is removed. Its
  packet and output are under `/private/tmp/s05-i01-review/coverage-*`.
- The host's actionlint 1.7.12 hit a Darwin deadlock. The unchanged workflow check
  passed with the maintainer's merged fix, commit
  `011a6d15e749bb3f2d771eed9c7aa0e7e3e10ee7`, built only under `/private/tmp` and
  selected through a scoped PATH. ShellCheck remained enabled; global tooling
  and repository checks were not weakened. Logs retain the timeout and passing run.
- First instrumented candidate `89d04ce6` passed 45 unit tests, then job 34708
  failed because cargo-llvm-cov 0.9.1 rejects combining `--no-report` with
  `--no-clean`. No request/database coverage or complete report was credited;
  owned run resources were cleaned. A regression reproduced the CLI rejection.
  Removing the redundant `--no-clean` flags preserves accumulation through
  `--no-report`; 24 runner and 137 policy checks pass after the correction.
  Devin accepted this correction in the same `living-pick` session. Repeat
  measurement and cumulative CI passed as recorded below.
- Existing hosted unit coverage at `2808f38` measured 1,931 of 57,239 application
  lines (3.37%), including zero API-controller lines. This `--lib` baseline is
  preserved under `/private/tmp/s05-i01-evidence/hosted-unit-coverage-2808f38/`.
  It does not measure the full suite.
- Combined coverage passed on signed `ca314adb`, before the latest main update:
  job 75191 exited zero with 45 unit, 12 request and 500 care API tests passing.
  The request completeness gate and fixture hook remained intentionally ignored.
  LCOV and a complete manifest are preserved in
  `/private/tmp/s05-i01-evidence/coverage-baseline-ca314adb/`; the manifest records
  the unrelated preserved Serena change and an unchanged source fingerprint.
  Application line coverage is 36,202/57,242 (63.24%); the API-controller
  directory is 6,717/7,287 (92.18%), the OAuth-controller directory 10/344 (2.91%),
  and models 27,775/38,185 (72.74%). These directory metrics do not establish
  public API acceptance or meet every skill target. The log proves all three
  selections ran; LCOV does not label each selection independently.
- Docker pool exhaustion delayed measurement. After the owner approved removing
  one named empty fixture network, reinspection found it already absent and
  capacity restored. This task removed no unknown networks or storage.
- After preserving PR #2604, coverage passed on signed source
  `88bf94777710ff3444017109bc7e1474ea59b18a`: job 28692, exit zero,
  73 unit, 12 request and 500 care API tests. Complete manifest, LCOV and assessment:
  `/private/tmp/s05-i01-evidence/coverage-baseline-88bf9477/`. Raw application
  coverage is 36,654/57,626 (63.61%); API controllers remain 92.18% and OAuth
  controllers 2.91%. Removing only the three new inline test modules yields
  36,270/57,242 (63.36%) application coverage and 27,843/38,185 (72.92%) models.
  Older inline test bodies may remain counted. These figures do not establish
  operation acceptance or satisfy every coverage target.
- Initial cumulative CI job 8997 passed Rust/integration checks, then failed
  before browser tests because the stale asdf `npx` shim could not find asdf.
  Selecting the existing mise Node installation through scoped PATH made the
  unchanged browser prerequisite pass. No source or global configuration changed.
- Full `task --concurrency 1 ci` retry on the same signed source passed:
  job 68704, exit zero; 73 unit, 12 request, 486 selected care API and all 662
  desktop/mobile browser cases. Care reported one ignored and fourteen filtered;
  requests intentionally ignored the incomplete whole-API gate. Other integration
  shards and repository gates passed. Log:
  `/private/tmp/s05-i01-evidence/full-task-ci-88bf9477-path-recovery.log`.
  Separate workflow gate job 13924 passed all 137 policy checks with the scoped
  upstream actionlint fix; log `ci-check-88bf9477-final.log` in the same folder.
  Owned fixtures were cleaned and generated screenshots restored after retaining
  their patch. The unrelated Serena configuration remains preserved and excluded.
- Ownership coordination allocated rate-limit wiring, isolated modules/config
  and nine missing 429 entries to this API work. It also transferred the narrow
  missing-household error adapter correction; care/review edits remain owned by
  their existing coordinator. Three unpublished review-lease operations will
  require reconciliation when integrated; the current contract still has 119.
- Publication and exact-published-head hosted checks remain pending. Next is
  shared HTTP behaviour, beginning with failing full-router rate-limit tests.

Coverage publication and hosted review correction:

- PR #2607 was published on the harness branch at signed `b64532f9`, 423 additions
  and 34 deletions. PR #2600 was updated to signed `d8201065`, 1,115 additions
  and 21 deletions against `main` at `60e7de5d`. Both remote heads were verified;
  neither is merged or deployed. The generated title's initial validation failure
  was superseded by successful corrected-title run 38440132257.
- Hosted review comment 4261986300 identified valid custom coverage output under
  an unignored checkout directory being counted as a source mutation. A real-Git,
  fake-provider runner regression reproduced it: 25 passed, one failed. The
  correction excludes only untracked files in this run's UUID report directory.
  Tracked and unrelated untracked changes remain detectable, including siblings
  in the report parent. The initial 27 runner tests passed; logs are
  `coverage-output-fingerprint-red.log` and
  `coverage-output-fingerprint-isolated-green.log` in the evidence folder.
- Same-session review found asymmetric decoding of unusual Git filename bytes.
  A real-Git index regression reached the intended fingerprint mismatch before
  correction. The final filter preserves retained status bytes and all 28 runner
  tests pass. Evidence: `coverage-fingerprint-nonutf8-red-corrected.log` and
  `coverage-fingerprint-nonutf8-green-final.log`. An earlier macOS fixture error
  and failed provisional correction are retained but not credited as success.
- Hosted coverage on published `b64532f9` passed, with the clean merge revision
  `4a0dce7ec149b6a65c77e21d3cb04f569c880b41` in its complete three-selection
  manifest. Artifact 11835597271 is retained under
  `/private/tmp/s05-i01-evidence/hosted-combined-coverage-b64532f9/`. Generated
  client determinism and Swift/Kotlin compilation also passed on that PR run.
- The correction remains on its owning coverage branch. The next rate-limit
  branch's test-only work and progress record are preserved in a named stash.
  Same-session `living-pick` re-review accepted both corrections (job 73680,
  exit zero; `coverage-output-final-*` review packet, output and export). It
  confirmed existing instrumentation evidence remains applicable because Cargo
  commands and selections are unchanged. Formatting, documentation, workflow
  policy and whitespace checks passed.
- Signing failed twice through 1Password; the owner explicitly authorised an
  unsigned fallback. Commit `62422c280d7daf2d6539d1944b6881256f0a6ee6` uses only
  a per-command signing override. Global signing settings remain unchanged.
- Cumulative `task --concurrency 1 ci` passed, job 80817 exit zero: 73 unit,
  12 request, 486 selected care API and all 662 browser cases (18.2 minutes).
  The run began against the frozen staged patch on `b64532f9`; its SHA-256
  `9ccdad38333596068f4df89b8c3d7805a86947d277898c8b533de61b9e3ad72e` matches the
  correction subsequently committed as `62422c28`, and all three file hashes
  remained unchanged after CI. Full log:
  `/private/tmp/s05-i01-evidence/coverage-fingerprint-full-ci-b645-staged.log`.
  The 31 generated screenshots were preserved as a patch and restored. Task
  cleanup completed; no separate post-run Docker inventory was credited.
  Republication, exact-head hosted checks and propagation remain required.

Shared HTTP rate-limit work:

- The coverage correction and its final record are published at `2e84496b` on
  PR #2607, 582 additions plus 34 deletions against its harness base. The exact
  hosted review thread was answered in comment 4262482794 and verified resolved.
  Both earlier published heads passed hosted checks; new-head checks are active.
- `codex/s05-i01-http-rate-limits` is rebased onto `2e84496b`. The writer's
  preserved request tests are restored. Focused RED job 46801 compiled and ran
  all three TestServer cases; each expected 429 after exhausting an export bucket
  but received 401. Log: `/private/tmp/s05-i01-evidence/http-rate-limit-red.log`.
  The failure proves missing runtime limiting, not a fixture or compilation fault.
- The API writer owns the bounded model, middleware, view, configuration and
  nine approved 429 contract corrections with their tests. The coordinator owns
  records/publication; the verifier owns builds and fixtures. Existing care and
  review ownership remains unchanged. No operation is credited complete yet.
- Initial GREEN attempt 98002 stopped at a configuration-error adapter type
  mismatch; no tests executed. After that compile correction, policy RED job
  41810 ran eight tests: six passed and two failed on an undocumented descendant
  path receiving a special limit and a trailing empty trusted-proxy entry being
  accepted. Missing-peer RED job 97959 executed one TestServer case and failed
  because the error response was plain text instead of JSON. The writer owns
  these corrections. Logs are `http-rate-limit-policy-red-escalated.log` and
  `http-rate-limit-missing-peer-red.log` under the existing evidence directory.
- Policy GREEN job 33947 passed: eight limiter unit cases and one existing push
  retry case matched the selector. This proves the focused model corrections;
  request GREEN job 85558 then passed all four TestServer cases. The verifier
  released the writer for the Loco rendering adapter and approved OpenAPI and
  inventory edits before broader verification. Independent review and final CI
  remain required. Focused logs are `http-rate-limit-policy-green.log` and
  `http-rate-limit-request-green.log` in the evidence directory.
- Broader inventory job 7207 exposed a configuration regression: 13 passed,
  three baseline boot cases failed and one completeness test remained ignored.
  Empty unquoted proxy settings render as YAML null, rejected by the strict
  string parser. The writer owns quoting all three environment templates;
  the passing focused tests had overridden the setting with an explicit string.
  Inventory evidence will also name the exact unchanged medication-take, stock,
  audit and sync projections instead of the overly broad `no-write` label.
- Independent Devin SWE-2 Max review uses session `ambiguous-jupiter`. Initial
  job 87550 stopped at unavailable noninteractive Serena permissions without a
  verdict. Read-only review resumed as job 71657 with the source and verification
  reference supplied directly; it has no build or source-edit ownership.
- Quoted configuration and precise evidence labels passed inventory job 5736:
  16 passed, one completeness gate ignored. Focused request job 95758 passed all
  four rate cases again. Route reconciliation remains 118 of 119, with the
  documented push-subscription test route still missing. Logs are
  `http-rate-limit-inventory-green-v2.log` and
  `http-rate-limit-request-green-v2.log`; these results do not close acceptance.
- All 59 hosted checks passed on published coverage head
  `2e84496b3222c1d60595d4d783b85d1dc6b0c82c`. Coverage PR #2607 remains open and
  unmerged. The check snapshot is `coverage-head-hosted-checks-latest.json` in
  the evidence directory. Rate-limit unit job 81742 passed all nine matching
  cases (eight limiter cases); formatting job 93354 passed.
- Strict Clippy job 50528 passed across all targets with `-D warnings`. The full
  contract group job 3852 passed 16 cases and failed only its intended 119-operation
  completeness gate. These are recorded separately from full root CI, still pending.
- Devin session `ambiguous-jupiter` completed its supplied-source review as job
  24562 after read-tool permission prompts stopped the earlier attempts. It
  accepted the bounded implementation conditionally on an explicit HEAD-policy
  decision: Axum's implicit HEAD read routes may bypass the GET-specific buckets.
  The writer added a test-only authenticated HEAD export regression; verifier
  job 81844 is checking reachability before any policy or contract correction.
  The proposal is `/private/tmp/s05-i01-evidence/proposed-head-rate-policy.md`.
  Non-blocking review observations remain in the retained review output; no
  unrelated framework or authentication rewrite is included.
- HEAD regression job 81844 confirmed reachability and the bypass: authenticated
  HEAD returned 200, nine GETs from the same client returned 401, and the final
  authenticated HEAD returned 200 instead of 429. The owner has been asked to
  approve sharing GET buckets for implicit HEAD reads plus an explicit OpenAPI
  sentence. The exact unapplied diff is `proposed-head-rate-policy.patch` in the
  evidence directory. Production changes, full CI and publication are held for
  this contract decision; elapsed time is not approval.
- Pure shared GET/HEAD-bucket regression job 61734 also failed as intended:
  one case executed, 81 filtered, with no rejection where the shared read bucket
  should be exhausted. Both new HEAD regressions remain test-only changes;
  the policy and OpenAPI proposal has not been applied. No build is left running.
- The owner authorized the HEAD correction on 13 October if HEAD poses a
  denial-of-service risk. The condition is met: the authenticated request test
  proves HEAD executes the export handler while avoiding its stricter quota.
  Apply the three shared GET/HEAD read buckets and clarify the contract wording.
  The earlier pending-decision entries above are superseded. Verification remains
  held while the migration coordinator runs final dm+d/care CI and prepares its
  authorized merges; its completion message will release the shared build lane.
- Approved HEAD correction is now applied locally: all three read rules share
  GET/HEAD buckets, the OpenAPI introduction is clarified, and its test pin is
  `701b093a1fd87203769e19a393e7524bd6e9a0dc6f4c7394c20750d643e9ffa4`.
  Inventory remains 119 operations with partial export evidence and unchanged
  native pins. Static whitespace and JSONL checks pass; focused GREEN, same-session
  Devin review and full CI remain pending the shared-lane release. The complete
  local patch is `http-rate-limit-head-candidate.patch` in the evidence directory.
- On 14 October the migration coordinator released the build lane after dm+d
  PR #2615 merged. Refreshed main is `99154e36686db793952cfc0120de4d7469079320`.
  GitHub confirms prerequisite PRs #2600 and #2607 are also merged. The stack
  rebase skipped those merged layers and based this rate-limit layer on main.
  All WIP was preserved in named stash `s05-i01 approved HEAD candidate before
  merged-main refresh`; the recovery copy remains. Three configuration conflicts
  were resolved by keeping main's centralized settings and adding the quoted
  rate-limit setting. No module collision was found. Refreshed GREEN verification
  is starting; the prior shared-lane hold is superseded.
- Refreshed unit job 48584 passed ten matching cases (nine limiter cases,
  including shared HEAD/GET buckets, plus the existing push retry case).
  Initial request job 52501 stopped at compilation because upstream changed
  the identity exchange helper signature; no request evidence was credited.
  After matching existing fixture calls, request job 90471 passed 17 cases with
  one intentionally ignored acceptance gate, including authenticated HEAD export
  throttling. Route inventory stays 118/119. Formatting passes. Devin correction
  review resumed in `ambiguous-jupiter` as job 8036; full CI remains pending.
- Devin correction review job 8036 approved the refreshed candidate for full CI;
  F1 is resolved by the approved policy, model regression and authenticated request
  regression. Remaining observations are non-blocking and retained in
  `rate-limit-head-correction-output.log`; they do not establish full API acceptance.
- Frozen source `95259214276c2cad7deebcf2bea551757a5b1296` ran full CI as job
  61497 and exited 201 after normal cleanup. All 99 units and 664 browser cases
  passed; OAuth integration had 16 passes and one failure: the existing socket
  fixture's capabilities request returned 503 instead of 200 because its raw
  router server omitted ConnectInfo. Standard Loco production serving supplies
  it. The writer owns the narrow fixture adapter, with no middleware weakening.
  Docs build passed. The 33 generated screenshot diffs are preserved in
  `ci-95259214-generated-screenshots.patch` and restored; unrelated Serena changes
  remain preserved. Focused regression, correction review and final full CI are
  required before publication.
- Socket-fixture correction now matches Loco production ConnectInfo serving;
  the missing-peer rejection and expected successful capabilities response are
  preserved. Focused OAuth job 46234 passed, then full OAuth job 98800 passed
  all 17 cases. Unit job 90234 passed ten matching cases (nine limiter cases),
  inventory job 80205 passed 17 with one intentional ignore, and formatting job
  67443 passed. Initial unit job 89634 stopped at a missing test-only enum import;
  no Rust test execution was credited for that compile failure. The new tables
  cover strict configuration types and HEAD negative paths without production
  changes. Same-session Devin correction review is running as job 53420 before
  the final commit and required full CI rerun.
- Devin correction review job 53420 approved the production-equivalent socket
  fixture and typed negative cases. No blocking findings remain for this bounded
  rate-limit candidate; full root CI on its committed freeze is the remaining
  publication gate.
- Final full CI job 47196 on `56a3b1f3439e186dcb081ad2ffe4aded5ed705ac`
  exited 201 after normal Task cleanup. All 99 units and 664 browser cases passed
  (browser duration 17.1 minutes); request inventory passed 17 with one intentional
  ignore. Care API integration passed 489, failed one and ignored two, with 14
  filtered: the dm+d timing test reported `missing extract timing: []`. The
  sequential integration runner stopped there, so later OAuth tests did not run;
  separate full OAuth job 98800 remains the valid 17/17 result. The 31 generated
  screenshot diffs were preserved in `ci-56a3b1f3-generated-screenshots.patch` and
  restored, retaining unrelated Serena configuration.
- Original-source timing job 2141 passed alone (one pass, 505 filtered), but
  dm+d module job 1917 reproduced the same failure (17 passes, one failure, one
  ignore). This establishes an interaction in the broader run, without proving
  its cause. All verifier jobs have exited. A narrow diagnostic patch records a
  tracing probe, persisted import statuses and raw events; it remains unapplied
  pending permission to coordinate ownership of the dm+d test. No production
  correction, unchanged full-CI retry, rate-limit PR publication, merge or
  deployment occurred. The reviewed GET/HEAD policy and all-119 acceptance scope
  are unchanged; required CI failure blocks dependent work.
- Care/review PR #2617 is now confirmed merged at 08:38:23 UTC on 14 October as
  `672dbc0d1520ee586ce625f0dbfec7fb3eba0544`; the coordinator released its hold
  and reported local cleanup. GitHub merge identity was checked before refresh.
  The API stack rebased onto that main, retaining unsigned commits as authorised;
  current replayed head is `7880e85c9`. Progress and unrelated Serena changes were
  preserved in named stash `s05-i01 preserved progress and Serena before care2617
  refresh`, then restored while retaining the recovery stash. The sole request-test
  conflict keeps main's 122-operation count and lease metadata alongside all rate
  tests and partial evidence. Main already contains the equivalent inventory patch,
  so its earlier unapplied handoff is superseded.
- The current schema plus the approved rate declarations hashes to
  `bef3d7a3db6d51d6756c864301f24821cd6b53ed8f6dbc68196bc6f7eff1f063`.
  Static reconciliation found 122 operation rows, 121 route pins, no extra routes
  and the preexisting missing push-subscription test route. Coverage remains two
  partial and 120 unverified operations. Acceptance now includes the three merged
  lease operations; the historical 119-operation results remain separate. Main
  also binds the dm+d timing subscriber to each import future; this test-only
  correction is retained, with local regression verification still required.
  New lease responses omit the existing global 429 contract, so the next bounded
  correction starts with a failing metadata assertion before updating those
  declarations and inventory. No new rate policy or native pin update is planned.
- Initial lease metadata job 70308 stopped at test compilation (E0716); no
  behavioral result was credited. After a test-only binding correction, job
  72917 executed the assertion and failed on exactly the three lease operations
  missing shared RateLimited 429 responses. The correction adds those three
  declarations, their inventory response/schema/header entries and schema SHA
  `acfdc42aee410991c16183a65074bb1e2961397722731a88b4477b34fad831eb`.
  Inventory job 69691 then passed 18 with one deliberate ignore; limiter unit
  job 51413 passed ten matching cases. Merged dm+d module job 45508 passed 18
  with one ignore and 504 filtered, including the formerly failing timing case.
  No new timing diagnostic or production importer correction was applied.
  Evidence and coverage states remain unchanged. Independent correction review,
  current coverage measurement and full CI remain required before publication.
- Frozen unsigned candidate `bbd971dd13fda0b3450d0a5e656f2c8ccb26ac5e` passed
  formatting, strict all-target lint job 46272, docs build and diff checking.
  Same-session Devin `ambiguous-jupiter` correction job 99421 accepted it with
  no blocking findings. Report-only coverage job 6021 exited zero: 102 units,
  18 requests (one ignored) and 521 care API cases (two ignored) passed. Manifest
  `rate-coverage-main672/3332732b-b1f3-47c6-85e4-d1898b33e063/manifest.json`
  records that revision and source-state SHA
  `511e72ee1181e0f15c371dcbdec0189a87cdef24f37bc41de2b0b0f968f107b0`;
  dirty state is the preserved unrelated Serena file. Raw LCOV is retained there.
  Excluding new inline test bodies/helpers, limiter model coverage was 161/171
  (94.15%), controller 46/46 and view 16/16. Model gaps were error-display text,
  malformed settings rejection and stopping a forwarded chain at the first
  untrusted hop. API controllers aggregate 92.33%; raw source aggregate 63.20%
  and models 73.17% include inline test bodies. OAuth controller coverage is
  2.88% because this selection omits its separate test binary. These are baseline
  limits, not operation acceptance or proof that all business paths pass.
- Two test-only additions now exercise actual quota sharing across valid and
  malformed spoofed forwarding prefixes, and five invalid configurations with
  their operator-visible startup errors. Production code is unchanged. Focused
  unit job 85432 passed 12 matching cases (11 limiter cases plus push retry).
  Formatting passes after correcting the new test layout; no behavioral result
  was inferred from the formatting failures. Same-session correction review,
  coverage remeasurement and full CI remain before publication.

- Final runtime candidate `e124b30da85cab62445fd94eac42bb94eb3f7f3e` passed
  `task --concurrency 1 ci`, job 37169, exit zero: 104 units, 507 care API cases
  (two ignored, fourteen filtered), 17 OAuth cases, 18 request cases (one ignored)
  and all 744 desktop/mobile browser cases. Other integration binaries passed.
  Strict all-target lint job 88178, formatting, docs and whitespace also pass.
  The full log is `rate-limit-full-ci-e124.log` in the evidence directory.
  The Task exited normally. Its 40 regenerated screenshots were preserved in
  `ci-e124-generated-screenshots.patch` and restored; unrelated Serena work remains.
- Coverage job 72508 exited zero on the same runtime candidate: 104 units,
  18 requests (one ignored) and 521 care API cases (two ignored) passed. Manifest
  `rate-coverage-e124/a192519b-ef8b-416c-8d55-14e88ce6d7cd/manifest.json` records
  source-state SHA `4368f2d688c442f4c2b8251446b2d1db1d5627532a4cdfd91c93a0b35d744158`.
  Runtime line coverage is 171/171 for the limiter model, 46/46 for its controller
  and 16/16 for its view, excluding inline tests and test helpers. Broader source
  coverage remains a report-only baseline; this is not complete API acceptance.
- Same-session Devin `ambiguous-jupiter`, SWE-2 Max correction job 37262, accepted
  the final test additions without blocking findings after job 99421 accepted
  the merged-care reconciliation. The coordinator owns publication and evidence;
  `/root/api_writer` owns code/tests and `/root/verifier` owns verification and
  resources. The reviewed runtime diff is 1,354 lines against its immediate base.
  The following documentation update records results without changing runtime.
- Hosted PR #2623 review raised capacity retry timing, HEAD bodies and a proxy-trust
  warning. Same-session adjudication job 55410 confirmed the capacity defect:
  unit RED 53145 returned retry 20/reset 1020 instead of 200/1200. The correction
  uses the earliest retained expiry only when capacity prevents a new counter.
  Mock-transport full-router HEAD tests passed unchanged for 503 (85332) and 429
  (36073); Axum 0.8.9 strips the body after middleware, so no duplicate handling
  was added. The proxy warning was a deployment documentation gap: configured
  proxies must overwrite forwarding values or append their observed client peer.
  A trusted-looking spoofed prefix now also passes the existing protection test.
  The description-only OpenAPI update has SHA
  `fae2da8212338c04dea75628e35c5892e5e5bbde722d160b3072048c6ea31036`;
  operation metadata and native pins are unchanged. Unit GREEN 9441 passed 13;
  request GREEN 2433 passed seven. Formatting, strict lint 60133, inventory 74148
  (20 passed, one ignored) and whitespace pass. Final correction review, coverage
  remeasurement and full CI remain required before publishing the updated head.

Session close-out on 11 October 2026:

- [Profile PR #2504](https://github.com/damacus/med-tracker/pull/2504) is merged
  as `b50951e09323060ec7a8d6c6f08a0750db0c3b25`.
  [Drawer/account-menu PR #2521](https://github.com/damacus/med-tracker/pull/2521)
  is merged as `a1e18b0172c398c18c33cf967e7857104692aa52`. Profile settings use
  accordions showing current values; navigation belongs in the drawer and
  appearance settings live on the profile page. Parent CI issue #2522 is closed.
- [Dashboard PR #2505](https://github.com/damacus/med-tracker/pull/2505) remains
  open at signed commit `ab0ad668acc8586443d8c6557738c1022dc28873`.
  Review fixes preserve account-local schedule dates, taper previews and recorded
  doses, and avoid repeated stock resolution. The dashboard reuses nine existing
  profile/navigation SVG symbols instead of duplicating them.
- Local `task ci` passed, including all 586 desktop/mobile browser cases.
  Fresh [hosted run 38121077030](https://github.com/damacus/med-tracker/actions/runs/38121077030)
  passed all Rails unit and system shards on `ab0ad668`. At the close-out snapshot,
  Loco Foundation was still running; all other reported checks passed. This does
  not establish complete hosted acceptance, merge or deployment.
- The earlier Rails scanned-stock failure passed on hosted rerun. Intermittent
  notice, menu and lookup assertions remain tracked in
  [#2526](https://github.com/damacus/med-tracker/issues/2526). No Rails assertions
  were weakened and no automatic retries were added.
- A screenshot alleged a dashboard Cedar view-access bypass. Current grant and
  membership checks were inspected; eight dashboard tests and the Cedar
  grant/expiry test passed. No bypass was reproduced. No security finding was
  dismissed, and no corresponding unresolved GitHub review thread was present.
- [Profile acceptance #2502](https://github.com/damacus/med-tracker/issues/2502)
  and [archive restoration #2513](https://github.com/damacus/med-tracker/issues/2513)
  remain open. Gravatar remains disabled. Production cutover is not ready and
  no deployment is authorised by this close-out.

Next: inspect the final Loco result for `ab0ad668`, address any failure on that
revision, then obtain dashboard acceptance. The shared HTML report remains at
`/private/tmp/medtracker-migration-status.html`; its authoritative structured
input is `/private/tmp/medtracker-progress-report.md`. This ledger update is
published directly to `main` at the owner's request, separately from PR #2505.
Other sections below retain their historical evidence dates.

## Earlier delivery checkpoint — 6 October

Continuation [PR #2460](https://github.com/damacus/med-tracker/pull/2460) publishes
the Rails shared-ignore correction at signed Dan Webb commit
`730942fd2680439d77f626f5c7fef24fa4836c70`. Exact-head hosted
[CI 37504518956](https://github.com/damacus/med-tracker/actions/runs/37504518956),
GitGuardian and title validation pass. Local Rails baseline passed 6,112 examples
with no failures. The separate release PR #2381 proposes version 0.6.0; it is
not merged or deployed.

The current care sync and dose-record browser candidate passes final root
`task ci` run 85878: 141 care API tests, all remaining Rust targets and all
192 desktop/mobile browser tests. The browser run completed in 11.6 minutes
without skipped or flaky cases. Persistence includes its one explicitly ignored
schema-capture utility. Task-owned test resources were cleaned up.

Devin SWE-2 Max cleared the occurrence browser changes and all corrected sync
operation families. The final location regression first reproduced permission
checks occurring after missing/stale version checks; all 24 focused sync cases
now pass with authority checked first. Snapshot visibility, deletion delivery,
replay permissions and exact response behaviour are covered by the final run.
These care changes await publication and exact-head hosted acceptance.

Identity integration awaits the owner's durable Better Auth patch-source choice.
Reminder workers compile and pass six controlled delivery tests. Reviewed retry,
partial-delivery and scheduling gaps are being corrected within this delivery.
Production reminder lifetime and uncertain sends after a crash still await owner
decisions. PDF downloads and existing report information are being implemented;
their first compilation follows care verification. Scratch configuration passes
four tests and its orchestration passes five; actual images remain unproved.
Saved-state rollback preparation passes eight orchestration tests; populated
restore and Rails operation remain unproved.
Remaining operations, integrations, browser journeys, workers, both scratch
architectures and rollback remain required. The authoritative current report is
`/private/tmp/medtracker-migration-status.html`.

## Approved cutover scope — 6 October

Rollback restores an independent pre-cutover dump or frozen replica and restarts
Rails; new Loco data may be lost. Prove restoration and Rails operation, rather than
Rails compatibility with Loco-created data. Stop Rails servers, workers and schedulers
before Loco starts; stop Loco writers before restoring Rails. Old pending/failed Rails
jobs may be lost without queue transfer or individual delivery reconciliation.

Unsupported historical passkeys may be invalidated without counting affected accounts
as a gate. Invalidate existing sessions/tokens with a proved mechanism, preserving
fresh sign-in and MFA. Historical links, export formats, offline queues and push
subscriptions need not survive. Preserve records/files, new authorised downloads,
new exports, future offline replay and future push behaviour.

Use daisyUI, the same colour schemes and clear Loco browser routes. Exact Rails pixels,
CSS geometry, theme-export fidelity and browser bookmarks are excluded. Required
information/actions and accessibility remain. PDF reports must match existing appearance
and work in scratch. Inspect renderer/font and retained Rust work before choosing.

FHIR/SMART, MCP, AI and external medication lookup are deferred from the first
production gate, with no exact external provider/configuration parity requirement.
They remain in the full migration goal. Core medication entry/care remain mandatory.
Report first production readiness and complete migration acceptance separately.
None of these design decisions authorises live data access, invalidation or deployment.

The subsequent first-release answers keep NHS dm+d import/reconciliation/scanner,
medication reviews/background refresh, complete administration, time-limited support,
household export/closure/retention holds/deletion, avatar uploads, complete devices/
sessions and automatic live dose/stock updates in the core production gate. Offline
capture/replay, portable imports and both-native-app acceptance are deferred from
that gate but remain in the full goal. Optional AI/provider deferral cannot exclude
catalogue/scanner or review outcomes.

## Published

[Baseline PR #2451](https://github.com/damacus/med-tracker/pull/2451) was squash
merged on 6 October 2026 as
`3f4a7f952ef857b0f4f6e26105d9c39747771b0a`. Its exact source head
`2d95d11aeff02a42251b802d11f09080d54ca536` passed
[CI 37492218152, attempt 2](https://github.com/damacus/med-tracker/actions/runs/37492218152),
GitGuardian, title validation and advisory Clippy. The Loco job passed 117 care API
and 178 browser cases. The unchanged failed Rails schedule-card file passed locally
and in its single hosted shard retry; no application change was needed.

Loco owns root commands, routes, configuration and templates. Rails is independently
runnable under `rails/` as the reference and rollback application. Published
work includes account setup/invitations, supported authentication, stored API
credentials, medication/dosage/location/people management, household administration,
treatments, recorded pauses, dose outcomes and OAuth consent/token/revocation.

All eleven approved GitGuardian classifications are verified IGNORED: ten synthetic
fixtures use `test_credential`, and incident 37907524 uses `false_positive`
for fixed audit labels. No scanner exclusion or history rewrite was used.

## Published behaviour and evidence

Published Recovery passes all 14
desktop/mobile cases, including exhausted OTP, passkey-only recovery, one-use
codes, current-account checks and bound OAuth continuation.

People passes eight API cases and its desktop/mobile browser journey. Creation
preserves capacity, carer/grant/Home associations, permissions and audit/sync
effects. Partial updates retain the contract without mandatory `If-Match`.
An actual Rails service probe supplied the exact saved POST/PATCH retry digests,
including filtered email; both replay correctly. New Rust keys still bind the
submitted body. Forced audit failure rolls back every dependent write. A reproduced
lost permission-version increment is corrected with an atomic stored-value
increment. Extra request fields are filtered as Rails does, protected account IDs
cannot be assigned, and an empty filtered person returns 400.

Devin SWE-2 Max approved both Recovery and People corrections. Final combined
`task ci` passed all Rust/static/lifecycle checks, 31 care API cases and all 74
desktop/mobile browser cases. Owned test containers, volumes and networks are
absent after cleanup. The pre-correction partial run is superseded by this result.
The corrected delivery is committed and pushed. Household administration now passes
eleven API cases and six desktop/mobile journeys. Devin SWE-2 Max accepted the final
requirements, correctness and security review with no outstanding defects.
Administration is published with the passkey delivery below.

Native passkey registration, passwordless and second-factor sign-in, restart and
password-confirmed removal pass. Supported ES256 and RSA-2048 RS256 credentials
generated by Rails WebAuthn 3.4.3 sign in through the native browser without
re-enrolment, advance their retained counters and survive a server restart. Import
does not create a registration audit. The maintained library verifies the ceremony;
there is no custom signature verifier. A real concurrent removal reproduced an
account/key/session foreign-key lock-order failure; consistent account-before-key
locking now produces a clean rejection without issuing a session.

The user approved dropping unsupported historical passkey algorithms on 6 October.
This intentional compatibility break is recorded in [the deployment guide](../../deployment.md#approved-cutover-decision-unsupported-passkeys).
Supported keys work on mixed-key accounts. Security settings identify keys needing
replacement, and unsupported-only accounts receive recovery instructions after
password verification. Recovery, supported-key registration and explicit old-key
removal pass without bypassing MFA. Unsupported rows remain for Rails rollback
until authenticated removal. All fourteen focused transition, import, concurrency
and CSRF/ownership cases pass across both viewports. Devin SWE-2 Max accepted the
final requirements and correctness/security review with no outstanding defects.
Final combined `task ci` passed all Rust/static/lifecycle checks, all 41 care API
cases, three unit cases including exact challenge expiry, and all 124 desktop/mobile
browser cases on the frozen candidate. Documentation verification, commit and push
are complete for administration and passkeys. This does not accept
the remaining identity, care, browser or release requirements.

## Current implementation

Invitations and account setup are next. A real signed-in browser reached household
administration and failed at the absent Invitations link. Existing API tests also
proved the missing invitation routes. Nine invitation API tests and four persisted
credential tests pass. Desktop/mobile invitation management passes with real Loco
worker email delivery, resend and cancellation through an isolated Mailpit inbox.
Guest password and OTP sign-in resume the invitation. Acceptance committed the
expected effects, but a stale synthetic fixture ID sequence changed the selected
account person. After correcting the fixture, all six invitation journeys pass
on desktop/mobile, including password/OTP acceptance, atomic effects and replay.
Stored sessions and app tokens now pass real care-route checks, including source
household binding and immediate permission-version withdrawal. Four API-session
and five resource-scope regressions also pass. Account creation with actual email
verification remains required in this delivery. The missing POST handler is now
registered. A valid submission initially returned 503 because its conflict rule
did not match the preserved partial email index. The corrected insert passes the
representative account creation, real email delivery and verification journey.
All ten desktop/mobile signup cases now pass, including invalid, expired, revoked,
cancelled and existing-account rejection. All ten open-registration desktop/mobile
cases also pass: first-household bootstrap and verification, environment overrides
and closed-registration denial. A database permission test proves that owner
detection exposes only a boolean and preserves the application's tenant row
visibility. Final verification resend passed both desktop/mobile cases with real
SMTP delivery, strict recent-email suppression, retained key reuse and verified/
missing-account refusal. Completed queue jobs and verification-key rows remain
stored, matching the pinned framework and Rodauth lifecycles; tests count those
rows while proving that rejected requests add no effects.
Eight dosage API/removal cases pass, covering protected inactive references,
live manager revocation and audit rollback. Both desktop/mobile dosage browser
journeys pass actual create/edit/delete and parent-stock changes. Deleting the
last tracked option resets stock to untracked, matching Rails. Medication
list/detail reads now pass both route tests after two actual missing-GET cases
failed, including conditional reads, stock projections, visibility and auditing.
Valid OAuth credentials do not substitute for the invitation API's
required user session. These changes remain local and have not passed acceptance.

## Whole-migration acceptance

| Area | Implemented evidence | Still required |
| --- | --- | --- |
| Foundation | Accepted: both runners, audit, corrective review, local/docs and exact-head hosted CI pass; record published | Complete; product and populated rollback gates belong to later areas |
| Persistence | Guarded schema adoption, restricted roles/RLS, persistence and tenant tests, normal seed CLI | Populated Rails rollback and complete schema acceptance |
| Identity | Password/session and bearer tests; OAuth library integration; published TOTP, Recovery and supported passkeys | Account creation/verification, remaining security settings and complete client interoperability |
| Care | Dose, stock, medication, location and People management; published household settings, membership and care-access administration | Invitations, remaining household operations, dosage, treatment and sync |
| APIs | Actual Loco listener checks for migrated care and OAuth routes | Complete native/FHIR/SMART/MCP contracts |
| Browser | Tera/daisyUI themes and migrated care/OAuth journeys on desktop/mobile | Remaining workflows, accessibility evidence and PWA/offline |
| Workers | Owner-provisioned queue and restricted queue startup/recovery configuration | Every real job, recurrence, reminder, retry/idempotency and crash/drain outcome |
| Release | Retained source/PR inputs and rollback application preserved | Both scratch architectures, complete journeys, populated rollback and obsolete tooling removal |

## Working rules and reports

Use [the current working rules](team-charter.md). Historical relocation/workspace
and source-preservation audits run explicitly through `task migration:audit`.
Do not regenerate historical ledgers for each delivery. Duplicate runners, unused
demos and the proposed temporary worker probe were removed. Keep real database
isolation, process cleanup and side-effect verification.
The current runner has no duplicate replacement to remove. One representative
browser test now establishes a shared missing entry point; all distinct behaviour
and security cases still run after implementation. Retire helpers when their last
caller is replaced, within the same delivery.
The remaining plans use named model methods and existing tenant transactions.
The proposed command dispatcher, execution wrapper and parallel household test
binary have been removed from the plan. The existing browser command accepts a
standard test filter for prerequisite checks; it remains the full-suite runner.

Migration defects stay in this work. Create follow-up issues only for independent
scope. Useful retained PR inputs are recorded in [pr-inputs.md](pr-inputs.md);
close superseded work only after its replacement is published. Incident #2453
remains separate. Original Rails tests remain the rollback reference; Loco work
does not trigger unrelated RSpec repair.

Update the local reports at `/private/tmp/medtracker-migration-status.html` and
`/private/tmp/medtracker-2450-review-status.md` at each completed slice or material
blocker. Detailed logs stay outside the status page.
Prior delivery history is preserved in Git and the existing evidence artifacts.
Last recorded retrospective: 18:24 UTC on 6 October. Next: 20:24 UTC.
