# Sole verification queue

Owner: completion_build, Luna Medium. No other seat runs compiled/runtime jobs.

Read plan.md and writer-brief.md. Validate actual Task selector and routes before
expensive runtime jobs. Source hashes certify frozen inputs, not subsequent
edits. Use one batched fully awaited manifest, exact command, task exit status,
fixture SHA and detailed outer logs. Store private logs under /private/tmp or
ignored evidence; no secrets in durable records. Write compact receipts here.
Use separate grant-mutating token lifetimes and fixed-clock dashboard fixtures.
All runtime actions preserve Rails, isolate fixtures and clean only owned files.

Current state (2026-10-01 18:48 UTC): D is published in PR #2351. E's stock
baseline is classified as an existing timestamp precision failure; the four
new E HTTP cases and ten browser cases reached expected RED assertions. The
new four-case boundary suite and focused option-editor assertion also reached
expected REDs. No E runtime acceptance GREEN has run. The #2350 audit-description fix is part of
the captured option-job source, but has no GREEN evidence yet. Earlier dosage
API baselines passed 14/14; the original route and locale regressions produced expected REDs. Final formatting, API check,
selected contract compile and all 51 web tests passed. D-CORE-GREEN-001
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
