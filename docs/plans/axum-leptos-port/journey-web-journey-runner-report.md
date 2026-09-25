# First-party web journey: runner report

## Stable selection

Three isolated runs used the frozen read API modules, focused session test, browser journey test, fixture provisioner, and runner wiring. The product read modules compiled and passed local API check, formatting, Clippy and six unit tests before the runs. Each Compose project had its own immediately validated `/28` subnet.

| Selection | Project / subnet | Result |
| --- | --- | --- |
| Rust medication browser RED: `task api:browser-rust` | `mtcontract-232df6665f68446e` / `192.168.72.0/28` | 1/5 passed; 4 failed; 0 skipped. Task exit 201. |
| Focused browser-cookie/session RED: `task api:web-session-acceptance` | `mtcontract-25a62bffe4a94baa` / `10.255.42.0/28` | 8/9 passed; 1 failed. Task exit 201. |
| Shared-reads acceptance: `task api:web-reads-acceptance` | `mtcontract-32355f36debc4014` / `10.255.43.0/28` | 4/4 passed. Task exit 0. |

## First failures

The new `browser_cookie_lists_only_current_operational_households_with_bearer_precedence` case received 401 instead of 200 at `rust/contract-tests/tests/web_session_api.rs:466`. The other eight selected session tests passed. This is the focused RED for cookie-backed household listing.

In the browser run, the negative-credentials/foreign-medication case passed. Four desktop/mobile cases failed after waiting 30 seconds for the fixture medication H1, such as `Contract browser desktop dose df46e7c7e5692b87e8d45127`. The calls were in `stockOnMedicationPage` at line 52 or the journey assertion at line 127 of `rust/web/tests/medication-journey.test.mjs`. The Escape-focus cases did not reach the Log dialog; they do not establish a focus-restoration defect. The positive journeys establish the missing rendered medication entry as the current browser boundary.

The shared-read suite passed all four cases against the compiled Rust read implementation.

## Fixture and input identity

- Browser fixture: `tmp/contract-tests/run.3hSMPN/fixture.json`, 48,609 bytes, SHA-256 `6a3b5bf9c2fdefc8ab1da339864657ad2f3c46fd336dd1793a332b93d35daf2b`.
- Session fixture: `tmp/contract-tests/run.32COjZ/fixture.json`, 48,609 bytes, SHA-256 `59e1ae1da15215982ecb13129b190213a0008e5f41402df430bc3ab21a4b0288`.
- Shared-reads fixture: `tmp/contract-tests/run.VofmvH/fixture.json`, 48,609 bytes, SHA-256 `3a94adbb5d9572e67be5f9bdecf37115363a8d9363bf0de0cda4c2126b44e834`.
- The pre/post runtime manifest covers 131 files under `rust/api`, `rust/contract-tests`, `rust/web`, and `scripts`, plus root `Taskfile.yml`, `compose.yaml`, `.dockerignore`, and browser Dockerfile/ignore inputs. It includes new runtime and test paths but is not a complete Rails-source manifest.
- Pre/post per-file hashes match; aggregate SHA-256 `6134c75e456f81f1cfa5232ac747998416ea26b9d47fb8613b7fbae8562ffa44`. Supporting files: `/tmp/journey-medication-runtime-paths.txt`, `/tmp/journey-medication-runtime-pre.manifest`, `/tmp/journey-medication-runtime-post.manifest`.
- Rust base: `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`. Browser base: `mcr.microsoft.com/playwright:v1.63.0-noble@sha256:eff16c30e6f3f4af0a03fa4b706120d5e9b0891c344a27d64559aff5900a4a27`.

| Image | Manifest | Config | Manifest list |
| --- | --- | --- | --- |
| `medtracker-contract-runner:mtcontract-232df6665f68446e` | `d57a2d7394e754a3b6ce0c5a5a13bcb90e2744a45933a94d96e8e10cc915a403` | `c43e2206b662af96a575da77877bc844fff86bc5eb6bde0b391dc5195f14af83` | `060d2468f31d772ac757abbfbb4884d306c04baf823cb87dda33e204f4a180f3` |
| `medtracker-contract-browser:mtcontract-232df6665f68446e` | `526b3e80399615ac5605e15e29428b4f0750ce6d433f7b23616b34f56c5e54e6` | `02ee9865084a37934dbf773c047b46625d0d7a386716d3a4e957170e5e27e149` | `b7ec0015b2bb60020425d92d9eddb1e1a816e9a3e2fce634d8c479a26875b0d1` |
| `medtracker-contract-runner:mtcontract-25a62bffe4a94baa` | `915120d922c4baae3f7644f7b803d07cfddbe9d02de581639abb56c9c0e24da1` | `909b1e8e66c033e399c44a1a39ea9d1f1263a96e6677ca0903e378205ff4c5d6` | `37bf8cbdfb7fcc7c93e413b5192debdba85da688a1d58b36d964980c7d2feda0` |
| `medtracker-contract-runner:mtcontract-32355f36debc4014` | `e70178b324023f9e3dab6f3dfb2f601f6129eb1cdce407d6c652309a4484adbf` | `6f4fa6abc6a143b59cd25d79e28eb431374117248432129b3a343e8175862f55` | `d47f098b5018d3d9e8141f34420fdbc2449cf5a13605b6a22451f3fdc3e44ad4` |

The Rails `web-test` base image was `rubylang/ruby:4.0.7-resolute@sha256:a8ee881bc5dff85f0a001ba0a9ee9e59e7c18a14da5d7662a53b308ff63a9d65`. Its project image digest was emitted for the focused session project only; separate Rails image manifest/config/list evidence was not retained for the browser and reads projects. Do not treat the scoped runtime manifest as substituting for those image identities.

## Timing, screenshots and cleanup

The three fixture provisioners reported ready after 74, 70 and 72 seconds (browser, session, reads). The Rust API servers reported ready in approximately 39–41 seconds. Browser tests ran for 122.4 seconds; the session contract tests for 2.69 seconds. The shared reads passed in the same run; a distinct test-stage elapsed value was not retained.

No screenshots were present under `docs/screenshots/journey-medication-rust` after the browser run. The positive flows timed out before a medication page was rendered, so there is no visual screenshot evidence for the missing page.

All three owner-scoped cleanups removed their containers, networks, generated images, volumes and temporary storage. The fixture files were hashed while present and removed by teardown. No peer project was inspected or cleaned. The product source freeze was released after all runs were terminal and the post-run manifest matched.

## Logs

- Browser tests: `/Users/damacus/Library/Application Support/rtk/tee/1790350900_task_api_db4caa.log`.
- Focused session tests: `/Users/damacus/Library/Application Support/rtk/tee/1790350785_task_api_455680.log`.
- Browser Rust runner build: `/Users/damacus/Library/Application Support/rtk/tee/1790350774_task_api_fcd971.log`.
- Session Rust runner build: `/Users/damacus/Library/Application Support/rtk/tee/1790350774_task_api_5efc36.log`.
- Shared-reads Rust runner build: `/Users/damacus/Library/Application Support/rtk/tee/1790350774_task_api_2a7e5e.log`.
- Rails image build output: `/Users/damacus/Library/Application Support/rtk/tee/1790350682_task_tes_439461.log`.
