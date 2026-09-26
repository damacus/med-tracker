# Required dosage field schema remediation

The six required `DosageOption` fields now have PostgreSQL NOT NULL constraints in `db/migrate/20260926000000_enforce_required_dosage_fields.rb` and `db/schema.rb`: amount, unit, frequency, default_max_daily_doses, default_min_hours_between_doses and default_dose_cycle. The Rust SeaORM dosage entity uses non-optional types for the same fields. Request validation still rejects missing or invalid values with 422. Inventory fields remain nullable.

The migration supplies no default, backfill or deletion. PostgreSQL checks every row when each constraint is applied, independently of household row visibility. A pre-existing NULL makes the migration fail with a column-specific `ActiveRecord::MigrationError`; Rails and PostgreSQL roll the DDL back. Operators must repair such data deliberately before retrying. Apply this migration before deploying the Rust binary with non-optional dosage fields. No production migration was run.

## Evidence

- RED: elevated `task test:preflight TEST_FILE=spec/migrations/enforce_required_dosage_fields_spec.rb` reached Docker and the focused spec; initial matcher setup was corrected. Focused `task test TEST_FILE=spec/migrations/enforce_required_dosage_fields_spec.rb` then failed 2/2 on nullable `amount` and the absent migration, log `/Users/damacus/Library/Application Support/rtk/tee/1790399585_task_tes_63031d.log`.
- GREEN: the focused Rails migration spec passed 2/2, log `/Users/damacus/Library/Application Support/rtk/tee/1790399711_task_tes_63031d.log`. It verifies all six constraints, PostgreSQL NULL rejection, intact valid rows, column-specific migration failure for each injected NULL, no replacement data, and rollback of earlier constraints.
- GREEN: isolated OpenAPI dosage acceptance passed 13/13 in project `mtcontract-3bf970a8e12e4871`, log `/Users/damacus/Library/Application Support/rtk/tee/1790400186_task_api_d3a3cd.log`. The former artificial legacy-row repair case now asserts SQLSTATE 23502 for all six columns and an unchanged HTTP response. The project was removed.
- Rust `api:check`, `api:clippy`, `api:test` (15/15) and the selected dosage test compile passed.
- Elevated full Rails `task test` passed 6,104 examples with zero failures in 12 minutes 43 seconds, log `/Users/damacus/Library/Application Support/rtk/tee/1790400701_task_test.log`.
- RuboCop initially found style offenses in the new migration spec. After test-only helper refactors, full `task rubocop` passed 1,893 files with no offenses, log `/Users/damacus/Library/Application Support/rtk/tee/1790401011_task_rubocop.log`. The focused Rails spec passed 2/2 again after the final refactor, log `/Users/damacus/Library/Application Support/rtk/tee/1790401032_task_tes_63031d.log`. The full suite was run before these test-only refactors.
- API and dosage contract-test formatting, `docs:build`, JSON validation, the 118-operation inventory rebuild and `git diff --check` passed.

No production data, migration or deployment action was taken.
