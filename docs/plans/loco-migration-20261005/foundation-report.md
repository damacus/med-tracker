# Foundation implementation report

Current status, 6 October: the original foundation runtime, review and publication
gates are complete on published `4423ecf9756869dd7993f733823af4ce6902af61`.
The acceptance record is published in `bda18ac97870802085380da20095bde22a26fc3e`.
[Hosted CI 37414918830](https://github.com/damacus/med-tracker/actions/runs/37414918830)
also passes on that exact commit. Foundation is accepted; complete application
and release acceptance remain separate.

Both complete root and standalone dashboard runners recorded 35 passing tests
on the same captured source and cleaned their owned resources, as detailed below.
The corrective source review returned requirements MET. Final local Loco CI
passed all Rust checks and 58 desktop/mobile cases.
[Hosted CI 37409443565](https://github.com/damacus/med-tracker/actions/runs/37409443565)
completed successfully on that exact published commit, including the rollback
and retained Rust checks. This closes the earlier failed published-head gate.

Fresh `task migration:audit` passes (87262): runtime ownership, both helper
namespaces, migration workspaces, relocation and preservation tests, source
inventory and preservation checks. All 2,933 original inputs remain present;
existing comments and modes are preserved. Only four stale current-file hashes
needed reconciliation, for the already published CI workflow, root Tasks, CI
classifier and merged Rails dependency lockfile. Original hashes, source revision
and relocation history remain intact. Historical audits remain outside normal CI.

The sections below retain earlier evidence in chronological order. In particular,
the original [source review](foundation-review-20261005.md) returned requirements
NOT MET; the later [corrective review](foundation-final-review-20261005.md)
returned MET for the reviewed source. Neither review independently ran tests.
Earlier failures and pending checks must be read alongside the newer evidence.

The coordinator pushed foundation commit
`f54630d41e1388a5fd552d9de385ea78995b7653` on `codex/loco-migration` and opened
[draft PR 2451](https://github.com/damacus/med-tracker/pull/2451). Remaining work is
tracked in [issue 2450](https://github.com/damacus/med-tracker/issues/2450).
Publication is a verified foundation waypoint; fixes and independent re-review
remain required.

## Implemented shape

The root package is a standard Loco 1.2.0 application, pinned in Cargo.toml and
resolved in Cargo.lock. `src/app.rs` implements framework Hooks, standard
AppRoutes and the official Tera initializer pattern. It does not call or wrap
the legacy Axum router. `/up` retains the existing health URL as process liveness;
`/health` is an alias. `/_health` is framework database readiness. `/` renders a
real Tera template. Product API and browser operations are not migrated yet.

All three Loco environments disable auto migration, truncation and schema
recreation. The empty migration ledger is a boot prerequisite only, not schema
adoption. Root migrate/seed/release-image tasks fail with explicit tranche gates.
The PostgreSQL 18 Compose resource uses dynamically published loopback ports and
requires an explicit owned project for foundation lifecycle tasks.

Loco's standard PostgreSQL BackgroundQueue is enabled. Framework queue startup
may create the queue table in the disposable test database; product-role DDL and
least-privilege provisioning remain an adoption gate. No live database has been
used for foundation verification.

Rails app/config/db/bin/lib/public/spec/test/data/vendor/compose, manifests,
runtime configs and scripts moved under `rails/`. Its Taskfile remains usable
both through root `rails:` and standalone `task --dir rails`. Shared docs and
OpenAPI remain authoritative at root; Rails uses an explicit symlink and Compose
read-only mount. Shared Node CI implementation remains canonical at root, with
a Rails symlink for host tooling. Container coverage helpers use an explicit
`RAILS_APPLICATION_ROOT=/app` and canonical `/workspace/scripts/ci` bind. A
read-only `/scripts/ci` alias resolves the existing Rails symlink, and shared
Renovate policy is mounted at `/workspace/renovate.json` with explicit repository
root configuration for mounted development/test/tooling services.
The same bind-mounted services expose the canonical root `.gitignore` read-only
at `/app/.gitignore` and `/workspace/.gitignore`, restoring generated-source
exclusions without maintaining a second ignore authority.

Root commands belong to Loco. Legacy Rust crates and contract tasks remain
migration inputs and are never called by root dev/test controllers. CI classifies
new Loco paths, retains Rails and native/client checks, gives Ruby jobs Rails
working directories, and keeps published product images built from Rails until
the final migration is accepted. Hooks invoke Rails Task wrappers rather than
root Bundler. Ignore patterns protect relocated Rails keys/env/generated data.

## Source records and preservation

`capability-inventory.md` records concrete source and behavioural test pointers,
jobs, credential/session/signature gates, themes/assets, native contracts and PR
salvage dispositions. `source-inventory.json` is generated from reconciled current
source, including route declarations and all job paths. `task inventory:check`
rejects added/changed input source. At the user's explicit request, the coordinator
closed PRs 2397, 2399, 2402 and 2403 as superseded framework choices after verifying
their captured heads. Their source refs remain protected locally and their
branches were not deleted. Useful behaviour, themes and assets still require
actual Tera/daisyUI migration evidence.

`preservation.json` records original revision, each tracked source path,
relocated destination and before/after SHA-256. The preservation task also checks
existing source comment lines and tracked modes, comparing baseline symlink target
bytes without following them. New shared symlinks are covered by ownership tests.
Check mode now rejects byte drift against the recorded ledger, and root CI runs
workspace, negative drift and real inventory/preservation checks. Its `force_stage_tracked_relocation` entries
identify only previously tracked files whose new destinations are ignored; Bucky
must never force-add entire directories or ignored secrets/generated data.

No source comments were intentionally added or removed. Cargo.lock includes
Cargo's generated header. Historical dated reports and UI sweep evidence retain their original
commands; active setup/testing/deployment guides use the Rails namespace.

## Verification recorded so far

The exclusive verifier owns builds, installs, runtime and acceptance resources.
The initial immutable RED checkout was
`838e79da76aceed1155d047dae038e1bd2bad5f1` with test-only wiring.

| Check | Observed result |
| --- | --- |
| `task foundation:test` before product edits | RED: missing `rails/app` and root `dev` command |
| `task foundation:test-http` before product edits | RED: root application could not boot because `dev` command was absent |
| `task foundation:test-ci-paths` before policy edits | RED: `src/app.rs` unmapped and relocated Rails UI omitted Lighthouse |
| `task deps`, `task fmt:apply`, `task build`, `task check` on Loco source freeze A | GREEN; final format/check/lint/test also passed |
| `task foundation:test-http` on disposable PostgreSQL 18 | Verifier reported actual `/up`, `/_health`, Tera `/` and absent product-route assertions passed |
| `task test` on freeze A | Cargo test passed with zero Rust tests; observable acceptance comes from HTTP test. Later hook regression caused layout stage to fail before repeated HTTP |
| Isolated `task foundation:test` with hook regression | RED: hook did not invoke `task rails:rubocop`; root/standalone Rails helper dry runs passed |
| `task foundation:test` after hook fix | Verifier reported all four then-current checks passed |
| `task ci:coverage:test CONTRACT_PROJECT=mtloco-coverage-20261005-verifier` | RED: tools container could not find `/Gemfile`; GREEN after explicit app-root/container-path fix; complete results accepted and missing/duplicate/below-threshold results rejected |

The HTTP test reserves an ephemeral loopback port, requires its owned child to
remain alive and terminates/awaits the owned process group. The initial fixed
port was replaced before GREEN. Readiness checks do not label liveness as proof
that the database is available.

The verifier also recorded workspace isolation GREEN for five legacy/client crates,
117 focused Rails examples GREEN after correcting a relocated task path, and
`rails:gems:audit` GREEN with 1,252 advisories and no vulnerabilities. Worker mode
started against owned PostgreSQL 18 and reported online; graceful SIGINT
propagation remains unclear and is not claimed as verified shutdown.

Initial published-waypoint verification recorded Loco format/check/lint/test GREEN, shared CI 82 checks
GREEN, actionlint and three Loco path checks GREEN, and docs build GREEN. The
preservation ledger covers 2,933 inputs: 2,893 unchanged and 40 path/tooling edits,
with original source comments and modes preserved. Final Rails RuboCop inspected 1,893 files with zero offenses, its tooling
regressions passed three examples, and canonical CI helper lint inspected two
files with zero offenses. Final inventory write/check and preservation write/check both passed.
The initial source inventory SHA-256 is
`de525e50c2300bd2a3fe2a995891f72d956f2b0c096c12dd9724270a6d1ddbab`;
the initial preservation ledger SHA-256 is
`00c532b71bf4c8c442787a8dfb2a8dcebcf3fa4f1bc19c1333490d59f2638051`. Bounded owned-process cleanup RED was captured for the missing helper; fixtures
cover a TERM-resistant child and a surviving descendant. The helper now performs
bounded TERM/KILL group termination and child-exit confirmation. Both cleanup
fixtures passed, including the surviving descendant. Final root test passed
Rust targets, five ownership checks, both cleanup fixtures and actual HTTP.
Startup is bounded to 60 seconds and route requests abort after five seconds.

The verifier captured actual desktop/mobile browser views; the coordinator
inspected the plain foundation page. Evidence is in
[desktop](../../screenshots/loco-foundation-desktop.png) and
[mobile](../../screenshots/loco-foundation-mobile.png). The observed favicon 404
is not a migrated-product claim.

Rails RuboCop target-selection RED demonstrated generated tmp files entering
lint after relocation removed the container Git root. Explicit generated-directory
exclusions replace Git-dependent discovery while preserving application
entrypoint coverage; target-selection and full Rails lint are GREEN. Independent Devin source review
completed with requirements NOT MET after the user explicitly authorized
implementation-diff sharing through 16 October 2026. The earlier automatic
approval rejection was resolved by that authorization; fixes require re-review.

## Remaining prerequisites and next action

Hosted run [37303106746](https://github.com/damacus/med-tracker/actions/runs/37303106746)
for published head `959be240e66f2731d5cba34cf66ace42e7ce4b4e` passed the Loco
foundation, workflow, documentation, security and style jobs but failed overall.
Confirmed relocation failures include doubled `rails/rails/compose.yaml` in
legacy contract execution, root Renovate and Rails Taskfile lookups in two Rails
specs, and formatting of two changed legacy Rust include paths. Markdown lint
also rejected leading PR-number tokens in the coordinator plan. The system
mobile audit encountered a schedules-controller exception page. These failures
were addressed by scoped source/test fixes and await final GREEN/re-review.
The exact schedule request proved ParameterMissing person_id/HTTP400. Actual
card callers use the nested edit route; the audit now verifies that supported
edit form. Controller/routes are unchanged, and no executed-baseline proof or
new top-level show/edit compatibility is claimed.

The review-fix source is frozen while the verifier completes relevant checks.
The earlier 24-file runtime-delta manifest at
`/private/tmp/medtracker-foundation-review-fix-freeze-20261005.sha256` had
SHA-256 `0bb8921de3bc19b37facf9b22b9e0345b492f9536fecc8cb144db12aa0bcabc4`.
It records selected changed runtime/test/configuration inputs, not a substitute
for the final full source inventory and preservation checks.
That earlier freeze was superseded by the owned HTTP database fix. The subsequent
27-file runtime-delta manifest at the same path had SHA-256
`0927c803fc3ed38ccea25dd3efa917cb6310bbd022bb66076cfc02c4f3eb9942`.
F1's current 29-file runtime-delta manifest has SHA-256
`7d1f3ae6adb9c68b7e3ec3a980c66b8699bdcf603cbbc610601d1d35bfbb959a`.
Focused Rails requests/configuration/mobile audit, shared CI, Compose/context,
coverage host/container/lint, byte-drift rejection and relocation guards have
recorded GREEN during the repair phase. Later lint edits and scanner restoration
received clean checks on the 24-file freeze above: Rails RuboCop inspected 1,894
files with zero offenses, and focused JSON/schema/shared-path/schedule/mobile
specs passed 39 examples with zero failures. The initial ledger counts and hashes
above do not identify these fixes. Final regeneration on the 29-file freeze
recorded 3,213 source-inventory inputs, SHA-256
`16e4b8fdd63fc3dec1d3fcbbfaec6901a766de674fc8f47449ca91a52fe6abd0`.
The preservation ledger records 2,933 inputs: 2,886 unchanged and 47 modified,
SHA-256 `8d0ea19d6c3f5e4c072409f6d8f344b27d55ad4e33bdc466c096f6604f6bfc9a`.
Inventory and preservation write/check passed. The verifier restored the tracked
generated preview WASM to published HEAD bytes before final reconciliation;
its SHA-256 is `3669d0a2760e1bda19a3711d01baf6342ce2f465b2e7048b483322ff7a4174d7`.
The regenerated binary was an authorized build output, not a product change
required for publication.

A real bundled Tailwind CLI fixture demonstrated an exclusion regression:
without the canonical ignore file it generated both application and temporary
source classes; with the ignore file it retained the application class and
excluded the temporary class. Both fixture runs completed in 35–36 milliseconds.
This proves exclusion semantics, not the cause of the contract runner's Tailwind
SIGKILL. No Docker OOM event was established. Actual restored-context build and
full contract runner results are distinct checks. The dry-run-verified public
`task rails:test:exec CONTRACT_PROJECT=mtloco-tailwind-restored-20261005 CMD=true`
completed the real restored-context Tailwind 4.3.3 build in approximately one
second and exited zero. Full root and standalone contract runner results remain
pending; the exact kernel cause of the earlier SIGKILL is unproved.

During a diagnostic temporary Task wrapper, project variables were not preserved.
The default project `med-tracker-e2aaaf76` ran a migration at 13:27:15 UTC and
exited zero with silent logs; no seed execution was proved. Its effect on the
preexisting test database is unknown. The coordinator verified cleanup of newly
started containers, network and new bundle volume while preserving preexisting
PostgreSQL and node_modules volumes. No incident resource remains, but database
contents are not proved unchanged. No reset or repair was attempted. Diagnostics
now use an existing public Task with an explicit unique project and require a
dry-run check of every expanded Compose project before execution.
The unresolved test-database effect is tracked in
[issue 2453](https://github.com/damacus/med-tracker/issues/2453).

A subsequent bare `task ci` reached the HTTP test but failed before listener
startup with PostgreSQL code 28000, because the default endpoint lacked role
`medtracker`. The HTTP harness had relied on an ambient/default database URL.
It now provisions a unique disposable PostgreSQL 18 project through existing
foundation database Tasks, derives its explicit loopback URL, and tears down only
that project in `finally`. Ambient Compose overrides and database URL are not
accepted for this fixture. Bounded Task subprocesses use owned process-group
cleanup; the listener retains its bounded startup/request/cleanup behavior.
New regression cases cover project uniqueness, explicit fixture URL, setup-error
cleanup, invalid endpoints and preservation of the original application failure.
The helper regression passed four cases. Actual HTTP under an intentionally
unusable ambient `DATABASE_URL` passed one test in 6.15 seconds; the verifier
confirmed no owned HTTP fixture containers, volumes or networks remained.
Bare `task ci` passed on the final frozen source and reconciled ledgers, as did
`task docs:build` and whitespace checks. Cleanup failures are reported explicitly and must not be described as
successful cleanup.

F1's actual Docker scratch COPY/export regression used the repository build
context and a byte-identical copy of the runner's Dockerfile-specific ignore
file. RED omitted Rails locales, AI YAML configuration and the font licence,
despite the authoritative files existing on the host. It failed before Compose
resources were used, and its temporary fixture was cleaned. The owning filter
now whitelists the relocated `rails/config` YAML/locales and licensed
`rails/vendor/fonts` inputs. The production Dockerfile COPY paths were already
correct and remain unchanged. The probe compares each exported font, licence,
configuration and locale file byte-for-byte before any long Rust image build.
A synthetic Docker boundary regression then proved that simple parent negations
admitted a dummy credential marker. Explicit descendant exclusions now reopen
only required Rails configuration/locales and font/licence inputs. The fixture
contains synthetic markers only and verifies unrelated configuration, credential,
environment, application and log inputs are excluded before the real context
probe runs. Both this exclusion check and the actual byte-identical export are
GREEN, as are composed browser contexts and Docker-free `task ci:check`.
The first full root runner stopped on an EOF while pulling the Playwright base
image. A focused actual browser-image build subsequently passed. The full root
retry then stalled during Rails fixture preparation for 8 minutes 1 second and
was stopped with exit 130. Exact-project cleanup initially encountered HTTP 500,
then succeeded; the verifier confirmed no owned leftovers. The standalone runner
has not run. Full runner verdicts remain required; image creation or fixture
cleanup does not substitute for complete browser contract success.

The 816 KB corrective source-review request to Devin SWE-2 Max ended at its output
token limit without a verdict. Two subsequent bounded packets used the same
independent Devin SWE-2 Max reviewer. The exact
[contracts review](foundation-contracts-review-20261005.md) returned requirements
MET at source level and technical soundness with one important screenshot-path
regression. The exact [foundation corrective review](foundation-corrective-review-20261005.md)
returned requirements NOT MET and technical soundness with targeted findings.
These are separate scoped source verdicts, not full foundation acceptance.
A Docker restart was requested asynchronously because it
can affect other agents, but approval remains pending and no restart is claimed.
F1 and the foundation slice remain unaccepted despite the focused and root CI
checks above. Product migration is unfinished.

The bounded findings were checked against current source. Host-only RED reproduced
relative screenshot overrides, the empty Docker-created Rails ignore placeholder,
and documentation-agent globs pointing to absent root application directories.
The screenshot overrides now use absolute workspace paths, documentation globs
use `rails/app`, and the host ignore path is a canonical `../.gitignore` symlink.
Container services explicitly mount the canonical file at `/.gitignore`, which
is the symlink target from `/app`; the `/workspace/.gitignore` alias is retained.
Actual container mount/identity proof remains pending backend availability.
The 31-file runtime-delta manifest has SHA-256
`cfaf40acd5bef249b2d99d219eeafd9639b0b1b98ab0641fb0dbfe974874d729`;
the symlink target is recorded separately because checksum tools follow it.
That freeze was superseded by verified ownership-shim repairs. Four actual-shim
RED cases proved namespaced execution skipped its fixture, namespaced port lookup
returned empty output, relocated API source was rejected with status 51, and
absolute browser inputs were rejected with status 47. The shim now follows the
actual Rails Task namespace, captured Rails source locations and absolute paths.
Host-only `ci:test` passed 87 checks and `foundation:test-relocation` passed seven.
The current 32-file runtime-delta manifest SHA-256 is
`6efed2a254cacc7ef81e009d845e446131e37558757bdb73b8e5a002d9e9d04b`,
with the canonical ignore symlink target still `../.gitignore`.

The audit-selector query-string claim is unconfirmed and contradicts the owning
routes' household-slug scope; no selector change was made pending actual helper
and rendered-form proof. A direct Ruby 4.0.7 host probe now disproves the collator
lexical-symlink claim: the actual collator ran through the Rails symlink from Rails
cwd with its application-root environment variable truly unset. Complete shards
passed and missing, duplicate and below-threshold shards were rejected. Earlier
coverage-harness runs explicitly selected application root and did not alone
exercise this fallback. No collator source change was made. The minor copied-ignore
comparison was removed because successful copy status already checks that operation; the actual Docker
export and synthetic boundary probes carry the acceptance evidence. Actual
container/audit proof remains pending. The final source review and offline ledger
checks are recorded below.

The subsequent [final corrective review](foundation-final-review-20261005.md)
preserves the reviewer wording with trailing whitespace normalised; the original
raw CLI output remains preserved separately. Devin SWE-2 Max reviewed the corrective source packet
against parent `035bf52f` and the 32-file manifest
`6efed2a254cacc7ef81e009d845e446131e37558757bdb73b8e5a002d9e9d04b`.
Requirements are MET at source level and technical quality is sound, with no
Critical or Important findings. The reviewer explicitly withdrew the audit
selector finding and accepted the direct collator fallback evidence. One
actionable Minor requests symmetric dashboard validation in the actual ownership
shim. Two actual-shim RED cases proved the dashboard branch accepted a wrong
screenshot directory and omitted source-snapshot validation. The minimum fix
shares the existing browser validation while selecting the dashboard directory.
No collator change was made.
This scoped source verdict does not accept the foundation or replace its blocked
container/browser and published-head verification gates.

The supplemental dashboard fix follows that review. Its frozen 32-file manifest
SHA-256 is `74b6e18b70323b42e9d1e522a9021b3800b754ec35680b3a91383e30795887ae`.
The exclusive verifier completed offline `ci:test` with 89 passing checks and
`foundation:test-relocation` with seven. Inventory write/check, preservation
write/check, `docs:build` and `git diff --check` passed without Docker or database
activity. Preservation records 2,933 inputs: 2,885 unchanged and 48 path/tooling
edits, with existing comments unchanged. The inventory SHA-256 is
`53e5106978273f3f6ead76644976f81a5ff89c3f679d29b070e1ead08588c13f`;
preservation SHA-256 is
`102c7a143025c4ab5b86aa78d67d44f0caee3105f90296890a462417aabe018d`.
Inventory commands emitted Git's warning for the Rails ignore symlink. Host
target identity and Rails ignore coverage passed; Git deliberately does not
follow working-tree ignore symlinks, as documented in its
[ignore rules](https://git-scm.com/docs/gitignore#_notes). No warning suppression
or speculative filesystem repair was applied. Root `task ci` was not repeated
on this supplemental freeze because its HTTP fixture requires Docker PostgreSQL
18. Full root and standalone browser runners, container ignore-mount proof and
published-head checks remained blocked or unverified at that offline checkpoint.

Subsequent actual Docker execution closed the focused runtime gaps on the
unchanged source freeze. Tailwind CLI 4.3.3 passed the canonical ignore scan and
positive-control checks; this is actual scanner execution, distinct from the
earlier rule-presence spec. The real Docker context probe passed required
font/licence/configuration/locales inclusions and synthetic exclusions.
The root browser runner (session 87157, owned project
`mtcontract-cef98a52b5bf4166`) passed 35 of 35 browser tests in 16.93 seconds,
with total runner duration approximately 6 minutes 30 seconds. The standalone
runner (session 70154, project `mtcontract-0e5689b863f449bd`) passed the same
35 tests in 16.96 seconds. Both captured source SHA-256
`a54ee92ab5c949e52efcbb5aad7fcaba6ef3ea0dbbaccc05e4375a3396a2f833`;
the verifier confirmed owned resources were cleaned and the source manifest
remained unchanged. Exact restoration of generated preview WASM precedes fresh
root CI and final reconciliation. Required whole-Rails/lint and publication
checks remain gates; these focused successes do not accept F1.

Published-head CI remains distinct. GitHub run `37328409697` on published
`035bf52f` failed: household job `111825049532` used
`rails/rails/compose.yaml`, and RSpec shard `111825049071` could not find
`Rails.root/renovate.json`. The current local source contains the absolute
Compose path and canonical repository-root fixes and passed the local checks
above. It has not yet received a passing published-head CI run.

The final auth-server dependency is not chosen. Loco JWT/client OAuth support
does not replace Rodauth SMART authorization-server behaviour. The existing
`oxide-auth` dependency is a candidate for complete server grant flows, not proof
that the old custom grant orchestration should be retained. Persistence/auth and
API tranches require maintained server APIs, credential/storage compatibility,
PKCE/refresh/revocation and negative/interoperability evidence.

Tranches 2–6 still own schema/RLS/auth, complete API/native/integration journeys,
Tera/daisyUI/browser/offline behaviour, worker recovery and two-architecture
scratch runtime acceptance. The current Tera page is a foundation smoke surface.

The earlier published draft received requirements NOT MET from Devin SWE-2 Max;
the later scoped corrective review returned MET at source level and technical
soundness. Runtime and published-head acceptance remain separate gates.
Nightingale retains product/test fix ownership.
Only Bucky integrates, commits, pushes and publishes. No deployment, live schema
migration, PR merge or data reset has been performed.

## Latest final-gate checkpoint, 5 October 2026

After both complete dashboard runners passed, generated preview WASM was
reconciled to published bytes. Final root `task ci` exited zero on the 32-file
runtime freeze, including formatting, Clippy, Cargo tests, layout/workspace,
relocation/preservation/cleanup/database checks, both ledgers and the actual
HTTP listener (one example, 6.07 seconds). Documentation and whitespace checks
passed. The root CI raw log is
`/Users/damacus/Library/Application Support/rtk/tee/1791221910_task_ci.log`.

The full Rails gate then ran all `spec` files under the explicitly owned
`mtloco-final-rails-d62e930f` project. RSpec reported `6112 examples, 1 failure`
in 11 minutes 2 seconds. The restore-documentation spec expected the old root
Task literal while the current runbook correctly uses the relocated
`task rails:hosted-restore:rehearse` interface. One expectation line was corrected;
all security assertions and the dated historical audit remain intact. Its focused
rerun passes two examples, zero failures. The full-suite rerun passes all 6,112
examples with zero failures in 10 minutes 31 seconds against the 33-file manifest
`fe8aa2db4e0d81be20782a19440658b2bc745346929c0ceb19320755d135bc31`.
The passing rerun log is
`/Users/damacus/Library/Application Support/rtk/tee/1791223912_task_rai_9dc8f6.log`.
The original failing suite log is
`/Users/damacus/Library/Application Support/rtk/tee/1791223068_task_rai_9dc8f6.log`.

Ruby lint before that one-line correction passed 1,894 files with zero offences.
The relevant lint is being repeated. These ordinary Rails Task invocations use
the default `COVERAGE=false`; they do not claim coverage-threshold acceptance.
Final planning-ledger regeneration and documentation checks follow, then
corrected publication and hosted CI. Foundation acceptance remains pending.

## Framework sources

Context7 was queried for `/loco-rs/loco`, `/go-task/task` and
`/evilmartians/lefthook`. Implementation follows the official
[Loco App example](https://github.com/loco-rs/loco/blob/master/examples/demo/src/app.rs),
[Tera initializer](https://github.com/loco-rs/loco/blob/master/examples/demo/src/initializers/view_engine.rs),
[server page controller](https://github.com/loco-rs/loco/blob/master/loco-new/base_template/src/controllers/page.rs)
and published Loco 1.2.0 APIs validated by compilation/runtime checks. Cargo.lock
pins registry checksums; upstream master examples are explanatory references,
not the dependency pin. Serena initial instructions were called before source
exploration; its active language server is Ruby, so Rust navigation used `rg`.


## Deferred Rails stock-notice failure

The verified foundation Rails suite passed before the upstream dependency merge.
The combined tree later ran 6,112 examples with one missing scanned-stock notice.
The same original example subsequently failed in a focused run, while its page
showed 30 units. This establishes a missing user-visible notice; it does not
establish a failed stock write or a cause attributable to the dependency merge.

The added browser diagnostic passed its message and stock-write assertions, but
controlled page-visible response delivery rather than cookie application. Its
linked request context could commit cookies before releasing the held response,
and its release event preceded complete redirected-page processing. That result
neither proves nor rejects a stale-session-cookie cause. A later isolated-context
variant was never executed. No application or session fix was made.

As requested, Rails flash diagnosis is stopped. The original stock
system spec is restored byte for byte to HEAD; every original assertion and
comment remains intact. The exact diagnostic diff, spec, report and held manifest
are archived outside the repository at
`/private/tmp/medtracker-stock-diagnostic-archive-20261005`. Existing failure logs
and screenshots are retained; no test resources were removed in this cleanup.
The investigation does not gate further Rust implementation or claim medication
feature migration is complete.
