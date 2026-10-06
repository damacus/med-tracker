# Clippy advisory and small refactoring plan

The required `task lint` and existing CI gates remain unchanged. This delivery
adds an optional report so we can measure useful checks while care and identity
implementation continues. It does not add a research gate or require unrelated
application refactors before a feature can ship.

## Running the report

Run `task --taskfile Taskfiles/lint-advisory.yml report` through the sole build
runner at a safe checkpoint. It invokes locked Clippy with explicit `--workspace`
and `--all-targets`, covering the application and migration packages. Excluded
`client-tools` keeps its existing separate check.

Read `test-results/lint-advisory/report.md`. The matching `findings.json` contains
only run metadata and lint names, paths, categories and line numbers. The report
groups repeated findings by lint and file, separating production, test and
generated paths. It excludes source snippets and diagnostic messages. Inline
unit tests in a production file remain marked production; inspect the location
before treating a finding as an application defect.

Compilation failure, unknown lint configuration or malformed diagnostics makes
the report **incomplete**. An incomplete report cannot establish zero findings.
The optional CI workflow uploads the report and adds it to its run summary;
failure does not join or relax the required checks.

The isolated `CLIPPY_CONF_DIR` belongs only to the advisory command. No root lint
configuration, production lint attributes or existing Task commands change.

## Selected checks

| Concern | Checks | Treatment |
| --- | --- | --- |
| Crash surface | `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro` | Newly selected advisory warnings; review production first. |
| Numeric conversion | `cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss` | Newly selected advisory warnings; fix concrete range or precision risks first. |
| Maintainability | `too_many_lines`, `excessive_nesting`, `fn_params_excessive_bools` | Start at 100 function lines, nesting depth 4 and one boolean parameter. Tune from actual findings. |
| Async safety | `await_holding_lock`, `await_holding_refcell_ref` | Already enabled by default; explicit report coverage is not a new safety claim. |

IronBridge provides relevant prior art: its `src/lib.rs` selects crash-surface
checks and separates test exceptions. Its `clippy.toml` documents particular
configuration and process-exit boundaries. It also chooses cognitive complexity
over function length. MedTracker adopts the focused checks, rather than copying
all of IronBridge's pedantic/nursery rules or its exemptions. The inspected UniFi
release announcer manifest has no workspace lint policy to copy.

## First measured report

The sole build runner completed the actual workspace command successfully:
Cargo exit `0`, no invalid diagnostics, Rust `1.99.0`, revision
`910fb6154224450a43b306d413abd09601004fa9` with local changes. This is evidence
for that working-tree snapshot, not a claim about a later commit or hosted CI.

The report contains 1,333 deduplicated lint/location findings: 88 in production
paths and 1,245 in test paths. No generated-path findings were emitted. Some
production-path rows are inline unit tests, including Cedar matrix and identity
token tests; do not treat the 88 as a count of runtime defects.

| Lint | Production paths | Test paths | Total |
| --- | ---: | ---: | ---: |
| `cast_possible_truncation` | 4 | 0 | 4 |
| `cast_sign_loss` | 17 | 1 | 18 |
| `excessive_nesting` | 13 | 7 | 20 |
| `expect_used` | 17 | 11 | 28 |
| `fn_params_excessive_bools` | 2 | 0 | 2 |
| `panic` | 0 | 10 | 10 |
| `too_many_lines` | 22 | 3 | 25 |
| `unwrap_used` | 13 | 1,213 | 1,226 |

No selected async-guard, precision-loss, `todo`, `unimplemented` or `dbg_macro`
warnings were emitted. This does not prove the absence of every async or numeric
defect. Test unwraps dominate the report; removing them mechanically would slow
delivery without making request handling safer.

## First corrective priorities

1. **Saved HTTP status conversion.**
   `src/controllers/api/care/locations.rs:23` narrows a saved `i32` response
   status to `u16` before validating it. Normal writes store a valid `u16`, but a
   corrupted or incompatible saved row can wrap into a valid HTTP status.
   The location replay regression reproduced this for both a positive overflow
   and a negative saved value. Checked conversion now rejects both with HTTP 500,
   no replay header, no clinical effect and a failed-request audit. The focused
   test passes. This is a concrete defensive improvement, not a claim that normal
   requests currently corrupt saved statuses. The measurements above describe
   the earlier snapshot and have not been rerun after this repair.
2. **Make already checked numeric boundaries explicit.**
   The other truncation rows already have visible safeguards:
   `treatments/input.rs:311` bounds `max_daily_doses` to positive `i32` values;
   `medications/crud/validation.rs:336` follows validation of schedule types
   `0..=6`; dosage `storage_decimal` callers use the fixed exponents `3` and `8`.
   Prefer a checked conversion or a narrow documented invariant when their
   owner next edits these files. Pagination sign-loss rows validate positive
   pages and page sizes before conversion; retain their saturating offset
   behaviour. Do not change clinical values to convenient defaults to silence
   these warnings.
3. **Review runtime panic sites separately from test setup.**
   `treatments.rs:315–316` unwraps person and medication IDs after validation.
   A validated input type or error propagation could make that contract
   explicit during the treatment delivery. The request-ID/ETag header expects
   in `controllers/api/care/response.rs:169,174` and fixed midnight construction
   in `doses/timing.rs:59` require invariant review rather than blanket removal.
   JSON projection expects and test-only Cedar/identity rows are lower priority.

No exemptions, runtime fixes or gate promotions were introduced by this report.

## Measured hotspots

All three initial candidates were confirmed by `too_many_lines` in the measured
report. Physical line counts include blanks and differ from Clippy's function
count. No live feature-owned source was edited for this plan.

| Candidate | Responsibility to separate | Existing behaviour to preserve |
| --- | --- | --- |
| `src/models/care/treatments/projection.rs`, `source_context` (physical lines 37–245 in the inspected version; module 328 lines) | Extract batched current permission resolution and batched pause/actor projection into small helpers. Keep the calling transaction, household scope, database clock and shared medication/stock maps explicit. Assignment integration owns this file next, so its writer should combine any justified extraction with that change. | Schedule collection/read/update projection and ETag tests in `tests/care_api/schedule_lifecycle.rs`; current-grant and retirement exclusions in `schedule_scope.rs`; eligible stock and dose history journeys. Preserve batched queries and avoid fetching within row rendering. |
| `src/models/care/treatments/source_stock.rs`, `source_stock` (physical lines 6–123) | Separate eligibility filtering from result shaping after the existing batch query. Preserve child/adult dosing, location eligibility and quantities; keep numeric conversions checked rather than returning convenient defaults. | Treatment creation and schedule read/update tests; existing take/stock consistency journeys. A smaller function must not change eligible stock IDs or available amounts. |
| `src/models/care/doses/writing.rs`, `create_with_failure` (module 127 lines) | Consider separating mutation preparation from transaction writes. Keep replay lookup, row locking, current permission checks, stock changes and audit failure handling in one clear transaction sequence. Only extract if the measured finding or a feature change makes this useful. | `tests/care_doses.rs` and care API dose tests protect repeated requests, current access withdrawal, stock deduction and audit rollback. Preserve failed-request audit persistence and rollback of clinical effects. |

The identity implementation is being replaced with the selected maintained
library. Do not spend this delivery splitting obsolete authentication functions.
Use normal account and household journey tests while integrating that library.

## Promotion and verification

First review correctness and numeric findings with the owning feature writer.
Fix or narrowly justify a finding; a temporary exemption must name the concrete
invariant using `reason = ...`. Do not add blanket exceptions, convert failures
to defaults, or run `clippy --fix` across live source.

After an agreed lint's existing findings are fixed or justified, promote it at
the next normal integration checkpoint and run the existing relevant journeys.
Keep the initial scope small; this document is not a backlog of refactoring
issues. Module length alone does not justify extracting another abstraction.

Tooling verification: seven Node tests pass, including subprocess report success
and compiler-failure cases using fake Cargo; `actionlint` accepts the standalone
workflow. The genuine initial test failed before `report.mjs` existed. The sole
build runner then completed the actual application/migration report, as recorded
above. Hosted execution and any later required-gate promotion remain separate.
