# Independent Foundation Review — Tranche 1

**Reviewer:** Devin CLI `swe-2-max` (Hubble)
**Scope:** supplied frozen diff `838e79d..959be24` plus supplied planning/verification records only. No tools, workspace, or commands were used. All runtime/test results cited below are **supplied evidence** (verifier report and hosted CI run 37303106746), not independent execution. I did not verify omitted bytes: `Cargo.lock`, `source-inventory.json`, `preservation.json`, byte-identical renames, screenshots, and any unchanged files.

## Verdicts

**Requirements: NOT MET — return for fixes.** The architectural intent is genuinely delivered (standard Loco Hooks app, no Axum wrapper, namespaced Rails ownership, safe DB config, preservation machinery), but the published head is CI-red and several path contracts are provably broken in the supplied source. Tranche acceptance is blocked pending fixes and re-review.

**Technical quality: sound skeleton with specific defects.** Ownership/isolation patterns (owned compose projects, ephemeral ports, fixture owner markers, bounded TERM→KILL group cleanup, gated tasks, generated lint exclusions) are well-designed. Defects cluster around path resolution after relocation and verification that exists but isn't wired in.

## Requirements resolution

| Requirement | Result |
|---|---|
| Real Loco boot/routing/Tera, no Axum wrapper | **Met** — `src/app.rs` uses `create_app::<Self, Migrator>` + `AppRoutes::with_default_routes()`; no path dep on `rust/` (workspace `members=["migration"]`, `exclude = ["rust","client-tools"]`); root tasks never call legacy crates. |
| Independent Rails relocation | **Structurally met** — `includes.rails` with `dir: rails`, no flatten; `task --dir rails` asserted; runtime green only per supplied evidence. |
| Namespaced Rails commands | **Mostly met** — docs/hooks/CI updated; misses remain in `docs/operations/hosted-private-beta-runbook.md` and `docs/testing.md` (I3). |
| CI/contract-runner path correctness | **Not met** — C1–C4 plus hosted-CI-confirmed failures. |
| Safe DB configuration | **Met** — `auto_migrate/dangerously_truncate/dangerously_recreate` all `false` in all three envs; `db:migrate`/`db:seed`/`release-image`/`Hooks::truncate`/`seed` all fail explicitly; PG 18 everywhere. |
| Readiness vs liveness | **Met** — `/up` process liveness, `/_health` framework DB readiness, `/health` alias; test asserts each separately. |
| Inventory/PR preservation | **Partially met** — mechanisms exist and ledgers are recorded, but `preservation:check` never compares against the ledger (I1) and none of the drift checks run in CI (I2). |
| Queue feasibility | **Appropriate for tranche** — official Loco 1.2 `BackgroundQueue` + `kind: Postgres` enabled; reliability contracts explicitly deferred to tranche 5. |

## Critical findings

**C1 — Contract harness COMPOSE_FILE resolves to `rails/rails/compose.yaml` for every `rails:` task call.** Confirmed by hosted CI.
- `rust/contract-tests/run.fish`: `set -gx COMPOSE_FILE rails/compose.yaml:rust/contract-tests/storage.compose.yaml`, then `rtk task rails:test:server`, `rtk task --force rails:test:exec`, and `contract_web_port`'s `rtk task rails:test:port`. All `rails/tasks/*.yml` and `rails/tasks/internal.yml` now run with `dir:` resolving to `rails/`; `docker compose` resolves relative `COMPOSE_FILE` entries from that cwd → `rails/rails/compose.yaml`. Same defect in `rust/contract-tests/isolation_test.fish` (`rails/compose.yaml:rust/contract-tests/runner-subnet.compose.yaml`) and `rust/contract-tests/browser_context_test.fish`.
- `isolation_test.fish` additionally calls unprefixed `test:port`, `test:server`, `test:exec`, which no longer exist at root (the `test` task is now Loco's cargo test) — task-name error regardless of COMPOSE_FILE.
- **Remedy:** export `COMPOSE_FILE` with absolute entries (`(pwd)/rails/compose.yaml:...`) in all three scripts, and rename the unprefixed calls to `rails:test:*`.

**C2 — `rails-browser-tests` build context resolves into a nonexistent directory.** `rust/contract-tests/runner.compose.yaml` `rails-browser-tests` has `build: context: ./rust/web`. With `COMPOSE_FILE=rails/compose.yaml:...`, the project directory becomes `rails/` (first file's parent), so the context resolves to `rails/rust/web`. `api:browser-rails` / `browser-journey-rails` cannot build; `browser_context_test.fish`'s `$workspace/rust/web` assertion fails on the same root cause (inspection-verified; `contract_compose_regression.mjs` can't catch it — `config --quiet` doesn't check context existence). The `./docs/screenshots/*` binds work only via the `rails/docs` symlink. **Remedy:** `context: ../rust/web` (or env-driven absolute path); prefer `../docs/...` for the binds to remove the symlink dependency.

**C3 — Relocated specs read repo-root files through `Rails.root`.** Confirmed by hosted CI: `rails/spec/config/json_spec.rb` reads the shared `renovate.json` (still at repo root; no `rails/renovate.json` or symlink exists); `rails/spec/lib/schema_inventory_spec.rb` reads `Rails.root.join('rails/Taskfile.yml')` → `rails/rails/Taskfile.yml`. **Remedy:** resolve shared root files via `Rails.root.parent` (or an explicit symlink), fix the Taskfile path, and sweep all specs for `Rails.root.join` targets that stayed at root (`renovate.json`, `rust/`, `client-tools/`, `mobile/`, `Taskfiles/`) — only `docs/` and `scripts/ci/` have symlinks.

**C4 — Shared CI coverage helpers break under `working-directory: rails`.** In `.github/workflows/ci.yml` `coverage`, steps run `bundle exec ruby scripts/ci/tests/coverage.rb` and `bundle exec ruby scripts/ci/collate_simplecov.rb ...` from `rails/`. `__dir__` then lexically resolves under `rails/scripts/ci/` (symlink not realpath'd), so `coverage.rb`'s `repository = File.expand_path('../../..', __dir__)` → `rails/` → spawned `BUNDLE_GEMFILE=<repo>/rails/rails/Gemfile` (missing → subprocess fails), and `collate_simplecov.rb`'s fallback `File.expand_path('../../rails', __dir__)` → `rails/rails/.simplecov` (missing → `load` raises). The job exports no `RAILS_APPLICATION_ROOT`; `ci:coverage:test` only went green because the task sets `RAILS_APPLICATION_ROOT=/app` explicitly. **Remedy:** add `RAILS_APPLICATION_ROOT: ${{ github.workspace }}/rails` to the `coverage` job env, and prefer `git rev-parse --show-toplevel`/realpath over `__dir__` arithmetic in both scripts so invocation cwd can't corrupt root resolution.

## Important findings

**I1 — `preservation:check` never compares to the stored ledger.** `scripts/migration/preservation.mjs` only writes on `--write`; in check mode it re-asserts existence/mode/comment-line invariants and prints a summary. The recorded `original_sha256`/`current_sha256` are recomputed but compared to nothing — a byte-level drift touching no comment line or mode passes silently. This undercuts the "machine-checkable preservation" acceptance claim. **Remedy:** in non-write mode load `preservation.json` and `assert.deepEqual` recomputed entries (paths, hashes, modes, destinations), mirroring `inventory.mjs`'s approach.

**I2 — Declared protections are dead tasks.** `Taskfile.yml` `test` runs `cargo test` + `foundation:test` + `foundation:test-cleanup` + `foundation:test-http`; `ci` is `fmt`+`lint`+`test`. `foundation:test-workspaces`, `inventory:check`, and `preservation:check` are invoked by no task chain and no workflow job. (`foundation:test-ci-paths` is covered indirectly via `ci:test`'s `scripts/ci/tests/*.test.mjs` glob in the policy job.) A commit that mutates hashed migration inputs without regenerating ledgers stays green. **Remedy:** add the three tasks to `task test` (uniform local/CI enforcement).

**I3 — Stale unprefixed task names in active docs.** `docs/operations/hosted-private-beta-runbook.md` still instructs `task household-lifecycle:export|download|hold|release-hold|offboard|purge` and `task hosted-restore:rehearse`; all are now `rails:`-only — an operator-run book whose commands fail verbatim. `docs/testing.md` still shows `task mutation` / `task mutation:since` (→ `task rails:mutation*`).

**I4 — In-container symlink gap at `/app/scripts/ci`.** `.:/app` (now `rails/`) makes `/app/scripts/ci` a symlink to `../../scripts/ci` → resolves to `/scripts/ci`, which does not exist in the container. The updated `ci:coverage:*` tasks correctly use the canonical `/workspace/scripts/ci` bind, but any other in-container reader of `/app/scripts/ci` (specs, helpers, runbooks) breaks silently. **Remedy:** confirm no in-container consumer uses `scripts/ci` under `Rails.root`/cwd, or change the symlink/mount so the container path resolves (e.g., mount at `/scripts/ci` or absolute `../../workspace` equivalent). (`/app/docs` works because its bind resolves through the `rails/docs` symlink; `/app/scripts/ci` has no matching mount.)

**I5 — Confirmed pending defects (hosted run 37303106746), restated for the fix list:** `docs/plans/loco-migration-20261005/plan.md` ~L100-101 lines beginning `#2389`/`#2402` trigger MD018 (indent or reflow the list continuation lines); `include_str!` path updates in `rust/api/src/external_integrations/{ai.rs,facts.rs}` and `rust/web/src/household_i18n.rs` need `rustfmt`; a Rails mobile audit shows an exception page on a schedule-edit route — controller and test are byte-identical to base, so attribute it to environment/paths only after capturing the actual exception (likely the same class as C3/C4 or I4).

## Observations (minor / carry-forward)

- `config/production.yaml` is identical to dev/test: loopback `binding`/`host`, default `medtracker_password`, and dev-scale pool/timeouts. Acceptable only because `task release-image` hard-fails; must be fixed before tranche 6 — recommend recording it explicitly in issue #2450's gate list.
- `task dev`/`worker`/`routes` use `cargo run` without `--locked` while build/check/lint/test use `--locked`.
- `loco_foundation` is the only job without `timeout-minutes`.
- Lefthook `rubocop` now lints the whole repo (was `{push_files}`/`{all_files}`) and `gems audit` moved from host `bundle audit` to a Docker `tools-test` run — heavier hooks; verify intended.
- `inventory.mjs` `roots` omit the contract harness itself (`rust/contract-tests/*.fish`, compose fragments, `test_support/`, `fixtures/`) and `rails/{bin,scripts,tasks}` — preservation covers them, but inventory drift scope should be deliberate.
- Report accuracy: "comparing symlink target bytes" applies only to base-revision symlinks; the two new symlinks are actually verified by `foundation.test.mjs` readlink assertions (good coverage, slightly overstated claim). New symlink targets (`rails/docs`, `rails/scripts/ci`) aren't in the ledger's entries since they post-date the pinned revision.
- `docs/pdf-renderer-compatibility.md` rewrote a historical command to `task rails:test`, contradicting the stated "dated reports retain original commands" policy.
- `AGENTS.md`/`agents.md` are byte-identical (same blob `f145e79b`) — case-insensitive-FS requirement **satisfied**; suggest adding a trivial equality assertion to `foundation.test.mjs` so they can't silently diverge.
- `foundation-http.test.mjs`'s ephemeral-port reservation has a small TOCTOU window and a 60s bound that could flake on cold caches if run standalone (CI order compiles the bin first via `cargo test`, so it's fine as ordered).
- `docker-publish` `workflow_dispatch` on pre-migration tags will fail (`rails/` context doesn't exist at those refs); acceptable, worth a comment.
- `hooks`: `rspec` job runs full `task rails:test` on any `rails/**/*.rb` push — matches prior behavior, noted for cost awareness.

## Cannot-verify limitations (needs sweep or evidence)

- `rust/contract-tests/Taskfile.yml` and `Taskfiles/contract.yml` are unchanged and not supplied — any `task:`/`docker compose`/`internal:*`/`test:*` references that previously resolved via the removed `legacy-rails` flatten are now unresolved. Given `run.fish` needed the same renames, audit this file.
- `classify.mjs`/`gate.mjs` internals for the new `loco` output (partially covered by `loco_paths.test.mjs`; gate presumably keys off `policy.jobs`).
- `rails/Dockerfile` and `rails/.dockerignore` under the new `rails/` build context; `docker-publish` image correctness (the prod-image CI job exercises it — result not supplied).
- `renovate.json`, `release-please-config.json`, `.release-please-manifest.json`, other workflows under `.github/`, `openspec/`, `.github/agents/` — path-scoped rules/commands may assume the old root layout (the confirmed `json_spec` failure corroborates drift here).
- Unchanged docs outside the diff (`docs/operations/production-environment.md`, Kubernetes runbooks, compliance/ADR pages) — recommend a scripted sweep for unprefixed `task (dev|test|prod|local|lighthouse|audit|household-lifecycle|hosted-restore|mutation|rubocop|brakeman|playwright|stop-all)` references.
- `.kamal` relocation: the supplied diff shows `deleted` for `.kamal/secrets`; renames were omitted, so the destination is unverifiable — `preservation.mjs`'s `existsSync(rails/.kamal/...)` would fail if absent, so it is *probably* moved, but confirm.
- Worker graceful shutdown: report itself marks SIGINT propagation unverified (correctly not claimed).
- Recorded ledger SHA-256s and verifier GREEN tallies (117 examples, 82 CI checks, 1,893 RuboCop files, HTTP route evidence, screenshots) — supplied evidence only.
- `Cargo.lock` and generated JSON bodies: omitted per diff policy; not inspected.

## Suggested fix order for Nightingale

1. C1+C2 (contract harness compose paths — unblocks the rust_port/browser lanes), 2. C3+C4+I4 (Rails suite/coverage/container symlink path sweep), 3. I5 (MD018, rustfmt, audit-exception triage), 4. I1+I2 (make the ledgers actually enforce in CI), 5. I3 + doc sweep, 6. observations.

Re-review of the same writer's fixes is required before the foundation tranche can be accepted; later tranches are explicitly out of scope and correctly not claimed complete.


## Nightingale response dispositions (verification in progress)

The reviewer text above is retained exactly. This response distinguishes supplied
review observations from execution by the exclusive verifier; no acceptance is
claimed.

| Finding | Writer disposition and evidence |
| --- | --- |
| C1 | Confirmed by hosted double-prefix failure; absolute Compose paths and Rails task names corrected in all three harness scripts. Focused context/Compose checks passed; full root and standalone runner verification remains pending. |
| C2 | Runner browser contexts/default API context and screenshot binds corrected for the Rails first-file directory. Existing executable context test is the regression gate. |
| C3 | Both focused specs reproduced ENOENT. Shared-root policy uses explicit container repository location with host parent fallback; Rails Taskfile lookup uses its actual root. |
| C4 | Lexical symlink claim disputed: Ruby 4.0.7 host execution canonicalized the helper and loaded the correct Rails .simplecov. A separate synthetic coverage-root failure produced zero files and missing API group. Explicit source root fixed that failure; host helper, container helper and focused helper lint passed without lowering thresholds. Workflow app-root env clarifies ownership. |
| I1 | Intended RED from a real small isolated Git fixture: baseline succeeded, byte-only drift was accepted. Checker now compares full recomputed ledger to stored ledger; synthetic negative test passed (1/1). Original large-fixture timeout was not counted as RED. |
| I2 | Dry-run CI RED omitted workspace/drift guards. Root test now executes workspace, negative drift and relocation tests plus both real ledger checks; executable dry-run confirmed wiring. |
| I3 | Active lifecycle/restore/mutation instructions corrected; exact command assertions retained. Dated PDF evidence restored byte-for-byte from the baseline. |
| I4 | Disposable-container RED independently proved missing symlink target and shared policy. Canonical CI has a read-only /scripts/ci alias to resolve the Rails symlink; policy is mounted at /workspace/renovate.json. A later scanner contract proved /app/.gitignore absent; bind-mounted Rails services now use the canonical root ignore file read-only. No copied authorities. |
| I5 | Coordinator fixed Markdown MD018. Legacy Rust format output is verifier-owned. Exact current top-level edit request proved ParameterMissing person_id/HTTP400; actual cards use nested edit and no top-level show action exists. Audit follows supported nested edit with form evidence. Controller/routes remain unchanged; no executed-baseline proof or CSS workaround is claimed. |

Additional observations: root CLI Cargo commands now use --locked; the Loco job
has a 20-minute timeout. Release metadata now references the existing Rails
version file. Inventory scope includes the entire contract harness and Rails
bin/scripts/tasks, with source-only generated/ignored filtering; the baseline
preservation ledger covers base symlinks, while new shared symlinks have explicit
ownership tests. Agent guides remain byte-identical. Existing full-suite hooks
remain enforced. Production config, graceful worker shutdown, two-architecture
release and feature migration remain later gates. Historical tag dispatch does
not support refs lacking rails/; no production-image acceptance is claimed.

The credentials attributes now match Rails paths, and setup documents the
official relocated CLI driver. A synthetic Git fixture verifies entrypoint
dispatch without any real credential/key access; the coordinator refreshed only the local Git driver setting and read it back,
without invoking real credential diffing. Re-review is required after final verification.

### Cannot-verify sweep disposition

The actual contract namespace comes from rust/contract-tests/Taskfile.yml;
Taskfiles/contract.yml does not exist. Its internal helper references retain the
existing namespace include and are part of runtime verification. Gate evaluation
iterates policy.jobs, so the declared Loco output is required; existing CI tests
cover selection/gating. Rails Docker context inputs remain inside rails/; hosted
production-image medication-search passed on the reviewed head, which does not
prove two-architecture release acceptance. Renovate remains canonical at root;
release-please version source is corrected under rails/. Current Copilot,
OpenSpec and documentation-agent guides reflect actual ownership. Specific active
operations/testing commands were swept; dated evidence was preserved.

The original preservation ledger verifies tracked .kamal destination existence,
modes and hashes without publishing secret content. The lockfile and generated
ledgers retain verifier identities and checks; the reviewer did not inspect
omitted bytes, and these execution checks are not called independent review.
New migration outputs are not historical parity inputs: root product compilation
and observable route tests cover them. Inventory hashes actual tracked/new
nonignored input source and assets, including retained ignored tracked assets,
while filtering generated build/cache directories.

F1 runtime disposition: a real Docker scratch export reproduced the missing
relocated YAML/locales/font licence inputs. The Dockerfile already consumed the
correct Rails paths; only its owning filter required correction. A second real
Docker test used dummy credential/environment/unrelated markers and caught broad
parent negations before the real repository probe. Narrow descendant exclusions
now pass that boundary test, the byte-identical authoritative asset export,
Compose context checks and `ci:check`. Full root/standalone runner gates and a new
independent verdict remain required. The current 29-file runtime-delta manifest
SHA-256 is `7d1f3ae6adb9c68b7e3ec3a980c66b8699bdcf603cbbc610601d1d35bfbb959a`.

Coordinator metadata proof for .kamal relocation: .kamal/secrets maps to
rails/.kamal/secrets, mode 100644 preserved, unchanged=true, before/after
SHA-256 `4908cf19e3deaa7930fa4253302ec47b37cd5480e7efd3515619bd375288b883`.
The tracked destination blob is `9a771a398563109137a72c2dfc10a10b99d0a745`.
No contents were opened for this assessment; secret/credential paths remain
omitted from reviewer packets. Final real-ledger execution remains required.
