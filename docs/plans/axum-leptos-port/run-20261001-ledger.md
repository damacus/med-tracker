# Run ledger

| UTC time | Owner | State | Evidence | Next action |
| --- | --- | --- | --- | --- |
| 08:27 | Coordinator | Started | Clean detached baseline 026c27e2; branch codex/rust-household-journeys-20261001 | Save specs and prepare missing-route red cases |
| 08:32 | Coordinator | Research accepted | Native forms; conditional leptos_i18n adapter; WebAuthn/OTP legacy compatibility gates | Dispatch bounded owners |
| 08:50 | Runner | Baseline RED | Original Rust image: People, Locations, medication creation and capability discovery return 404; editing representation lacks friendly_name. Six failed browser tests; digest d78ff0355198b24dce59b8792116af8d7df001ae8688b6b40ad54b4267979cbd | Implement adapters against existing APIs |
| 08:57 | Coordinator | Integration checkpoint | API check passed; nine renderer tests passed; all five locale catalogues covered. Independent source review found no blocking security or data-loss defect after fixes | Run disposable persistence and browser acceptance |
| 08:59 | Runner | Acceptance started | Product snapshot ee1f625dd4357de7dfef110076e6d47f92ec0f4797b42407dc0443fbc9831f74; project mtcontract-dfaa00f2263c488a; ten black-box tests precede browser suites | Diagnose any failure before repeating bootstrap |
| 09:01 | Auth owner/reviewer | Bounded helper accepted | Twelve synthetic Rails-compatible OTP tests pass. No factor login enabled; database atomicity and legacy WebAuthn import remain open | Keep full authentication parity unaccepted |
| 09:09 | Coordinator | Infrastructure interruption | Host disk exhausted; generated incremental caches removed. Docker engine then failed a five-second health request. User took responsibility for Docker restart | Continue checks that do not need Docker |
| 10:09 | Coordinator | Recovery resumed | 220 GiB free; Docker responds. Old Rust/Postgres images and build cache absent; unrelated user Rust skill remains untracked and preserved | Rebuild isolated acceptance |
| 10:19 | Coordinator | Recovery source frozen | Optional-dose decision and stale-edit precedence each recorded a failing assertion before fixes. Medication versions retain microseconds. Inventory/detail navigation tests pass in all five locales | Run final quality gates and preserve live acceptance limits |
| 10:25 | Coordinator | Quality gate passed | task ci:rust-port passed after correcting test-module ordering. Independent source review records one remaining between-read draft-retention race. No new Stock/Assignments dispatch | Publish reviewable source and assess live acceptance at original deadline |
| 10:27 | Coordinator | Assessment delivered | Runtime still rebuilding after disk recovery; no completed journey claimed. Two-hour timer paused | Finish current verification without starting new journeys |
| 10:33 | Runner | Recovery runtime RED | Both medication lifecycle cases and owner navigation passed. Viewer dashboard setup returned 503 before inventory assertions; remaining API/browser targets did not run | Record dashboard failure and correct inventory test setup |
| 10:43 | Coordinator/runner | Inventory acceptance rerun | Independent review accepted direct inventory/detail starting pages with unchanged login and permission assertions. Final Rust quality gate passed after rebase onto current main | Complete isolated HTTP/browser run |
| 10:55 | Runner | Household HTTP GREEN | All 14 household HTTP cases passed after correcting stale fixture credentials and inventory CSRF setup | Finish browser regressions |
| 10:58 | Runner | Household workflows GREEN; dose regressions RED | New People, Locations and scalar medication workflows passed on desktop/mobile. Six existing dose cases could not find Log; fresh household screenshots inspected | Repair the existing API dose-source projection mismatch |
| 11:04 | Coordinator/implementer | Regression repair | Existing serializers omit record permission and eligible stock IDs consumed by the dose UI. Isolated ownership and failing projection tests precede the repair; renderer permission gate stays intact | Independently review, run Rust gate and repeat affected acceptance |
| 11:13 | Runner/reviewer | Dose projection GREEN; history selectors RED | All 15 HTTP cases passed. Log controls, dose writes, stock updates and Escape focus passed; four history assertions incorrectly expected a separate dose text node | Preserve row/stock/replay assertions and correct scoped history selectors |
| 11:18 | Runner | Setup interruption | Concurrent runner self-test and acceptance rebuilt shared UI dependencies; acceptance stopped before assertions on a missing npm module | Serialise remaining builds |
| 11:28 | Coordinator/runner | Runner self-test GREEN | All runner sequences and failure cleanup checks passed. Full Rust gate is green; source review accepted exact history dose assertions | Run final combined acceptance alone |
| 11:47 | Runner/reviewer | Mobile layout RED | Viewport 390px, document width 471px; grid sections expand to 455px. History text wrapping alone is insufficient | Constrain the mobile grid track and item minimum widths |
| 11:52 | Runner/reviewer | Combined acceptance GREEN | Frozen source 453d76d198e59dd96a22373c8953a0555feeda615951f87e33a11e2f0d5e96dc; all 15 HTTP and 21 browser cases passed, including strict mobile page width. Fresh desktop/mobile screenshots retained; owned runtime cleanup passed | Publish bounded slice and preserve follow-up scope |
| 11:58 | Coordinator | Published | Branch pushed; [PR #2346](https://github.com/damacus/med-tracker/pull/2346) open against main, attached to this chat. [Issue #2345](https://github.com/damacus/med-tracker/issues/2345) records remaining scope | Await review and CI; agree the next slice before expanding implementation |

## Recovery scope

People, Locations and scalar medication forms passed the recovered household HTTP
and desktop/mobile browser checks. Combined acceptance passed all 15 HTTP and 21
browser cases against frozen product snapshot
453d76d198e59dd96a22373c8953a0555feeda615951f87e33a11e2f0d5e96dc.
Dose recording, stock effects, exact history values, CSRF/replay protection and
the strict mobile page-width check all passed. Fresh desktop/mobile screenshots
were retained. Final Rust checks passed on the formatted source rebased onto
current main before the CSS-only mobile correction; the final browser build and
runtime checks cover that correction. Runner self-tests and documentation build
also passed. Independent review accepts publication of this bounded slice.

The remaining medication race is documented in the independent review: an
option inserted between reading a medication and its options can discard a
scalar draft with 400 instead of retaining it with 409. Existing API row locks
and supplied If-Match still protect writes. Full dosage-option editing, stock,
assignments/schedules, full authentication parity and dose-modal localisation
remain open. No production deployment or Rails retirement is authorised here.

Publication remains unmerged. GitGuardian check 110357918541 reports three generic
high-entropy findings in synthetic OTP compatibility vectors from the first
commit. Independent review confirmed test-only inputs and pure derivation,
without deployed credentials. Issue #2345 records the exact incidents and the
required authorised GitGuardian false-positive disposition. No scanner bypass,
test weakening or history rewrite was performed.

## Personal maintainability review

Eight mixed-purpose Rust files were split into private modules with explicit
existing entry points. `read_resources.rs` is now 13 lines, down from 1390.
Independent source and runtime review accepted the refactor. The full Rust gate,
four harness safety tests, People/Locations contracts and final 15 HTTP/21 browser
household checks passed. Fresh screenshots were retained.

Broader audits reproduced identical pre-refactor failures in three sets (56/16,
7/2 and 21/2); [issue #2347](https://github.com/damacus/med-tracker/issues/2347)
tracks these existing gaps. See the [refactor record](refactor-20261001.md) for
source boundaries, final snapshot and independent evidence. PR #2346 remains
unmerged; the external GitGuardian disposition remains outstanding.
