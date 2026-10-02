# Rust module extraction verification

All source writers frozen before verification. No production logic edited by verifier.

- API format-write and web format tasks completed.
- Contract root/module formatting used existing api:fmt-file task (edition2024),
  despite contract crate edition2021; coordinator notified of possible import ordering.
  Corrected through coordinator-added api:contract-harness-fmt:write; subsequent
  api:contract-harness-fmt passed, both exit0.
- api:check all targets passed, 13.90 seconds.
- api:test passed:36 unit tests,12 auth compatibility tests; exit0.
  Raw log: /tmp/medtracker-refactor-api-test.log.
- Web task test passed; exit0. Raw log: /tmp/medtracker-refactor-web-test.log.
- Disk available155GiB after focused checks.

Earlier focused GREEN predates the final dashboard projection extraction.
Selected existing serial task plan: /tmp/medtracker-refactor-verification-plan.md.

## Final source gate and first runtime result

Final dashboard extraction frozen; independent source review cleared. API formatting
and ci:rust-port passed, exit0. Raw gate log: /tmp/medtracker-refactor-rust-port.log.

api:acceptance exited201:56 passed,16 failed across12 targets. Snapshot
dc8f260be5c955ba720c2b8617fca80f4d8e54e688f2cd553cf6a47984d512dd.
Project mtcontract-454d04dd55784074; storage tmp/contract-tests/run.B9rCo1/storage.
Owned project containers/network/volumes/images and storage cleanup completed.
Raw untruncated suite: /tmp/medtracker-refactor-http-base-raw.log.

- dose_mode_transition_api:0/2; fixture INSERT omits dosages.default_dose_cycle,
  SQLSTATE23502 at line41.
- management_sync_events_api:1/3; same omitted non-null column at line128.
- medication_stock:2/9; seven timestamp precision assertions expect string length20,
  actual27 including microseconds, line57.
- source_capabilities_api:0/2; eligible stock ordering reversed and paused-source
  inventory unexpectedly nonempty, lines376/293.
- web_reads_api:5/6; People per_page999 request rejected422 versus expected200
  clamping, line167.
- web_session_api:7/9; dashboard login helper expects200 but returns503, line209.
- Remaining six targets passed: dose_write_api6, medication_forecast_api3,
  medication_management_security_api5, medication_mobile_oauth_api7,
  medication_read_api9, oauth11.

Additional runtime tasks were paused after this first failure for baseline comparison.
Disk146GiB at that point; final outcomes below supersede the temporary pause.

## Pre-refactor differential baseline

Clean independent clone at /tmp/medtracker-refactor-baseline-20261001, detached
dfab7fe8463a032102da203596c4720723099ef5. Identical api:acceptance exited201,
56 passed/16 failed. All16 failing test names and assertions match final refactor,
ignoring dynamic IDs/timestamps/request IDs; no new failure detected in this suite.
Baseline source digest453d76d198e59dd96a22373c8953a0555feeda615951f87e33a11e2f0d5e96dc.
Raw baseline suite: /tmp/medtracker-refactor-baseline-http-raw.log.
Project mtcontract-3fff5a407b2542d5 cleanup completed.
Sequential baseline-only symlinks reused original UI target/node_modules caches;
removed target symlink after runtime; npm ci replaced node_modules symlink with an
ignored baseline-local directory. Baseline Git status clean. No tracked edits.

## Additional resource acceptance

- People:17 passed, exit0; project mtcontract-2221d404f3d34063, cleanup complete.
  Main log /tmp/medtracker-refactor-http-people.log.
- Locations:29 passed (20 write/cascade,8 read,1 rate-limit), exit0;
  project mtcontract-dfb09ded2e774c6b, cleanup complete.
  Main log /tmp/medtracker-refactor-http-locations.log.
- Focused medications:7 passed/2 failed, exit201; project
  mtcontract-b8804051ef9b4fcb, cleanup complete. Both resource key assertions at
  openapi_medications.rs58 differ by extra barcode,friendly_name,warnings.
  Raw /tmp/medtracker-refactor-http-medications-raw.log; focused baseline pending.

All refactor snapshots retain digest dc8f260be5c955ba720c2b8617fca80f4d8e54e688f2cd553cf6a47984d512dd.

Focused medication baseline reproduced exact7PASS/2FAIL key-set differences;
/tmp/medtracker-refactor-baseline-medications-raw.log. No new failure detected.

Read-completion refactor and identical baseline each21PASS/2FAIL (20 location-write
checks plus1 read-completion check pass). Both failures expect Cache-Control no-store
but receive private, no-store, at openapi_read_completion.rs110/218. Exact failures
match; no differential regression. Baseline project mtcontract-f67e7773bded4cd3
cleanup complete and target-cache link removed; baseline Git status clean.
Raw evidence /tmp/medtracker-refactor-http-read-completion-raw.log and
/tmp/medtracker-refactor-baseline-read-completion-raw.log.

## Final household and harness acceptance

New api:contract-harness-test ran all4 moved URL/write-safety library tests:PASS,
exit0. Log /tmp/medtracker-refactor-harness-test.log.

Final combined household acceptance:15HTTP/21browser PASS, exit0.
Project mtcontract-fc237996acdb42e7; cleanup completed including storage run.egCNnU.
Digest082028057307078aec8296ee10983459c89456a0795493cf247ae06ca1fa79ba
differs from prior refactor hash only by the coordinator-added harness-test Taskfile
target; production Rust unchanged. Raw browser /tmp/medtracker-refactor-household-browser-raw.log.

All16 screenshots refreshed13:56:46–13:56:52BST in
docs/screenshots/journey-medication-rust. Inspected desktop/mobile dose dialog,
stock and history plus representative household forms. History wraps within390px,
times visible, strict responsive assertions unchanged and passed. Decimal1.25ml dose
leaves18.75ml stock. The mobile household navigation remains horizontally scrollable.

Schedule/person-medication-write HTTP suites NOT RUN by coordinator scope decision:
their implementations untouched, route/body equivalence reviewed and shared auth
exercised elsewhere. No further test broadening. Final disk154GiB; all command sessions
completed and owned fixtures cleaned. No new runtime failures found versus baseline;
existing18 broad/focused medication failures and2 cache-header failures remain RED.
