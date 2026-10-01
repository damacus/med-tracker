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

## Recovery scope

People, Locations and scalar medication forms are implemented, but no complete
journey is accepted until the recovered HTTP/browser run passes. The latest
product snapshot is 76655c4da501d5d2bcb934942323cf31c84078de477684c560528f2aaaf21bb4.
Moving a test module to the end of its file after that snapshot changes no
production behaviour; final Rust checks use the formatted worktree source.

The remaining medication race is documented in the independent review: an
option inserted between reading a medication and its options can discard a
scalar draft with 400 instead of retaining it with 409. Existing API row locks
and supplied If-Match still protect writes. Full dosage-option editing, stock,
assignments/schedules, full authentication parity and dose-modal localisation
remain open. No production deployment or Rails retirement is authorised here.
