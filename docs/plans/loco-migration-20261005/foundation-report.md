# Foundation implementation report

Status: implementation and verification in progress. Independent Devin SWE-2 Max
review has not yet run. This report does not accept the foundation or the full
feature migration.

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
`RAILS_APPLICATION_ROOT=/app` and canonical `/workspace/scripts/ci` bind.

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
rejects added/changed input source. No PR has been closed as superseded.

`preservation.json` records original revision, each tracked source path,
relocated destination and before/after SHA-256. The preservation task also checks
existing source comment lines and tracked modes, comparing symlink target bytes
without following them. Its `force_stage_tracked_relocation` entries
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

Final verification recorded Loco format/check/lint/test GREEN, shared CI 82 checks
GREEN, actionlint and three Loco path checks GREEN, and docs build GREEN. The
preservation ledger covers 2,933 inputs: 2,893 unchanged and 40 path/tooling edits,
with original source comments and modes preserved. Final Rails RuboCop inspected 1,893 files with zero offenses, its tooling
regressions passed three examples, and canonical CI helper lint inspected two
files with zero offenses. Final inventory write/check and preservation write/check both passed.
The source inventory SHA-256 is
`de525e50c2300bd2a3fe2a995891f72d956f2b0c096c12dd9724270a6d1ddbab`;
the preservation ledger SHA-256 is
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
is awaiting user authorization after automatic approval review rejected the
implementation-source payload as outside the previous plan-only approval.

## Remaining prerequisites and next action

The final auth-server dependency is not chosen. Loco JWT/client OAuth support
does not replace Rodauth SMART authorization-server behaviour. The existing
`oxide-auth` dependency is a candidate for complete server grant flows, not proof
that the old custom grant orchestration should be retained. Persistence/auth and
API tranches require maintained server APIs, credential/storage compatibility,
PKCE/refresh/revocation and negative/interoperability evidence.

Tranches 2–6 still own schema/RLS/auth, complete API/native/integration journeys,
Tera/daisyUI/browser/offline behaviour, worker recovery and two-architecture
scratch runtime acceptance. The current Tera page is a foundation smoke surface.

After final verification, Bucky stages the rename-aware diff and supplies it to
Devin SWE-2 Max for separate requirements and technical-quality verdicts.
Nightingale retains all product/test fix ownership and waits for that review.
Only Bucky integrates, commits, pushes and publishes. No deployment, live schema
migration, PR merge or data reset has been performed.

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
