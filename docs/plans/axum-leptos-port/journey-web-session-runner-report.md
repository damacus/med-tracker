# Web session API: runner report

## Red baseline

At baseline `39904af0`, all four `web_session_api` cases failed while the shared helper extracted the CSRF token from `/login`. That run established the missing standalone-login boundary; it did not verify cookie, CSRF, logout or invalidation behavior.

## Acceptance results

Both runs started after joint product, contract-test, reviewer and browser-source freezes. They used separate disposable Compose projects and subnet preflights.

| Selection | Command | Project / subnet | Result |
| --- | --- | --- | --- |
| Focused web session | `task api:web-session-acceptance` | `mtcontract-1e60c7f5aa174de7` / `192.168.72.0/28` | Passed: 8/8 `web_session_api` tests, 0 failed, in 2.36 seconds. Task exit 0. |
| Canonical API and browser | Fish set `CONTRACT_BROWSER_TESTS=true`, then `task api:acceptance` | `mtcontract-317183228df94655` / `10.255.42.0/28` | Passed: selected HTTP contract set (36 tests) and browser smoke (7/7, 0 skipped). Task exit 0. Browser stage reported 3.69 seconds. |

The canonical HTTP selection comprised 9 medication-read, 7 mobile-OAuth, 3 forecast, 11 OAuth and 6 dose-write cases. The browser selection covered desktop/mobile standalone login and logout, keyboard login error handling, and desktop/mobile OAuth consent.

The subnet checks passed immediately before dispatch. Candidate ranges `192.168.88.0/28`, `192.168.96.0/28`, and `172.30.240.0/28` were rejected because they overlapped active Docker routes; no Compose project was started on those ranges.

## Stable input and runtime evidence

- The pre/post manifest covers 129 files under `rust/api`, `rust/contract-tests`, `rust/web` and `scripts`, plus root `Taskfile.yml`, `compose.yaml`, `.dockerignore`, `rust/web/Dockerfile.smoke` and its `.dockerignore`. It includes untracked/new files present at the freeze. It excludes generated reports/screenshots and the full Rails app/schema/config, so this is a scoped Rust/runtime and fixture-provisioning manifest, not a complete Rails source manifest.
- Pre and post individual hashes match exactly. Aggregate SHA-256 of the manifest: `5950c95fb5df771c0224f0d2896621fa8cb7428c42bbd63d6ba5a38ad7e8ee8a`. Path list and manifests: `/tmp/journey-web-session-runtime-paths.txt`, `/tmp/journey-web-session-runtime-pre.manifest`, `/tmp/journey-web-session-runtime-post.manifest`.
- Focused fixture: `tmp/contract-tests/run.3PdmWy/fixture.json`, 48,609 bytes, SHA-256 `ad294ace448c925e4787bed96fe8abcc56aeb325fd720ff088d01043037d81fb`.
- Canonical fixture: `tmp/contract-tests/run.977ymH/fixture.json`, 48,609 bytes, SHA-256 `3ba47d1b33dbe6063177d5a8f469e82afc8ec4b85b7edf296d47859ceaa2db8c`.
- Both Rails app images used `rubylang/ruby:4.0.7-resolute@sha256:a8ee881bc5dff85f0a001ba0a9ee9e59e7c18a14da5d7662a53b308ff63a9d65`. The focused run's Rails image was `mtcontract-1e60c7f5aa174de7-web-test:latest`, manifest `sha256:6bb4d897f0d82f2de23ad9c04a4d0b166b564b50ae86531dd7b5f3046c407e71`, config `sha256:0c54c6732893882d07bbe7c003f2908db60cf4c398ebef335416d6a7df2a78fd`, manifest list `sha256:d4405a7a9330c803c0f3cd4ac89570a994406ed72c13e246a00aa897a9648f21`.
- Focused Rust runner image `medtracker-contract-runner:mtcontract-1e60c7f5aa174de7`: manifest `sha256:0d1efab812f39ccfbf4f7ce2cd6c45ed1d0c47d0b1cbe902c81e8f59933d33a6`, config `sha256:386079484b482b05f28b4c750be19afa91789f7eff3d6f78628ace3767bcc494`, manifest list `sha256:03e04a2478849b71a9a9b53b605e21b9d42a9ebe50f7f4ae0a32c0e5c6550360`.
- Canonical Rust runner image `medtracker-contract-runner:mtcontract-317183228df94655`: manifest `sha256:0d154ec366de35e84d8043fe7ac7896a32af16825c16635c6f7c45e1e6221fab`, config `sha256:d2292cf6d4f4bd8aba483e9d89ab79744718b0be5dc5344a7879bb884bdc36a5`, manifest list `sha256:50ff76ed5c0240b1b4fcdf8b2f469eb4515f35fdcd5bef27e2a5aa1475989f4c`.
- Canonical browser image `medtracker-contract-browser:mtcontract-317183228df94655`: manifest `sha256:ebf0a03d924ba47d0420680ca259860ab6df6834240a6515e70d2096f723f6b7`, config `sha256:2d61507e5fd73227ffaf33ceb0860123641e027341434ba6ce353bd2f74844cb`, manifest list `sha256:ff840db7b01aa68b9137f83f57a90cab437d6db11778851a7c2d88893c7999f4`.
- Runtime logs: focused runner build `/Users/damacus/Library/Application Support/rtk/tee/1790349367_task_api_9339d0.log`; canonical runner build `/Users/damacus/Library/Application Support/rtk/tee/1790349367_task_api_da04f6.log`; canonical browser stage `/Users/damacus/Library/Application Support/rtk/tee/1790349382_task_api_32a4eb.log`; Rails image build `/Users/damacus/Library/Application Support/rtk/tee/1790349280_task_tes_9379eb.log`.
- Fixture readiness was reported at 57 seconds (focused) and 58 seconds (canonical); Rails servers were ready at 29 and 30 seconds; Rust runner builds took 26.7 and 26.8 seconds. Focused tests took 2.36 seconds; browser took 3.69 seconds. Overall both concurrent runs completed in roughly two minutes.
- Both projects cleaned their containers, network, named volumes, generated images and temporary storage. The focused fixture and canonical fixture files were removed during teardown after hashing. No unrelated Compose project was touched.

## Screenshot inspection

The canonical run generated the standalone login and consent screenshots under `docs/screenshots/journey-auth/`. Desktop and mobile screenshots show readable labels and descriptions, visible primary actions, checked/labelled consent controls, and cards that fit the viewport without horizontal overflow. The action focus outline is visible in the captured screenshots, consistent with the keyboard interaction checks.

## Evidence limits

This proves the focused Rust session API and canonical Rust HTTP/browser acceptance selections passed for the frozen inputs recorded above. The Rust image manifests differ by project; both are recorded. The Rails image manifest/config/list were emitted for the focused image build, while the canonical Rails image used the same source and base image but its separate output manifest was not captured. The canonical Rails image is therefore identified by its project tag and pinned base image digest only. The independent shared-read Rails baseline is outside these selections and remains separately tracked.
