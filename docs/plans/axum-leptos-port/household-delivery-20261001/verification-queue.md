# Sole verification queue

Owner: completion_build, Luna Medium. No other seat runs compiled/runtime jobs.

Read plan.md and writer-brief.md. Validate actual Task selector and routes before
expensive runtime jobs. Source hashes certify frozen inputs, not subsequent
edits. Use one batched fully awaited manifest, exact command, task exit status,
fixture SHA and detailed outer logs. Store private logs under /private/tmp or
ignored evidence; no secrets in durable records. Write compact receipts here.
Use separate grant-mutating token lifetimes and fixed-clock dashboard fixtures.
All runtime actions preserve Rails, isolate fixtures and clean only owned files.

Current state (2026-10-01 23:22 UTC): D and E are accepted and published. F's
37 HTTP/12 core browser cases, 2/2 fixed-clock taper amount cases, one
permission HTTP case plus seven medication browser regressions, and 35
existing dashboard browser regressions all pass. The display shows the
effective taper amount. #2361's native time control is now verified in the
focused fixed-clock browser cases, and the affected core regressions pass on
the same candidate. The full Rust gate passed; only final Markdown/docs gates
remain. D-CORE-GREEN-001
passed all four HTTP tests and existing household route/workflow regressions,
but only 26/34 browser tests passed: eight non-English immediate-administration
cases failed. The writer made a reviewed test-only pagination correction.
D-CORE-BROWSER-002 then passed all 20 dosage browser cases on a fresh isolated
fixture. D-PERMISSION-003 passed the isolated grant-mutating permission case
and a fresh 20-case browser suite. The full `ci:rust-port` gate passed on
unchanged frozen authored inputs. No D runtime job is active. Fixed-clock
dashboard tests remain separate.

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
