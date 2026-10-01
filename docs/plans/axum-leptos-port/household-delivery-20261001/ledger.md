# Delivery ledger

- 17:09 BST: resumed on `codex/rust-household-management-20261001`, based on
  published #2349. Starting acceptance 14/20. #2349 CI has no failures; Rust
  source and browser jobs still pending. Two-hour retrospective scheduled.
- Writer owns D first. E/F wait for preceding journey acceptance. Verifier is
  the sole runtime owner. Reviewer remains independent and read-only for code.
- 17:14 BST: #2349 CI run 36887533975 completed successfully, including both
  Rust source/build and browser jobs. The continuation baseline is published
  and green; no rerun or baseline repair is required for that PR.
- Verifier traced `api:openapi-dosages-acceptance` through the runner: `rails`
  refers to fixture generation; the `openapi-dosages` selector builds and tests
  captured Rust API. Prior scout target ambiguity is resolved by dispatch code.
- Independent #2349 review found medication unknown-error rendering remains a
  planned D gap, already excluded from accepted A/B/C; writer will fix it before
  task 4.3 acceptance. No extra People/Locations validation regression found.
- D tests authored before implementation: three browser HTTP cases exercise
  dosage persistence, tracked/null stock, exact custom units, rejected drafts,
  original stale/missing tokens, precision, CSRF and parent mismatch. A separate
  five-locale renderer regression covers unknown medication errors. Verifier
  owns baseline/RED execution; no new acceptance claimed until actual results.
- D baseline passed 14/14 Rust dosage HTTP tests. Verifier retained raw log,
  fixture identity and stable 302-path pre/post manifest. Full copied-source
  comparison raced cleanup; only the two critical new test/unit files were
  directly checked against the actual copy. Do not overstate that provenance
  or rerun the passing baseline solely for metadata. Next capture will compare
  a synchronously validated runner-equivalent snapshot with its emitted digest.
- Process ruling applied now: runtime freezes cover captured inputs; unrelated
  browser test authoring resumes at capture boundaries. Production changes
  still wait for relevant RED assertions. Keep one writer and one runtime owner.
- Independent reviewer resolved all three previously ambiguous #2347 contracts
  from Rails/OpenAPI/browser sources. Coordinator accepted the bounded rulings
  in plan.md; runtime proof remains G work. No premature baseline-test weakening.
- Snapshot preparation encountered known sandbox DNS denial before test execution.
  It is a setup failure, not RED. Verifier applies approved networking permissions
  to snapshot tasks too; avoid an extra source build solely for bookkeeping.
- D RED reached actual assertions: four new HTTP cases return 404 for the missing
  add route; the new medication unit rejects raw unknown diagnostics; focused
  dosage-locale integration also rejects raw diagnostics. The generic crate test
  stopped at its failing library test, so the existing TEST_FILE selector was
  used to run integration explicitly. Captured HTTP copy matched its validated
  manifest; web pre/post hashes matched. Writer now implements the affected
  browser routes and renderer. Existing dosage API extraction uses its green
  14-test baseline; no new acceptance is claimed yet.
- Native timer timestamp verified: created 17:07:45 BST; two-hour checkpoint
  19:07:45 BST (18:07:45 UTC). Root will run retro at this waypoint even if the
  busy-thread scheduler queues its prompt, then delete the one-shot automation.
- Independent static extraction review passed: 32 declarations retain original
  bodies after visibility-only normalisation, with policy, locks, transactions,
  inventory, audit and sync unchanged. Full D source review found no material
  security defect; browser permission, immediate use, aggregate/audit/sync and
  repeated-submit evidence still needed before acceptance.
- First GREEN fast compile caught a moved dialog-title String in medication.rs
  before Docker runtime acceptance. Same writer owns the correction. No new
  runtime fixture launched on that failed input; this is a compile failure,
  not a browser/test pass or a second failed implementation fix.
- Same-owner title clone fixed E0382. Fast API all-target check, selected new
  contract compilation and all Rust web unit/integration tests passed. Writer
  removed an unused extraction re-export and is adding the independent review's
  missing forged/member, audit/no-write and JS-enabled immediate-dose checks.
- Two intermittent Sol 6.1 capacity errors were recovered by resuming the same
  writer. No ownership change. One dosage baseline runtime job had already
  launched when the final-capture hold arrived; allow its healthy immutable run
  to finish and certify only that capture. Further captures wait for the writer's
  explicit final strengthened-test notice. Do not infer source freeze from an
  agent's capacity error or quiescent status.
- Rails source policy review confirms ordinary manage-granted members cannot
  create/edit dosage options; the dosage policy delegates to medication update,
  not the broader medication create rule. Existing Rust owner/admin restriction
  matches that case. Runtime member-denial proof still pending.
- Bounded E source review resolved tracked-stock adjustment: parent endpoint
  changes only the parent and must not be presented as reconciled option stock.
  Existing option PATCH is the coherent path. Filed dropped scalar audit reason
  as #2350, verified labels `rust,bug`, and scheduled its RED/GREEN fix in E.
  No new endpoint or invented reason field is authorised.

## Waypoints

| Journey | State | Evidence |
| --- | --- | --- |
| D dosage management | Accepted; publication in progress | Writer report, independent review and verification queue |
| E stock | Waiting for D | Plan |
| F assignments and schedules | Waiting for E | Plan |
| G parity and final acceptance | Waiting for F | Plan |
| Two-hour retro | Scheduled | Native heartbeat |

- 18:26 BST: D core HTTP passed 4/4. Browser retry passed 20/20 after a
  test-only pagination repair; all assertions were retained. A separate fresh
  fixture passed the grant-mutating permission case 1/1 and browser cases 20/20.
  Independent review verified the actual assertion logs and translated dialogs.
  Full `ci:rust-port` is running on frozen source; acceptance remains 14/20
  until its result and final independent review. No product retry was needed.
- 18:27 BST: full `ci:rust-port` passed on frozen authored inputs; independent
  requirements and quality/security review passed. D/4.3 is accepted: 15/20.
  Publish this completed journey before starting E. Two presentation findings
  remain explicitly queued for E/shared integration. The actual two-hour retro
  remains at 19:07:45 BST; it has not occurred yet.
