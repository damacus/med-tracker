# Shared web reads: runner report

## Run scope

Two isolated selections ran against the frozen web-reads contract:

- `task api:web-reads-rails` runs the `web_reads_api` contract against the Rails web service. The frozen sidecar shares the Rails web-test network namespace and uses the internal `db-test` audit database URL. It does not start the Rust API.
- `task api:web-reads-acceptance` runs the same contract against the Rust API.

Both use disposable PostgreSQL 18 fixtures and selected explicit `/28` networks. Fixture hashes were captured while both fixtures were present. Runtime source hashes were compared before and after the runs.

## Results

| Selection | Project / subnet | Result |
| --- | --- | --- |
| Rails authority baseline | `mtcontract-3952bcd7fbcc4721` / `192.168.72.0/28` | Passed: 4/4 tests, 0 failed, in 3.50 seconds. Task exit 0. |
| Rust read acceptance | `mtcontract-d018fcf2c38d4138` / `10.255.42.0/28` | Expected initial RED: 0/4 passed; all four read routes returned 404 where the contract expected 200. Contract runner exit 101; task exit 201. |

The four cases were `revoking_a_person_grant_removes_that_person_and_linked_sources_on_next_read`, `shared_collections_expose_visible_picker_rows_and_view_permissions`, `shared_collections_filter_before_stable_pagination`, and `shared_details_match_collection_rows_and_hide_ungranted_records`.

The Rails result confirms the test sidecar and audit setup now work against the authority implementation. The earlier Rails run recorded below used the old host runner selection and received 401s; this current 4/4 pass supersedes that result for the corrected runner.

## Frozen inputs and identity

- The pre/post content manifest covers 129 paths under `rust/api`, `rust/contract-tests`, `rust/web`, and `scripts`, plus root `Taskfile.yml`, `compose.yaml`, `.dockerignore`, `rust/web/Dockerfile.smoke`, and its `.dockerignore`. This includes new and untracked runtime/test files present at freeze. It is a scoped Rust/runtime and fixture-provisioning manifest, not a complete Rails source manifest.
- Pre and post per-file hashes match. Aggregate SHA-256 of the manifest: `d2c663c69679d605b0b8c22c2dea043d1b64b16b9025b8904c2029e82e1da5ca`. Files: `/tmp/journey-web-reads-runtime-paths.txt`, `/tmp/journey-web-reads-runtime-pre.manifest`, `/tmp/journey-web-reads-runtime-post.manifest`.
- Rails fixture: `tmp/contract-tests/run.6RoDYx/fixture.json`, 48,609 bytes, SHA-256 `346e6069eaa514150f88a64c598af47a5ade401bc240a2dc89f251cef35c57f6`.
- Rust fixture: `tmp/contract-tests/run.7xOUG0/fixture.json`, 48,609 bytes, SHA-256 `10e50a6f8c49947da9cacfa5fb0ab131c96f5fa8aba8a83b0da534257ecdd06b`.
- Rails app base: `rubylang/ruby:4.0.7-resolute@sha256:a8ee881bc5dff85f0a001ba0a9ee9e59e7c18a14da5d7662a53b308ff63a9d65`. Rails project image `mtcontract-3952bcd7fbcc4721-web-test:latest`: manifest `sha256:79cffa4c259fb45052062381e230c1519b60f6cee597944067dd9464d1f682c8`, config `sha256:74a56ac66ab9d7a2391431bbfcca1260005ac1e5320b8f5fd9303cfd28fa6e49`, manifest list `sha256:829b67bc9fb50109c95c58f35cf4e052e7026255dc917c17343bd9db6060b714`.
- Rails-side contract-test image `medtracker-contract-runner:mtcontract-3952bcd7fbcc4721`: manifest `sha256:f64b4ad854bfd6f9cf6006e6944dbca1b32744f592327e29a84a0b092a0c20b4`, config `sha256:a0a9a8c2c33be1e4c7752d991bcacafa158d491f7ef5b9bc4bd3bc7eb60f4e49`, manifest list `sha256:8fd192a0c598e2a5650936a003ddb6ddfb366116cb1104c3fefa0933ddf80f45`.
- Rust API contract-test image `medtracker-contract-runner:mtcontract-d018fcf2c38d4138`: manifest `sha256:ad5f15a5f057be2f11b6e4f4841c198c0c6df24807be83f46e67d55bd072e5fa`, config `sha256:11bb2aa123c1774156528ab1b54b2279f0f8fbf234b414ca0b3279b09fc6c266`, manifest list `sha256:e497fdfaa1a216d3822d2bdb27decb38645e0cdb759a132ac662fd765d2a63c8`.
- Rust base image: `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`.

## Timing and cleanup

Both Rails servers reported ready after 31 seconds; both fixtures were ready after 58 seconds. Rust runner image builds took about 23 seconds. The contract stage took 3.50 seconds against Rails and 0.26 seconds against Rust. Both runs completed in about 90 seconds including build and cleanup.

Both disposable projects cleaned their containers, networks, named volumes, generated images and temporary storage. Fixtures were hashed before the cleanup hook removed them. No unrelated project was inspected or cleaned.

## Logs

- Rails test/build output: `/Users/damacus/Library/Application Support/rtk/tee/1790349979_task_api_8a66a8.log`.
- Rust 404 failure output: `/Users/damacus/Library/Application Support/rtk/tee/1790349979_task_api_6a3096.log`.
- Rust runner image build: `/Users/damacus/Library/Application Support/rtk/tee/1790349975_task_api_6d05a3.log`.
- Rails app image build: `/Users/damacus/Library/Application Support/rtk/tee/1790349893_task_tes_a28cd8.log`.

## Earlier Rails attempt (superseded)

The earlier `task api:web-reads-rails` run used the old generic host selection, before the same-namespace Rails sidecar and internal audit database URL were wired. It passed 1/4 and three cases returned HTTP 401 (`Authentication required`). That result diagnosed an invalid runner setup, not a Rails web-read defect. The corrected sidecar run above passed all four cases.

## Expanded read acceptance checkpoint (2026-09-25)

Root supplied an immutable snapshot at `/tmp/medtracker-journey-acceptance-20260925`, detached at `af25d6d4` with the frozen tracked and untracked changes. Its full snapshot manifest SHA-256 was `78d168450820bdd878486ca4e17d84d877f74588b83f19ea10e9d122757f8b17`. The Rust/runtime and fixture-provisioning manifest used for these two runs covers 131 paths and is deliberately scoped; it is not a complete Rails source manifest. Its pre/post manifests match, with aggregate SHA-256 `a130e8aab8e6756c4127991da78dec0ef81add1ce991c006e674e886e6ac9a8b`. Manifest files: `/tmp/journey-web-next-runtime-snapshot-paths.txt`, `/tmp/journey-web-next-runtime-snapshot-pre.manifest`, and `/tmp/journey-web-next-runtime-snapshot-post.manifest`.

| Selection | Project / subnet | Result |
| --- | --- | --- |
| Rails shared-read authority baseline (`task api:web-reads-rails`) | `mtcontract-c48f3e5775c1467f` / `10.255.42.0/28` | 1/6 passed; 5 failed; task exit 201. |
| Canonical Rust API acceptance (`task api:acceptance`, browser stage off) | `mtcontract-191bfbc38a7e4041` / `192.168.72.0/28` | 46/51 passed; task exit 201. Dose, forecast, mobile OAuth, medication reads, OAuth, and all 9 web-session tests passed. Five shared-read tests failed. |

The Rust shared-read failures match the Rails behavior observed in the baseline: two denial cases had no audit row where one was expected, and three positive collection/detail/revocation cases returned 403 instead of 200. The stable-pagination filtering case passed in both suites. The expanded Rust run also confirms that all 9 web-session tests pass, including the previously failing cookie household listing case. Browser tests were intentionally not selected.

Fixtures were hashed before cleanup (48,609 bytes each): Rails `tmp/contract-tests/run.j6m3U4/fixture.json`, SHA-256 `90be909055e75ff2b7bac318146078bd2fcd878d7ed30aa335f084088bf51677`; Rust `tmp/contract-tests/run.hdly6g/fixture.json`, SHA-256 `fcf8f24a66b2972ae9522aa8ed5ee890967c3f1d277eb98c3a1da4be7de464ea`.

The Rails contract runner image `medtracker-contract-runner:mtcontract-c48f3e5775c1467f` has manifest `sha256:99fd6fbdeb806fb7882ab781478431c7eed6904755df6666d514f2d629cde2b9`, config `sha256:9ec0ea62def5147000f934513d1e9ac17f3a246a9c6cd6048f0b47a699e61b41`, and manifest list `sha256:c00a55745bd7aaa964af3c717d37320f4428100c76b44a6a01dc43bb50d75310`. Both runner builds used `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`. The canonical Rust runner image digest/config were not retained in the streamed task output before the cleanup hook removed the image; no image identity is inferred for that project.

Both runs used separate checked `/28` networks. The Rails fixture was ready at about 61 seconds and the Rust fixture at about 60 seconds; test durations were 4.82 seconds and 4.47 seconds respectively. Both disposable projects removed their containers, networks, named volumes, generated images, and fixture storage. No unrelated project was inspected or cleaned.

Logs: Rails `/Users/damacus/Library/Application Support/rtk/tee/1790351687_task_api_f0feb2.log`; canonical Rust `/Users/damacus/Library/Application Support/rtk/tee/1790351695_task_api_cd8074.log`.

## Read audit and permission follow-up (2026-09-25)

Root supplied an immutable snapshot at `/tmp/medtracker-journey-acceptance-20260925`. Relative to the previous checkpoint, only `rust/api/src/read_resources.rs` and `rust/contract-tests/tests/web_reads_api.rs` changed; their frozen SHA-256 values are `2fc9ba42980634c6288466ae9fd2e6991ebd097a681068256ca80cb87b817719` and `ecc7e81dfaa8383f8ae063c82ef02196dde2c4e37717aae785dc593c775877f8`. The full 3,565-path snapshot pre/post manifests match, aggregate SHA-256 `7ed2722833a34165d7256d8da2868b56c44e729e6bd17e81c05d7b922f7dc1d7`; the scoped 131-path Rust/runtime and fixture-provisioning manifests also match, aggregate SHA-256 `f823cc917a05639dc6f77be1765fb785eb82b1b02a8208d8949ed4b228b9530d`. These are scoped runtime manifests, not complete Rails source manifests. Files: `/tmp/journey-web-reads-green-snapshot-paths.txt`, `/tmp/journey-web-reads-green-snapshot-pre.manifest`, `/tmp/journey-web-reads-green-snapshot-post.manifest`, `/tmp/journey-web-next-runtime-snapshot-paths.txt`, and `/tmp/journey-web-reads-green-runtime-{pre,post}.manifest`.

| Selection | Project / subnet | Result |
| --- | --- | --- |
| Rails shared-read authority baseline (`task api:web-reads-rails`) | `mtcontract-f90ceb46226f4c84` / `10.255.42.0/28` | 4/6 passed; 2 audit-only cases failed; task exit 201. |
| Canonical Rust API acceptance (`task api:acceptance`, browser stage off) | `mtcontract-052b52f429d94af7` / `192.168.72.0/28` | 51/51 passed; task exit 0. |

The two Rails failures were `invalid_updated_since_is_audited_without_exposing_clinical_data` and `non_adult_schedule_index_denial_is_audited_without_exposing_clinical_data`; each request was denied as expected, but the audit observer found 0 events instead of 1. The other four shared-read cases passed, including the formerly forbidden authorized collections/details and revocation behavior. All six Rust shared-read cases passed. The canonical Rust selection also passed all dose-write (6), forecast (3), mobile OAuth (7), medication-read (9), OAuth (11), and web-session (9) tests. Browser tests were intentionally not selected.

Fixtures were hashed before cleanup (48,609 bytes each): Rails `tmp/contract-tests/run.Y9aA0P/fixture.json`, SHA-256 `e2c5c2c0b6d040deecae64668a817d1b9f260bdb5648662f170f356340dd7407`; Rust `tmp/contract-tests/run.MbCIlc/fixture.json`, SHA-256 `ad209ed3980f7cffa9bdcf92e381584296ca1e42546ce53208c15a28939b957a`.

The Rails web-test image `mtcontract-f90ceb46226f4c84-web-test:latest` has manifest `sha256:56c3fe4d2b8634b04dd7978289a4d74ba57bcecb35eb4d504dc44764e86bca81`, config `sha256:5112814444d3a6270e6c8a26bfa624de7623e2893c7cd870c1fce0459806d306`, and manifest list `sha256:0dcc20499fe0a62d00d8a8d207900eeb4de0353470eade126d92275a5065cc81`. Its contract runner `medtracker-contract-runner:mtcontract-f90ceb46226f4c84` has manifest `sha256:3c9758f326ff115f90fa5038a9636bd9ed65a6e82c414cd465dffc5455bd2d9e`, config `sha256:3301ffae56c94642cd5b410ce1042757caa6a6f138a18d99c1106d0212ce6fb2`, and manifest list `sha256:cbf26983b5934559e10b7ad7f513e55f7955140b897c2684caf0026321af905d`. The Rust contract runner `medtracker-contract-runner:mtcontract-052b52f429d94af7` has manifest `sha256:1f8e7b090cedc995729c5a4bbeb13d7c03d6162921bf1342c680a34d003e7f90`, config `sha256:8d444f703e255dd129acae9aed5556fae48f4fe27d951d10eda4eeecb97d6aa5`, and manifest list `sha256:1ca8770381221363e26af1a3eae5e9ea201313dc36ee3266cb585ce3399f5210`. Both runner builds used `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`.

Each fixture was ready after 58 seconds. The Rails web-read test took 5.36 seconds; Rust selected test binaries took 10.18 seconds combined. Both runs cleaned their containers, networks, named volumes, generated images, and fixture storage. The subnet preflights passed immediately before launch. Logs: Rails `/Users/damacus/Library/Application Support/rtk/tee/1790352194_task_api_f82f56.log`; Rails image build `/Users/damacus/Library/Application Support/rtk/tee/1790352075_task_tes_326746.log`; Rust test output `/Users/damacus/Library/Application Support/rtk/tee/1790352202_task_api_3bbac5.log`; Rust image build `/Users/damacus/Library/Application Support/rtk/tee/1790352190_task_api_aa54d0.log`.

## Source-capability canonical RED checkpoint (2026-09-25)

The browser-off canonical selection ran in `/tmp/medtracker-journey-acceptance-20260925` on `192.168.72.0/28` using `task api:acceptance` with `CONTRACT_BROWSER_TESTS=false`. Project `mtcontract-7537e57d74ff4012` exited 201: 51/53 passed. All previously accepted suites passed, including all six shared-read tests; the two new `source_capabilities_api` cases failed as expected because the implementation did not yet return source stock eligibility and `can_manage`.

- `eligible_stock_uses_source_signature_current_supply_and_selected_tracked_dosage` failed at `rust/contract-tests/tests/source_capabilities_api.rs:207` (`source stock eligibility` assertion).
- `source_list_and_detail_separate_record_from_manage_permission` failed at line 252 with `left: Null`, `right: true`.

The 3,566-path pre/post scoped source manifest matched at SHA-256 `45e4457556f69918df033e9821cf5ea96446217136630c25a12bb4f7967b82ca`. It includes the accepted API snapshot plus `source_capabilities_api.rs` and the updated canonical test selector; it excludes UI changes. Files: `/tmp/medtracker-api53-pre-paths.txt`, `/tmp/medtracker-api53-pre.manifest`, and `/tmp/medtracker-api53-post.manifest`.

Fixture `tmp/contract-tests/run.K0CBiR/fixture.json` was 48,609 bytes, SHA-256 `82156dec8349895189a21ae8bf90614b69fdcb487d6b8e9dfebdc1db3ac1d247`. The Rust runner image `medtracker-contract-runner:mtcontract-7537e57d74ff4012` has manifest `sha256:4ea536a09b68a0e797a7406a188c6c3ecdbeae682925854c1ae07ae5452c350e`, config `sha256:d154023a074f7433b44197c47f074bfda549fdfcb8fae351e21e40e5ef3fbd84`, and manifest list `sha256:df52532a695b6014f6240b7098ed79798bbbc307d57f1c239c3fd964ab973900`. The Rails `web-test` image `mtcontract-7537e57d74ff4012-web-test:latest` has manifest `sha256:a962bdc118483720e53532b44036bc2bdd9c638496ad954f99de53c6bac746e9`, config `sha256:f89bfc69bf9d405c05287eb44bf1f97fb35735c56c5dd6f837aeb0b5ae2905db`, and manifest list `sha256:cdbad1b3e80f403e0b8634a8db80e8264cbd5c203704e0632ca1a179afe00592`. Rust base: `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`.

The fixture was ready after 43 seconds. Contract test durations for the prior passing suites ranged from 0.14 to 4.41 seconds; the new source-capability binary failed in 0.09 seconds. Cleanup removed containers, network, volumes, images and fixture storage. Logs: results `/Users/damacus/Library/Application Support/rtk/tee/1790353136_task_api_5f10ed.log`; runner image `/Users/damacus/Library/Application Support/rtk/tee/1790353126_task_api_873192.log`; Rails image `/Users/damacus/Library/Application Support/rtk/tee/1790353030_task_tes_d506ca.log`.

## Reviewed source-capability candidate (2026-09-25)

The browser-off canonical API acceptance ran again in the frozen API snapshot after root replaced the reviewed `dose.rs`, `read_entities.rs`, and `read_resources.rs`; the candidate also includes the source-capability test and canonical selector. On checked subnet `192.168.72.0/28`, project `mtcontract-b5b5450b67134bd5` completed with 52/53 tests passing (task exit 201). All prior 51 suites and the source-list/manageability test passed. The sole remaining failure is a test fixture SQL type mismatch in `eligible_stock_uses_source_signature_current_supply_and_selected_tracked_dosage`: at `source_capabilities_api.rs:81`, the test binds a Rust `&str` to a PostgreSQL numeric column (`WrongType { postgres: Numeric, rust: "&str" }`). No product endpoint assertion was reached for that test.

The 3,566-path full and 132-path Rust/runtime pre/post manifests match at SHA-256 `23b3054dbbf5d44881704dcdfa94ec88eb4bd08a5e8746d5b3bb0e7cf2e2f410` and `cd5faa160a8ea2ef72cd0a3c4ef8571d92e06d5da22ba44755a8b02746b53e1c`, respectively. Full manifest: `/tmp/medtracker-api53-pre-paths.txt`, `/tmp/medtracker-api53-candidate-full-pre.manifest`, `/tmp/medtracker-api53-candidate-full-post.manifest`. Scoped manifest: `/tmp/medtracker-api53-candidate-scoped-paths.txt`, `/tmp/medtracker-api53-candidate-scoped-pre.manifest`, `/tmp/medtracker-api53-candidate-scoped-post.manifest`.

Fixture `tmp/contract-tests/run.vftUuA/fixture.json` was 48,609 bytes, SHA-256 `e921259790ed7808b074e32969f6f3cc1c0cbc2c63416f7957f1527054b2b356`. Runner image `medtracker-contract-runner:mtcontract-b5b5450b67134bd5` has manifest `sha256:9bf8ef729bece48c226911105583e7770a3e9650c3be8ae784e5260012fc2405`, config `sha256:b23a2c6c9cadc2eeb1787a50b5fafc75b5fd42f4ba65f1bc8eb488b4f949a22e`, and manifest list `sha256:511ad5766ef4e0ba84b3814872858808fb057bb673311371231b707af559c804`. Rails web-test image `mtcontract-b5b5450b67134bd5-web-test:latest` has manifest `sha256:21e4c28cf25cd46cfe5c29c080cd51a2954289ffb339496974f5a6536d1ff122`, config `sha256:8a12cc352c739ccaed316e89e9e540a9645a2ca74e0438d395996c54ebf80a23`, and manifest list `sha256:c638e268d6c44aa13605282666aec7bb87dcbd3d2d979b52314093a9c4f66ba5`. Rust base image: `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`.

The fixture was ready after 44 seconds. Test durations ranged from 0.16 to 4.78 seconds; the source-capability binary took 0.40 seconds. Cleanup removed containers, networks, volumes, generated images and fixture storage. Logs: results `/Users/damacus/Library/Application Support/rtk/tee/1790353944_task_api_81948d.log`; runner image `/Users/damacus/Library/Application Support/rtk/tee/1790353931_task_api_5bd2b4.log`; Rails image `/Users/damacus/Library/Application Support/rtk/tee/1790353829_task_tes_d4093e.log`.
