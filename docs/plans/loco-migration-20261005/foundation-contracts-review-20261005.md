# Corrective Re-review — Scope F1/F2 (packet 1 of 2)

**Requirements verdict (this scope): MET at source level.** F1: `Dockerfile.dockerignore` now admits exactly the relocated `rails/config/{ai_medication_sources,nhs_dmd_curated_products}.yml`, `rails/config/locales/**`, `rails/vendor/fonts/{NotoSans-Regular.ttf,OFL-1.1.txt}` while `rails/**` excludes credentials, `.env`, and unrelated app files — the pattern ordering (`!rails/` → `rails/**` → narrow re-includes) is correct Docker semantics, and `browser_context_test.fish` adds both a synthetic-fixture negative boundary and a real-context byte-parity probe. F2: all three harness scripts use absolute `COMPOSE_FILE` entries, `rails:test:*` task names, generated `mtcontract-<hex>` project scoping, owner-marker-gated cleanup, and project-scoped `down --volumes --remove-orphans --rmi local` — foreign volumes cannot be touched. Prior C1/C2/F1 defects are resolved. Runtime/hosted-CI acceptance remains pending per supplied evidence and is outside this verdict.

**Technical verdict: sound, one Important regression.**

## Findings

1. **Important — `run.fish` reintroduces the screenshot-bind symlink dependency C2 was meant to remove.** The compose defaults were corrected to `../docs/screenshots/...` (resolved against project dir `rails/` → `<root>/docs`), but `run.fish` overrides them with `set -gx CONTRACT_RUST_BROWSER_SCREENSHOT_DIR ./docs/screenshots/dashboard-rust` and `./docs/screenshots/journey-medication-rust`. Relative bind sources resolve against the project directory (`rails/`), so these only work through the committed `rails/docs` symlink — precisely the fragility flagged before. Remedy: emit `"$workspace/docs/screenshots/..."` (absolute) in both branches. Evidence: `run.fish` `browser-dashboard-rust`/`browser-journey-rust` branches; `runner.compose.yaml` `rust-browser-tests` volume.

2. **Minor — tautological assertion.** `browser_context_test.fish` `cmp -s` compares `Dockerfile.dockerignore` to its own copy; it can never fail. Harmless but dead verification — compare against a distinct expected file or drop it.

## Cannot-verify (material limits)

- **Helper scripts not in packet:** `cleanup.fish`, `restart_web.fish`, `cleanup_status_test.fish`, `runner_isolation_test.fish`, `runner_failure_test.fish`, `browser_minor_wrapper.fish`, `remove_image.fish`, `validate_subnet.fish`, `run_*.fish` variants are referenced by the corrected harness and `contract:isolation` but not supplied. Any residual relative `COMPOSE_FILE` or unprefixed `test:*` call inside them would reproduce C1; inventory now covers them, but source wasn't in this packet.
- **`foundation-database.mjs` / `withOwnedDatabase`:** in manifest, source withheld — disposable DB provisioning, teardown, and failure-path ownership in `foundation-http.test.mjs` are unverifiable. Timeout 90→210s is consistent.
- **`contract-minor-viewer-sql` / `contract-browser-node`** lack `dir:` and build paths from `CONTRACT_API_BUILD_CONTEXT | default "."`; correctness depends on callers always exporting the absolute context (run.fish does; the minor wrapper's internals are unsupplied).
- Ledger bytes, `.kamal` destination blob, generated JSON, and unchanged renames were excluded per packet policy.

No other actionable defects in the supplied source. Root `task test` now executes workspaces/relocation/preservation/database tests plus both ledger checks (I2 resolved); `preservation.mjs` non-write mode deep-compares the stored ledger (I1 resolved); `RAILS_APPLICATION_ROOT`/`MEDTRACKER_REPOSITORY_ROOT` wiring resolves C3/C4 (noting `__dir__` was already realpath'd — my earlier mechanism claim was incorrect).
