# Sole verification queue

Owner: completion_build, Luna Medium. No other seat runs compiled/runtime jobs.

Read plan.md and writer-brief.md. Validate actual Task selector and routes before
expensive runtime jobs. Source hashes certify frozen inputs, not subsequent
edits. Use one batched fully awaited manifest, exact command, task exit status,
fixture SHA and detailed outer logs. Store private logs under /private/tmp or
ignored evidence; no secrets in durable records. Write compact receipts here.
Use separate grant-mutating token lifetimes and fixed-clock dashboard fixtures.
All runtime actions preserve Rails, isolate fixtures and clean only owned files.

Current state (2026-10-02): D, E and F are accepted and published. G's initial
stock/pagination, minor, People, portable-import, audit, source-capabilities and
dose RED/GREEN evidence is recorded below. The approved dose split is installed;
format, API check, warning-denying Clippy, selected compiles and API unit tests
pass. All seven required post-split dose contract targets pass on separate
fixtures, including full sync and replay. Independent final dose review passed.
The before-occurrence dose-occurrence and sync-read targets passed. The
occurrence split is installed and its six mapped API targets plus fixed-clock
dashboard check pass after the split. The brief's separate `schedules` Cargo
suite was subsequently run against reconstructed pre-split and current source;
both runs expose the same existing failures (3 pass, 8 fail, 4 ignored), so
occurrence acceptance remains open pending the reviewed schedule-only repair
and matched reruns. The first post-dose-split `doses` fixture hash is
runner-emitted only because cleanup preceded independent rehash; the other six
dose fixtures and all occurrence fixtures were independently hashed before
cleanup.

Baseline receipt: `api:openapi-dosages-acceptance` exited 0 on the Rust API
listener; all 14 dosage contract tests passed. Raw log:
`/private/tmp/household-d-20261001/D-baseline.raw.log`; project
`mtcontract-4853e7d49ce242e1`; fixture SHA-256
`b269e8fb9ea838d53756b028b1a2c2aac9fab51ca5eca1fbd722b7787a100c46`;
runner source digest `e76be3aa5966c568bc45cc8b76f8dcbc7762ac855b405d81f7ca18e55b558830`.
Pre/post 302-path manifest hashes match at
`1e987f843c01ea65974f2eaab1ff8c6037764cda7be0a582b684e3ecfb751286`.
The new HTTP test and medication management source each matched the runtime
copy before cleanup; a complete copy-manifest comparison raced copy cleanup
and is not claimed. Fixture was disposed by the Task.

HTTP RED receipt: the sandboxed source-snapshot attempt failed before copy
creation when Cargo could not resolve `index.crates.io`; the same existing
snapshot Task succeeded with approved network access. Pre-manifest contains
303 runtime-input paths; its digest is recorded at
`/private/tmp/household-d-20261001/HTTP-unit-pre.manifest`. The runner copy
matched the synchronously captured 301-path copy manifest at digest
`7bccbc391012269989e1ec3b426ce791c9d98bfa26f12da96075cb9e6c846aa3`; runner
aggregate digest was `82d84ba25cb550509bc42bb4bd8bd4ef74a1a91bb443550a3337e6b313590fc0`.
Command: `HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_dosage_options
rtk proxy task api:browser-rust` (default browser list; HTTP failure stopped
before browser tests). Project `mtcontract-ef801874b03d4ea9`; fixture SHA-256
`6e6c1b85e617e8aef9ba53e4be3bee78e978cc4d6b4860311d511a43563f5719`; task
exit 201. Four tests ran: all four received 404 when the add form was expected
to return 200 (three at line 89, one transition/default case at line 183).
This is the requested missing-route RED, not setup failure. Full raw log:
`/private/tmp/household-d-20261001/D-HTTP-RED.raw.log`; runtime copy manifest:
`/private/tmp/household-d-20261001/D-HTTP-RED-copy.manifest`.

Post-extraction dosage baseline receipt: `rtk proxy task
api:openapi-dosages-acceptance` exited 0; all 14 dosage contract tests passed.
This job was already in flight when the parent hold on new captures arrived;
it completed normally and left no active runtime. Raw log:
`/private/tmp/household-d-20261001/D-GREEN-baseline.raw.log`; project
`mtcontract-6243d0ac4e7a4c3a`; fixture SHA-256
`c99e026d09dc8fa3a0db65cb507602ac35060febae681c54ea1ee5542eca65c2`.
The fully awaited 314-path source premanifest is
`/private/tmp/household-d-20261001/D-GREEN-baseline-pre.manifest`, hash
`ccb2bdd6f8fa305e5aee5e2ea9a15103bf6df5885ad1dd9564e76dbf3bc52e7d`; the
runtime copy manifest is
`/private/tmp/household-d-20261001/D-GREEN-baseline-copy.manifest`, hash
`33536065ab758e95baea99c9947f643573f61e5fe27a0158f175ad3925a30b49`.
All copied paths matched the premanifest, and the runner emitted aggregate
source digest
`a53e028ebb3f8bcc56db20c8953d1d6162e5107aad734c4d5888497c73007e18`.
The Task removed the source copy and fixture after the run. This receipt is
limited to that captured 314-path snapshot; it is not evidence for the
writer's in-progress final test additions.

Final D source fast checks: API and web format/write Tasks, targeted
`api:fmt-file` for `household_dosage_options.rs` and
`household_dosage_permissions.rs`, and API/web format checks all exited 0.
`api:check` exited 0 (only Rust's existing future-incompatibility notice for
`proc-macro-error2`); `api:contract-selected-compile
TEST_TARGET=household_dosage_options` exited 0. The full Rust web unit and
integration Task exited 0: 3 library unit tests and 48 integration tests
passed, including `dosage_locale`, household translation, rendering and
navigation coverage. The writer then confirmed the final D source/test freeze;
review-report.md records independent static review with no blocking findings.
These checks are the prerequisite green checks only. They are not runtime
HTTP/browser acceptance and do not replace the forthcoming frozen copy and
fixture receipts.

D-CORE-GREEN-001 result: the approved `api:browser-rust` job captured project
`mtcontract-53107b9ea50a4377`, ran the four dosage HTTP tests successfully,
then ran 34 browser tests: 26 passed, 8 failed. All ten dosage CRUD locale and
viewport cases passed. The two English immediate-administration cases passed;
Welsh, Irish, Spanish and Portuguese each failed at desktop and mobile on
`assert.ok(option)` (`household-dosage-options.test.mjs:153`, actual `undefined`).
The existing household routes test and both desktop/mobile workflow suites
passed. Therefore this is a browser regression failure, not an HTTP or fixture
setup failure; the writer owns diagnosis. Task exit was 201. The four HTTP
assertions and their PASS summary are excerpted from the outer log at
`/private/tmp/household-d-20261001/D-CORE-GREEN-001/D-HTTP-4-results.txt`; the
full outer log is authoritative. The copied RTK tee
`D-contract-runner-build.raw.log` is Docker build output only. Full browser assertions/backtraces:
`/private/tmp/household-d-20261001/D-CORE-GREEN-001/D-browser.raw.log`; full
outer wrapper log: `/private/tmp/household-d-20261001/D-CORE-GREEN-001.raw.log`.
RTK source logs were preserved from
`1790874444_task_api_458e47.log` and `1790874466_task_api_0b3d04.log`.

The 316-path copy premanifest
`/private/tmp/household-d-20261001/D-core-pre.manifest` and the independently
captured runtime `copy.manifest` match exactly (manifest SHA-256
`6ffb4e19ab3b253d1673b9ba5f3bab9092cb10f6eff48dc37e4d440ec56a961c`; zero
differences). Runner aggregate source digest is the same. A live post-run
manifest captured at 17:09:47 UTC (18:09:47 BST) in
`D-CORE-GREEN-001/post.manifest` contains one later change to
`rust/web/tests/household-dosage-options.test.mjs`; its hash is now
`47f7b91b25703c44ad8974c4006fd471581626ec214d619fa58f46ca6e755af5`, versus
the captured job input `45ccbc5b830610a97ffc30a2ea5ac3574d300384b56eb3a90e34067654f53406`.
This post-run change is not part of the executed copy. The writer confirmed it
was the reviewed test-only pagination correction used in D-CORE-BROWSER-002;
product source did not change between those jobs.

The fixture SHA-256, recorded before cleanup, is
`0ba267d97500f31c6150fc5d996fdaeb6cf2ef3fa893c883783c56322f74781c`.
Twelve screenshots were produced before the failing assertions stopped the
remaining translated administration cases: ten dosage-option locale/viewport
screenshots and two English administration screenshots. They are archived and
checksummed under `/private/tmp/household-d-20261001/D-CORE-GREEN-001/`.
The ten pre-existing workflow screenshot files overwritten by this job were
archived, restored byte-for-byte, and all baseline checksums pass. New dosage
screenshots remain in `docs/screenshots/journey-medication-rust/` for review.
This first browser result is incomplete and is not a D acceptance pass. The
reviewed correction and successful focused browser rerun are recorded below.

Web unit RED receipt: command `rtk proxy task -d rust/web test`, exit 201.
Three tests ran; two existing unit tests passed and
`unknown_medication_errors_use_safe_localised_text` failed with the private
diagnostic instead of the selected locale's safe form-invalid message. Raw
log: `/private/tmp/household-d-20261001/D-web-unit-RED.raw.log`. The 59-path
web/locale pre/post manifest hash is
`f903fa85b722be427b8940a900eeccf2aa8daf2ff99157dfc6130552ed5b648b`.
No exact single-function Task selector exists; the writer requested the
existing crate-wide web test Task.

Focused dosage locale RED receipt: `rtk proxy task -d rust/web test
TEST_FILE=dosage_locale`, exit 201. One test failed because the dosage dialog
HTML includes `private database diagnostic` (`tests/dosage_locale.rs:15`).
Raw log: `/private/tmp/household-d-20261001/D-dosage-locale-RED.raw.log`.
The same 59-path web/locale pre/post manifest hash matches at
`f903fa85b722be427b8940a900eeccf2aa8daf2ff99157dfc6130552ed5b648b`.

First GREEN preparation: `rtk proxy task api:fmt:write` and
`rtk proxy task -d rust/web format` both exited 0. Follow-up `api:fmt`, web
`fmt`, and `git diff --check` also exited 0. The formatter changed 16 D-related
Rust paths only. After the narrow E0382 fix, `api:check` passed all targets,
`api:contract-selected-compile TEST_TARGET=household_dosage_options` compiled
the new contract target, and `task -d rust/web test` passed all 51 unit and
integration tests. Raw logs are `D-fast-api-check-retry.raw.log`,
`D-fast-contract-compile.raw.log`, and `D-fast-web-test.raw.log` under
`/private/tmp/household-d-20261001/`. The 314-path pre/post manifest for this
batch matches at
`47720beef68c4de7c4d442799f78336c9e6e4a989a411e6a64696bb30beb7820`.
The final frozen-source `api:check` and selected contract compile passed after
the unused `Pagination` re-export was removed. Runtime evidence is recorded in
D-CORE-GREEN-001 and D-CORE-BROWSER-002.

D-CORE-BROWSER-002 reran only the 20 dosage browser tests after the reviewed
test-helper pagination correction. `HOUSEHOLD_ACCEPTANCE`, `HOUSEHOLD_TEST_FILE`
and `HOUSEHOLD_TEST_FILTER` were unset, so the already-passing HTTP target was
not repeated. The wrapper exited 0. Raw browser log:
`/private/tmp/household-d-20261001/D-CORE-BROWSER-002/D-browser-20.raw.log`; outer
wrapper log: `/private/tmp/household-d-20261001/D-CORE-BROWSER-002.raw.log`.
Project `mtcontract-007f71ad43904750`; fixture SHA-256
`013dcbee1baff1e5333cd2a7794f537946bc209c05aa779f0d09c7667104baa3`.
Runner digest and exact 316-path runtime copy match the pre/post manifests at
`9fe5a02741c6822b55c2f8fb65b19dfe133752cf740e53fd7caa467e4d4e2f19`; zero
differences. The combined 319-path source plus Taskfile pre/post manifests
match at `f861b50977624ac515f05d547b1ebe752ab2682552ccfd6b57febbecbb68ea16`.
All 20 locale/viewport screenshots are archived and checksummed in
`/private/tmp/household-d-20261001/D-CORE-BROWSER-002/generated-screenshots.sha256`
and remain in `docs/screenshots/journey-medication-rust/` for review.

D-PERMISSION-003 ran the single grant-mutating case last in its own fresh
fixture, with a new browser session and the 20-case dosage browser file.
`ordinary_member_with_person_manage_grant_cannot_manage_dosage_options` passed
1/1; browser passed 20/20; wrapper exited 0. Raw permission assertion excerpt:
`/private/tmp/household-d-20261001/D-PERMISSION-003/D-permission-http-results.txt`;
browser raw log:
`/private/tmp/household-d-20261001/D-PERMISSION-003/D-browser-20.raw.log`; outer
wrapper log: `/private/tmp/household-d-20261001/D-PERMISSION-003.raw.log`.
Project `mtcontract-55068d0ad70945ab`; fixture SHA-256
`b62e724c65daed04b616e0ba74352c4e6d2ac2209e368ccbcf1e1f260d134208`. Runner
digest and exact 316-path copy/pre/post manifest hash match at
`9fe5a02741c6822b55c2f8fb65b19dfe133752cf740e53fd7caa467e4d4e2f19`; the
combined 319-path source plus Taskfile pre/post hash matches at
`f861b50977624ac515f05d547b1ebe752ab2682552ccfd6b57febbecbb68ea16`. Twenty
screenshot checksums are retained in
`/private/tmp/household-d-20261001/D-PERMISSION-003/generated-screenshots.sha256`.

The approved `ci:rust-port` gate then exited 0. It passed UI-preview fmt/test/
lint/build (1 unit test), API fmt/clippy/tests (36 library tests and 12
compatibility tests), all web unit/integration tests (3 unit and 48 integration
tests), and contract-tests all-target checking. Raw log:
`/private/tmp/household-d-20261001/D-CI-RUST-004.raw.log`. The 311 authored
source inputs match pre/post at
`99dd8583140c3c72d0274c95c3bdb6e8779231e622bd631324c1ca676a45a80b`; four
Taskfiles match at `0d3a97b8c0c73758701b9e5f9608e156c93a7510c7540850836c64cb1932ebf1`;
the full 315-path manifest is unchanged at
`275321d1740069268dff438d955caf7d56c1b7a391a79802efd8570b85d6dde5`.
Only dependency future-incompatibility notices for `proc-macro-error2` appeared.

The 20 latest D screenshots were byte-compared to their private job archive and
published under `docs/screenshots/journey-medication-rust/d-20261001/` with
`sha256.manifest`. The duplicate root-level untracked dosage screenshots were
removed only after the destination copies verified; original tracked workflow
screenshots remain restored and checksum-verified. The reviewer and root own
the final D acceptance verdict and following documentation gate.

E initial RED packet is writer-frozen and the first runtime checks are underway.
The focused stock baseline `api:openapi-stock-workflows-acceptance` ran against
the Rust API. `openapi_stock_workflows` passed 3/3; `medication_stock` passed
2/9 and failed seven existing cases at `assert_utc_second_timestamp`
(`medication_stock.rs:57`), which requires length 20 while the response has
length 27. The overall baseline exited 201; this is the known #2347 precision
mismatch, not an E feature RED. The later `medication_management_security_api`
target did not run after Cargo stopped at `medication_stock`. Project
`mtcontract-8fbdfa63e36349b3`; fixture SHA-256
`a76957679f6c6776d33ece696ebff68f272e48a68cd412e976361c7b8c35ce57`; runner
source digest `88e6f970dcd741c85cffe9ee0a6d1ebcbfaf0d3cbe7613838568702cbf21c151`.
Full wrapper raw log: `/private/tmp/household-e-20261001/E-BASELINE-001.raw.log`;
full contract test output:
`/private/tmp/household-e-20261001/E-BASELINE-001-contract-tests.raw.log`.
The source copy was cleaned before a full copy manifest could be retained, so
the runner digest and fixture are recorded without claiming copy-to-premanifest
equality.

E HTTP RED ran the four frozen `household_stock` cases on the Rust API. It
reached all four intended assertions: the two stock action pages returned 404
instead of 200; tracked option creation returned 422 instead of 201; and the
scalar adjustment audit description was `adjust inventory` instead of
`adjust inventory (qty: 15.12, reason: Counted after delivery)`. No browser case
ran in that wrapper. Project `mtcontract-ba9b39c18b9e43ff`; fixture SHA-256
`cfa0a74d6706c9915b6f2e8f4cb4bf49e2f48b34e1fa397aa6e58e86f6fdc571`; runner
source digest `88e6f970dcd741c85cffe9ee0a6d1ebcbfaf0d3cbe7613838568702cbf21c151`.
Full wrapper raw assertions:
`/private/tmp/household-e-20261001/E-HTTP-RED-001.raw.log`. The actual 318-file
runtime copy manifest and live-after comparison match each other at
`3bc791262631f94eef302f1becaa1276ac36a698743d2d183bda50758793d48c`; the E
Rust/JS test hashes match the captured premanifest. The initial 338-path
premanifest omitted five runtime-copy inputs; the 318-path copy manifest and
live comparison preserve the actual snapshot evidence.

E browser-only RED ran all ten locale/viewport cases and all ten failed at
`household-stock.test.mjs:57`: the medications page exposed zero stock-action
links where the test expected one. This is an actual UI RED, not a setup
failure. Project `mtcontract-cdf19f456e884d4f`; fixture SHA-256
`3d08d4da6706c9915b6f2e8f4cb4bf49e2f48b34e1fa397aa6e58e86f6fdc571`; runner
source digest `88e6f970dcd741c85cffe9ee0a6d1ebcbfaf0d3cbe7613838568702cbf21c151`.
Full wrapper log: `/private/tmp/household-e-20261001/E-BROWSER-RED-001.raw.log`;
actual Node browser output:
`/private/tmp/household-e-20261001/E-BROWSER-RED-001-browser.raw.log`. The
318-path runtime copy manifest matched the captured source paths and hashes;
copy/live manifest file SHA-256
`3bc791262631f94eef302f1becaa1276ac36a698743d2d183bda50758793d48c`, with
premanifest SHA-256
`0cf0ab3d5197330f4cac87fa0d81a59d9dae7ccbbd352d51b3454e98e998e31f`. No
screenshots were produced because each case failed before its screenshot step.
The test writer may now resume edits to the initial E HTTP/browser inputs.
Both initial RED test inputs are now released for writer updates. The boundary
and focused-option test sources were captured and kept frozen through their
respective runtime copies; product-only implementation is now released.

E-BOUNDARY-RED-001 ran the approved four-case `household_stock_boundaries`
selector. All four assertions failed on the missing stock UI/contract
behaviour: option mode submitted the parent threshold (`:60`); mixed-unit
individual option inventory failed (`:91`); scalar fallback returned 404
instead of 200 (`:116`); removal returned 404 instead of 200 (`:128`). Project
`mtcontract-7d1c315eb959473f`; fixture SHA-256
`3df4c3f6b823bd6b2a335c255ea82d93d1742d4f307194dcbb8aba001401d6b7`; runner
source SHA-256 `09dd89bad788f6d887686fb4d83f529180642aec2ec48ac0c353285a0c49c5b9`.
Full wrapper and test assertions: `/private/tmp/household-e-20261001/E-BOUNDARY-RED-001/outer.raw.log`.
The captured runtime source directory was `tmp/contract-tests/run.D1QeWo/source`.
All 319 copied source paths match the premanifest; premanifest file SHA-256
`1a58f5501d0de7f2c7fc4a897b962a3d5b92193cc8c68692a0d0027e88015eb4`, copy
manifest file SHA-256
`321ed0dbb0c66bf40749c45eeb73113bd73cf71dc624f711b76430afcc13e983`.

E-OPTION-RED-001 ran the focused
`option_stock_uses_real_option_editor_and_rejects_parent_adjustment` filter;
it failed with stock-action form status 404 instead of 200 (0 passed, 1
failed, 3 filtered). Project `mtcontract-2eb52237c5e54563`; fixture SHA-256
`134f475bdbc3bd9d8beb2f971ba49a3b055d7489038451c08e8a734530031703`; runner
source SHA-256 `09dd89bad788f6d887686fb4d83f529180642aec2ec48ac0c353285a0c49c5b9`.
Full wrapper and assertion output:
`/private/tmp/household-e-20261001/E-OPTION-RED-001/outer.raw.log`. The captured
source is `tmp/contract-tests/run.U7oX8x/source`; all 319 copied source paths
match their prehashes. Pre-manifest file SHA-256
`a51204fbb1866cba3ea559d0518b08a1fe583074eb256402ee0eb1aa35322eb0`; copy
manifest file SHA-256
`da5869ead7225515f2a6d2981f2f5f164996dd55348e59eca97cc4283cf84319`. The root
Taskfile hash also matched before launch at
`6211a8d6623ff9bcf6c1c8f2e530a514c095bd2989c7693a2623525a75c91c5a`. Both
wrappers stopped before browser execution when the HTTP selector failed; the
separate ten-case browser-only RED above remains the browser evidence.

The first bounded E implementation fast checks passed. `api:fmt:write` and
`task -d rust/web format` exited 0, `api:check` exited 0, the
`household_stock_boundaries` contract target compiled with `--no-run`, and the
full Rust web unit/integration task passed 60 tests (3, 8, 9, 1, 2, 11, 8, 9,
9; zero doc tests). Raw logs are in
`/private/tmp/household-e-20261001/E-FAST-GREEN-001/`. The 320-path
post-format and post-test manifests match exactly; manifest-file SHA-256
`4a2e25676a373934e39b7cfd8d37a91ca94d2f6a5c78a46e7c2e22103b6e52d1`. Source
was released for final acceptance-test additions. No final E acceptance GREEN
has run yet.

E race RED packet then reached all four frozen HTTP assertions. The missing,
blank and whitespace token test returned 422 instead of 428 (`:305`); the stale
original-token test returned 422 instead of 409 (`:337`). The two deterministic
interleaving cases returned 303 instead of 409 (`:258`). These status
assertions failed before the later read-back/no-write assertions, so those
state outcomes are not yet evidence. The wrapper exited 201 before browser
tests. Project `mtcontract-40111b65b9be44f6`; fixture SHA-256
`874ab10f859a6f07ad7ef241e3fa444d9e57c96e11d9cd8bc6d48669deeee93f`; runner
source SHA-256 `f2309c8202fbddb52f4dcbf46bae2adf8ca3407ef2cd82ce35b4068af27d27e5`.
Full test output: `/private/tmp/household-e-20261001/E-RACE-RED-001/runtime.raw.log`.
Its 323-path runtime copy matched captured hashes; premanifest SHA-256
`99cff1fb10c76883ea3f4052142cf983ffd1bb964fd242cda537b946160b6591`, copy
manifest SHA-256
`35a8ede616d98509a5639b4e5456314f0f498e631b45e1863b31060e513d48ac`.

The fast compile prerequisite passed for `household_stock_concurrency`.
The focused web target compiled and reached a genuine RED in
`stock_locale`: `untracked_parent_is_distinct_from_zero_scalar_fallback_in_every_locale`
failed in English because a null parent did not render the explicit untracked
stock message. Full log:
`/private/tmp/household-e-20261001/E-RACE-RED-001/stock-locale.raw.log`.
The 324-path pre/post fast-check manifests match at SHA-256
`99cff1fb10c76883ea3f4052142cf983ffd1bb964fd242cda537b946160b6591`.

Final frozen-source fast checks now pass: API and web formatting, `api:check`,
no-run compilation for `household_stock`, `household_stock_boundaries`,
`household_stock_concurrency`, `household_stock_permissions` and
`medication_stock`, plus the complete `rust/web` test suite (52 passed, zero
failed; zero doc tests). Dry Task rendering confirms that
`HOUSEHOLD_STOCK_ACCEPTANCE=true` selects exactly the three core stock targets
in one command and does not add the completion dashboard target. With stock
flags unset, the existing default still selects its original three household
targets. Raw receipts are in
`/private/tmp/household-e-20261001/E-FINAL-FAST-001/`.

The first stock-selector compile attempt failed before runtime because of a
mismatched delimiter at `rust/web/src/stock.rs:41:30`; it is retained as a
setup failure, not a RED. The writer corrected the rendering and the focused
`api:check` passed on the corrected source (raw log
`/private/tmp/household-e-20261001/E-STOCK-SELECTOR-RED-002/api-check.raw.log`).

The corrected selector-only browser job reached its assertion and produced a
real UI RED: the selected medication option displayed the combined scalar
`15.75 ml remaining` rather than its separate `12.25 tablet` and `3.5 capsule`
balances. One test ran, zero passed, one failed at
`tests/household-stock-selector.test.mjs:57`. Project
`mtcontract-6a21b5385b7240a7`; fixture SHA-256
`889d2b442401e1f19405227a1927a5d48a870747408a2122c91079cd527c788b`; runner
source SHA-256 `031035fc4d386a13756b9962eaff0a05cb64abeb2c4ce5fe57439ee10ff9c7ee`.
Full wrapper log:
`/private/tmp/household-e-20261001/E-STOCK-SELECTOR-RED-002/browser.raw.log`;
actual browser output:
`/private/tmp/household-e-20261001/E-STOCK-SELECTOR-RED-002/browser-node.raw.log`.
The immutable copy is `tmp/contract-tests/run.wb8u3n/source`; all 325 copied
application inputs matched the premanifest. Copy manifest SHA-256
`e315d2d1a9fc8fb91e9bd54c49bde41e4190928ea29e479b4038103858510304`. Root
Taskfile was separately captured and matched at
`6211a8d6623ff9bcf6c1c8f2e530a514c095bd2989c7693a2623525a75c91c5a`. The 326
live inputs also matched pre/post; manifest SHA-256
`ebaab7691e21e6492658662dee317e7fbcb9a3e06da69a7a190caa928ef1cdf3`. Fixture
SHA was recorded before runner cleanup. This selector job wrote no screenshots.

The repaired existing stock baseline now passes all 17 cases:
`openapi_stock_workflows` 3/3, `medication_stock` 9/9 and
`medication_management_security_api` 5/5. Project
`mtcontract-27b5d66921894e7b`; fixture SHA-256
`4e59ed6d4be45ec84a36c0af944a652858d298d0006b180ead7b74935651b468`; runner
source SHA-256 `be624f6f75863fcb591d8741796e3745231f32e36d9490418c08240a059bb02a`.
Full wrapper/test output is
`/private/tmp/household-e-20261001/E-FINAL-BASELINE-001/runner.raw.log`;
the separate RTK task tee is a build-only log at
`/private/tmp/household-e-20261001/E-FINAL-BASELINE-001/rtk-task-full.raw.log`.
The immutable source copy at `tmp/contract-tests/run.09O0Ry/source` matched all
325 copied application paths; copy manifest SHA-256
`cc22f8c2ad6cb397ea80ebe613daffa2b2683d6a65965fffa00bbb405bb32b5d`. The root
Taskfile matched separately. Captured pre/post source manifests are identical
at SHA-256
`b30a159ef70105c4ee903954fbfee1ad97aa6b2f8d3a311888c4138ab52134a4`.

The first combined stock HTTP/browser GREEN attempt reached `household_stock`
and stopped on a test convention mismatch: 3/4 cases passed, while
`scalar_adjustment_records_reason_quantity_and_request_linkage` expected audit
quantity strings `["20", "15.12"]` but the actual persisted event returned
`["20.00", "15.12"]` (`household_stock.rs:113`). No boundary or concurrency
target or browser test ran in this wrapper, so none of those are GREEN evidence.
The writer classified this as a test convention error: persisted audit values
use `Decimal.to_string()` and preserve the database's initial `20.00` scale;
the API's independent read-back formatting remains `20.0`. No product change
is required; the test expectation is being corrected.
Project `mtcontract-e7e42adfa0484880`; fixture SHA-256
`a1cb362097272d041e0824547ff05e4f1d85ea1d5f81eda064a71ff0457b0724`; runner
source SHA-256 `be624f6f75863fcb591d8741796e3745231f32e36d9490418c08240a059bb02a`.
Full output:
`/private/tmp/household-e-20261001/E-CORE-GREEN-001/runner.raw.log`. The
immutable copy at `tmp/contract-tests/run.gkWfOI/source` matched all 325 copied
inputs; copy manifest SHA-256
`68e17f2e3efcc531f60380987f012c95050cbe62cebd5d4d4f753fd55f2ad390`. The 326
pre/post live manifests match at SHA-256
`b30a159ef70105c4ee903954fbfee1ad97aa6b2f8d3a311888c4138ab52134a4`. The
existing 16 root-level screenshot files were archived with checksum manifest
`f8e82044573d2aba8b904a39c24d44175a23426dafbb38ee6ca8204bf4ab760a`; this
wrapper stopped before browser tests and did not write screenshots.

The selector UI RED is with the writer for the bounded presentation fix. The
writer is correcting the audit assertion to the existing persisted-decimal
convention. Once the writer sends a corrected READY/FROZEN packet, rerun the approved core stock
HTTP and browser batch on a fresh validated copy, then run the permission-
mutating case in its own fresh fixture last. If those pass, run the authorised
full `ci:rust-port` gate. The first combined wrapper is cleaned; no E runtime
process is active.
The #2354 Rails system CI failure remains a separate
focused investigation. `task test:ps` showed no Rails test containers. The
preflight task would run `npm ci` because the host dependency marker is absent;
that dependency refresh is outside this queue. No Rails runtime job is active.

E target mapping was checked against the current runner. The stock baseline's
historical `rails` selector dispatches into the Rust API lane; the generic
`contract:run-dosage-health` task remains Rails reference evidence only.
`api:openapi-dosages-acceptance` is a separate existing Rust dosage baseline.
Root approved the bounded E baseline/RED/fast-GREEN/HTTP/browser/full Rust gate
sequence after writer source freeze.

After each journey: focused HTTP plus browser acceptance, all affected locales,
desktop/mobile screenshots with prior baseline capture, and relevant regressions.
After G: full Rust gate and docs build on final stable input. Root alone commits
and publishes. Do not edit product source; return failures to the writer.

## E runtime receipts (2026-10-01)

The isolated stock permission test passed: one contract case and one selected browser case. Project `mtcontract-a25d1ffd38b24d02`; source digest `4ae39a39e47bb6d62ef8cae99345d81ddaebf135fbe4eaea1c9472050dacae57`; all 325 copied application inputs matched. Runner-emitted fixture SHA-256 (before cleanup) was `0c6011584a753f2994e358150c3ca9e1ebda233399418ae92adef2368ec461e2`; an independent rehash after cleanup was unavailable. Full output is `/private/tmp/household-e-20261001/E-PERMISSION-GREEN-001/runner.raw.log`.

The separate existing medication browser regression passed 7/7 cases, with no HTTP target or default browser cases invoked. Project `mtcontract-d44b4cae07e6425b`; fixture SHA-256 `cd70356dc42fd0a6993526c5d3dc1a8b74d4aaefdbcbe63fe3b2684317dcd33e`; source digest `4ae39a39e47bb6d62ef8cae99345d81ddaebf135fbe4eaea1c9472050dacae57`. Its 326-input pre/post manifests match at SHA-256 `9c141b93fb0b0e7d47f35c7665544afcf6c575cc460179607babb36aa31d5adf`; all 325 copied application paths matched. The root Taskfile was separately verified at `6211a8d6623ff9bcf6c1c8f2e530a514c095bd2989c7693a2623525a75c91c5a`. Full runner output is `/private/tmp/household-e-20261001/E-JOURNEY-REGRESSION-001/runner.raw.log`; actual Node test output is RTK tee `/Users/damacus/Library/Application Support/rtk/tee/1790882747_task_api_a9e8c9.log`. The six generated journey screenshots are preserved in `docs/screenshots/journey-medication-rust/e-20261001/regressions/` with checksum manifest SHA-256 `333e90f2bc50d3e2c0454e9d5df836ec83c15465b4eb756961683b11a1f55e11`. The 16 root-level prior screenshots were restored byte-for-byte; their restored manifest SHA-256 is `f8e82044573d2aba8b904a39c24d44175a23426dafbb38ee6ca8204bf4ab760a`.

The first authorised full `ci:rust-port` run passed formatting and initial Rust tests, then stopped in `api:clippy` with three `-D warnings` in `rust/api/src/web_pages/stock.rs`: `clippy::obfuscated_if_else` at line 175, `clippy::unwrap_or_default` at line 194, and `clippy::needless_borrows_for_generic_args` at line 358. These are product-source lint findings, not infrastructure failures. The writer owns the three minimal fixes. Gate raw output: `/private/tmp/household-e-20261001/E-RUST-GATE-001/runner.raw.log`; the 326-input pre/post manifests are identical at SHA-256 `9c141b93fb0b0e7d47f35c7665544afcf6c575cc460179607babb36aa31d5adf`. Once the writer sends stable notice, rerun the authorised full Rust gate on freshly captured inputs. No E acceptance reruns are otherwise planned.

The writer applied the three lint-only fixes and an explicit error-map type. `api:fmt:write` left the 326 captured inputs unchanged. The first focused Clippy attempt exposed an E0282 type-inference issue at `stock.rs:193`; the writer added the explicit `BTreeMap<String, Vec<String>>` annotation. Final `api:clippy` passed with stable 326-input manifest SHA-256 `6ac0208380cd549bf697d7ad6111384880a87f86d2992e2c63c6927c5c705c8e`; raw output is `/private/tmp/household-e-20261001/E-API-CLIPPY-003/runner.raw.log`.

The final full `ci:rust-port` gate passed. Formatting, Rust tests, Clippy, SSR/hydrate builds, and contract-test check completed without errors. Raw output is `/private/tmp/household-e-20261001/E-RUST-GATE-002/runner.raw.log` (SHA-256 `cd2a0ada9da3d78bc36fc250c71fd6e54b2681e95fd7ec788df3f78967d1b893`). Its 326-input pre/post manifests match at SHA-256 `6ac0208380cd549bf697d7ad6111384880a87f86d2992e2c63c6927c5c705c8e`; `git diff --check` passed. The earlier gate attempt remains classified as Clippy failure on the three newly added style findings, not a pass.

## F unchanged baseline receipts (2026-10-01)

All three existing contract targets compiled: `openapi_person_medication_writes`, `openapi_schedule_writes`, and `openapi_pause_lifecycle`. The compile wrapper used the three exact `api:contract-selected-compile TEST_TARGET=...` selectors. Logs are under `/private/tmp/household-f-20261001/F-BASELINE-COMPILE-001/`; the captured 326-input pre/post manifest SHA-256 was `6ac0208380cd549bf697d7ad6111384880a87f86d2992e2c63c6927c5c705c8e`.

The unchanged person-medication baseline ran 11 cases: 6 passed and 5 failed at the shared strict response-key assertion (`openapi_person_medication_writes.rs:416`). The unchanged schedule baseline ran 10 cases: 3 passed and 7 failed at the matching response-key assertion (`openapi_schedule_writes.rs:332`). These were test-helper shape mismatches for two documented OpenAPI response keys; no production failure was inferred. The pause lifecycle baseline passed 10/10. Initial full runner logs are `F-PERSON-BASELINE-001/runner.raw.log`, `F-SCHEDULE-BASELINE-001/runner.raw.log`, and `F-PAUSE-BASELINE-001/runner.raw.log` beneath `/private/tmp/household-f-20261001/`. Fixture SHAs respectively: `fd70e7d0513770d059dbed53883610821424c11149716711259d64ada50f511a`, `b588460b356b13d76c1ffb9004e06353e7a100430dc28aa47e866052692d76ad`, and `46cc4a1906173bcb3d3f464cbb5fb435a03dce3fd4ee058c31b573af55d168fb`. All three had 325 listed application paths with stable 326-input live pre/post hashes; root Taskfile was separately hashed.

The writer corrected only the two response-shape helpers by including the documented keys and asserting their types. Both changed targets were formatted and compiled successfully. One orchestration attempt passed a combined target and variable as a single Task argument; Task printed help and no checks ran. The exact four Tasks were then run separately and passed; receipts are in `F-SHAPE-REPAIR-FAST-002/`.

The corrected person-medication baseline passed 11/11. Project `mtcontract-76de254479af404c`; fixture SHA-256 `c3ac21fc73c07d78b68e7ee20922b4f6a4ae1c9452621e8f49c1486ef79612b8`; runner source SHA-256 `e05895021224476316c18018c9103e603776b6c1628a9e02470e1e6c44feb50b`. The 325 listed application copy hashes matched, copy manifest SHA-256 `68403be95f8502237fa86b22dbddb6cda8ef8a99eb5a4afd2f6366c2411ce8a3`; 326-input pre/post manifest SHA-256 `beb378947aa974fca0406230d0f7485183526c6bdcdc100b6d5d6d3892e85e42`. Full test output: `/private/tmp/household-f-20261001/F-PERSON-BASELINE-002/runner.raw.log`.

The corrected schedule baseline passed 10/10. Project `mtcontract-b140b8b721294288`; fixture SHA-256 `228734fcce764937c01310496548b374c42872530d74fb5bb03a26551f0ca4e0`; runner source SHA-256 `231828dc97cf0ce899f7f7dee6770e10c02a0fe594938314d6bb5157d04fe968`; 326-input pre/post manifest SHA-256 `beb378947aa974fca0406230d0f7485183526c6bdcdc100b6d5d6d3892e85e42`. Full test output: `/private/tmp/household-f-20261001/F-SCHEDULE-BASELINE-002/runner.raw.log`. The source digest and unchanged live-input manifest were retained, but the runner removed the temporary copy before a per-file copied-source manifest could be saved. The writer held source inputs during the run; no source drift is indicated.

## F extracted-module and initial treatment route checks (2026-10-01)

The new `household_treatments` target formatted and compiled. After the approved
responsibility split and removal of ten unused-import warnings, `api:fmt:write`,
`api:check`, `api:clippy`, and the selected target compile all passed. The
352-path source pre/post manifest for these final extraction checks matches at
SHA-256 `b5aec893f9660499e4d38f2df9d3194b451c4d2f1a501b19cc324739e26849ad`;
the raw API check and Clippy outputs are in
`/private/tmp/household-f-20261001/F-EXTRACTION-FAST-002/`.

All three existing acceptance baselines passed against the extracted API
modules: person medication 11/11, schedule 10/10, and pause lifecycle 10/10.
Each run's 351 application-file copy matched its pre-copy manifest at
`23e24ccd62752387bda8734e930c8ba495cf6b9870e4b5e1e30dda804ac816dc`, and each
352-path source pre/post manifest matched at `b5aec893f9660499e4d38f2df9d3194b451c4d2f1a501b19cc324739e26849ad`.
Fixture SHA-256 values and full raw logs are retained in
`F-EXTRACTION-BASELINE-001/`, `F-EXTRACTION-BASELINE-002/`, and
`F-EXTRACTION-BASELINE-003/` beneath `/private/tmp/household-f-20261001/`.

The first treatment wrapper passed household selectors as Task CLI variables.
`run.fish` therefore did not execute the nine-case HTTP target: the outer log
contains no `rust-api-tests-run` container or Cargo test output. The browser
selector did run all ten desktop/mobile locale cases, and all ten failed at
`household-treatments.test.mjs:74`: the person page had zero assignment-create
links instead of one. This is a real browser UI RED; it is not HTTP evidence.
Project `mtcontract-c35e51b6bc1a4bd4`, fixture SHA-256
`16533702d1a27f0fa46c02d02ebc4e715d59f3ee461751a5afb02aff29ca5b86`, source
digest `3a9b92a8d2b150381b4e704b27d160f4918aecfcf54af7c1ccb3a29b6efab9f3`.
All 327 copied application paths matched at manifest SHA-256
`ef6116640cd77c5f7ba32cbb16e31816b78761fff5682bc71f5e79a11924741c`; the
328-path source pre/post manifest matches at
`5e217bfcb76fd653084fba941d378f15152c2637a1b4ae0cdc6c77685b52e466`.
Full outer log: `/private/tmp/household-f-20261001/F-TREATMENT-RED-001/http.raw.log`;
full browser assertions: `/Users/damacus/Library/Application Support/rtk/tee/1790885995_task_api_3bfb49.log`.

The corrected invocation exported the household selectors in Fish before
calling `api:browser-rust`:

```fish
set -gx HOUSEHOLD_ACCEPTANCE true
set -gx HOUSEHOLD_TEST_FILE household_treatments
set -gx BROWSER_TEST_FILES tests/household-treatments.test.mjs
rtk proxy task api:browser-rust BROWSER_TEST_FILES=tests/household-treatments.test.mjs
```

This run selected and executed the intended nine HTTP cases; all nine reached
the existing missing-treatment-editor assertion at
`household_treatments.rs:84` and received 404 instead of 200. The wrapper
stopped before browser tests. This is the expected treatment route RED. Project
`mtcontract-1940e8e0b9a340b6`, fixture SHA-256
`a0bd7a7c1a2c078e669a4d434ab659ca9e4c11fc4585a75c8ba751f9ad709d87`, source
digest `99411d95ab1901581c09321d34dfc6425ad52959d51bc95ea9b10d62f8190bfa`.
All 351 copied application paths matched at manifest SHA-256
`23e24ccd62752387bda8734e930c8ba495cf6b9870e4b5e1e30dda804ac816dc`; the
352-path source pre/post manifest matches at
`b5aec893f9660499e4d38f2df9d3194b451c4d2f1a501b19cc324739e26849ad`. The root
Taskfile was separately verified at SHA-256
`6211a8d6623ff9bcf6c1c8f2e530a514c095bd2989c7693a2623525a75c91c5a`. Full raw
output: `/private/tmp/household-f-20261001/F-TREATMENT-HTTP-RED-002/http.raw.log`.
No browser-only duplicate was run; the initial wrapper already executed the
ten browser cases. No permission grants were changed.

The separate public-wrapper selector probe was first run with
`rtk proxy task api:contract-household-selector-test`; it exited 201 before
Docker or application tests. The controlled fake Fish runner received blank
`HOUSEHOLD_ACCEPTANCE`, completion, stock, test-file and filter values for the
explicit case, while `BROWSER_TEST_FILES` was forwarded. This is an actual
CLI-selector forwarding RED, not an application test result. Raw output is
`/private/tmp/household-f-20261001/F-SELECTOR-RED-001/runner.raw.log`; exit
code, UTC start/end and four-input SHA-256 manifests are alongside it. Their
combined pre/post manifest digest is
`0445132596990aab9a8b750cca2aac8a934a8eaa6ab1804c3d3a9930d6f94e3d`; the
root Taskfile, API Taskfile, selector test and fake-runner fixture all match.
The UTC run interval was 2026-10-01 21:02:36–21:02:37. No Docker/runtime
process was started. After the root-owned Task forwarding fix passed static
review, the same probe passed with exit 0 and printed “Household selectors
preserve defaults, forward requested tests and keep dashboard isolation
order”. This includes the default, explicit, false, inherited, CLI-over-env
and quoted-filter cases. GREEN raw output is
`/private/tmp/household-f-20261001/F-SELECTOR-GREEN-001/runner.raw.log`;
the four-input pre/post manifest digest is
`c615c19afdb089e4f2d7ca82a4e784cf57ed2f39259997bea46263c6a8049a5f`.

The first fast pass passed both formatting steps but `api:check` stopped with
exit 201. Rust reported E0364/E0603 for the extracted
treatment `rows` visibility/re-export (`treatments.rs:17`, `overview.rs:5`,
used at `people.rs:123`) and E0432 for an unresolved `rust_decimal` import in
`treatments/intentions.rs:2`. There was also one non-fatal Leptos warning for
unnecessary parentheses in `schedule_controls.rs:57`. Full output is
`/private/tmp/household-f-20261001/F-FAST-001/api-check.raw.log`. The writer
fixed those issues and added four test-only cases (13 total).

On the renewed frozen inputs, API and web format tasks passed and `api:check`
passed, with only the existing `proc-macro-error2` future-incompatibility
notice. `api:clippy` then stopped with exit 201 on two findings in the new
treatment modules: `context.rs:93` has a complex return type and
`schedule_config.rs:32` compares an owned string. Full output is
`/private/tmp/household-f-20261001/F-FAST-002/api-clippy.raw.log`. The writer
fixed both findings without changing the reported payload behaviour. On the
renewed frozen source, API/web formatting, `api:check`, `api:clippy`, and
`contract-selected-compile TEST_TARGET=household_treatments` passed. The full
`task -d rust/web test` suite passed 52 tests: 3 library, 8 calculation, 9
rendering, 1 dosage-locale, 2 household-completion-i18n, 11 household-i18n, 8
navigation, 9 household-rendering and 1 stock-locale. The only compiler notice
was the existing `proc-macro-error2` future-incompatibility warning. Raw logs
are in `/private/tmp/household-f-20261001/F-FAST-003/`. No Docker job has run
on this final source yet.

The corrected wrapper then ran the frozen 14-case treatment HTTP target with
`HOUSEHOLD_ACCEPTANCE=true`,
`HOUSEHOLD_TEST_FILE=household_treatments`, and
`BROWSER_TEST_FILES=tests/household-treatments.test.mjs`. It executed all 14
HTTP cases: nine passed and five failed, so the wrapper stopped before any
browser case. The failures were:

- `blank_required_end_date_keeps_original_draft_token_key_and_saved_schedule`
  observed a changed `submission_id` (actual `22489ef5-9a11-465a-b032-36848a5fd880`,
  expected `cd39dd49-c9a1-4f41-be37-a0fe9ec2079d`);
- `ordinary_assignment_can_leave_optional_guidance_blank` and
  `ordinary_schedule_can_leave_optional_guidance_blank` returned 422 instead
  of 303;
- `assignment_edit_preserves_existing_null_cycle` and
  `schedule_edit_preserves_existing_null_cycle` returned 422 instead of 303.

Full output and teardown are in
`/private/tmp/household-f-20261001/F-TREATMENT-RED-003/runner.raw.log` (exit
201). Project `mtcontract-82bcdc2ea03f4fec`; fixture SHA-256
`dc722f76135275d1318f40ada413efa1eacc60018cb4b39e3f4c3d24aabb405b`, checked
locally while `fixture.json` existed and also emitted by the runner. The
immutable copy is `tmp/contract-tests/run.Jv2PRV/source`, runner source digest
`882dc83a6ac8d37c8c15b40f3684956ac86dcdcc66cf349323fc0f7c4380db83`; all 344
captured application paths matched; the checked list is
`F-TREATMENT-RED-003/source.copy-check.manifest`.

The source manifest rows are in `source.pre.manifest` and
`source.post.manifest`; their digests differ because root added the approved
runner-lint command to `rust/api/Taskfile.yml` after the runtime command was
resolved (pre `f619b011...`, post `1dcfed8c...`). Root `Taskfile.yml` was
unchanged at `6211a8d6...`, and the validated copied application files were
unchanged. The writer later strengthened only the live browser test after
this run; browser tests were not reached, so that edit was not part of this
HTTP result. The reviewer-approved Fish lint then passed with exit 0; its
eleven command/script inputs match at manifest digest
`98baef65f3d226eee1db6edc1cba07ed785d79ef78a2035bf14f57a046418034`.
Raw lint output is
`/private/tmp/household-f-20261001/F-SELECTOR-LINT-001/runner.raw.log`.

The writer corrected the five test expectations/setup inputs. On a fresh
capture, the API/web format tasks, `api:check`, `api:clippy`, target compile,
and all 52 web tests passed. Their logs are in
`/private/tmp/household-f-20261001/F-FAST-004/` (only the existing
`proc-macro-error2` future-incompatibility notice remained).

The corrected wrapper then ran the 14-case treatment HTTP target and all 14
passed. It proceeded to all ten locale × viewport browser journeys. All ten
hit the same test expectation at `pauseResume` line 91: resumed
`saved.active === true` was false for a schedule starting in 2030, although
`paused: false` passed and the schedule was date-inactive. The writer and
reviewer classified this as a test expectation error and approved comparing
resumed activity with the original activity while retaining exact start/end,
dose, configuration and history checks. Browser output is
`/private/tmp/household-f-20261001/F-TREATMENT-GREEN-001/browser.raw.log`
(SHA-256 `bfe393cc0f7c7bf3e9679a1935e38edf77f9d00f7daa19df7158d0161b1d441c`);
the wrapper log is `runner.raw.log` (exit 201). No product failure was
inferred from this date-inappropriate assertion.

Project `mtcontract-18860ef5f3fc4763`; fixture SHA-256
`fdb11deea63e3f5c538df44ce52cf0b75c05b8742a0b224f7a120e761d37d66a`, checked
locally before cleanup. The copied source was
`tmp/contract-tests/run.Rsv1Sb/source`, runner SHA-256
`86439601c66ff98ef78947c2f255cddf6b5fb12901116ffd2a967e4ff0ad497f`; all 344
captured application paths matched. The pre/post manifest digests are
`803fb26be6aad184ec2fa8cc05d4e2a1d483666b6699bb123ee882ef2632ab85` and
`5f587faff2d51ac6153caebf17b32f4d79127c5b9a6b5cf1b9198546a64e4a72`. Their
only difference is the later-added, unselected
`household_treatment_pause_guards.rs`; the captured API/web application and
selected test inputs matched the copy.

The browser run created 30 new screenshots in the legacy directory. I
verified their source and archive copies byte-for-byte, moved them into
`docs/screenshots/journey-medication-rust/f-20261001/`, and removed only the
30 newly generated originals. Names and checksum receipts are in
`F-TREATMENT-GREEN-001/screenshots.files` and
`screenshots.checksums.txt`; manifest digests are recorded alongside them.

The separate pause-guard target's corrected compile passed after replacing an
unavailable direct `uuid` dependency with a distinct valid constant UUID and
an explicit inequality assertion. Compile log:
`/private/tmp/household-f-20261001/F-PAUSE-GUARD-COMPILE-002/compile.raw.log`.

The 18-case HTTP run used project `mtcontract-95a05b8d04ae4c5c`, copied
application `tmp/contract-tests/run.dQ0rK4/source`, and runner digest
`08f9127a0279bbf73bc26d41bf468e4bfba58a95d3dfc4d356d6f900bf6a9eec`. The
captured copy was validated before releasing the writer. The runner emitted
fixture SHA-256
`609b32b430f55609541349788171c46733b9a8498ca418f8e94395fa517d997b` before
cleanup; it was not independently rehashed. Full output is
`/private/tmp/household-f-20261001/F-PAUSE-GUARD-RED-002/runner.raw.log`.
Actual result: 18 tests ran; 2 public-contract controls passed and 16 guard
assertions failed. The failures expose absent stale/interleaved/mismatched
source-write protection (observed 303 where 409 was expected) and absent
source/period-token handling (observed 303 where 428 was expected). Three
additional retained-form checks reached valid rejected responses (409) but
failed because required `source_type`/`source_id` were absent from the retained
form. The writer classified these as real form-contract failures, not invalid
seed/setup errors. No browser target followed the HTTP failures.

The source manifests differ only by the root-added, unselected
`rust/web/tests/household-treatment-calendar.test.mjs`: pre
`d008e728b435b419dda8e9d15b08bf18b8b62c484fea61e05dd3e514c4f9ec7e`, post
`70bb9c4923c22d3040d87f719893af9630e73fe25dcc448dca07fcec23910302`.
The writer then fixed retained-form source identity and the pause guard. On the
stable candidate, `api:fmt:write`, web format, `api:check`, `api:clippy`,
`api:contract-selected-compile TEST_TARGET=household_treatment_pause_guards`,
and the full web test task all passed. The web suite passed 52 tests; only the
existing `proc-macro-error2` future-incompatibility warning remained. Logs are
in `/private/tmp/household-f-20261001/F-GUARD-FAST-001/`.

The combined guard/form runtime then passed all 18 pause-guard HTTP cases and
all ten corrected treatment browser cases (five locales, desktop and mobile).
Command: `api:browser-rust HOUSEHOLD_ACCEPTANCE=true
HOUSEHOLD_TEST_FILE=household_treatment_pause_guards
BROWSER_TEST_FILES=tests/household-treatments.test.mjs`. Project
`mtcontract-7f64a1bcfc944e39`; copied source
`tmp/contract-tests/run.ZVDmRL/source`, runner digest
`1c3083109189123ffee69a456c02caacc7068bcbd9f335c7e2c455b25b02519f`. I
compared hashes for all 371 copied config/rust/vendor paths; all matched. The
runner emitted fixture SHA-256
`04c005434dedc285c114cd3b9693ea8df6a2d8e2500be9e5e85a1078ccf61692` before
cleanup. Full wrapper output is
`/private/tmp/household-f-20261001/F-GUARD-GREEN-001/runner.raw.log`; the
browser subprocess log records 10 pass, 0 fail. The run produced 100
screenshots, archived at
`docs/screenshots/journey-medication-rust/f-20261001/guard-green-001/`; the
original and archived checksum manifests match and are in the private job
directory.

The two filtered retry-key regressions compiled and ran in project
`mtcontract-f8b8e2069c2a42e7`. Both actual HTTP assertions failed as intended:
pause and resume returned 303 instead of 428 when the original replay key was
missing (`household_treatment_pause_guards.rs:356`). Eighteen other tests were
filtered out and no browser target ran. Full output is
`/private/tmp/household-f-20261001/F-RETRYKEY-RED-001/runner.raw.log`; runner
source digest `a1f3e2c2778365993076856e0f56f0335de2d7b037447a8565c8c724c8abe30d`;
fixture SHA emitted before cleanup
`82a5dd07766af2790701b7c05e6960e0afb791d104f6ec043bda4fd186f9624f`. The
runner removed its temporary source copy at teardown before I could compare its
paths to the live inputs, so independent copy-match evidence is unavailable
for this focused RED. Hash-list attempts made after teardown were invalid
(empty/mismatched lists that included generated `target` paths) and are not
used as source-equality evidence. This is recorded as an evidence limitation;
the test result is the runner's actual assertion output.

The focused retry-key RED compiled and selected exactly two HTTP cases; both
returned 303 instead of the expected 428 for a missing original replay key.
See `/private/tmp/household-f-20261001/F-RETRYKEY-RED-001/runner.raw.log`.
The runner emitted source digest
`a1f3e2c2778365993076856e0f56f0335de2d7b037447a8565c8c724c8abe30d` and
fixture SHA `82a5dd07766af2790701b7c05e6960e0afb791d104f6ec043bda4fd186f9624f`.
The runner removed its source copy before independent comparison; post-cleanup
manifest attempts were invalid and are not treated as copy-match evidence.

On the final composed candidate, all fast checks passed: API and web formatting,
API check and Clippy, selected compile for `household_treatment_acceptance`
and `household_treatment_permissions`, and the full 52-test web suite. Logs are
in `/private/tmp/household-f-20261001/F-FINAL-FAST-001/`.

The composed runtime command selected 37 HTTP cases (14 form, 20 guard, 3
boundary) followed by ten treatment editor and two real-time administration
browser journeys. All 37 HTTP and all 12 browser cases passed. Project
`mtcontract-cbbc07458333490a`; copied source
`tmp/contract-tests/run.xG8PCP/source`, digest
`3f6ec2283f6c47f518236b62637fb657dc9a555eff7a7b52c7827d819479460e`; all 375
copied config/rust/vendor paths matched live. Fixture SHA
`016e0eb5350aceb8e668aebae35ab9fef220511d977ca8d8f9c56f249c787b45` was
independently checked before cleanup. Full output:
`/private/tmp/household-f-20261001/F-FINAL-GREEN-001/runner.raw.log`. The 102
browser screenshots are checksum-verified at
`docs/screenshots/journey-medication-rust/f-20261001/final-core-green-001/`.

The separate fixed-clock calendar run selected two browser cases, but both
stopped at test line 111 while creating the PRN schedule: the fixture submitted
`max_daily_doses=3` and `min_hours_between_doses=0`, received 422 rather than
303, and never reached the date, DST or taper assertions. No calendar
screenshots were saved. Full browser log
`/private/tmp/household-f-20261001/F-CALENDAR-GREEN-001/browser.raw.log`;
project `mtcontract-577c2def8dd644fe`; copy digest
`3f6ec2283f6c47f518236b62637fb657dc9a555eff7a7b52c7827d819479460e` with all
375 paths matching; fixture SHA
`dfb5b2f19b2d3f1adf0e729aa0a09f71b0abf72aaa19260c72bee4a492f5da0c` was
independently rehashed. The test owner is diagnosing the invalid PRN input; the
strict date/DST/taper assertions remain unchanged. The fixed clock was scoped
to the child process and is now unset.

Calendar retry 2 used the reviewed PRN minimum interval of 1 and a case-safe
availability check. Both viewports reached the strict taper assertion, where
all four matching task rows displayed `1.25 ml` instead of effective `0.75 ml`.
The taper projection assertion failed in both viewports; no screenshots were
produced before failure. Raw browser log:
`/private/tmp/household-f-20261001/F-CALENDAR-GREEN-003/browser.raw.log`
(SHA-256 `831a5a9afec35ad2a4336836f5e2ffd67eebff9cf93bb4a6f16989c75f5cb424`).
Project `mtcontract-67aed673e52a44db`; copied-source digest
`258b9a3a221d052d588d4391b62f006425839628ad953dfad94abc469cfd486f`, with all
375 paths matching; fixture SHA-256
`c86c3f72fdd7a6a80dcd7068f44c9b560b5e9b6cedcddf0dc7f5ccb3769213e8` was
checked before cleanup. The March 29 clock override was scoped to that child
process and is now unset.

The isolated permission fixture stopped on its first HTTP assertion, before
the seven selected browser regressions: expected 303 but received 404 at
`household_treatment_permissions.rs:53`. This is not a permission pass and
does not test revocation. Raw wrapper log:
`/private/tmp/household-f-20261001/F-PERMISSION-GREEN-001/runner.raw.log`
(Task exit 201). Project `mtcontract-a99bd63e8d1142b6`; application copy
`tmp/contract-tests/run.0UJoCq/source`, digest
`258b9a3a221d052d588d4391b62f006425839628ad953dfad94abc469cfd486f`; all 375
paths matched the premanifest and copy manifest, whose SHA-256 is
`d964407e7668ac75a71620a5e2c712586041f750009f4fb526cfc83faf3548b6`. The
runner-emitted fixture digest was independently rehashed before cleanup:
`772b7e898f7725a9690be8e8f20a201d6ca2517a91e4552998c805c682d04658`. Clock
override was unset. No screenshots were produced; the six pre-run images
remain untouched.

The permission fixture first failed during ordinary assignment creation at
`household_treatment_permissions.rs:53` (404 instead of 303), before its grant
or revocation assertions. After the reviewed setup repair, the selected
permission case passed 1/1 and the explicitly selected fresh-login medication
browser regressions passed 7/7. Task exit 0; project
`mtcontract-66b063c637524ab4`; source copy
`tmp/contract-tests/run.ai41bj/source`, runner digest
`806e1028ce6e5df16474335b3d81c2398c8b168686c82b98f4253a7b7b6ad3a0`; all 375
paths matched premanifest and copy manifest (SHA-256
`0e2726b6e723b2e839ea84d5bcf5a7149599646839b9f81a582063b2dea3db77`). Fixture
SHA-256 `1948395aa5c55a12c24b82f804785dbf5f2c88c5392b4b4428bd9535417a8cb0`
was rehashed before cleanup. Full wrapper log:
`/private/tmp/household-f-20261001/F-PERMISSION-GREEN-002/runner.raw.log`.
The selected seven browser cases produced six screenshots, archived and
checksum-verified under
`docs/screenshots/journey-medication-rust/f-20261001/permission-green-002/`;
all six tracked pre-run screenshots were restored byte-for-byte.

The reviewed taper display candidate passed `api:fmt:write`, `api:check`,
`api:clippy` and `git diff --check`; only the existing proc-macro-error2
future-compatibility notice remained. The 375-path fast pre/post manifests
match at SHA-256 `26a6c9fb847a0637527b0fcd341708c2f572da9c216c451de1e170369953fde4`.
The focused fixed-clock calendar job then passed both desktop and mobile
cases, including the strict current-day effective amount `0.75 ml`. Project
`mtcontract-f3f5d04bf481417d`; runner digest
`a6429066ce424d0348c3f6d3fe57945cf9a39cceeae29106ae38365c3601cabc`; all 375
copied paths matched the premanifest and copy manifest (SHA
`26a6c9fb847a0637527b0fcd341708c2f572da9c216c451de1e170369953fde4`). Fixture
SHA-256 `881af988e73bc358405529a215c399209ae69c517175cf605a6a5eb42996bd6f`
was rehashed before cleanup. Full wrapper log:
`/private/tmp/household-f-20261001/F-CALENDAR-GREEN-004/runner.raw.log`; two
screenshots are checksum-verified at
`docs/screenshots/journey-medication-rust/f-20261001/calendar-green-004/`.
The fixed clock was scoped to the child process. This verifies displayed dose
amounts, not native due-time selection; #2361 remains open.

The existing fixed-clock dashboard regression also passed 35/35. Project
`mtcontract-9774673fdbaa4c17`; copied source `tmp/contract-tests/run.GYRxtE/source`,
runner digest `a6429066ce424d0348c3f6d3fe57945cf9a39cceeae29106ae38365c3601cabc`;
all 375 copied paths match the premanifest and live postmanifest (SHA
`26a6c9fb847a0637527b0fcd341708c2f572da9c216c451de1e170369953fde4`). Fixture
SHA-256 `004a8989e07ba9ca7875c0cbbf6ac8bf2c0138760d7387d6cabecf4c4fc3254a`
is runner-emitted; the fixture was removed before an independent rehash. Full
browser output:
`/private/tmp/household-f-20261001/F-DASHBOARD-GREEN-001/browser.full.log`
(SHA-256 `bda3676d1919ad0163cb046873332350885c620e3c40fbbe8cf5d604b710cf3a`);
wrapper log `runner.raw.log`, Task exit 0. Ten screenshots were archived at
`docs/screenshots/dashboard-rust/f-20261001/dashboard-green-001/`; seven
tracked baselines were restored byte-for-byte and three new original files
were removed after archive checksum verification.

Next: run the approved Markdown and docs gates once against the final accepted
records. The focused time-control result proves actual due-task times and
control values; displayed taper amount alone did not prove due-time behaviour.
Calendar projection 2 and dashboard 35 results remain scoped to the
amount-display change.

### F-TAPER-TIMES-RED-002 (#2361)

The corrected fixed-clock job passed its visible London date precondition,
then both desktop and mobile cases failed the intended assertion at
`rust/web/tests/household-taper-times.test.mjs`: `Native taper control time_0
must exist` (`0 !== 1`). This is a product RED; the earlier
`F-TAPER-TIMES-RED-001` attempt was setup-only because the test compared title
case against CSS-rendered uppercase `SUNDAY, MAR 29`, and is not evidence for
the product failure. The test-only normalization was reviewed before retry.

Command: `api:browser-rust` with process-scoped
`CONTRACT_DASHBOARD_NOW=2026-03-29T00:30:00Z`, `API_TIME_ZONE=UTC`,
`HOUSEHOLD_ACCEPTANCE=false`, and the sole browser selector
`tests/household-taper-times.test.mjs`. Project
`mtcontract-2fec15ba18ae42f6`; task exit 201; 2 failed, 0 passed. Runner source
digest `a9ccc47d93469deebaa3c1346e3654a90dc39ce832d3efc38fefd25360805f66`.
The captured source-copy manifest reproduces that digest; all 371 authored
premanifest paths match, with five extra copied config/hidden generated paths.
Pre/post authored manifest SHA-256 is
`4eeb110adfd4b7a0c686ed34228e24118b3848926f463406e8f8061588a95710`; the
root and API Taskfile input manifest is unchanged at
`5d88f11ca0cacfc603f397a7c609e4b19877d638129b7faec8df032404a9e4be`.
Runner-emitted fixture SHA-256
`3783a0345f54bdb8a15609aa0b678ae405c5d31b7cc1f79da12f380d5b8b18e9` was
independently rehashed before cleanup. Full raw wrapper log:
`/private/tmp/household-f-20261001/F-TAPER-TIMES-RED-002/runner.raw.log`;
full browser assertion output:
`/Users/damacus/Library/Application Support/rtk/tee/1790895691_task_api_693521.log`.

### F-TAPER-TIMES-GREEN-001 and F-CORE-GREEN-002 (#2361)

The stable candidate passed formatting (`api:fmt:write` and `task -d rust/web
format`), `api:check`, `api:clippy`, and all 52 Rust web unit/integration tests.
Only the existing `proc-macro-error2` future-incompatibility notice appeared.
The focused fixed-clock timing browser run passed both desktop/mobile cases,
including creation and editing of shared times and corresponding due tasks.
Project `mtcontract-b799792b923e4029`; task exit 0; runner digest
`b7b76b52d9061904bebb33bca03f3357e970f32e903c1cf057dbf40c47367c62`; fixture
SHA-256 `5aa7fbeabfcfe6269efce99f697951cb8528deaefa050481fd66671b2faa9f24`
was rehashed before cleanup. All 371 authored source paths match the copied
source; five config/hidden generated paths are additional. Source pre/post
manifest SHA-256 is `677bde3ee6254bbed9e893e97263d23af2d0846b4ef02b2db75e4ba11aace14d`.
Raw wrapper log:
`/private/tmp/household-f-20261001/F-TAPER-TIMES-GREEN-001/runner.raw.log`;
six checksummed desktop/mobile create/edit/due/invalid screenshots are under
`docs/screenshots/journey-medication-rust/f-20261001/taper-times-green-001/`.

The final real-time core run then passed all 37 HTTP and 12 browser cases in
one fixture, with dashboard clock override unset. Project
`mtcontract-bdea7115ce3945c9`; task exit 0; runner source digest matches the
371 authored source paths above. Fixture SHA-256
`98221eb28948e56820740c972537444beca1055d1493c54ead2fce99d56ce12a` was
independently rehashed before cleanup. Source pre/post manifest SHA-256 is
`677bde3ee6254bbed9e893e97263d23af2d0846b4ef02b2db75e4ba11aace14d`; the
root/API Taskfile manifest is unchanged at
`5d88f11ca0cacfc603f397a7c609e4b19877d638129b7faec8df032404a9e4be`. Full
raw wrapper output:
`/private/tmp/household-f-20261001/F-CORE-GREEN-002/runner.raw.log` (SHA-256
`588181c504d5166e55d1afc2f3f3230262078694868fbdeec57d0b1670865a44`). The
102 desktop/mobile locale and journey screenshots are archived and checksummed
at `docs/screenshots/journey-medication-rust/f-20261001/core-green-002/`.

`ci:rust-port` passed after the final candidate, including format check, Clippy,
SSR/WASM builds, API tests, web tests, and contract-test compile. Task exit 0;
raw log `/private/tmp/household-f-20261001/F-FINAL-RUST-001/ci-rust-port.raw.log`
(SHA-256 `d7c8f81e55f55fe33769a2c3f14dbfe7542b09a03a7b28fdb698f221eb07c62d`).
The 371-path authored source manifest remained unchanged at SHA-256
`677bde3ee6254bbed9e893e97263d23af2d0846b4ef02b2db75e4ba11aace14d`; the
root/API Taskfile manifest remained
`5d88f11ca0cacfc603f397a7c609e4b19877d638129b7faec8df032404a9e4be`.

Publication screenshot set: 102 files at
`docs/screenshots/journey-medication-rust/f-20261001/core-green-002/`; six at
`docs/screenshots/journey-medication-rust/f-20261001/permission-green-002/`;
two at `docs/screenshots/journey-medication-rust/f-20261001/calendar-green-004/`;
six at
`docs/screenshots/journey-medication-rust/f-20261001/taper-times-green-001/`;
and ten at `docs/screenshots/dashboard-rust/f-20261001/dashboard-green-001/`.
Obsolete F intermediates are retained privately at
`/private/tmp/household-f-20261001/obsolete-screenshots/`: 102
`final-core-green-001` and 100 `guard-green-001` images. Before/after checksum
manifests show no changed bytes. No earlier D/E screenshot paths were altered,
and no other F screenshot directories remain in the checkout.

## G initial stock and pagination RED

The first four-case run was setup-only: all four tests stopped creating the
parent medication because the helper sent `reorder_threshold: null`, which the
canonical validator rejects as blank. No stock or pagination assertion ran.
The setup review confirmed `"3"` is a valid medication threshold while null
option stock and nullable dosage fields remain valid stimuli. This first run's
task transcript preserves the result; its private job directory has manifests
only, and the available RTK tee files contain image-build output rather than
the test assertions.

After the writer's test-only correction, selected compilation passed for
`household_inventory_completion`. The corrected HTTP run executed all four
cases: two controls passed; parent-stock fallback removal returned 422 instead
of 303, and the native page for the 501-option fixture returned 503 instead of
200. These are the intended product REDs; browser smoke was skipped because the
HTTP target failed. Task exit was 201. No product source changed before these
failures.

Command: `HOUSEHOLD_ACCEPTANCE=true HOUSEHOLD_TEST_FILE=household_inventory_completion
API_TIME_ZONE=UTC rtk proxy task api:browser-rust
BROWSER_TEST_FILES=tests/household-routes.test.mjs`. Project
`mtcontract-493bcdcbb3a54234`; runner source digest
`93ff81de351d3b8469103801a8d8cef47d4d8e782e4eefaf16d55c003e44c378`; fixture
SHA-256 emitted before cleanup
`6fda8987b4c1b19c8fb2a91494ce740b243d7b26dc29f51222f6e0d738b325bb`. The 370
application paths match exactly between
`/private/tmp/household-g-20261002/G-INITIAL-RED-002/source.app-pre.manifest`
and `source.copy.manifest`; each manifest hashes to
`c327f4db007a91375b7f7fcf8ba2509a109c917d26407f97aa7bcadf41898063`. The
four runner/command inputs, including the root Taskfile, are recorded
separately in `command-inputs.manifest` (SHA-256
`87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`). Full
raw task log:
`/private/tmp/household-g-20261002/G-INITIAL-RED-002/runner.raw.log` (SHA-256
`40485820728b23bd6a4eabd37a977235e38a1fd3357e8c351cad35e70dc6aa12`). The
374-path premanifest is
`/private/tmp/household-g-20261002/G-INITIAL-RED-002/source.pre.manifest`
(SHA-256 `0fdc527bd5cae98a9e8a9e95a2f3886deb4266e078c7d89d92298678d0cd30e2`);
the four-input subset is separated from the app-copy comparison. No browser
run occurred.

## G focused stock and pagination GREEN

The reviewed focused fix passed the same four HTTP cases and the selected
household routes browser smoke. HTTP: 4 passed, 0 failed. Browser: 6 passed,
0 failed. Task exit 0. The parent-stock fallback removal and 501-option native
medication page now pass; both negative stock controls remain green. This is
focused acceptance only; broader callers, permission-change consistency and
desktop/mobile stock journeys remain queued. The routes smoke produced no
screenshots.

Before the runtime, `api:fmt:write`, `task -d rust/web format`, selected compile
for `household_inventory_completion`, and `api:clippy` each exited 0. Runner
project `mtcontract-cfff38d818d74e18`; source digest
`f33c7aa0be99d84d7d68351361b5e3406bb6563cf83b49d204749c05255d651c`; fixture
SHA-256 emitted before cleanup
`31a6adb3f6bee42ef626a6a09d5635d5ff545805b11527e2bb3197dfd68d74d5`. Full raw
log: `/private/tmp/household-g-20261002/G-FOCUSED-GREEN-001/runner.raw.log`
(SHA-256 `1aab6a8511a56bffd636df287632910947195180e3bf9d990a3b7bc46824da3a`).
The 370 application paths matched the captured copy exactly (pre/copy manifest
SHA-256 `8874bc1cc0454b84cce05b6a5385d2b5843c4312353454ae4271b83b7e85e10b`).
Full 374-path pre/post source manifests match at
`e3490e9dead526a525bec2d5c20207f66690e4489bc196ba08ec151a1cfcc337`; the four
Task/runner command inputs are separately recorded and unchanged at
`87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`.

## Portable-imports baseline preparation

The named compatibility targets compile successfully before any importer
extraction: `api:contract-selected-compile
TEST_TARGET=openapi_portable_writes` and the same Task with
`TEST_TARGET=portability` each exited 0. Logs:
`/private/tmp/household-g-20261002/G-PORTABLE-BASELINE-COMPILE-001/openapi_portable_writes.raw.log`
and `portability.raw.log`. The first target has two cases, including encrypted
bundle strict-shape validation, dry-run and apply; the second has three, including
encrypted export and public readback plus authority/rejection cases. The exact
existing runtime wrappers are `api:openapi-portable-writes-acceptance` and
`api:openapi-portability-legacy-acceptance`; each owns an isolated project and
fixture. Runtime baselines remain queued until the next frozen input/copy is
captured. The 374-path compile-input pre/post manifests match at
`e3490e9dead526a525bec2d5c20207f66690e4489bc196ba08ec151a1cfcc337`; the four
runner command inputs match separately at
`87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`.

## G minor readiness and locale guidance

The isolated `household_minor_readiness` runtime reached its actual HTTP
assertion: the authorised person page returned 403 instead of 200 (0/1 passed,
locale `en`, assertion at `household_minor_readiness.rs:124`). This is an
access-behaviour RED, not a fixture setup failure. Runner project
`mtcontract-7a7a52b8fa684c35`; emitted source digest
`73ad1609c6062abc4fff179ace800a7ac7cec7c81c3a7d525008f84892d7b9f8` and
fixture SHA-256 `0590bc1c746d8f2c3d7ac05ae9fe52bdf569f01a20c2ce4ef87a6bbaacaea0d3`.
All 373 application paths in the captured copy matched the pre-copy manifest
(SHA-256 `5039bef3a9a0d4fe7ec5bff3665fb03f9c4e5ca269c3227cbc7c4eb35e00bee1`).
The writer was released after that validation and edited only the locale/form
guidance paths; the minor test result remains evidence from the captured copy,
not a claim that the live checkout stayed unchanged. Full raw log:
`/private/tmp/household-g-20261002/G-MINOR-RED-001/runner.raw.log` (SHA-256
`f5a20f7c5539e03ef93600f2c645078756ea0dae0fdce95b8961295bd1eba883`).

After the guidance candidate passed independent static review, the web formatter
and focused `readiness_locale` task completed successfully. The task ran all ten
cases: five locale-specific missing-version guidance cases and five honest
editing-placeholder cases, 10 passed/0 failed (Task exit 0). The focused source
list, including all five locale files and the test, matched before and after;
the pre/post manifest files have identical SHA-256
`223a97b87159c6d1fa6e55aef95af052b6ed071d307bbed9cdbd346d4b2118a5`. This is
focused-input stability, not a whole-checkout unchanged claim. Formatter log:
`/private/tmp/household-g-20261002/G-READINESS-GREEN-001-format.raw.log` (SHA-256
`f4eabc9ffeef72558bf83646d1ddc938dafc1b734452112c9ce0b3487d1d59a6`). Test log:
`/private/tmp/household-g-20261002/G-READINESS-GREEN-002/readiness-locale.raw.log`
(SHA-256 `abe393c3612446f57081744434b3b516d860d1d612ddb25a51d3ac33561057e7`);
exit receipt is `task-exit.txt`. One earlier invocation did not start because its
private job directory was absent; it ran no test and is not counted as a result.

Next: after independent review of the frozen minor fix, run its one-case GREEN
## G minor readiness focused GREEN

The reviewed one-case GREEN passed. `household_minor_readiness` ran 1/1 HTTP
case (Task exit 0), including the previously denied authorised person page and
its schedule-only forbidden path. Runner project
`mtcontract-858a464417cc4b84`; emitted source digest
`9e90c2352490886cce2ca6933c36838c4a9c5d27f4ff25ceb35190ff5d3545f4`; fixture
SHA-256 emitted before cleanup
`b76c577497a80caaa762f63149a9f146b6000533f2e792cb6b465a60809f2eaa`.
All 373 captured app paths matched the runner copy (no mismatches; app-pre
manifest SHA-256 `4e3ae04ac7c970698cc93789120153551ab238ad33a3c3d84d7ea43c105d2466`;
copy manifest SHA-256 `9e90c2352490886cce2ca6933c36838c4a9c5d27f4ff25ceb35190ff5d3545f4`).
The runner copy had seven additional generated/hidden files, listed in
`/private/tmp/household-g-20261002/G-MINOR-GREEN-001/source.copy-extras.manifest`
(SHA-256 `0fd19f12a50c3c551b095fe2da98a79dee25a41688a874c9b2f825abb67443c1`).
All 377-path pre/post manifests match at
`fb6457408ceafafcd464b0e8e8fdc3d340e9de6758750ed740665a59037fc36e`; the four
command inputs also match at
`87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`. Raw log:
`/private/tmp/household-g-20261002/G-MINOR-GREEN-001/runner.raw.log` (SHA-256
`8cee83f24dc14379161f56f5502fd2d3c7014051fccff6c8f859f210fe12652b`).

The writer was released immediately after copy validation; subsequent live
changes are unrelated locale/form guidance work. The runner used the validated
copy. Independent review found an additional empty-state truthfulness issue
outside this one-case fixture: when schedules are forbidden and assignments
are empty, the renderer must not claim there are no schedules. A two-case
renderer regression (unavailable empty state plus available-empty legacy
control) is queued before final acceptance.

## G overview empty-state renderer RED/GREEN

The two-case `overview_access_locale` target ran without a fixture. The normal
available-empty control passed; the unavailable-schedules/empty-assignments
case failed as intended because the renderer still claimed schedules were
absent (`Restricted schedules cannot be reported absent in en`, assertion at
`overview_access_locale.rs:10`). This is a real truthfulness RED, not setup.
Task exit 201. The focused pre/post source list matches at SHA-256
`70947c5f7f771715d73643d476199d04bc1a376fb5427c938dd61ea3f06c45d6`. Raw log:
`/private/tmp/household-g-20261002/G-OVERVIEW-LOCALE-RED-001/overview-locale.raw.log`
(SHA-256 `d5924146800895318cb8a7ea1585bfba3d9712bde985b2c3b14d43672e21fa16`).
The narrow renderer fix passed both cases (2/2, Task exit 0): the available-empty
legacy message remains, while unavailable schedules no longer imply that no
treatments exist. Focused source pre/post manifests match at
`70947c5f7f771715d73643d476199d04bc1a376fb5427c938dd61ea3f06c45d6`. Raw log:
`/private/tmp/household-g-20261002/G-OVERVIEW-LOCALE-GREEN-001/overview-locale.raw.log`
(SHA-256 `5d2be08c02c3c3b4e834fb47467ee9a07c5c1267bbf892689d75f04742e1fa7c`).
Metadata limitation: the focused list omitted the changed implementation file
`rust/web/src/treatments/overview.rs`; no hash or before/after equality is
claimed for it. The successful local Cargo task ran against the live checkout,
which contained the fix, but this receipt has no immutable source-copy proof.

## G assignment persistence controls

The reviewer-approved `household_assignment_readiness` target compiled and ran
in its own fresh fixture. Both existing-behaviour HTTP controls passed: linked
assignments preserve a blank selection and require an explicit matching
replacement without rejected writes; unlinked manual doses stay unlinked. The
selected `household-routes` browser smoke also passed all six cases. Task exit
0. Project `mtcontract-fef6d1715f294a4a`; source digest
`18dd9f89eb75950cc080fb6cbc28dac7ef5d153d1a108f4049e432692a723d9b`; fixture
SHA-256 `461cfe6dccdc8f3ee5befe856c3e21bc4f7335e6a996ff321a7b7ce58c4e8a35`.
All 374 app paths matched the copy (zero mismatches; app manifest SHA-256
`4945e42a48bfdc819422fa19734e46bab31952e31250b47f9d4d4bbd9b5dc2f6`; copy
manifest SHA-256 `18dd9f89eb75950cc080fb6cbc28dac7ef5d153d1a108f4049e432692a723d9b`).
The eight generated/hidden extras are recorded in
`/private/tmp/household-g-20261002/G-ASSIGNMENT-GREEN-001/source.copy-extras.manifest`.
Full 379-path pre/post manifest SHA-256 is
`4c4446fec9f638e9ab24159ecf1c6f6f32c56c3402583058b773a4545df5f305`; command
inputs match at `87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`.
Raw log: `/private/tmp/household-g-20261002/G-ASSIGNMENT-GREEN-001/runner.raw.log`
(SHA-256 `2676d83cc520bf100423f26ca8d56eb59f1b7c70ef10e1307fbb8c2677f6ce4d`).

## G portable-imports existing-behaviour baselines

Before any importer move, both existing portable suites passed serially, each
on a fresh fixture using the same source candidate (`18dd9f89...`).
`api:openapi-portable-writes-acceptance` passed 2/2, including strict encrypted
bundle validation, dry-run/apply and sync-batch result; project
`mtcontract-242da249726346ee`, fixture SHA-256
`cae8f7a45b5d23af7f540bbd9b246b4ea22a2775524e54c0f533e370e909241b`, raw log
`/private/tmp/household-g-20261002/G-PORTABLE-WRITES-BASELINE-001/runner.raw.log`
(SHA-256 `627454ab5649232f63972211457bfc22dbe34da1aa54cf5792f655986f421a92`).
`api:openapi-portability-legacy-acceptance` passed 3/3: encrypted export/public
readback, authority enforcement and cross-person reference rejection; project
`mtcontract-ea33331e4cb64b15`, fixture SHA-256
`87289533a6cf8d3e71ceb74235c57c8712f4dba22806bb656f27d1a9ea45878c`, raw log
`/private/tmp/household-g-20261002/G-PORTABILITY-BASELINE-001/runner.raw.log`
(SHA-256 `93bafce6da35dddd33e155b28f21ae5e2935335b8415f98c395ff822c34c51a1`).
Both jobs had 374 app paths match the runner copy (zero mismatches, eight
generated/hidden extras) and unchanged 379-path source pre/post manifests
(`4c4446fec9f638e9ab24159ecf1c6f6f32c56c3402583058b773a4545df5f305`); four
Task/runner input files match at `87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`.
All five pre-extraction baseline cases passed. The one approved importer split
has now passed its formatter/check/Clippy and three selected compiles. The
post-split `api:openapi-portable-writes-acceptance` passed 2/2 on fresh project
`mtcontract-638acc27b7ad4a9d`; `api:openapi-portability-legacy-acceptance`
passed 3/3 on fresh project `mtcontract-4913708771d6458b`. Both emitted source
digest `959deae28f2e1a8f41f7752ac2422f53c73b6c41751ed73b0049148c38a84d19`.
For each job, a 392-file copied-source manifest matched the live app manifest
with no differences; both manifests hash to that source digest. The first
fixture SHA-256 was `6c9db253dcaf73a94d92974f6a7a42cb2f721e27fcf143525da9a94b2a5649f8`;
the second was `ff8c995a735d28dc8ad18b04cc62420458ab015ae1129db5bfb6d1332c2fafa4`.
The post-split raw logs are
`/private/tmp/household-g-20261002/G-PORTABLE-SPLIT-GREEN-001/runner.raw.log`
(2/2; SHA-256 `0c7da514725b7bb63d58f4d644646660b8e8c86ab1ac65bb5e537b3aabe6b2a5`)
and `/private/tmp/household-g-20261002/G-PORTABILITY-SPLIT-GREEN-001/runner.raw.log`
(3/3; SHA-256 `7d8624c1c1b70a15dda604c005bd6aeabec4875ae4c2a449f61c9044391c06a0`).
The split fast batch is at
`/private/tmp/household-g-20261002/G-PORTABLE-SPLIT-FAST-002/`; formatter exit
0, `api:check` exit 0, Clippy exit 0, and selected compiles for both portable
targets and `household_final_acceptance` all exited 0. Its source pre/post
manifests match at SHA-256
`6f2f5359d1e157e9f8b6fa08787ca0ef932b367ba687018be4f81ba1864c4482`; formatting
was run before the final compile/runtime candidate. The 41-body move receipt
remains a pre-format identity comparison, not a whole-file identity claim.

The first private wrapper-probe invocation was a setup failure and is not the
RED: its evidence is retained at
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-001/red.raw.log` (exit 1;
SHA-256 `8ee169301d529ee9a8418fc8db764bc78619e08f4613686ae38b2f2d9738cf0a`).
After the `fish --no-config` correction, the legacy helper produced the intended
RED: its node-only selection missed the prepare/restore phases required for a
separate minor fixture; the injected browser failure status was also observed.
Eight assertions failed as expected. Raw output:
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-002/red.raw.log` (RTK Task
wrapper exit 201; inner probe Task exit 1; SHA-256
`c750af5e9efa3b165ff32b465f837802f3bba1a7d277bc440d43e877b08e8f97`).
The candidate GREEN then stopped in fixture validation because jq rejected the
escaped dot in the email regex; no valid-fixture calls reached the wrapper.
Raw output:
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-002/green.raw.log` (RTK Task
wrapper exit 201; inner probe Task exit 1; SHA-256
`39d1f500a9e13910369a6d4f02e1c86067577be5e398f0135e393648effda063`).
The private helper is being corrected to express a literal dot with `[.]`;
candidate GREEN retried after that correction but the fake returned 92 because
its fixture expectation used the original path while the wrapper deliberately
passes `realpath`. Raw output:
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-003/green.raw.log` (RTK Task
wrapper exit 201; inner probe Task exit 1; classify as fake expectation/setup,
not application behaviour; SHA-256
`f88b38b162524776b3b58b26ddd3bda06ee459020e7e854bb3b3e98cb9401b41`). The
canonical-path repair then passed candidate GREEN with all nine fake-Task
assertions (exit 0), covering normal node-only selection, prepare/node/restore
ordering, browser failure return, prepare/restore failure and invalid identity
guards. Raw output:
`/private/tmp/household-g-20261002/G-WRAPPER-PROBE-004/green.raw.log` (SHA-256
`a828ebb74d92e451f9072cff93b9722f7aa3e461f73888679efc55f3f6a47588`). The
writer installed the isolated minor HTTP/browser packet: one
`household_minor_readiness` HTTP case plus ten explicit minor browser locale /
viewport cases, with fixture-only prepare/restore SQL. Root Task wiring and
review are required before runtime; do not repeat the wrapper RED/GREEN.

## G large People collection check

`household_people_collections` compiled and passed its isolated HTTP case,
which confirmed complete large medication, assignment and schedule collections
on the person detail and treatment forms. The selected route smoke passed all
six cases. Task exit 0. Project `mtcontract-949b62aa267a427f`; source digest
`18dd9f89eb75950cc080fb6cbc28dac7ef5d153d1a108f4049e432692a723d9b`; fixture
SHA-256 `d28b6c8fea28beb421e1d0c03172e6da8fd099e1f135bedd2b481dc0cbd640ef`.
All 374 app paths matched the copy (zero mismatches; app manifest SHA-256
`4945e42a48bfdc819422fa19734e46bab31952e31250b47f9d4d4bbd9b5dc2f6`; copy
manifest SHA-256 `18dd9f89eb75950cc080fb6cbc28dac7ef5d153d1a108f4049e432692a723d9b`).
Eight generated/hidden extras are listed in
`/private/tmp/household-g-20261002/G-PEOPLE-COLLECTIONS-GREEN-001/source.copy-extras.manifest`.
Full source pre/post manifest SHA-256 is
`4c4446fec9f638e9ab24159ecf1c6f6f32c56c3402583058b773a4545df5f305`; command
inputs match at `87f26392fc4b6a7938ea08e8d92900477d7137a9d3ee6d3cd1a3e8407808afb1`.
Raw log: `/private/tmp/household-g-20261002/G-PEOPLE-COLLECTIONS-GREEN-001/runner.raw.log`
(SHA-256 `5eae194b25b757444a60407577ef83edcb907129e200912e52612daeff70ffc0`).

## G composed inventory acceptance

The final composed `household_final_acceptance` target passed all seven serial
HTTP cases: two assignment controls, four stock cases and the complete People
collection case. The selected inventory browser file passed all ten cases in
five locales on desktop and mobile, including the mobile navigation gesture.
The same isolated wrapper run used project `mtcontract-ba9fe25cbd834640`, code
digest `8611a75140874abd71a8d904ae6f7209e13f422d85095628e0482c7505f71592`, and
runner-emitted fixture SHA-256
`220a619a8521fafd479f2e3fbfd4a108b3ce674917f5c378805b8ce264131862`. Its
393-file copied-source manifest matched the live app manifest exactly; both
hash to the code digest. Task exit 0. Selected target compile log:
`/private/tmp/household-g-20261002/G-FINAL-INVENTORY-GREEN-001/compile.raw.log`
(SHA-256 `27cb7d8b87f50cfc91fd6ac37f4bf55fefab5bb7e239a1a68e2abe5694e9c319`).
Full wrapper output:
`/private/tmp/household-g-20261002/G-FINAL-INVENTORY-GREEN-001/runner.raw.log`
(SHA-256 `c986e54ac728c799bdd806a85edc32e257beb7defb64c518164a733239be3ef8`).
The 25 new untracked screenshots were moved into
`docs/screenshots/journey-medication-rust/g-20261002/inventory/`; no tracked
image was overwritten. Their checksum list is
`/private/tmp/household-g-20261002/G-FINAL-INVENTORY-GREEN-001/screenshots.sha256`
(SHA-256 `b202b8d70bbe51d500140d6ce63a2e4705bd871d966663fd2f93a1b3e7173a82`).

## G minor readiness — setup failure after HTTP pass

The installed isolated minor packet passed its HTTP case (1/1): the ordinary
viewer could read the authorised person and assignments while the schedule
index remained forbidden. The browser phase did not start. Its fixture-only SQL
prepare task was stopped by the Task precondition with “Minor browser setup
requires an owned disposable contract project and a valid phase”; therefore
this is a harness setup failure, not a browser or product failure. The wrapper
then completed disposable-project teardown, including removal of its storage.
No screenshots were produced.

The job used project `mtcontract-9af1f246d51d49fc`. The runner emitted fixture
SHA-256 `c01535acf4987400e46a7650a11c6f86179fb071b30e42cb937fa961fbbcbe94`
before teardown. Its contract source digest was
`d0cb04876a6f487de0b1591a488716c1414bdcf448289033d2a65dba231b9d19`; the
400-file copied and live application manifests matched exactly and each
manifest hashed to that digest. Raw output is
`/private/tmp/household-g-20261002/G-MINOR-ACCEPTANCE-001/runner.raw.log`
(Task exit 201; SHA-256
`b604cbc20a5702cc695c9c718cc7f011c6ce65cef2ef18fe7c13e6d75ebfc459`). The
copy and live manifests are in the same job directory. The rejected task was
`api:contract-minor-viewer-sql`, invoked for the `prepare` phase with the
valid-looking owned project name; no SQL mutation or browser assertion ran.
The SQL Task guard contained an unescaped dollar sign inside a Fish double-
quoted expression. Root escaped the regex anchor; independent review confirmed
the corrected guard still rejects non-owned projects and invalid phases.

The corrected isolated retry passed the HTTP case (1/1), fixture SQL prepare
and restore, and all ten browser cases (five locales on desktop and mobile).
Task exit was 0. Project `mtcontract-2189e058ce2748d7`; runner-emitted source
digest `9bae6448d851ce01846b1630c7c3f9d4f43ba20ecf67af2b855132d860dcaf2f`;
runner-emitted fixture SHA-256
`54a19104a27f8215c6e8c550ff5ad911eebbc3abacccd9b932c232ea93f83e73`. The
source digest is the available source receipt for this run; independent
per-file copy/live manifests from the original run were not retained, so
equality to that deleted copy is not claimed. A later read-only
`api:contract-source-snapshot` copied the current application into
`/private/tmp/household-g-20261002/G-MINOR-SOURCE-RECON-001/source`; its
400-file manifest, generated with the runner's normalized relative-path,
sorted per-file SHA-256 algorithm, hashes to the same emitted digest
`9bae6448d851ce01846b1630c7c3f9d4f43ba20ecf67af2b855132d860dcaf2f`. This
reconstructs current source identity and does not independently compare the
original deleted runtime copy. The full wrapper log is
`/private/tmp/household-g-20261002/G-MINOR-ACCEPTANCE-003/runner.raw.log`
(SHA-256 `5efd39e1ba1546691d5fc4a8f83062cd05bb23f085b2e115a124253a4cf4c038`);
the complete browser output is
`/private/tmp/household-g-20261002/G-MINOR-ACCEPTANCE-003/browser.raw.log`
(SHA-256 `6bc68f395217fefef9efb5bb0e286e5753539bce3d3bdc9e1a7ac3350483dccf`).
Twenty screenshots were moved to
`docs/screenshots/journey-medication-rust/g-20261002/minor-readiness/`; their
checksum manifest is
`/private/tmp/household-g-20261002/G-MINOR-ACCEPTANCE-003/screenshots.sha256`
(SHA-256 `ec3062702d1d2b4c25598d34fc4189d2e208fa67d180dd3e973469579736eebd`).

One intervening non-escalated retry stopped during pinned dependency setup
because sandbox DNS was denied, then could not access the Docker socket during
cleanup; it ran no application assertions. Its raw log is
`/private/tmp/household-g-20261002/G-MINOR-ACCEPTANCE-002/runner.raw.log`
(exit 201; SHA-256
`556c4ae4df3b1036b3ab7717dcf6878b9f9fed1da8b269f4b6198eff567ae216`). The
only retained run was owned project
`mtcontract-e11db9b9ed98432c` at `tmp/contract-tests/run.6rhTtw`; after
escalation, the existing `contract:cleanup` Task exited 0 for that project and
its empty owned run directory was removed.

## G adjustment-reason audit probe

`household_adjustment_reason` compiled with two unused-helper warnings and no
compile errors. Its isolated HTTP run passed the 300-character adjustment
reason case, including the stock and audit write. The varied 8,192-character
case reached the API and returned 500 `internal_error`; stock remained `20.0`
and version/sync counts remained `(1, 1)` before and after. The diagnostic also
reported an audit column type of `character varying` with no reported length,
and only the event index on `versions`. This is an actual oversized-input
failure with no partial stock/sync write; it does not establish a maximum-size
or indexed-boundary contract. The six route smoke cases did not run because the
HTTP target failed.

Project `mtcontract-e3f2ea78e3cd4ece`; emitted source digest
`c20d950e0c188326b23de7e4f60447b4623bb27d7d565c4d4962874bea0b127b`; fixture
SHA-256 `1f1421256623fbf90b58f692e7d153d9215605e1b7feb000cdbd67a39311ec47`.
The wrapper exited 201 after the contract test's exit 101. Raw log
`/private/tmp/household-g-20261002/G-AUDIT-PROBE-001/runner.raw.log` (SHA-256
`4f85ed7410d1dc757b4661af748c19fe87c4013819c2a953ea078b47e921e01f`); focused
compile log `compile.raw.log` in the same directory (exit 0; two dead-code
warnings; SHA-256
`91735dd2dc7aba1e4b0551e45d05b2c6b46402be24999012e16a01f113a54dfb`).

### Audit event-budget RED

The writer's four focused `event_budget` cases compiled successfully and all
four reached their assertions (two other tests were filtered). Both exact
1,024-byte inputs, ASCII and multibyte, stored no structured `detail` reason.
Both 1,025-byte inputs retained the full reason in `versions.event` rather than
the required quantity-only label. This is the expected RED for the bounded
storage fix; no browser smoke ran after the HTTP failure.

Project `mtcontract-3624013cc2824551`; runner-emitted source digest
`3c9a483664f20179363760138fe51574e9f79ed1cc92241f65436c29af56af45`; fixture
SHA-256 `8a875a6bb76171d49f7f37b74e17f866701f975d12b54d60aa685f7483b6a400`.
The wrapper exited 201 after the selected Cargo test exited 101. Raw log
`/private/tmp/household-g-20261002/G-AUDIT-BUDGET-RED-001/runner.raw.log`
(SHA-256 `f7d87c6bfe000f8d635ca7dd85a64d0085d6fc299ad9df6dedfc001753c02c6b`);
compile log `compile.raw.log` in the same directory (exit 0, two unused-helper
warnings; SHA-256
`c92ef527662301be3d55a2c84983eaa7c325f204e54a3333411eb7eaf2cdf160`).

### Bounded audit fix GREEN

After the four boundary REDs, the writer kept the complete reason in
structured audit detail through 1,024 UTF-8 bytes and switched to a concise
quantity-only event label above that budget. API formatting, `api:check`,
Clippy and selected contract compilation all passed. The full
`household_adjustment_reason` suite passed 6/6: exact ASCII and multibyte
reasons at 1,024 bytes, quantity-only labels at 1,025 bytes, and the original
300- and 8,192-character probes. The six route smoke checks passed 6/6. The
existing stock regression
`household_stock::scalar_adjustment_records_reason_quantity_and_request_linkage`
then passed 1/1 in its own fixture, with six route checks passing 6/6.

The audit suite project was `mtcontract-e712d51282754e66`, source digest
`93dfde80f4cc6755aac1bcacb07642e21304243ea8ceabd7d89e1a804c91b113`, fixture
SHA-256 `c1a4aa1823c3485462fe23393e33d579b9752cdf76013c373b45c52a6bce677c`.
Its raw runner log is
`/private/tmp/household-g-20261002/G-AUDIT-BUDGET-GREEN-001/runner.raw.log`
(Task exit 0; SHA-256
`5d914a7a310538a908af57f35500cd7c14a4a4fd3c2954bddca7877c372962b0`). Fast
logs are `fmt.raw.log`, `check.raw.log`, `clippy.raw.log` and `compile.raw.log`
in that directory; each task exited 0. The existing stock regression used
project `mtcontract-0aa3306050184332` with the same emitted source digest,
fixture SHA-256
`1986b449173f25466a6b3107d95e29a7be2286b6935575d02797bd9f58d40756`, and raw
log `/private/tmp/household-g-20261002/G-AUDIT-STOCK-REGRESSION-001/runner.raw.log`
(Task exit 0; SHA-256
`415ff9a0b72ecc42211611587e7575930691bd3fac8fe99cabe7f6fc810d5314`). The
runner-emitted digests identify the executed source snapshots; independent
per-file copy/live manifests were not retained for these two runs.

## Standalone source-capabilities baseline

The selected `source_capabilities_api` target compiled, then both existing HTTP
tests ran on an isolated fixture and failed at their assertions. In
`eligible_stock_uses_source_signature_current_supply_and_selected_tracked_dosage`,
the actual eligible IDs were `[1036427674, 1036427675]` while the expected IDs
were reversed. In
`source_list_and_detail_separate_record_from_manage_permission`,
`eligible_ids(&paused).is_empty()` was false. These are baseline failures on
the pre-edit candidate; the six route smoke cases did not run. No test or
product change was made before this baseline.

Project `mtcontract-a62fa1e977b442a9`; fixture SHA-256
`1ddc7796d9fe929eb4f04f353a329a1498a36705c5ebc556eec723cb9ebbc84a`. The
runner-emitted source SHA-256 was
`ef0f3abf34f8dddd590969e9a042d1e21f533c6d484fe99a8532c0053ae393b9`. Before
cleanup, 401-entry pre-run, copied and live file manifests matched exactly;
each hashes to the same digest. The manifests are in
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-BASELINE-001/`.
Root Taskfile hash was recorded separately because it is not copied into the
application source; the API Taskfile is included in the 401 copied paths. Raw
log `runner.raw.log` in that directory (Task exit 201 after Cargo exit 101;
SHA-256 `33af16383f06ee3129b2ce14e5ab0c3a8cb09f5929a5489b86734a92042241fb`);
focused compile `compile.raw.log` (exit 0; SHA-256
`9e60845486da4da03162176f45f20f56581ecb619f57c7328a7db82336c7c422`).

### Source-capabilities candidate check

The writer's compatibility patch passed API formatting, `api:check`, Clippy
and selected compilation for `source_capabilities_api`. On a fresh fixture,
the permission-separation case passed. The eligible-stock case still failed at
its assertion: actual IDs were `[1036427674, 1036427675]`, expected
`[1036427674]`. The six route smoke cases did not run because HTTP failed, so
this candidate is not accepted. The raw Task output is
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-GREEN-001/runner.raw.log`
(wrapper exit 201 after Cargo exit 101; SHA-256
`0f41ea301b138dc67f1219c6ade686157547d0ffb4088e4ca4843c81e6d7ccac`). The
fixture SHA-256 emitted before cleanup was
`2fe1afce801f1bdf0df47c431558460488636955b634a740aacd759afdf47f30`.

Before launch, the 401-entry source manifest was captured. Before teardown,
the copied and live manifests were captured and both matched the pre-run
manifest exactly; all three manifest hashes are
`5e0e6d21a54b486b2bc6f3878fe9bf6d025b84aef38b086ab19eded4b7943de6`. The
copy was `tmp/contract-tests/run.TBKDOV/source`. Evidence, including empty
manifest diffs and fast-check logs, is under
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-GREEN-001/`. The
separate root and API Taskfile hashes were recorded before launch.

The test-only correction aligned positive-but-insufficient stock expectations
with the public contract and added actual rejected-take/no-write checks for
ordinary and tracked stock. The unchanged production candidate then passed both
`source_capabilities_api` tests and all six route smoke checks. API format,
`api:check`, Clippy and selected compilation passed. Task exit was 0; the
runner-emitted fixture SHA-256 was
`1d51b3e699c8f438ffbe2df37d01ba71a3aa9ae13f83117fd331d50c51e684df`. Its raw
log is
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-CORRECTED-002/runner.raw.log`
(SHA-256
`166452cd0317362b2b31ab8f2bca500730639d0e9e6e76b4cdf0361173433109`). The
fresh 401-entry pre-run, copied and live manifests matched exactly, each with
digest
`f9c97a8221a6ccb9eca5a08e786b612cf23c243f5fa658a2c637e900de444286`; the
application copy was `tmp/contract-tests/run.dCEWu0/source`. Complete evidence
is under
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-CORRECTED-002/`.

### Shared collection pagination baseline

The selected `web_reads_api::shared_collections_filter_before_stable_pagination`
test compiled and reached the People `per_page=999` assertion. The current API
returned 422 with `per_page must be between 1 and 100`; the test expected 200
and a clamped page size of 100. This is an actual baseline compatibility
failure. The wrapper exited 201 after Cargo exited 101; one test ran and five
were filtered. No route smoke ran after the HTTP failure. Raw output is
`/private/tmp/household-g-20261002/G-WEB-READS-PAGINATION-RED-001/runner.raw.log`
(SHA-256
`ad5da5c0519e02dc0f8cc77ab1c57907e2b12221b3595d27560ca0859c035017`). The
runner-emitted fixture SHA-256 was
`6aeaf908fcccaa211596af5d175f6edf9dd36c34ea936b370ad2b7f2f367ee07`.

Before launch and before teardown, 401-entry pre-run, copied and live source
manifests matched exactly at digest
`f9c97a8221a6ccb9eca5a08e786b612cf23c243f5fa658a2c637e900de444286`; the
application copy was `tmp/contract-tests/run.8Q6TPF/source`. Root and API
Taskfiles were recorded separately. The selected target compiled successfully;
its log is `selected-compile.raw.log` in the evidence directory, with exit
status 0. Full evidence is under
`/private/tmp/household-g-20261002/G-WEB-READS-PAGINATION-RED-001/`.

### Shared collection pagination GREEN

The local People-only clamp now passes the same focused test: the request with
`per_page=999` returns 200 and reports a page size of 100. The test's other
query validation and collection controls also pass. API formatting, `api:check`,
Clippy and selected compilation passed. The wrapper exited 0 with the one
selected HTTP test passing and all six route smoke cases passing. Fixture SHA
was `dfcf717d227acd5afd42ee0624ef2b044634bb79f29293d986941f9ee083253b`; raw
runner log
`/private/tmp/household-g-20261002/G-WEB-READS-PAGINATION-GREEN-001/runner.raw.log`
(SHA-256
`2e30b0e048cbcececbc4d90bd76ee28732c27f068443ae9d60446378455e2270`). The
401-entry pre-run, copied and live manifests matched exactly at digest
`1e1786fc8e7842d5329f91df9ccc103633f66c4d031e588f6442ff9bfc94e3bc`; the
application copy was `tmp/contract-tests/run.1ebrvn/source`. Fast-check logs,
manifests and empty diffs are under
`/private/tmp/household-g-20261002/G-WEB-READS-PAGINATION-GREEN-001/`.

### Dose-mode transition baseline setup

The exact `dose_mode_transition_api` target compiled and ran its two named
tests. Both stopped in the shared multi-dose helper before reaching behavior
assertions: PostgreSQL returned SQLSTATE 23502 because the helper inserted a
null `dosages.default_dose_cycle`. This is a fixture/setup failure, not a
product RED. No route smoke ran. The wrapper exited 201 after Cargo exited 101.
Raw output is
`/private/tmp/household-g-20261002/G-DOSE-MODE-BASELINE-001/runner.raw.log`
(SHA-256
`e859fd380c06fd9f72a7b07d137c47ffd9a9a118de744eacf3c26fad8d4dca53`). Fixture
SHA-256 emitted before cleanup was
`7ca5c3c21767fc071d3fa5f9260806f07bab9980b74fac8c16c71b2932d5b2f6`. The
target compiled successfully; its selected-compile log is in the same evidence
directory (exit 0). Before launch and teardown, all 401 pre-run, copied and
live source paths matched exactly at digest
`1e1786fc8e7842d5329f91df9ccc103633f66c4d031e588f6442ff9bfc94e3bc`; the
copy was `tmp/contract-tests/run.MomSNF/source`. Full evidence is under
`/private/tmp/household-g-20261002/G-DOSE-MODE-BASELINE-001/`.

After the test-only helper repair supplied valid daily defaults (cycle 0,
maximum 4, minimum interval 0), the same two behavior tests passed. The six
route smoke checks also passed; wrapper exit was 0. Fixture SHA-256 was
`29f62f2792db0bae608b2da9076a48b5ec7933128512c27dffc4a5858824d636`. Raw log
`/private/tmp/household-g-20261002/G-DOSE-MODE-CORRECTED-002/runner.raw.log`
(SHA-256
`6e989af9d8a57fc28fe28e9f3ec08e3312174fdd8d39364dd5b6cfdd71b47985`). The
401-entry pre-run, copied and live manifests matched exactly at digest
`e750f56aac2f42c5fc04da134cc23c9b870846ef9d6faa6a5f9bdc4f799f2e45`; the
copy was `tmp/contract-tests/run.tf0ROq/source`. This confirms the original
setup failure was confined to fixture defaults. Corrected-run evidence is in
`/private/tmp/household-g-20261002/G-DOSE-MODE-CORRECTED-002/`.

### Doses baseline

The existing `doses` target contains 15 test attributes: 14 executable tests
and one pre-existing ignored fractional-second timestamp case. The baseline
ran all 15: two passed, twelve failed, and one was ignored. Eleven failures
stopped in the shared `create_medication` helper at line 224, where a create
request returned 422 instead of 201; that helper does not surface the response
body, so the required-field cause remains to be diagnosed. One separate
failure reached `medication_take_rejects_invalid_time_source_and_future_without_stock_loss`:
the `source_type="unknown"` request returned 422 where the test expects 404.
No route smoke ran after the HTTP target failed. No fixture or product edit was
made before this baseline.

The target compiled successfully (see `selected-compile.raw.log`, exit 0). The
wrapper exited 201 after Cargo exited 101. Raw output is
`/private/tmp/household-g-20261002/G-DOSES-BASELINE-001/runner.raw.log`
(SHA-256
`d5f8336af94051c84b8695a8f0faddbdd4dadb3654b118d3df6a9ec3641c2fa0`). Project
was `mtcontract-9c5863c786fc4941`; fixture SHA-256 emitted before cleanup was
`6a049ecb11d321e48cc262a319dad4d2fc806f0ddef0386fc1e1e4b49d35ecc7`. All 401
pre-run, copied and live source paths matched exactly at digest
`e750f56aac2f42c5fc04da134cc23c9b870846ef9d6faa6a5f9bdc4f799f2e45`; the
copy was `tmp/contract-tests/run.0xEXFF/source`. Complete evidence is under
`/private/tmp/household-g-20261002/G-DOSES-BASELINE-001/`.

The writer has since frozen a test-only repair adding the currently required
`reorder_threshold=3` and public response-body diagnostics to the shared helper.
The reviewer confirmed the validator requirement and that reversing only these
helper additions reconstructs the original test file. Selected compilation
passes. A separate status conflict remains unresolved: this target expects
404 for an unknown `source_type`, while `dose_write_api` has a 422 assertion;
the reviewer notes the Rails request spec expects 404. The corrected behavior
run is held pending the shared error-taxonomy ruling; no production change has
been made.

### Dose-write baseline

All six unfiltered `dose_write_api` HTTP tests passed, including its existing
invalid-time/source/unit/stock-selection 422 controls. All six route smoke
checks passed. The wrapper exited 0. Fixture SHA-256 was
`a346a8a34ad7803cd1cb9c82d71878e6980b4d55b3eb44b4f9d2a14e5b49fe11`; raw log
`/private/tmp/household-g-20261002/G-DOSE-WRITE-BASELINE-001/runner.raw.log`
(SHA-256
`aea497f8b02428369b7a393ae4d1b1f9bfd101ad60ab1093660533bd38da410d`). The
selected target compile passed. All 401 pre-run, copied and live source paths
matched at digest
`43009eccbfe57883dbefca91688ccddc20286fef741b1ea80bf531373699912a`; the copy
was `tmp/contract-tests/run.vxxbIR/source`. Evidence is under
`/private/tmp/household-g-20261002/G-DOSE-WRITE-BASELINE-001/`.

### Full sync baseline

The unfiltered `sync` target passed all 23 HTTP tests and the six route smoke
checks; wrapper exit was 0. Fixture SHA-256 was
`19799cc3ff58873a0f30aebb8d4151eea34a14ee1b8d462ae506de02f56c76fd`; raw log
`/private/tmp/household-g-20261002/G-SYNC-BASELINE-001/runner.raw.log`
(SHA-256
`d006b368cb0f776ab5695c97b96ee80ea255eea68be9452794b4a5f33f2d4357`). The
selected compile passed. All 401 pre-run, copied and live source paths matched
at digest
`43009eccbfe57883dbefca91688ccddc20286fef741b1ea80bf531373699912a`; the copy
was `tmp/contract-tests/run.AKPoBD/source`. Evidence is under
`/private/tmp/household-g-20261002/G-SYNC-BASELINE-001/`.

### Replay baseline

The unfiltered `replay` target passed all eight HTTP tests and the six route
smoke checks; wrapper exit was 0. Fixture SHA-256 was
`c688ec45b8e66e56b3dc333a7021911f87d75798db629f864673c4b09fb21877`; raw log
`/private/tmp/household-g-20261002/G-REPLAY-BASELINE-001/runner.raw.log`
(SHA-256
`acaffdf3a2c9749953af752a9aceb284c31c80ae231a929c48c9dba05bf7abaf`). The
selected compile passed. All 401 pre-run, copied and live source paths matched
at digest
`43009eccbfe57883dbefca91688ccddc20286fef741b1ea80bf531373699912a`; the copy
was `tmp/contract-tests/run.gFqpXT/source`. Evidence is under
`/private/tmp/household-g-20261002/G-REPLAY-BASELINE-001/`.

### Direct and sync unsupported-source RED

The new focused `dose_source_errors` target compiled, then ran two tests. In
both direct and sync requests, all malformed-reference/unknown-field controls
and the complete no-write snapshots passed. The final valid-reference
unsupported-source attempt (attempt 19) returned 422 `invalid medication
source` where 404 is required. No route smoke ran after the HTTP failure. The
wrapper exited 201 after Cargo exited 101. Fixture SHA-256 was
`1b5d252ac9ef2129627ef9b8b676b09a9d1db2ca61fe2dd0be244f45cc13f941`; raw log
`/private/tmp/household-g-20261002/G-DOSE-SOURCE-ERRORS-RED-001/runner.raw.log`
(SHA-256
`0ad30e7634b10a4d513c1c2eece2d9a39238ce56b95ee45793ef4feb15f5ffaf`). The
402-entry pre-run, copied and live manifests matched exactly at digest
`6362974393e8b5f945782c2580da5892aa9c279c29cf558cb812ca1c499820cf`; the
copy was `tmp/contract-tests/run.MSlUnk/source`. Selected compilation passed.
Full evidence is under
`/private/tmp/household-g-20261002/G-DOSE-SOURCE-ERRORS-RED-001/`.

### Direct and sync unsupported-source GREEN

After the narrow validation-order fix, `dose_source_errors` passed both direct
and sync cases; malformed-reference and unknown-field controls, all no-write
snapshots, and the valid unsupported-source 404 checks passed. The six
household-route smoke checks also passed. Wrapper exit was 0. Fixture SHA-256
was `6f5f87d27b0c1a75fa9bcff3ae066505cf9e4296d6bb8a8c916d9674267d2e6b`.
Raw output is
`/private/tmp/household-g-20261002/G-SOURCE-ERRORS-GREEN-001/runner.raw.log`
(SHA-256
`0a99a785207817d5b799ecb0150fba2a7335cc74f89174e7b16ff5cb2738f6a5`). The
pre-run source manifest is
`/private/tmp/household-g-20261002/G-SOURCE-ERRORS-GREEN-001/source-inputs.pre.manifest`;
the copied and live manifests are `source.copy.manifest` and
`source.live.manifest` in the same directory. All 402 entries matched, with
manifest digest
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`. The
runner copy was `tmp/contract-tests/run.9vlV3G/source`. The API formatter,
check, Clippy and selected compiles for `dose_source_errors` and
`dose_write_api` all exited 0; their raw receipts are in this evidence
directory. This is a focused result, not the full final composed acceptance.

### Corrected dose-write acceptance

The corrected unfiltered `dose_write_api` target passed all six HTTP tests,
including its unknown-source 404 control, and all six household-route smoke
checks. Wrapper exit was 0. Fixture SHA-256 was
`ce3f2e147f30eecf30c838399be2f59072da3053e05c0e74fb9bee0044e511e4`; raw log
`/private/tmp/household-g-20261002/G-DOSE-WRITE-GREEN-001/runner.raw.log`
(SHA-256
`0b043fa7b79809abb084e07f208d73954430ce83a825f78d8f54ad215f4980b0`). The
402-entry pre-run source manifest is
`/private/tmp/household-g-20261002/G-DOSE-WRITE-GREEN-001/source-inputs.pre.manifest`;
its digest `e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`
matches the runner-emitted copied-source digest. The runner removed its
transient source copy during cleanup before a copied per-file manifest could
be retained, so this receipt does not claim an independent file-by-file
copy/live comparison. The target selected compile and the API formatting,
check and Clippy gates passed in the immediately preceding source-error
verification on this frozen source.

### Repaired doses target

The selected `doses` target compiled successfully, then ran all 15 test
attributes: eight passed, six failed and the existing fractional-second case
was ignored. The failures are real assertion mismatches, not fixture setup:
three endpoint checks at `tests/doses.rs:205` expected
`api/v1/dose_occurrences` but received `api/v1/person_medications` or
`api/v1/schedules`; the paused-take check expected `paused` but received
`unprocessable_content` at line 1672; the take-listing case expected 200 but
received 422 at line 1076; and invalid-occurrence validation expected
`validation_failed` but received `unprocessable_content` at line 1487. Cargo
exited 101 and the outer wrapper exited 201, so the six route checks did not
run. The dose owner has the exact failures for diagnosis; sync and replay are
held until this result is classified.

Fixture SHA-256 was
`4d925a1f4b1a3e8906adbe899d0dc272358b18234b9ed991c12a3869fbf83a1b`; project
was `mtcontract-194be8bafc3c4a94`. Raw output is
`/private/tmp/household-g-20261002/G-DOSES-GREEN-001/runner.raw.log` (SHA-256
`16b874a716881b7ccb3d4312c18ce47d6858dcb580a36049c882501e6e5bf5a1`). The
selected compile receipt is
`/private/tmp/household-g-20261002/G-DOSES-GREEN-001/selected-compile.raw.log`
(exit 0). The 402-entry pre-run and post-run live manifests both have digest
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`, matching
the runner-emitted copied-source digest. The transient copy was cleaned before
its per-file manifest could be retained; this evidence does not claim a
per-file copied-source comparison. A post-run lookup briefly selected the
unrelated stale `run.BlqqHW` path; its mislabeled copy-location, copy manifest
and two diff files were removed. Manifests and raw output are under
`/private/tmp/household-g-20261002/G-DOSES-GREEN-001/`.

### `management_sync_events_api` compatibility baseline

The exact target compiled successfully and ran three cases. The ordinary
management-write case passed. The tracked-dose and tracked-removal cases
failed before their event assertions at `tests/management_sync_events_api.rs:128`:
their shared dosage insert omitted the required `default_dose_cycle`, producing
PostgreSQL SQLSTATE23502. This is fixture setup, not a sync-event product
failure; no route smoke ran after Cargo exited 101 and the outer Task reported
201. The dose owner has the exact helper correction. Raw output is
`/private/tmp/household-g-20261002/G-MANAGEMENT-SYNC-EVENTS-BASELINE-001/runner.raw.log`
(SHA-256
`a3f15bba2594e4b194110ac9abe75730c13176d07bc61bc430c697da38b76646`). The
selected compile passed (receipt in `selected-compile.raw.log`). Fixture SHA-256
was `f01f2034dba3eb067a885a7ddaceac69c48b65cf3c67d7007a04cdf556c85b5d`; the
independent pre-cleanup receipt is `fixture.independent.sha256`. Project was
`mtcontract-d7e58fea42e74e68`, copy
`tmp/contract-tests/run.b6DDo6/source`. All 402 pre-run, copied and live source
paths matched at digest
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`. The
background monitor's Fish `wait` returned 0 despite the nested Task failures;
that value was removed as an invalid runner-exit receipt. The raw Task/Cargo
failure lines and this qualification are preserved in
`runner-exit-observation.txt`.

### `openapi_medications` compatibility baseline

The selected target compiled successfully and ran nine cases: seven passed and
two failed at the OpenAPI response-shape assertion in
`tests/openapi_medications.rs:58`. The actual medication response includes
`friendly_name` and `warnings`, while the expected key set omits them. Cargo
exited 101 and the wrapper exited 201; no route smoke ran after the target
failure. This is a baseline assertion mismatch, not a setup failure. Raw output
is `/private/tmp/household-g-20261002/G-OPENAPI-MEDICATIONS-BASELINE-001/runner.raw.log`
(SHA-256
`76ed6313c2247c212c4e436aa1c52c50c9093dc2d5f504ceeb0dba83d3bc5927`). The
selected compile passed; its raw receipt is in the same directory. Fixture
SHA-256 `c537459672853911f04d70c1be5119b92582cc629f0067f768a3a29d3f775546`
was independently captured before cleanup. Project was
`mtcontract-9d4aea5965f94eac`, source copy
`tmp/contract-tests/run.8r1DDo/source`. Its 402-path pre-run, copied and live
manifests matched with digest
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920` and zero
diffs. The complete source receipts and fixture record are under
`/private/tmp/household-g-20261002/G-OPENAPI-MEDICATIONS-BASELINE-001/`.

### `openapi_read_completion` compatibility baseline

The selected target compiled successfully and all three tests failed. Two
cache-control assertions expected `no-store`, but the response was
`private, no-store` (`tests/openapi_read_completion.rs:110` and `:218`). The
People query case expected `per_page=101` to return 422, but received 200
(`:299`). This expectation conflicts with the accepted positive-size clamp to
100; the test is being amended to assert the bounded response. Cargo exited
101 and the wrapper exited 201; no route smoke ran after the target failure.
The complete RTK log is
`/private/tmp/household-g-20261002/G-OPENAPI-READ-COMPLETION-BASELINE-001/runner.full.log`
(SHA-256
`24972cfd7c216d2f79970a7e2c6ab5eb32717d5a88fc2eb7495ec92f4b222f8f`); the
captured wrapper output is `runner.raw.log` in the same directory. The selected
compile passed. Fixture SHA-256
`aae8d11230c8e37749ccb06b2fa123d6edba32fe7d17d2a37fd852703dfb6057` is the
runner-emitted pre-cleanup receipt; an independent rehash missed teardown. The
project was `mtcontract-858cde154d0e4e30`, source copy
`tmp/contract-tests/run.WL2tXI/source`. All 402 pre-run, copied and live source
paths matched with digest
`e5cc6bc7e6abb580f45a1c234b86a63e57c508eb021ff6e67ed1ad0b32d87920`; both
manifest diffs are empty. These receipts and the selected compile are under
`/private/tmp/household-g-20261002/G-OPENAPI-READ-COMPLETION-BASELINE-001/`.

### Focused zero-stock dose attempt — interrupted before assertions

The `doses` target compiled successfully. The first launch did not start because
the private evidence directory had not yet been created; this invocation error
is recorded in `initial-invocation-setup.txt`. The corrected filtered run used
`HOUSEHOLD_TEST_FILE=doses` and
`HOUSEHOLD_TEST_FILTER=zero_stock_occurrence_retains_domain_error_and_direct_take_remains_generic_without_writes`.
It was interrupted during the browser setup (`npm run css`) after review found
that the selected test explicitly chose a zero-stock medication. No Cargo test
or assertion ran. The wrapper exited 130; raw output is
`/private/tmp/household-g-20261002/G-DOSES-ZERO-STOCK-RED-001/runner.raw.log`
(SHA-256
`897a5e2ee733f8a3e4b8b903e20e2dd5e0d4cc03676234a2e70a2ccf47641520`). The
selected compile passed. Fixture SHA-256
`49ca2c075cc1035febee8d4fd8c1e36b3fc2b77ed55c584bfcc89d40617c18d3` was
independently recorded before cleanup. Project was
`mtcontract-34048075154a45fb`, source copy
`tmp/contract-tests/run.xz8YJA/source`. All 402 pre-run, copied and live source
paths matched at digest
`70feadb67bd5d421d97d762354ddfdae33185dc66c21ec4d7ea6c755273a52cf`; both
diffs are empty. The owned disposable project and run directory were removed
with `contract:cleanup` (exit 0). The initial case used explicit selection of
the zero-stock medication. Root is reviewing whether to preserve that as a
separate parity case alongside an automatic-selection case; a frozen two-case
test request is pending. Evidence is under
`/private/tmp/household-g-20261002/G-DOSES-ZERO-STOCK-RED-001/`.

### Focused zero-stock dose RED

After the reviewed test-only update added separate automatic- and explicit-
selection cases, the selected `doses` target compiled successfully. The
`zero_stock_` filter ran exactly two cases; both reached the intended
assertions after their no-write snapshots. Explicit selection returned
`Selected location is unavailable for this medication.` instead of the
expected `Cannot take medication: out of stock`
(`zero_stock_explicit_selection_retains_empty_stock_priority_without_writes`,
`tests/doses.rs:1942`). The automatic-selection occurrence returned
`unprocessable_content` instead of `out_of_stock`
(`zero_stock_occurrence_retains_domain_error_and_direct_take_remains_generic_without_writes`,
`:1828`). Cargo exited 101 and the wrapper exited 201; the six route smoke
cases did not run. This is the actual focused RED. Raw wrapper output, including
both Cargo assertions, is
`/private/tmp/household-g-20261002/G-DOSES-ZERO-STOCK-RED-002/runner.raw.log`
(SHA-256
`8e5ff19b611c9b0a2d93b7c17a5e04b436cf46b0458e0a0df820befea937d5ec`); the
build-only RTK tee is separately labelled `docker-build.tee.log`. The selected
compile passed immediately before the run; its receipt is copied into this
evidence directory with its origin noted. Fixture SHA-256
`47190c5d638ca8689147ca43b6cf82eb970a102b082f3e8df05fd357b872397f` was
independently captured before teardown. Project was
`mtcontract-bb489551acaf478b`, source copy
`tmp/contract-tests/run.rQEKZ5/source`. All 402 pre-run, copied and live source
paths matched at digest
`70536947125c1d262d62becbae1d48b22b34ecca0671c281d71717624affccc5`; both
manifest diffs are empty. The raw output, source manifests and fixture receipt
are under
`/private/tmp/household-g-20261002/G-DOSES-ZERO-STOCK-RED-002/`.

### Repaired dose suite and focused write compatibility — GREEN

After the reviewed zero-stock mapping correction, the unfiltered `doses`
target passed all 17 executable tests; one pre-existing fractional-second
response test remains ignored. The six household-route smoke checks also
passed. Runner exit was 0. This does not alter the existing ignore or claim
that the Rails response-timestamp case passed. Raw output is
`/private/tmp/household-g-20261002/G-DOSES-FIX-GREEN-001/runner.raw.log`
(SHA-256
`b4f8e1f114b9310987b60020e13e8428b308e9ac0b6f1f9826a4ca04f0a9455e`).
Fixture SHA-256 `9d11ac8108e6d1815f6323bc1f38f67d097236f66a970c0b3b8506235dc51ab0`
was independently captured before cleanup. Project was
`mtcontract-f562104625f4483a`, source copy
`tmp/contract-tests/run.VmnjI5/source`. All 402 pre-run, copied and live
source paths matched at digest
`4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722`; both
manifest diffs are empty. The route smoke count is six (one parent suite with
five nested checks), not one. Independent review audited the raw assertions,
manifests and fixture receipt.

The two focused `dose_source_errors` cases passed, covering malformed-reference
422 controls and unsupported-source 404 for direct and sync requests, with
no-write snapshots. The route smoke checks passed 6/6; wrapper exit was 0.
Raw output is
`/private/tmp/household-g-20261002/G-DOSE-SOURCE-ERRORS-GREEN-002/runner.raw.log`
(SHA-256
`b65a5b3395b39f569fb2b7cfd4ec5e371dea6438227cf30bef7905a2cbff652d`).
Fixture SHA-256 `e8f8e5b39864fc62ef04eca018cc327fd4d94b776517a507763c541cf04dc759`
was independently captured before cleanup. Project was
`mtcontract-d960e1128f1b430b`, source copy
`tmp/contract-tests/run.9IrGFj/source`. All 402 pre-run, copied and live
source paths matched at digest
`4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722`; both
manifest diffs are empty.

The corrected `dose_write_api` target passed all six direct-write tests, and
the route smoke checks passed 6/6. Wrapper exit was 0. Raw output is
`/private/tmp/household-g-20261002/G-DOSE-WRITE-GREEN-002/runner.raw.log`
(SHA-256
`cbe493c921d9912a2ef40d41f8eee065cd0ad0fdacbc6d6f423ed23c27f5bc2d`).
Fixture SHA-256 `66fb96c2d7f6913ec9366e5131fcd91036fac94937a27028407178ce0843466f`
was independently captured before cleanup. Project was
`mtcontract-24eb4e635e8e4194`, source copy
`tmp/contract-tests/run.gT1CSi/source`. All 402 pre-run, copied and live
source paths matched at digest
`4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722`; both
manifest diffs are empty. The fast format, API check, Clippy and selected
compiles for all three targets passed on the same frozen application inputs.

### Remaining unsplit dose and compatibility checks — GREEN

The full `sync` target passed 23/23 HTTP tests and six route smoke checks;
wrapper exit was 0. Its raw log is
`/private/tmp/household-g-20261002/G-SYNC-FIX-GREEN-001/runner.raw.log`
(SHA-256
`da95f1f4315302a3fb381089f2d809eaa0607f2ca3490af0f5b87b7680aeb3f9`), fixture
SHA-256
`e74f1158bc5420caf50f1900340ea8983240894970c849f67ea4042b4f2b0b3f`, project
`mtcontract-e701aab894544c8d`. The full `replay` target passed 8/8 HTTP tests
and six route smoke checks; wrapper exit was 0. Its raw log is
`/private/tmp/household-g-20261002/G-REPLAY-FIX-GREEN-001/runner.raw.log`
(SHA-256
`9f5b8dc383a1a52ee28e2937c052a7872e91bb8afe6704121a209fd64f709eab`), fixture
SHA-256
`205c081dc31deb377bb8f11bea1eff01a10b7eaa2cf6e8f94cb7b39d79561c41`, project
`mtcontract-a3e566b65b894814`. Both used separate fixtures; all 402 pre/copy/live
paths matched at digest
`4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722`.

`dose_mode_transition_api` passed 2/2 HTTP tests plus six route checks; wrapper
exit was 0. Raw log
`/private/tmp/household-g-20261002/G-DOSE-MODE-FIX-GREEN-001/runner.raw.log`
(SHA-256
`5a938423c7ac71c0c7042d476cb72d7d7c44e0ce0b5390bff5a1315b2c8098b9`), fixture
`53d4f0c66aac78950c5a9c01ade26c807febaa8b99ce5cec3bdd95423a774b4d`, project
`mtcontract-85ab31b807464e88`. `source_capabilities_api` passed 2/2 HTTP tests
plus six route checks; wrapper exit was 0. Raw log
`/private/tmp/household-g-20261002/G-SOURCE-CAPABILITIES-FIX-GREEN-001/runner.raw.log`
(SHA-256
`8b3d971ed597200dd8921c0f96e25a165c5d741228f669c1a555f1f8f06fd9c3`), fixture
`9897b0a85b8ecaa24366839706743ce14c7a3c58644505c4933c0271f882398f`, project
`mtcontract-aed61781c71440e5`. Both source manifests match all 402 paths at
the same digest above.

The existing API unit task passed 36 library tests, including the four
`dose::tests` and the `dose_occurrences` unit; the web crate's 12 unit tests
also passed. Raw output is
`/private/tmp/household-g-20261002/G-DOSE-UNIT-GREEN-001/task.corrected.raw.log`
(SHA-256
`23ffdd74a280837f1585038b04d5c3f84a1fc3ded1e52db4eb4f01446cd7ba5a`), exit 0.
The initial `task -d rust/api test` invocation was an invocation error because
the nested Taskfile already uses repository-root manifest paths. The corrected
existing command was `task api:test`. The exact 18 executed/ignored dose test
names, extracted from the already passing full target raw log, are preserved in
`/private/tmp/household-g-20261002/G-DOSES-FIX-GREEN-001/test-name-list.txt`
(SHA-256
`35a5e5f2c2514116c939a0e8cafc466cb3b9212a2d43fb9b171e89a911fad026`).

All three amended compatibility targets passed with six route checks apiece:
`management_sync_events_api` passed 3/3, raw log
`/private/tmp/household-g-20261002/G-MANAGEMENT-SYNC-EVENTS-FIX-GREEN-001/runner.raw.log`
(SHA-256
`8cee720bb484c33571e9a7b6629cabe65d2eb06a42ae8a1bea32f90d04d9257f`), fixture
`176d4850cf2d5ff7fcc88e8b8258b7e98ed7a5d73f633a46fdf6dd8374a9c154`, project
`mtcontract-384fd0c45f5a4838`; `openapi_medications` passed 9/9, raw log
`/private/tmp/household-g-20261002/G-OPENAPI-MEDICATIONS-FIX-GREEN-001/runner.raw.log`
(SHA-256
`e03e752780d13f1a74bc0ff871d36fc26c2502c7d064cc62deb716c35562ba9f`), fixture
`5a671eb705829c23570c04041f1417b0ea0a7288a6831de2beb4b6d362b1fc15`, project
`mtcontract-68db743a3dc244bc`; `openapi_read_completion` passed 3/3, raw log
`/private/tmp/household-g-20261002/G-OPENAPI-READ-COMPLETION-FIX-GREEN-001/runner.raw.log`
(SHA-256
`28a56bf4cce3c54a355d348654d0324f2501c6f8e8b3a26b957c6c32cb813700`), fixture
`c3e420d0ebdee1a426bc59ce25b962c71d3c22eb2259f354b09d5acb0c9032b0`, project
`mtcontract-905236bb103e445f`. Each wrapper exited 0, and all three had
matching 402-path pre/copy/live manifests with digest
`4e2cb970ae7901c725e05726b4fed375b6319b4a0e18b14a7a4c56932f44a722`.

The dose CI policy regression first failed as expected because the new
`dose-compatibility` row was missing (76/77 tests passed). After root added the
row, `task ci:check` passed 77/77, including the focused policy test. The RED
raw log is
`/private/tmp/household-g-20261002/G-CI-DOSES-POLICY-RED-001/task.raw.log`
(SHA-256
`9d22be70d7576791e6f11e8008e4eede5b9d863c8b77e362f891d0a993d7232b`); the
GREEN check log is
`/private/tmp/household-g-20261002/G-CI-DOSES-POLICY-GREEN-001/task.raw.log`
(SHA-256
`200b1e5b3b871fbdf9b214aae83c35baaaa11c720c45b6c499b676a24543cab1`). The
standalone CI row's actual browser runtime remains pending after the approved
dose module split.

### Dose split post-install verification — GREEN

The approved dose production/test split is installed. The initial warning-denying
Clippy attempt found seven unused imports in the facade. The reviewed three-file
correction restored five definitions to the facade and gated the two unit-only
imports; it added no lint allowances and changed no routine bodies. On the
frozen corrected source, `api:fmt:write`, `api:check`, `api:clippy`, selected
compiles for `doses`, `dose_source_errors` and `dose_write_api`, and `api:test`
all exited 0. The unit task ran 36 API and 12 web tests. Format/check/Clippy logs
are in `/private/tmp/household-g-20261002/G-DOSE-SPLIT-FAST-002-*`; the corrected
unit raw log is
`/private/tmp/household-g-20261002/G-DOSE-SPLIT-UNIT-GREEN-002.raw.log`.

All seven separate post-split contract runs passed with wrapper exit 0 and six
read-only household-route smoke checks apiece. Every run emitted source digest
`6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5`; its
422-file copied-source manifest exactly matches the 422-file live manifest.
Each run directory retains `source.copy.manifest`, `source.live.manifest`, an
empty `copy-live.diff`, runner digest, separate Taskfile command-input hashes,
raw output, and exit status. The identical copy/live manifest file hash is
`258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952`.

| Target | Result | Project | Fixture SHA-256 | Raw log and SHA-256 |
| --- | --- | --- | --- | --- |
| `doses` | 17 passed; 1 existing ignored; 0 failed; route smoke 6/6. All 18 executed names match the before-split list. | `mtcontract-e18922ebd12f4d36` | `3ed7c4b5cbb649d6cb08e5128a9a3d53348752cb466b5071ada02652d8ffa430` (runner-emitted only; independent rehash was unavailable after cleanup) | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-DOSES-001/runner.raw.log` — `f8d279fbcb48c06331843333522eb24c93ecb317097a60e2f0f7b62df39b174f` |
| `dose_source_errors` | HTTP 2/2; route smoke 6/6 | `mtcontract-057979a277f84d84` | `c33cf6305efc6347be5b0b87cac5cdb848d4833a95fe8385ca71a90ecf107dc2` | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-SOURCE-ERRORS-001/runner.raw.log` — `15653a1b2822a07c1ecf91f145f97c800893bbf2340894e6faa51e07516c23b2` |
| `dose_write_api` | HTTP 6/6; route smoke 6/6 | `mtcontract-5fab3a093dee4536` | `d87667a2c0b094b3220c7f7e693793c8e68db70db13f248e4e5fbf31a8bb361c` | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-WRITE-API-001/runner.raw.log` — `099be401f5f605dbdc79f288a6e58f715d4d38c673340a9e6ba8a630fde9a346` |
| `dose_mode_transition_api` | HTTP 2/2; route smoke 6/6 | `mtcontract-8e0f4cbb3ac54fd8` | `5f9e6a3afc90580b191036aa25f2e58a84d5357005331e3ca80e2946701da60f` | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-MODE-TRANSITION-001/runner.raw.log` — `16c4d791d96e9d463e3eee95681ded5bada04e3aaab763238facf1f6d63e2e46` |
| `source_capabilities_api` | HTTP 2/2; route smoke 6/6 | `mtcontract-4c64a1498b254dee` | `57a4d81e671232b297a9d5e905af069a18f50232a055e9e56f6fb6787afaa63c` | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-SOURCE-CAPABILITIES-001/runner.raw.log` — `e6019f1b25426dafa490a05a84315401690bd8bb9da6202457c37774229b3834` |
| `sync` | HTTP 23/23; route smoke 6/6 | `mtcontract-3134cf245ebf4c1d` | `72a904ec1fea23b34d47bf0c2bb0d40be2a1e5129583d2084bece0c22c7dff71` | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-SYNC-001/runner.raw.log` — `c4f8dacd798f52522f13db39c3cd1da14c0a65101bf50e3958419d09aad561cb` |
| `replay` | HTTP 8/8; route smoke 6/6 | `mtcontract-cbaec75cb07940ea` | `135113167fedac82161117346421cd10b9f82dd800c26fb650d0f8ddf104df7b` | `/private/tmp/household-g-20261002/G-DOSE-SPLIT-REPLAY-001/runner.raw.log` — `be6aafd0f13ccce32306f5a0d0b17342906efc34a403ef6d81c8a7e809d3b1ce` |

The `doses` before/after test-name lists are preserved under its evidence
directory and compare equal. Its fixture hash is explicitly limited to the
runner-emitted receipt; no fixture-only rerun was made. Independent final review
of the dose split passed. The standalone CI row runtime remains a separate
workflow-level check.

## Before-occurrence split compatibility checks

On the unchanged, reviewed dose-split source, the `openapi_dose_occurrences`
target compiled and passed all 10 HTTP tests plus the six route smoke checks.
Project `mtcontract-99da08dde5824224`; source digest
`6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5`; fixture
SHA-256 `59230aa293025b7e848191f469b96eedbb75eb5dd236d5aa2c631f4ea709bd66` was
independently captured before cleanup in
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-HTTP-001/fixture.independent.sha256`;
the runner path was `tmp/contract-tests/run.Yt8AA8/fixture.json`. All 422 copied
source paths match the live source manifest; the identical manifest SHA-256 is
`258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952`. Raw log
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-HTTP-001/runner.raw.log`
(exit 0; SHA-256
`03bfefa5d83022f79b6a567beb5ad5e52e341b27ed41c822becebddd0eacc5fa`).

The `openapi_sync_reads` target then passed both HTTP tests and six route smoke
checks in a separate fixture. Project `mtcontract-643b68f64f9146f9`; the same
source digest; fixture SHA-256
`3f59e5d64e14d8820be93416214642d8c47e097e2d9690412eee3f09abc7eafe` was
independently captured before cleanup in
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-SYNC-READS-001/fixture.independent.sha256`;
the runner path was `tmp/contract-tests/run.uSGIZI/fixture.json`. Its 422 copied
source paths also match the live source with the same manifest SHA-256 above.
Raw log
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-SYNC-READS-001/runner.raw.log`
(exit 0; SHA-256
`2fbaba44448ef16292da14feaf17cd5b8d2678f9c464899f3004cd7e69558b19`).

The existing fixed-clock `api:browser-dashboard-rust` task dispatched the Rust
dashboard runner and its default `dashboard.test.mjs` and
`leptodon-dashboard.test.mjs` files. It passed 35/35 Node tests; wrapper exit
was 0. Project `mtcontract-c0f1e15ddf9b4374`; source digest
`6710c57383bf78ecc1c3db29e199d4ab6b0c2432fe4e0b67b17586a22470c4d5`; the
422-entry copied and live manifests match (both manifest files have SHA-256
`258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952`). Fixture
SHA-256 `f62e0e75a5d39fd5ff621400cb3eff9d0d82df1fd0f31343768f09a7648681ac` is
runner-emitted only; cleanup removed the fixture before an independent rehash.
The outer log is
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-DASHBOARD-001/runner.raw.log`
(SHA-256
`0254eabb44ca381ce89e34de802da75552ce99e566e12361b0f22e73e42c9a11`; exit 0).
Full Node output is preserved at
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-DASHBOARD-001/browser.full.log`
(SHA-256 `fe3fa17d1433948aeb86c151149c3cf54a674b5db997d58e926896ff19814f37`);
it contains all 35 passing test names and the zero-failure summary. The original
RTK tee was
`/Users/damacus/Library/Application Support/rtk/tee/1790918882_task_api_206bd2.log`.
All ten generated desktop/mobile screenshots are retained at
`/private/tmp/household-g-20261002/G-BEFORE-OCCURRENCE-DASHBOARD-001/screenshots/`;
their checksum manifest SHA-256 is
`76c68bcc182749a678305519ed3a1f29015f83ea43907e8dfbfeb8b68848dce5`. The seven
pre-existing top-level screenshots were archived before the run and restored
byte-for-byte; the three new top-level Leptodon images were archived and removed.
The F screenshot archive was not changed.

## After-occurrence split checks

The occurrence split was installed from the independently reviewed candidate.
All 14 installed files matched its pre-format manifest. API formatting changed
only `dose_occurrences.rs`, `dose_occurrences/input.rs`, and
`dose_occurrences/keys.rs`; the exact pre-format and post-format manifests and
diff are retained under
`/private/tmp/household-g-20261002/G-OCCURRENCE-SPLIT-FAST-001/`. The focused
API format, check, warning-denying Clippy, and unit gates all exited 0. API
units reported 36 library tests and 12 web tests passed; doctests had zero
tests. Full logs and checksums are in that evidence directory.

Each after-split contract run used the existing `api:browser-rust` task, an
isolated fixture, `HOUSEHOLD_ACCEPTANCE=true`, the named `HOUSEHOLD_TEST_FILE`,
`API_TIME_ZONE=UTC`, and the six route smoke cases from
`tests/household-routes.test.mjs`. No household filter, completion/stock flag,
or dashboard clock was set. All six runtime copies match their captured
pre-manifests and all six fixtures were independently hashed before cleanup.
The common 435-path application manifest hash is
`d31251ad535b03d110a108ef9dbf3c731cba02aa4c1bf9f0687279769246d58f`; runner
source digest is
`a5fedf06dbef8ac386f0c40eb741ed9f03cb8ff9294f6356a82ec9c2705b957e`.

| Cargo target | Result | Project | Fixture SHA-256 | Raw log (SHA-256) |
| --- | --- | --- | --- | --- |
| `openapi_dose_occurrences` | 10/10 HTTP + 6/6 route smoke; task passed | `mtcontract-97fdac2cdec64aff` | `8461f60ad4d8af4dfd3df40687dba3eee5471ed8addfbe7a0a0744d9214f2221` | `/private/tmp/household-g-20261002/G-OCC-SPLIT-OCCURRENCES-001/runner.raw.log` (`075118adb910c3a4df5c92f8021c689513a9b45a74ffe6dfbe202a1414465d6c`) |
| `doses` | 17 passed, 1 pre-existing ignored; 6/6 route smoke; task passed | `mtcontract-64cd565c94a24abe` | `5a3d8eeac3a8d5d72c94c54463d58c9fea409e1dc734bded6e8220bed814e8cd` | `/private/tmp/household-g-20261002/G-OCC-SPLIT-DOSES-001/runner.raw.log` (`fed0d9833bf5da39e5c3fb0af365471d600867b108b7b682c54691c8c2007bdf`) |
| `sync` | 23/23 HTTP + 6/6 route smoke; task passed | `mtcontract-c49a349afeaa4a07` | `64ff8a95c65934fd9912a363521d5ab7c5fbc1d7b34a9f87d69a82c7a6f52760` | `/private/tmp/household-g-20261002/G-OCC-SPLIT-SYNC-001/runner.raw.log` (`e669ca9e0e671175875576d254c8e2ddc4eee41e7aa9c3f2217b11adb2608d27`) |
| `replay` | 8/8 HTTP + 6/6 route smoke; task passed | `mtcontract-8d66cd73a35c4858` | `20851dc618655c5ad118d37d7f88a37aaa4738f2e25991d874dc0104ce771884` | `/private/tmp/household-g-20261002/G-OCC-SPLIT-REPLAY-001/runner.raw.log` (`073e82414422813f3b721ab3bb3936a040e50bddd2527d28541cd343d74d27d5`) |
| `openapi_sync_reads` | 2/2 HTTP + 6/6 route smoke; task passed | `mtcontract-e707a0cbb3fa4519` | `2dc5bfdcf24183f1f5f3c267c2a70f7a3162386d8af139adb4719dd6292943db` | `/private/tmp/household-g-20261002/G-OCC-SPLIT-SYNC-READS-001/runner.raw.log` (`3993b266e7eb08501dcf1f29b36e22c2e903a221db64c8b7691a3cfe1d70eb60`) |
| `dose_source_errors` | 2/2 HTTP + 6/6 route smoke; task passed | `mtcontract-3f301061340f4281` | `cdfd1ddfe4846a5677c91e78fc97ea6e5a019181cf3b1e4ca768ad8db3a46ddb` | `/private/tmp/household-g-20261002/G-OCC-SPLIT-SOURCE-ERRORS-001/runner.raw.log` (`f92eb7bd52bdda85aec0fd1fc0040352087ebecb4445a4a5f8b6658e5d92799f`) |

The post-split fixed-clock `api:browser-dashboard-rust` task used its default
`dashboard.test.mjs` and `leptodon-dashboard.test.mjs` files and passed 35/35.
The Rust dashboard job used clock `2026-03-29T00:30:00Z`, project
`mtcontract-a1b2263c4d534dfc`, source digest
`a5fedf06dbef8ac386f0c40eb741ed9f03cb8ff9294f6356a82ec9c2705b957e`, matching
435-path copy/live manifests, and independently captured fixture hash
`12ee98d81e6782e2ea347bfa94f3d7d657f6b4bc1c941dc15013ee915058a999`. Outer raw
log `/private/tmp/household-g-20261002/G-OCC-SPLIT-DASHBOARD-001/runner.raw.log`
has SHA-256 `7381039d718ac0b65dc75e9545428d386d707df9f523fdf03242dcdb8c2b7fb0`;
the authoritative Node assertions are in
`browser.assertions.full.log` (35 pass, 0 fail; SHA-256
`2ce1b87e1e8669b117964bc4c43fc6012b8e2b2beca187687675f3bf5f9fd08e`). The
other `build.full.log` is Docker build output, not test output. Ten generated
screenshots and checksums are retained in the same evidence directory. The
seven original dashboard screenshots were restored byte-for-byte; three new
Leptodon images are archived privately.

## Required `schedules` suite before/after occurrence split

The brief names Cargo target `schedules`; `openapi_schedule_writes` and the
dashboard checks do not replace it. Because no executable before-split copy
remained, the pre-split application was reconstructed under
`/private/tmp/household-g-20261002/G-OCC-SCHEDULES-BEFORE-001/repo/`: the current
application was copied privately, only the 13 new `rust/api/src/dose_occurrences/`
children were removed, and `rust/api/src/dose_occurrences.rs` was restored from
the reviewed retained `dose_occurrences-before.rs`. The repository's existing
`api:contract-source-snapshot` task staged it. Its 422-path manifest exactly
matches the original before-copy manifest (both SHA-256
`258cb49d84ccc1bed63e1ea57fcb2b402779ea482390d418ef60127059700952`); the
runner's 422-path copy also matches. This is a reconstructed snapshot verified
against the original manifest, not the original deleted executable copy. The
private `api:contract-selected-compile TEST_TARGET=schedules` passed.

The existing Rust wrapper was dispatched from that private mirror with
`HOUSEHOLD_ACCEPTANCE=true`, `HOUSEHOLD_TEST_FILE=schedules`,
`API_TIME_ZONE=UTC`, and the six read-only route smoke cases. Project
`mtcontract-345d54ce747140b6`; fixture SHA-256
`ad0270d99e3364f78a3460b889df6101e1d8a0a9e0e15b67a46034267323454a` was
independently captured before cleanup. Cargo executed 15 declared cases: 3
passed, 8 failed, and 4 were ignored. The eight failures were pagination 400 vs
200 at `tests/schedules.rs:233`; unexpected `current_pause_period` at :422 and
:557; 422 vs 200/201 at :677, :605, and :309; and pause/resume version 24 vs 20
at :93 in two tests. The inner contract Task failed 101 and the outer browser
Task reported 201; the `rtk proxy` shell wait returned 0 and is not a Task
status. Raw log
`/private/tmp/household-g-20261002/G-OCC-SCHEDULES-BEFORE-001/schedules-before.raw.log`
(SHA-256 `136c9f64cb08bca8211c110902d3671e30b3c8cfbd38c8a571ef8a2f41652550`).

The current-source target was separately compiled and run from the main
checkout with the same selector and a fresh fixture. Its 435-path runtime copy
matches the captured current application manifest (SHA-256
`d31251ad535b03d110a108ef9dbf3c731cba02aa4c1bf9f0687279769246d58f`); runner
source digest is
`a5fedf06dbef8ac386f0c40eb741ed9f03cb8ff9294f6356a82ec9c2705b957e`. Project
`mtcontract-fe54c661b1a04aee`; independently captured fixture SHA-256
`3c05fea9ba2dec4cd0edeb8212e8ddf5ff7be4e4610e7d679cb1652473330ad4`. It had
the same 3 pass, 8 fail, 4 ignored results and task failure statuses as the
reconstructed baseline. Raw log
`/private/tmp/household-g-20261002/G-OCC-SCHEDULES-AFTER-001/schedules-after.raw.log`
(SHA-256 `0928363321fdbf524a23cfdfe1044e0bfea4613b1707e6cf7eb8740a54b55560`).
Both raw failures are being reviewed for a bounded test/setup correction before
the matched repaired baseline and after-split acceptance reruns.

The first repaired-schedule iteration installed the pagination fix and test
correction in `rust/api/src/read_resources/schedules.rs` and
`rust/contract-tests/tests/schedules.rs`. Formatting, `api:check`, warning-
denying `api:clippy`, and selected `schedules` compilation passed. The repaired
pre-occurrence run used a private reconstruction at
`/private/tmp/household-g-20261002/G-OCC-SCHEDULES-BEFORE-001/repo/`, with the
old occurrence facade restored and the 13 occurrence children absent. It began
from the original 422-path pre-occurrence manifest and overlaid only the two
reviewed schedule files. The existing snapshot dependency also regenerated
four `rust/ui-preview/public/pkg` files; these four generated-output hash
changes are recorded separately in
`/private/tmp/household-g-20261002/G-SCHEDULE-FIX-RECONSTRUCTED-BEFORE-002/expected-vs-snapshot.diff`.
All other copied paths match the original manifest. The staged 422-path source
manifest and runner copy match at SHA-256
`2b99afbf336a2a8a83951f110546e0df8adad1ae91d9b6332db0deaed2f512b7`; this is
reconstructed repaired source with regenerated UI assets, not a two-file-only
full-manifest match. Project `mtcontract-fa4f46d726934096`; independently
captured fixture SHA-256
`cc2f60326d8977f9c29262cf9eecc113aca94894b7656b3addc7d7b9eb496da9`.
Cargo ran 15 declared cases: 8 passed, 3 failed, and the same 4 remained
ignored. The failures were the locations `page=bogus` control expecting 400 but
receiving 422 at `tests/schedules.rs:264`, plus two validation-message
expectations (`must be a string`) receiving `must be a decimal string` at
`tests/schedules.rs:485` and `:731`. The HTTP Task exited 101, so route smoke
did not run; the outer `api:browser-rust` Task exited 201. Raw log
`/private/tmp/household-g-20261002/G-SCHEDULE-FIX-RECONSTRUCTED-BEFORE-002/schedules-before.raw.log`
(SHA-256 `45b998bdfc9cdbe234a8ecebc0f12ab55b3e0940b9a7fe429f86ce505d5363a2`).
The original reconstructed baseline run remains preserved separately and is
not replaced by this intermediate partial result. The final three-file
repaired comparison and its passing results are recorded below.

The CI policy test first failed as intended: 78 tests ran, 77 passed and one
failed because the `schedule-compatibility` fixture row was absent. Raw log
`/private/tmp/household-g-20261002/G-CI-SCHEDULE-RED-001/ci-check.raw.log`
(SHA-256 `285b551f912d431b72ba369d9675adfe3bc3d809b9f2b6a72d29dd4bab646eab`).
After the coordinator added the schedule matrix row, `task ci:check` passed
78/78, including `actionlint` and Node syntax checks. The 13-row matrix includes
`schedule-compatibility`, target `schedules`, with the six route smoke cases.
Raw log
`/private/tmp/household-g-20261002/G-CI-SCHEDULE-GREEN-001/ci-check.raw.log`
(SHA-256 `506745c141fe73cf72923c372fcb2d7e5c933673e03d7ebdad65cebdc5faf472`).

### Repaired schedules before/after occurrence comparison

The earlier repaired-candidate paragraph above records an intermediate
8-pass/3-fail/4-ignored run using only the first two schedule files. It is not
the final repaired comparison. After the separately reviewed follow-on was
installed, the final repaired source comprised exactly these authored paths:
`rust/api/src/read_resources/schedules.rs`,
`rust/api/src/schedule_writes/input.rs`, and
`rust/contract-tests/tests/schedules.rs`. Formatting, API check, warning-denying
Clippy, and selected compilation of `schedules` all passed. The installed
authored-file hashes match
`/private/tmp/household-g-20261002/schedule-compatibility-proposal/installed-preformat.sha256`
and
`/private/tmp/household-g-20261002/schedule-followon-proposal/installed-preformat.sha256`.
The numeric-only validation behaviour is tracked separately as issue #2368.

For the reconstructed unsplit run, the private mirror retained the original
occurrence facade and had no occurrence child directory. Its expected
422-path manifest and runtime copy match at SHA-256
`c34349b401efc507404191c3f22200fd446dcbbc480a3bc9aa89c2098a4a6a4e`; the
empty comparison is
`/private/tmp/household-g-20261002/G-SCHEDULE-FIX-CURRENT-BEFORE-002/expected-vs-runtime-copy.diff`.
The three authored schedule changes are the only authored differences from
the original manifest. Four regenerated `rust/ui-preview/public/pkg` outputs
are separately recorded in
`/private/tmp/household-g-20261002/G-SCHEDULE-FIX-RECONSTRUCTED-BEFORE-002/expected-vs-snapshot.diff`;
they came from the existing snapshot build, not authored changes. This is a
reconstructed repaired source with regenerated UI assets, not an original
executable copy. Project `mtcontract-8117cfdd69f64118` used fixture SHA-256
`4b84126769a79e7966a8690c5b0438fa5411714bd7f537d7dac8b14afced4621`, captured
independently before cleanup. All 11 active schedule cases passed, zero
failed, and the same four established cases remained ignored. The six
read-only route checks passed. The outer Task exited 0. Raw log
`/private/tmp/household-g-20261002/G-SCHEDULE-FIX-CURRENT-BEFORE-002/schedules-before.raw.log`
(SHA-256 `ee8f566e4c8ab33882631a40df64d5c9940f5b4cbb63e8149556a8b9d4ae395d`).

The current-source comparison used a separate fresh fixture. Its runner
emitted source digest `3ddaf3e03f370082def8fcef6d55f8e43ba83d9feb516998bad0e5e371e53600`,
project `mtcontract-a2962babab0a47eb`, and fixture SHA-256
`c79ab408a96d1cd0208f28870ed7eaf25773f9dab9feccd2f4610826650a8374`. The
fixture digest is runner-emitted in the retained raw output; an independent
rehash after cleanup is unavailable. The run passed all 11 active cases,
failed none, retained all four ignores, and passed all six route checks. The
outer Task exited 0. Raw log
`/private/tmp/household-g-20261002/G-SCHEDULE-FIX-CURRENT-AFTER-001/schedules-after.raw.log`
(SHA-256 `d3c992d2d3de27022354a7625d365fc0034a4c4216c88af80ecbb5555e9af3fc`).
This completed the repaired unsplit/current schedules comparison; it does not
claim broader Rails `to_i` parity than the reviewed pagination cases.

### Invitation baselines

The existing `api:openapi-invitations-acceptance` Task ran the complete
`openapi_invitations` target in a fresh isolated fixture. All six cases passed,
including `smtp_failure_rolls_back_invitation_rotation_without_receipt_or_audit`;
the dedicated Task started `rust-api-mail-fail` before its no-deps test
container. Project `mtcontract-804efa4f9b1e4b15`; source digest
`3ddaf3e03f370082def8fcef6d55f8e43ba83d9feb516998bad0e5e371e53600`; runner-
emitted fixture SHA-256
`9049142bcbf7752129b4ed9cfe17f05b4702b9c636c6dfd74c9cb155e714a2ef`.
The fixture digest is retained from the runner before cleanup; no independent
rehash is available. Outer Task exit was 0. Raw log
`/private/tmp/household-g-20261002/G-INVITATIONS-OPENAPI-001/invitations.raw.log`
(SHA-256 `120b889f80fbc7316d394dd8d8e2dff7b85a15d465a47c1817e5dc7f1d3195ba`).

The separate existing `api:openapi-invitations-legacy-acceptance` Task ran the
full `invitations` target in a different fixture. All eight cases passed;
outer Task exit was 0. Project `mtcontract-ce48be7c6a944a88`; source digest
matched the preceding run at
`3ddaf3e03f370082def8fcef6d55f8e43ba83d9feb516998bad0e5e371e53600`; runner-
emitted fixture SHA-256
`835fc82dcad808605314018c45557bcdc816470a52ca418f7f00f20775ec0b9e` (not
independently rehashed after cleanup). Raw log
`/private/tmp/household-g-20261002/G-INVITATIONS-LEGACY-001/invitations-legacy.raw.log`
(SHA-256 `2a02033d55903908633fcc783ed8fb9265b611255cb8cd3829f79731074d7ed0`).

### Invitation module split after-checks

The reviewed invitation split installed `rust/api/src/invitations.rs` and ten
child modules. Before formatting, all eleven installed files matched the
candidate manifest
`/private/tmp/household-g-20261002/invitation-extraction-candidate/installed-preformat.sha256`.
`api:fmt:write` changed only import ordering in the facade and line wrapping
in `responses.rs`; the exact formatter diff is retained at
`/private/tmp/household-g-20261002/G-INVITATION-SPLIT-FAST-001/formatter-only.diff`
(SHA-256 `b45124980b0aa032cc729887760c0f0d7183a89f9cd10c6bab4db32f0e048550`).
The post-format hashes for all eleven files are in
`/private/tmp/household-g-20261002/G-INVITATION-SPLIT-FAST-001/postformat.sha256`.
Formatting, `api:check`, warning-denying `api:clippy`, `api:test`, and selected
compilation of both `openapi_invitations` and `invitations` passed (all exit
0). `api:test` ran 48 unit cases across its nonempty binaries; all passed.
Their raw logs and exit receipts are in
`/private/tmp/household-g-20261002/G-INVITATION-SPLIT-FAST-001/`.

After the split, `api:openapi-invitations-acceptance` passed all six cases,
including the configured SMTP failure rollback test. It used project
`mtcontract-4f642a5b81984a33`, runner source digest
`0b8f532a35103a9774bbf2fa1f860e456b97c8e06dcba66f36d569c2dff4692c`, and
runner-emitted fixture SHA-256
`c22a750e1b6658843f109ab0d916f60b7c8c772f3f85f3007df78a238c3531eb`; the
fixture was not independently rehashed after cleanup. The Task exited 0. Raw
log
`/private/tmp/household-g-20261002/G-INVITATION-AFTER-OPENAPI-001/invitations.raw.log`
(SHA-256 `e7aedfd6bd5264f0527ac30d1d509e542c5d4725dd0e014e570e42a410c7754e`).

The separate `api:openapi-invitations-legacy-acceptance` run passed all eight
cases. It used a fresh project `mtcontract-ff04bcc1bad54773`, the same runner
source digest
`0b8f532a35103a9774bbf2fa1f860e456b97c8e06dcba66f36d569c2dff4692c`, and
runner-emitted fixture SHA-256
`0d24eb5df746588172a1cfe6ec28ba395c1de580f18a245da8f976a5b11e3a26`; an
independent post-cleanup rehash is unavailable. The Task exited 0. Raw log
`/private/tmp/household-g-20261002/G-INVITATION-AFTER-LEGACY-001/invitations-legacy.raw.log`
(SHA-256 `62df6298f57a4f7f132774a305ffdca464bc732644b67129572933e162e3c92b`).

### OAuth before-split baselines

These six before-split selections ran against runner source digest
`0b8f532a35103a9774bbf2fa1f860e456b97c8e06dcba66f36d569c2dff4692c` on
separate real-clock fixtures. The full `api:api-legacy-auth-acceptance` target
passed 12/12; project `mtcontract-1f0365410f134d3a`; runner-emitted fixture SHA
`326f6b931bf19664f73604add14283a25c15a43449243397428815130c1e4cb6`; outer
exit 0. Raw `/private/tmp/household-g-20261002/G-OAUTH-BEFORE-AUTH-001/auth.raw.log`
(SHA-256 `139e1746423fc062705ca62193152fee726c1a2338ef40dc0dd85d5c5d20a3c5`).

The full `oauth` target passed 11/11 in the existing browser wrapper, project
`mtcontract-865eeee96a444590`, fixture SHA
`d330f583caea705a5cbd33c08f7a858d5e06040490d78147af672dc455dd5bbc`, outer
exit 201 because the selected login smoke also ran and failed two dashboard
heading assertions. A separate fresh standalone `tests/login.smoke.test.mjs`
run reproduced the same 5/7 result, confirming this is not established as
fixture interference. In both runs, desktop and mobile failed at the obsolete
`h1` expectation in `rust/web/tests/login.smoke.test.mjs:88`; the other five
browser checks passed. The test correction remains private and is not included
in these before-source captures. The OAuth wrapper's initial
`contract:oauth-rust` attempt failed before assertions because the selected
runner overlay did not define the `rust-api` image/build context; this is a
setup failure tracked as #2369, not an OAuth assertion result.

For the OAuth wrapper run, raw outer log
`/private/tmp/household-g-20261002/G-OAUTH-BEFORE-OAUTH-001/oauth.raw.log`
(SHA-256 `a52d3b0c67fece9d104a312a1b049704623acffe96cd4a5600883b082f820fab`);
the authoritative full Node tee was preserved at
`/private/tmp/household-g-20261002/G-OAUTH-BEFORE-OAUTH-001/login-smoke-shared-failure.full-tee.log`
(SHA-256 `09e8e8bd4981a1f96ccf08db1ddd5cb45322485ea208c01d370408d328b5c953`).
For the fresh standalone browser run, raw outer log
`/private/tmp/household-g-20261002/G-OAUTH-BEFORE-LOGIN-SMOKE-001/login.raw.log`;
the authoritative full tee is
`/private/tmp/household-g-20261002/G-OAUTH-BEFORE-LOGIN-SMOKE-001/login-smoke.full-tee.log`
(SHA-256 `fe96d2f2f81e4979e65caa2f7f544c8ddc5f948ea8cb0c3e399830c4e4cd25ec`).
The standalone project was `mtcontract-7d1b1979247943e4`, fixture SHA
`5ab9197b777278ba9ea7168ef027cc9e45e89a13e83572637eebece568a1e5be`; this
digest is runner-emitted, with no post-cleanup independent rehash. Generated
login screenshots were archived and the five pre-run originals restored and
checksum-verified.

The full `api:web-session-acceptance` target passed 9/9; project
`mtcontract-904ac5256ffb43ba`, fixture SHA
`b2fc8a968cd2d92fe880f2a43629a9aeccc193110102561d3b7cd8caebad2fd7`, outer
exit 0. Raw `/private/tmp/household-g-20261002/G-OAUTH-BEFORE-WEB-SESSION-001/web-session.raw.log`
(SHA-256 `d4703a0c0f8808962bb067e6ea161078a875b181c413303cf2624feecc999186`).

The explicit `medication_mobile_oauth_api` selection passed 7/7 and its
read-only `tests/household-routes.test.mjs` smoke passed 6/6 in project
`mtcontract-63f25a4d193245d1`; runner-emitted fixture SHA
`08daffc22777e478636347dcb6f2d76a1e192abfa20713e8d470f089c83ec3d6`; outer
exit 0. Raw `/private/tmp/household-g-20261002/G-OAUTH-BEFORE-MOBILE-001/mobile.raw.log`
(SHA-256 `b4e6e513a0a9b005339fc205f507213af4b7fcccd427b806b593ebbd5684b685`).

The full `api:openapi-sessions-acceptance` target passed 8/8 in project
`mtcontract-b9de406b4e304bac`; runner-emitted fixture SHA
`07795edcb5c7c0a68e32b08dd4512aadb3d88a113dd4e5b724fcef48f91d8dc9`; outer
exit 0. Raw `/private/tmp/household-g-20261002/G-OAUTH-BEFORE-SESSIONS-001/sessions.raw.log`
(SHA-256 `3964d21d037ba57c3422ea46d7b429d708f623536e98e1e5c671d4e84f2af3ef`).

The mobile OAuth and auth sessions outputs both record the same source digest
as auth and web-session above. Their fixture digests are runner-emitted before
cleanup; independent post-cleanup rehashes are unavailable. No OAuth source or
test correction has been installed while these before captures are being
completed.

The standalone-login CI policy regression failed before the workflow row was
added: `task ci:check` ran 79 tests, 78 passed and one failed because the
`standalone-login` fixture row was absent. The policy test requires that row to
run the login smoke in its own fixture with household acceptance disabled, no
HTTP target, no fixed dashboard clock, and no test filter. The outer Task
reported exit 201. Raw log
`/private/tmp/household-g-20261002/G-CI-LOGIN-RED-001/ci-check.raw.log`
(SHA-256 `a5d6528289c5269e86da90d1323dc05b5ffa05ae410b3b3b6721e1fa8e4a69b4`).
After the coordinator added the standalone-login row, the next `task ci:check`
still failed: 79 tests ran, 78 passed and one failed because the general matrix
assertion forbids `acceptance:` for every non-clock row, while the dedicated
standalone-login assertion requires `acceptance: "false"`. The outer Task
reported exit 201. Raw log
`/private/tmp/household-g-20261002/G-CI-LOGIN-GREEN-001/ci-check.raw.log`
(SHA-256 `d564d6962c48cf05bc476e536f300796282b9ca568c7d7477ababac54a1875e3`).
The policy exception was narrowed to the standalone-login row, whose dedicated
assertion still requires `acceptance: "false"` and forbids a target, clock,
filter, stock or completion selector. The subsequent `task ci:check` passed
79/79 tests, including actionlint and the CI-script Node syntax checks. The
workflow matrix now has 14 rows. Outer exit 0. Raw log
`/private/tmp/household-g-20261002/G-CI-LOGIN-GREEN-002/ci-check.raw.log`
(SHA-256 `288042bd586df3682a45e50c821a8daa0bb21817a1069b8a8bd717174be2d862`).
After the reviewed test-only login correction, a new standalone real-clock
fixture passed all seven `tests/login.smoke.test.mjs` cases, including current
owner greeting, mobile logout, CSRF and revocation. Source digest
`310de470b49cd66e4f73e4720afe6bd74c05d90a46b67012a3e6084d5985b078`; project
`mtcontract-27807ae465084df7`; runner-emitted fixture SHA-256
`4faa7ee4b3e6628d27c953894eb21e29549227e69800ed0de193f33f246569bb`; outer
exit 0. The five generated login screenshot hashes match the pre-run originals;
those originals were restored after copying the run outputs privately. Raw log
`/private/tmp/household-g-20261002/G-OAUTH-BEFORE-LOGIN-SMOKE-FIXED-001/login.raw.log`
(SHA-256 `d36ffd3b0c3bf8603fe3fd9249d1fa695aabf6d782066eae1a7e320ab7cde9e4`).

### First-option stock race final proof

On the current source digest
`310de470b49cd66e4f73e4720afe6bd74c05d90a46b67012a3e6084d5985b078`, the
existing `household_stock_concurrency` target passed all four cases in fresh
project `mtcontract-c91291f6d8414528`, including the gated browser dosage-index
read racing insertion of the first tracked option, the 409 response, retained
draft, and unchanged parent/option stock, versions and sync state. The six
read-only `tests/household-routes.test.mjs` checks also passed; outer Task exit
0. The runner emitted fixture SHA-256
`6d70c0e1c515717b6857f4e532e3715c3e781d3927ebb2aba44155c0ff071465` before
cleanup; independent post-cleanup rehash is unavailable. Raw log
`/private/tmp/household-g-20261002/G-STOCK-CONCURRENCY-FINAL-001/stock-concurrency.raw.log`
(SHA-256 `c21800bb4a5803a4e87a02f442a2922aa066de40205a97a7fba90e0b8c0daf15`).

### OAuth visibility fix installed (resume 2026-10-02)

The reviewed three-type fix from
`/private/tmp/household-g-20261002/oauth-input-visibility-proposal/proposal.diff`
was applied with `git apply -p0` after `before` SHA-256 verification. Installed
hashes match the receipt `after` values: login.rs
`16a5f47071e321512e94ca395cd9d84c475eac081baa6df77f91fd980ce5528c`, tokens.rs
`3e8b1bf2730706144776cfb35ebd0a6deb241c50f4dc5083d9db6097c0921857`.
Post-install gates on the live tree: `task api:fmt` exit 0, `task api:check`
exit 0, `task api:clippy` exit 0 (`proc-macro-error2` future-incompat warning
is pre-existing), `task api:test` exit 0 (36 lib + 12 auth_compatibility unit
tests, 0 failed). After-split runtime baselines follow on fresh fixtures.

### OAuth after-split baselines and stock concurrency (resume 2026-10-02)

On post-visibility-fix source, fresh disposable fixtures each run:
`api:api-legacy-auth-acceptance` auth 12/12 (mtcontract-19d8aa63dcd84f1c);
`browser-rust` HOUSEHOLD_TEST_FILE=oauth + login.smoke — oauth 11/11, login
smoke 7/7 (mtcontract-6dc226028a8d4866; earlier attempt without
HOUSEHOLD_ACCEPTANCE skipped the HTTP target, evidence retained);
`api:web-session-acceptance` 9/9 (mtcontract-64f53ccdff604392);
`browser-rust` medication_mobile_oauth_api + household-routes — 7/7 + 6/6
(mtcontract-fefa249356d24f25); `api:openapi-sessions-acceptance` 8/8
(mtcontract-98c355d372644f9f). All match before-split baselines.

`browser-rust` household_stock_concurrency + household-routes passed 4/4 +
6/6 (mtcontract-45a93aadd7144f9d), exercising the installed zero-to-one
option assertions including tracked_option_created_after_browser_scalar_check.

### Final acceptance and gates (resume 2026-10-02, post-OAuth-fix)

On the visibility-fixed source, fresh disposable fixtures each run:

- `browser-rust` household_final_acceptance +
  tests/household-inventory-completion.test.mjs — 15/15 HTTP (assignments 2,
  inventory 4, people 1, dose_source_errors 2, stock_adjustment_audit 6) and
  10/10 browser cases across five locales at desktop and mobile.
- `browser-rust` people-locations-medications row with
  HOUSEHOLD_ACCEPTANCE+HOUSEHOLD_COMPLETION_ACCEPTANCE=true —
  household_completion_medication 4/4, household_lifecycle 2/2,
  household_navigation 2/2, household_web 11/11,
  household_completion_dashboard 9/9, browser 35/35
  (mtcontract-88cc3d3750b64507). First attempt surfaced one stale case:
  `editing_medication_identity_preserves_existing_dosage_options_and_tracked_supply`
  manually posted `reorder_threshold` for an options-mode medication; the
  browser form omits it, so the product correctly returned 422. The test now
  matches the real browser contract and reports the response status on
  failure. Rerun green.
- Fixed-clock rows (CONTRACT_DASHBOARD_NOW=2026-03-29T00:30:00Z,
  HOUSEHOLD_ACCEPTANCE=false): household-treatment-calendar 2/2
  (mtcontract-578478d9642d49c2), household-taper-times 2/2
  (mtcontract-12d7b761bc404cb1).

Gates on the final tree: `task api:fmt:write` normalised
rust/web/tests/dashboard_rendering.rs (edition-2021 style); `task
ci:rust-port` exit 0 (ui-preview fmt/test/lint/build, api fmt/clippy/test, web
tests, contract-tests check); `task ci:check` 79/79 including the 14-row
matrix assertions; `task ci:markdown` clean after reflowing a `#2367` line in
ledger.md that parsed as an ATX heading; `task docs:build` clean; `task
openspec:validate` 24/24. Self-review of the full
`origin/codex/rust-household-assignments-20261001` diff completed: no-write
rejections, audit reason preservation in `audit_context.inventory_adjustment`,
people per_page clamp vs schedule leniency vs strict resources, eligible stock
ordering (location name then medication id), and minor-access schedule
degradation all match the recorded rulings. Independent review was not
available this session; the PR discloses self-review.
