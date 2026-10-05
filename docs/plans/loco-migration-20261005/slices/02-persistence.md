# Schema adoption and tenant persistence Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Run Loco against the preserved MedTracker schema without losing data or Rails rollback.
**Architecture:** One Loco migration owner adopts the reconciled baseline; runtime roles cannot perform DDL.
SeaORM models and transaction-local RLS boundaries support shared authorised operations.
**Tech Stack:** PostgreSQL 18, SeaORM migrations/entities, Loco 1.2.0, isolated Rails reference database.
**Spec:** [implementation-spec.md](../implementation-spec.md), [persistence-brief.md](../persistence-brief.md).

## Global constraints

Foundation accepted first. Synthetic databases only. Additive rollback-compatible changes only.
Retain Rails ledger provenance. Credential byte preservation is not authentication proof.
Writer owns migration/model/tests; verifier owns database resources and generated entities.

## Review focus

Populated adoption preserves IDs/bytes (P2). Repeated adoption is a no-op (P2). Unexpected functions,
grants or RLS changes are rejected (P2). Reused pooled connections cannot leak tenants (P3).
Workers cannot gain owner privileges and Rails can still operate (P3).

### P1: Establish owned database acceptance and exact baseline inputs

**Files:** Create `scripts/migration/run-slice.mjs`, `scripts/migration/run-slice.test.mjs`,
`tests/persistence.rs`; modify `Taskfile.yml`; create `migration/baseline.sql` and
`docs/plans/loco-migration-20261005/schema-provenance.md` from inspected retained PR #2419 source.
**Interfaces:** Produce `slice:test TARGET FILTER` and `slice:test-runner` as defined in the specification;
`DATABASE_URL` always points at this invocation's PostgreSQL 18 fixture.

- [ ] Test `slice_runner_ignores_ambient_database`: set an unusable ambient URL; assert
  `owned_endpoint_used == true`, `foreign_resources_touched == 0`, `owned_resources_remaining == 0`.
- [ ] Register the test-only runner command and run `rtk task slice:test-runner`;
  record the new runner's failing ambient-endpoint regression before implementation.
- [ ] Implement owned test execution by reusing `foundation-database.mjs`; copy verified baseline SQL
  from retained ref, recording its exact path/hash and source migration count in provenance.
- [ ] Run `rtk task slice:test-runner` and `rtk task slice:test TARGET=persistence`;
  prove cleanup after success and forced failure.
- [ ] Review and commit `test(persistence): isolate schema adoption rehearsals`.

### P2: Adopt fresh and populated schemas, reject unknown drift

**Files:** Create `migration/src/m20261005_000001_adopt_medtracker.rs`,
`migration/catalog.sql`, `migration/allowed-differences.json`; modify `migration/src/lib.rs`;
test `tests/persistence.rs`; create synthetic records in `tests/fixtures/persistence.sql`.
**Interfaces:** Produce the standard `Migrator` baseline entry and schema verification result;
preserve original Rails `schema_migrations` and all stored record bytes.

- [ ] Add `fresh_catalog_matches_rails`, `populated_adoption_preserves_bytes`,
  `second_adoption_changes_nothing`, `unknown_catalog_drift_is_rejected`.
  Assertions: `normalised_catalog == rails_catalog`, `before_ids == after_ids`,
  `before_credential_bytes == after_credential_bytes`, `second_ddl_count == 0`,
  `unknown_drift_accepted == false`. Cover indexes, constraints, functions, views, extensions,
  triggers, policies and grants, not just table columns.
- [ ] Run `rtk task slice:test TARGET=persistence`; the empty current migrator must fail catalog parity.
- [ ] Implement guarded adoption using standard migrations; accept only explicitly named differences.
  Never run baseline DDL blindly against a populated database or erase its Rails ledger.
- [ ] Run all P2 cases on fresh and representative populated PostgreSQL 18; verify explicit drift diagnostics.
- [ ] Review and commit `feat(persistence): adopt the preserved MedTracker schema`.

### P3: Enforce runtime roles, tenant transactions and compatible fixtures

**Files:** Create `src/models.rs`, `src/models/access.rs`, `src/models/errors.rs`,
generated `src/models/_entities/`, `tests/tenant_access.rs`, `tests/fixtures/users.sql`;
modify `src/lib.rs`, `src/app.rs`, `config/test.yaml`, `Taskfile.yml` and migration role/queue provisioning.
**Interfaces:** Produce specification `Actor`, `HouseholdScope`, `OperationError`;
standard `App::seed` for synthetic fixtures; runtime roles provisioned before worker startup.

- [ ] Add tests `pool_reuse_does_not_leak_household`, `runtime_role_cannot_alter_schema`,
  `minor_and_dependent_lack_capacity`, `rails_rollback_preserves_operations`.
  Assert cross-household rows empty, DDL denied, enum values exactly `0/1/2`, capacities `false/false`,
  and preserved Rails reads/writes succeed on the adopted disposable database.
- [ ] Run `rtk task slice:test TARGET=tenant_access`; record absent role/context/fixture failures.
- [ ] Generate entities from the accepted schema; establish verified context with transaction-local settings,
  current membership checks and least privilege. Provision Loco queue storage with the migration role.
  Replace `db:migrate`/`db:seed` hard gates only after these isolated contracts pass.
- [ ] Run focused tests, `rtk task ci`, and the Rails rollback rehearsal under its explicit owned project.
- [ ] Review, commit and publish `feat(persistence): enforce tenant transactions and runtime roles`;
  report accepted schema/provenance and exact rollback evidence.

**Done:** Catalog parity, idempotency, drift rejection, populated preservation, least privilege,
pooled RLS isolation, fixtures and Rails rollback all pass on the reviewed published commit.
