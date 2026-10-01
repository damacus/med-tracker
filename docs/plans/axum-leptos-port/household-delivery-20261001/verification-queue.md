# Sole verification queue

Owner: completion_build, Luna Medium. No other seat runs compiled/runtime jobs.

Read plan.md and writer-brief.md. Validate actual Task selector and routes before
expensive runtime jobs. Source hashes certify frozen inputs, not subsequent
edits. Use one batched fully awaited manifest, exact command, task exit status,
fixture SHA and detailed outer logs. Store private logs under /private/tmp or
ignored evidence; no secrets in durable records. Write compact receipts here.
Use separate grant-mutating token lifetimes and fixed-clock dashboard fixtures.
All runtime actions preserve Rails, isolate fixtures and clean only owned files.

Current state (2026-10-01): both dosage API baselines passed 14/14; the
original route and locale regressions produced expected REDs. Final formatting,
API check, selected contract compile and all 51 web tests pass. D-CORE-GREEN-001
passed all four HTTP tests and existing household route/workflow regressions,
but only 26/34 browser tests passed: eight non-English immediate-administration
cases failed. The writer made a reviewed test-only pagination correction.
D-CORE-BROWSER-002 then passed all 20 dosage browser cases on a fresh isolated
fixture. D-PERMISSION-003 passed the isolated grant-mutating permission case
and a fresh 20-case browser suite. The full `ci:rust-port` gate passed on
unchanged frozen authored inputs. No runtime job is active. Fixed-clock
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

After each journey: focused HTTP plus browser acceptance, all affected locales,
desktop/mobile screenshots with prior baseline capture, and relevant regressions.
After G: full Rust gate and docs build on final stable input. Root alone commits
and publishes. Do not edit product source; return failures to the writer.
