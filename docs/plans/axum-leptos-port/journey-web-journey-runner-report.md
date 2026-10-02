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

## Medication browser candidate checkpoints (2026-09-25)

The browser candidate ran against two root-provided immutable snapshots. Both selected the five existing medication journey cases plus `medication-journey-form.test.mjs`. Each run used a fresh, immediately checked `/28` subnet. Both snapshots excluded `source_capabilities_api.rs`, which is outside this browser candidate.

| Snapshot | Project / subnet | Result |
| --- | --- | --- |
| UIv1, full input manifest `58489e9843f5d78ade5187c45451f6a495782debf12d6b01b0c56ba21036656a` | `mtcontract-2eb3f447ce6e4c81` / `10.255.42.0/28` | 5/6 passed; companion failed at 30s waiting for source B stock to remain 20. Task exit 201. |
| UIv2, full input manifest `ccfccbb726e781b74bbbf16fafb6b829675caf1c18cca319a39ed886bb38bb02` | `mtcontract-7d29a13e269e4edf` / `10.255.43.0/28` | 5/6 passed; companion received 303 rather than expected 403 on a crafted cross-source POST. Task exit 201. |

All five existing cases passed in both runs: the desktop and mobile medication journeys, desktop and mobile Escape focus restoration, and invalid credentials/foreign medication denial. In UIv1, the companion's missing/wrong-CSRF and cross-source denial requests passed; its subsequent stock assertion raced with the desktop journey, which was also consuming source B. The browser owner isolated source B in UIv2. In UIv2, the CSRF-denial assertions passed and the cross-source POST returned 303, so the replay assertion did not run. This is the outstanding form-action URL guard RED, not a failure of the CSRF checks. The exact UIv2 failure is in `medication-journey-form.test.mjs:154`.

UIv1 fixture: `tmp/contract-tests/run.2XyGBp/fixture.json`, 48,609 bytes, SHA-256 `9d5c613fc767d0b23f6f0cceee1c7061408bf48beaf958cf691a69fbb91d434e`. UIv2 fixture: `tmp/contract-tests/run.BMGS3B/fixture.json`, 48,609 bytes, SHA-256 `25b2306ac2521abc257c87338dfde918641b5d83b90276b42943f957d6d9be59`.

UIv1 full 3,569-path pre/post source manifests matched at SHA-256 `58489e9843f5d78ade5187c45451f6a495782debf12d6b01b0c56ba21036656a`. UIv2 full 3,569-path pre/post manifests matched at `ccfccbb726e781b74bbbf16fafb6b829675caf1c18cca319a39ed886bb38bb02`; its explicit eight-file UI source/test manifest also matched the frozen snapshot at `4106709cfb3861affa8b7258c081ab8db41be7df56e35ef9ca62117ffbd94b05`. UIv2 manifest files are `/tmp/medtracker-ui-v2-full-pre.manifest`, `/tmp/medtracker-ui-v2-full-post.manifest`, `/tmp/medtracker-ui-v2-snapshot.manifest`, `/tmp/medtracker-ui-v2-pre.manifest`, and `/tmp/medtracker-ui-v2-post.manifest`.

The six screenshots from each run were copied into `docs/screenshots/journey-medication-rust/`; UIv2 screenshots replace UIv1 images in that directory. Their UIv2 checksum manifest aggregate is `0c58beefb226b97e4cfda907a964529560a8cf1f9de928b1c95cd3eaadb67964` (`/tmp/journey-medication-rust-v2-screenshots.manifest`). I inspected the desktop/mobile dialog, stock and history captures: all sections render, and the desktop dialog fits. Its stock-source select truncates the long synthetic fixture medication label at the field edge.

| UIv2 image | Manifest | Config | Manifest list |
| --- | --- | --- | --- |
| Rails `mtcontract-7d29a13e269e4edf-web-test:latest` | `a962bdc118483720e53532b44036bc2bdd9c638496ad954f99de53c6bac746e9` | `f89bfc69bf9d405c05287eb44bf1f97fb35735c56c5dd6f837aeb0b5ae2905db` | `cdbad1b3e80f403e0b8634a8db80e8264cbd5c203704e0632ca1a179afe00592` |
| Rust runner `medtracker-contract-runner:mtcontract-7d29a13e269e4edf` | `3e1ee7c77b51b5610faa897da2471d0e1e708e94a04727f7844d649ab1a4ac51` | `423ba8eca45c3f35209d42bfebf9c75a9ca76636084e477b5b97cc59f43fb03f` | `16fea3da75d8e9b98dbf269a603e9c5cfb3d8b042811ace20e0701ba3413546e` |
| Browser runner `medtracker-contract-browser:mtcontract-7d29a13e269e4edf` | `28c54b963b064a0d3e70efe8fbe728f2381d7e2b90cc2ce99c230be15e4f9a58` | `edcec07d049e4a523cd42751fcf3d88c9a39030145336fa167bb3f7790c207d8` | `b44e5ad31ad2a847ca5b6ce209b0a9607e2ec3860ff2aef18e76335f05d47409` |

The v2 Ruby base was `rubylang/ruby:4.0.7-resolute@sha256:a8ee881bc5dff85f0a001ba0a9ee9e59e7c18a14da5d7662a53b308ff63a9d65`; Rust base was `rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e`; browser base was `mcr.microsoft.com/playwright:v1.63.0-noble@sha256:eff16c30e6f3f4af0a03fa4b706120d5e9b0891c344a27d64559aff5900a4a27`.

For UIv1, the Rails server was ready after 32 seconds, the fixture after 52 seconds, and browser tests took 33.34 seconds. For UIv2, the server was ready after 24 seconds, the fixture after 45 seconds, and browser tests took 5.06 seconds. Both projects removed containers, networks, volumes, images and fixture storage. Logs: UIv1 browser `/Users/damacus/Library/Application Support/rtk/tee/1790352813_task_api_aa2439.log`; UIv1 images `/Users/damacus/Library/Application Support/rtk/tee/1790352778_task_api_229a78.log` and `/Users/damacus/Library/Application Support/rtk/tee/1790352682_task_tes_a98d8a.log`; UIv2 browser `/Users/damacus/Library/Application Support/rtk/tee/1790353271_task_api_12c2c5.log`; UIv2 runner `/Users/damacus/Library/Application Support/rtk/tee/1790353263_task_api_777ed7.log`; UIv2 Rails image `/Users/damacus/Library/Application Support/rtk/tee/1790353170_task_tes_a63e54.log`.

## Final frozen Rust candidate

The refreshed immutable snapshot `/tmp/medtracker-medication-ui-acceptance-20260925` passed both authorized runs:

- Canonical `task api:acceptance` with `CONTRACT_BROWSER_TESTS=true`: 53/53 HTTP tests and 7/7 login browser tests passed; task exit 0. Project `mtcontract-bcdaeeb169104476`, subnet `192.168.72.0/28`.
- Dedicated `task api:browser-rust`: 7/7 medication browser tests passed, including CSRF rejection/replay, taper schedule, desktop/mobile Escape focus restoration, dose/history journeys, and invalid/foreign access; task exit 0. Project `mtcontract-b007384768f64c58`, subnet `10.255.46.0/28`.

The canonical fixture was `tmp/contract-tests/run.Z77YQk/fixture.json`, 49,163 bytes, SHA-256 `e87a2fa29bf846a6c4d0bf6c5668d3c4c2e3ddc906c9e4c3809197ce65613ab6`. The medication browser fixture was `tmp/contract-tests/run.yWKhf0/fixture.json`, 49,163 bytes, SHA-256 `e617666157fa20a8d63a78e4e1147c92d5cb70d03fa929c0c844974a6dcf7353`.

Full 3,570-path pre/post manifest matched at SHA-256 `17b9fe6d0efec175f5557561cf18576870be23bc9f929ffc9c0c6ddb7aec67c`; scoped 132-path runtime manifest matched at `baf9f414e71971152c1f08119a55f2d9ba842efe1ea3a432165a7d3edba214e9`; the root-provided 14-path copied-source manifest matched at `a19d4ac32fbce1697b80563f0964e8d0820f601155702eab89a2b4b068dd0e48`. The scoped manifest is Rust/runtime evidence, not a complete Rails source manifest. Supporting manifests are `/tmp/medtracker-final-full-pre.manifest`, `/tmp/medtracker-final-full-post.manifest`, `/tmp/medtracker-final-scoped-pre.manifest`, `/tmp/medtracker-final-scoped-post.manifest`, `/tmp/medtracker-final-copied-pre.manifest`, and `/tmp/medtracker-final-copied-post.manifest`.

The Rust runner image was `medtracker-contract-runner` manifest `8f5f619d418ceeb8ada4ae1815bc0502b47218846a8ff26327570453dc698787`, config `258962cf507192edbe7b86715690f4c4415c1c508442429426ac9ec53c3085cf`. Canonical login browser image manifest list was `dee9748e0745388fd15b3b951a82b2bc09819b4d9397c64df8764ea383195106`, config `9374d3e1269709ad0f72ce36c32c0537097a87200f4ed5bcf4edca8803ec190d`; dedicated journey browser image manifest list was `262d63d9877410b22e80c82fdc03afb9746fa892cbdc237a578bef180edb0bb6`, config `f10b69a196438d61774005014b218c6f00b8180b761c7530e8e89125a157a1be`.

All six final screenshots were copied to `docs/screenshots/journey-medication-rust/`: `journey-dose-dialog-desktop.png`, `journey-dose-dialog-mobile.png`, `journey-history-desktop.png`, `journey-history-mobile.png`, `journey-stock-desktop.png`, and `journey-stock-mobile.png`. Both project cleanups removed their containers, networks, images, volumes, and fixture storage. Logs: canonical `/Users/damacus/Library/Application Support/rtk/tee/1790354651_task_api_ac3b16.log`; dedicated browser `/Users/damacus/Library/Application Support/rtk/tee/1790354638_task_api_131bad.log`.
