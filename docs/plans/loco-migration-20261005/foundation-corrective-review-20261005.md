## Verdicts

**Requirements (this scope): NOT MET — one targeted respin needed.** The corrective patch genuinely resolves the prior in-scope defects: absolute `COMPOSE_FILE` entries and `rails:test:*` names in all three harness scripts (C1); `../rust/web`, `..` and `../docs/...` contexts/binds relative to the `rails/` project dir (C2); `MEDTRACKER_REPOSITORY_ROOT` + `Rails.root.parent`/`Taskfile.yml` spec fixes (C3); coverage job env + explicit `SIMPLECOV_SOURCE_ROOT` (C4); real ledger comparison wired into `task test` via `preservation:check`/`inventory:check` (I1/I2); runbook/testing command renames and restored dated PDF evidence (I3/I5); `/scripts/ci`, `/workspace/renovate.json`, `/app/.gitignore` container binds (I4); real scratch/buildx filter probes (F1). However, two source defects below keep this head CI-red on the bare-checkout lanes.

**Technical: still sound.** The dockerignore negation ladder is correct (parent dirs re-included before children), the synthetic-git preservation test is real RED coverage, and `withOwnedDatabase` correctly scrubs `COMPOSE_*`/`DATABASE_URL` and validates the endpoint.

## Defects

1. **Critical — `rails/.gitignore` is a zero-byte regular file** (manifest SHA `e3b0c44…`), but `rails/spec/config/rails_spec.rb` asserts `File.identical?(Rails.root/.gitignore, <repo>/.gitignore)` **and** that its lines include `/tmp/*`, `/rust/api/target/`. This passes only inside containers where `../.gitignore:/app/.gitignore:ro` over-mounts it; on the hosted `test_non_system` lane (bare checkout, `working-directory: rails`) and any host spec run it fails both assertions. **Remedy:** replace the empty file with a `../.gitignore` symlink — matching the existing `rails/docs`/`rails/scripts/ci` shared-file pattern — so host and container resolve identically.

2. **Critical — audit form selector can never match.** `rails/spec/system/mobile_ui_audit_spec.rb` (~L74) builds `action = household_path(:person_schedule_path, person_id:…, id:…)`, injecting `household_slug` into a route that does not accept it — the unchanged `schedules_spec.rb` uses two-arg `person_schedule_path(person, schedule)`, proving no slug param. Rails appends `?household_slug=…`, so `form[action='/people/…/schedules/…?household_slug=…']` won't match the rendered action → hosted `test_system` failure. **Remedy:** `person_schedule_path(people(:john), schedules(:john_paracetamol))` without `household_path`.

3. **Important — collator fallback still cwd-fragile.** `scripts/ci/collate_simplecov.rb` keeps `File.expand_path('../../rails', __dir__)`; invoked bare from `rails/` (e.g. `bundle exec ruby scripts/ci/collate_simplecov.rb` without env) `__dir__` lexically resolves under the `rails/scripts/ci` symlink → `rails/rails/.simplecov` → `load` raises. The CI env var masks this; the recommended realpath/toplevel resolution was not applied. Fails loudly, so acceptable if documented — otherwise switch to `git rev-parse --show-toplevel`.

4. **Minor — `.github/agents/documentation.yml`:** instructions now use `rails/app/…` but the `context:` block still globs unprefixed `app/models/**` etc. — the agent's scanned context silently finds nothing.

## Cannot-verify

- `isolation_test.fish` `$root` definition and `run.fish`'s other task calls (`contract_web_port`, `contract:prepare-db`, `api:contract-source-snapshot`) — the referenced Taskfile internals (`rust/contract-tests/Taskfile.yml`, root `compose.yaml` postgres service) are outside the supplied source.
- Ledger/`source-inventory.json`/`preservation.json` contents, `Cargo.lock`, secrets, unchanged renames; all execution evidence (bare `task ci` exit 0, scratch probes) is coordinator-supplied, not independently run.
- Broader doc/task-name sweep beyond the three corrected files; runtime/browser acceptance remains a separate pending gate per the brief.
