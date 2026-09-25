# Task 6D report: portable household export and import

## Status

DONE_WITH_CONCERNS. The target-independent HTTP contract passes against isolated Rails and is expected red against the absent Rust target.

Portable v1 excludes accounts, grants, and reviews. Task 6D therefore cannot prove full account and medical-data preservation. A disposable PostgreSQL migration and rollback rehearsal, plus Rust integration tests, must prove those invariants before migration is considered safe.

## Implemented

- Added `rust/contract-tests/tests/portability.rs` for encrypted export, dry run, apply, replay, public snapshot read-back, source preservation, audit redaction, and denied requests.
- Added dedicated disposable PostgreSQL 18 source and target households to the contract provisioner. The source has an owner account and a separate patient record with a manage grant, location, medications, dosage, schedule, direct assignment, schedule-backed and assignment-backed takes, notification preference, and review prompt. The target has owner, member, revoked and locked actors. Three fixture bundles carry malformed collection, embedded Rails numeric ID, and target name conflict cases. The source and target remain inside the per-run disposable database.
- Added Task-wrapped focused Rails and Rust runners and included portability in the full contract command. Updated the three operation rows in `rust/parity-matrix.md`.

## HTTP evidence and limits

- The exported envelope declares `medtracker.portable.encrypted.v1`, AES-256-GCM, PBKDF2-SHA256, a salt, checksum, and ciphertext. The response hides the source patient's name and passphrase. The exported opaque bundle dry-runs and imports into another household, providing round-trip evidence of the passphrase and content. The test does not independently decrypt ciphertext or verify cryptographic primitives; format and cryptography need Rust domain/integration coverage.
- Dry run returns `applied: false`, counts matching the public source snapshot, and no conflicts or errors. The public target clinical snapshot, grants, and import audits remain unchanged. Apply returns 201; a current mobile OAuth bearer reads back exact source portable IDs, clinical fields, and every relationship represented in the eight portable v1 collections, including person locations, medication dosage, schedule and assignment sources, both take source types, take inventory, and notification preference person. The source snapshot, reviews, and grants remain unchanged.
- Wrong passphrase, malformed collection, embedded numeric ID, and name conflict return structured 422 responses. After **each** failed request, the contract compares the target's public clinical snapshot, grants, and import audits with the same baseline and checks that no import event bears that request ID. Missing passphrase headers, a query-only export passphrase, unauthenticated export, foreign household, revoked account, locked account, and inadequate role are rejected. The denied requests leave the target clinical snapshot unchanged.
- Both successful export and apply audits are matched to the response request ID in the authorized household-scoped audit API, with actor account/membership, record counts, and passphrase redaction asserted. The public audit payload omits `household_id`; household scope is observable through the authorized route, not a response field. The public state comparisons do not prove physical rollback of every database table.
- Successful import creates target grants and invalidates the target owner's prior API session and app token through permissions versioning. A normal mobile OAuth bearer remains valid for post-import read-back. The contract asserts the old session's 401.
- Rails accepts an exact repeat of the same bundle as another 201 while preserving record counts and portable IDs. It does not reject a duplicate import. A distinct conflicting bundle returns 422. This is a source disagreement with the brief's “duplicate/conflicting import” failure wording; the HTTP contract records current Rails behaviour rather than inventing a rejection.
- The API export defaults to `single_person` portable v1. It does not include accounts, person access grants, or medication review prompts as portable record collections. The fixture and source read-back cover their presence and preservation, but migration of those records is outside this HTTP format. Task 6D cannot prove full account and medical-data preservation. A disposable PostgreSQL migration and rollback rehearsal and Rust integration tests must cover those records, row-level transaction rollback, physical table counts, cryptographic derivation, and deeper referential invariants. The HTTP test observes clinical snapshots and audit entries, not every database table.

## TDD evidence

- RED: `rtk task contract:portability-rails` first failed during fixture provisioning because a new dosage option omitted required dose defaults. After that correction, the HTTP dry run failed on a location-name conflict from source/target defaults. After making source names distinct, the post-apply snapshot failed with 401 because import invalidated the old bearer. These failures identified fixture and authorization assumptions before the final contract was settled.
- GREEN: `rtk task contract:portability-rails` passed both tests after the fixture and post-import bearer corrections. It passed again after source review/grant, audit, structured-error, and passphrase-header assertions were added.
- Review RED: full source/target field comparison first exposed a noncanonical fixture: the source medication combined legacy dose fields with dosage options, while the importer preserves the dosage option; nil schedule and assignment cycles defaulted to daily. An initial audit assertion also exposed that the public audit payload has no `household_id` field.
- Review GREEN: made the fixture canonical by storing dose in its dosage option and explicit daily cycles, and used the household-scoped audit endpoint. The expanded focused contract passed 2/2, including exact public read-back and request-correlated audits.

## Verification

- `rtk task contract:portability-rails`: 2 passed, 0 failed.
- `rtk task contract:rails`: 17 suites, 143 passed, 11 ignored, 0 failed after the review fixes.
- `rtk task test TEST_FILE=spec/requests/api/v1/portable_data_spec.rb`: 14 examples, 0 failures.
- `rtk task contract:portability-rust`: expected red, both tests failed to connect to absent `127.0.0.1:39999`; not a Rails failure.
- `rtk task contract:fmt`, `rtk task contract:clippy`, `rtk task rubocop` (1,886 files, no offences), `rtk task docs:build` (no issues), and `rtk git diff --check`: passed. The docs build used `UV_CACHE_DIR=/private/tmp/uv-medtracker-contract-docs` because the default cache path is outside the sandbox.

## Files changed

- `rust/contract-tests/tests/portability.rs`
- `rust/contract-tests/src/lib.rs`
- `scripts/contract_provision.rb`
- `Taskfiles/contract.yml`
- `rust/contract-tests/run.fish`
- `rust/parity-matrix.md`
- `.superpowers/sdd/api-tests-plan/task-6d-report.md`

## Self-review

The tests make only HTTP calls to the target; fixture setup is confined to the disposable Rails test database. All write calls retain the contract client's loopback guard. No product, schema, native, or OpenAPI file changed. No source comments were added or removed. The whole-file contract is long because it follows one encrypted bundle through export, dry run, rejection, apply, and read-back; separate tests cover authority without relying on the apply test's order.
