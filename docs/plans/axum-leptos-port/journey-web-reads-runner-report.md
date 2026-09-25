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
