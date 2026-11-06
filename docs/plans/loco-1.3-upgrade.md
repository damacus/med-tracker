# Loco 1.3 upgrade

## Outcome and boundaries

Complete PR [#3045](https://github.com/damacus/med-tracker/pull/3045) using the
published Loco 1.3.0 release, preserving application behaviour, API/MCP contracts,
restricted runtime credentials, RLS, atomic mail, audit and stock operations.
No deployment, live-data changes, database adoption or reset is authorised.
New notification, cache and session features are outside this upgrade.

The owner requested Sol planning and a goal, followed by Terra implementation
on Normal/default service and independent Sol review. This document is the shared
implementation and acceptance record. No product decisions remain unresolved.

## Verified starting point

- On 5 November 2026, fetched main is `ab3af2d0838fe138f9ec17f956649beefc1d7dbc`.
  PR head is `360291c2a8b07d61c9879249cd4605f473501fa4`, directly above main.
- The PR changes only Cargo.toml and Cargo.lock. Hosted Rust lint, unit,
  integration, browser and both image builds failed. Scheduler image acceptance
  was skipped. These results do not validate the upgrade.
- Hosted lint errors confirm missing `db-postgres` and `mailer` features.
  The old 1.2 scheduler patch no longer matches the dependency requirement.
- Official [1.3 release notes](https://github.com/loco-rs/loco/blob/v1.3.0/CHANGELOG.md)
  describe shell-free execution, cron replacement, feature gates, queue schema,
  storage and task argument changes. Grounded Docs has only 1.2.0 indexed;
  Context7 provides general guidance, not exact-release proof.
- Official [PostgreSQL queue source](https://github.com/loco-rs/loco/blob/v1.3.0/src/bgworker/pg.rs)
  serialises setup using an advisory lock but always issues ALTER TABLE and
  CREATE INDEX statements. Runtime has only DML grants. Existing migration queue
  schema lacks the primary key, attempts, unique_key and indexes expected by 1.3.
- Compression middleware is not configured. Existing storage_aws_s3 enables
  storage_opendal transitively. Four production cron jobs and native scheduler
  process/image acceptance already exist. Better Auth remains authentication
  authority; Loco's new session/JWT features need no adoption.

## Implementation plan

1. Reproduce failure before executable changes using existing Task checks. Add
   behavioural coverage where existing tests do not exercise concurrent startup,
   the queue migration and missing-schema rejection. Retain full App::boot with
   restricted credentials; a compile failure alone is not queue acceptance.
2. Enable exactly the required 1.3 features and retain published exact versions
   and lockfile integrity. Remove the obsolete scheduler patch only as part of
   validating released scheduler behaviour. Fix actual API compatibility errors.
3. Add a forward SeaORM migration for the queue primary key, required columns and
   indexes. Preserve jobs, reject duplicate IDs without deleting data, and retain
   DML-only runtime grants. Do not rewrite historical migrations as the sole fix.
4. Use the smallest supported Loco boot integration that avoids runtime queue DDL
   while retaining Loco's queue operations, worker registration, retries, reaper,
   shutdown and context lifecycle. Verify the prepared schema and fail clearly
   when missing. Inspect Hooks/create_context/run_app and Queue APIs first;
   prefer a narrow application integration over a fork, queue reimplementation,
   swallowed setup errors or privilege escalation. Use SeaORM for app queries.
5. Verify mail/outbox, queue retry/reconnection and concurrent full startup,
   storage and configured cron schedules. Run native scheduler process coverage
   for quoting, arguments, environment, failure and shutdown; use actual shell-free
   runtime image acceptance before declaring scheduler replacement complete.
6. Update current Loco version and fork guidance coherently, preserving historical
   evidence. Run task docs:build and git diff --check for documentation changes.
7. Freeze the candidate, independently review with Sol, address findings and run
   task ci. Before expensive final checks recheck main; inspect resulting hosted
   CI and amd64/arm64 image/scheduler results on the published candidate. Record
   unavailable gates explicitly. Publication belongs to the delivery controller;
   do not merge or deploy.

## Evidence to retain

Record exact commands, failing and passing outcomes, candidate revision, review
findings and outstanding CI/image risks below. Use disposable PostgreSQL and S3
fixtures supplied by Task. Preserve unrelated worktrees and changes. No Python,
raw SQL convenience, new Git dependency, broad lint suppression or global reset.

## Local implementation evidence

- Added `fixture_concurrent_restricted_application_boots_succeed` to exercise two
  simultaneous full application boots using the existing restricted fixtures.
  Its behavioural result is pending; the current PR does not compile with the
  missing Loco features, so a compile failure cannot establish queue behaviour.
- `rtk task test:integration TARGET=care_api FILTER=fixture` initially failed
  because the sandbox could not access the Docker socket. The escalated run
  built the test image and started disposable PostgreSQL and RustFS. Cargo first
  reported a shared registry unpack collision for `ctor 1.0.13`, so the test
  did not execute. The deferred fixture cleanup command waited for the shared
  build lock and began compiling; it was still running at handoff (exec session
  13663). No behavioural result was observed.
- The exact Loco 1.3 source exposes `boot::create_context`, `db::converge`,
  and `boot::run_app`. `create_app` invokes queue convergence between database
  convergence and `run_app`; `bgworker::converge` calls `queue.setup`, which
  invokes PostgreSQL DDL even when the queue schema already exists. The public
  context creation path creates the normal queue provider without calling setup.

The existing PR also lacks the `db-postgres` and `mailer` features and its hosted
checks fail at compilation. Enabling those features is an executable dependency
change, but the requested behavioural red test has not been observed. Resolve
that sequencing constraint before implementation; do not mistake either the
cache error or the hosted compile errors for the required behavioural failure.
Implementation, independent review, CI and image acceptance remain pending.
No deployment, publication or merge is authorised here.

## Continuation: dependency bootstrap and Docker storage

- The earlier disposable tool container exited with code 101. Enabled Loco's
  `db-postgres` and `mailer` features and updated the lockfile for their six
  additional packages. No queue boot or schema logic has changed.
- `rtk task test:integration TARGET=care_api FILTER=fixture_concurrent_restricted_application_boots_succeed`
  could not reach the regression: PostgreSQL failed to start and Cargo initially
  required the lockfile update. PostgreSQL's log reports `No space left on device`.
  Docker reported 100.5 GB in volumes and 20.12 GB in build cache; their ownership
  and safe removal were not established. Restore Docker space and rerun the same
  focused task to obtain the required behavioural failure before queue changes.
- `rtk git diff --check` passed. The application, focused acceptance, `task ci`,
  scheduler image acceptance and independent review remain incomplete.

## Sol review of the dependency bootstrap

- Reviewed the current Cargo.toml/Cargo.lock and concurrent-boot test diff.
  Published Loco 1.3.0 declares `db-postgres` and `mailer`; the latter enables
  lettre and include_dir. The added lock records retain registry sources and
  integrity checksums. This is source review, not a successful locked build.
- Traced both branches of the new regression through `Application::new`,
  `new_with_pool_size` and `boot_fixture`: each calls full `App::boot` in
  ServerOnly mode with the fixture runtime URI for both database and queue,
  with migrations, recreation and truncation disabled. The test has not run.
- Rechecked Docker storage after the implementation handoff. It still reports
  100.5 GB in volumes and 20.12 GB in build cache, including 14.03 GB of unused
  build cache. The external host volume has approximately 1.3 TB free; host
  capacity is not the immediate limit. Shared-cache cleanup awaits owner approval.
  No containers, volumes or build caches were removed by this review.
- Remaining acceptance is unchanged: observe the behavioural regression, add
  the queue migration and boot integration, run focused and full checks, validate
  scheduler images, then review the completed candidate and published PR checks.

## Restricted queue implementation, 6 November

- After Docker storage was cleared, the concurrent full-boot regression failed
  with PostgreSQL `42501`, because the runtime role does not own the queue table.
- Added a forward migration for the 1.3 queue columns, primary key and indexes,
  and an application boot path that checks the configured queue database before
  starting Loco without its DDL-producing queue convergence.
- The focused concurrent-boot regression passed. All four `queue_runtime` tests
  passed, including retry, reaper and reconnection coverage.
- Scheduler process checks, missing-schema and existing-job migration coverage,
  full CI, image acceptance and independent review are still required.

## Independent review and binary regression

- Sol found that Loco 1.3's CLI calls `create_app` directly for `start`, so it
  bypasses `App::boot`. A restricted-role worker binary regression reproduced
  PostgreSQL `42501`; the application boot test alone did not cover this path.
- Sol also found that checking index names does not establish their definitions.
  A focused regression replaced the unique queue index with a non-unique index
  of the same name; application boot incorrectly succeeded. Restore completed
  before the assertion. Startup must reject incorrect index definitions.
- Scheduler process acceptance fails because Loco accepts an embedded NUL in a
  later task argument. Alternate scheduler YAML files can encode this value;
  preserve early rejection at each effective configuration boundary.
- Main and the Renovate branch have moved to base `d27437d500f5ea1867d8ad7d9513a34dfda8dfad`
  and PR head `7a76c4cd5d74a837bae661c822f2d1995432428b`. The refreshed PR tree
  differs from the original candidate only in the Rust image digest in Dockerfile.
  Reconcile the branch before final validation.
- `rtk task test:integration TARGET=queue_runtime FILTER=queue_upgrade_` passed
  both migration tests. They reconstruct the previous schema inside a rolled-back
  transaction, preserve queued/processing/failed jobs, and show that duplicate
  IDs reject the migration and roll its DDL back to a savepoint.
- The original worker binary regression failed with PostgreSQL `42501`.
  Scheduler process acceptance also failed on NUL handling. The owner clarified
  that the supported operational concern is Docker startup. An application
  start adapter now routes container server and worker startup through App::boot.
- `rtk task test:integration TARGET=queue_runtime FILTER=boot_rejects_` passed
  three focused tests. They reject a non-unique or wrongly predicated queue index,
  a same-named unique index without a primary-key constraint, and the wrong
  `attempts` column type. The concurrent restricted-role boot test also passed
  with the correctly migrated schema. The added validation queries check column
  types, nullability and defaults, primary key, and unique-index metadata;
  the other two indexes are checked by name only. Further review is required.
- A separate `boot_rejects_wrong_queue_column_default` regression passed after
  changing and restoring the `attempts` default in the disposable database.
  The migration preservation test also verifies each job's status and payload.
  The working Dockerfile now matches the refreshed PR's Rust image digest;
  the branch was subsequently reconciled with the refreshed PR head.
- The restricted-role worker binary processed a tagged synthetic job, and the
  server binary returned a successful HTTP health response. The first health
  test blocked itself by leaving verbose child output in an unread pipe; after
  redirecting the test output, the readiness check passed.
- Loco 1.3 accepts a NUL in a later native task argument during scheduler
  construction. Startup now rejects it for the configured scheduler, including
  alternate scheduler files. The application-boundary regression failed first
  and then passed for the scheduler and combined start modes. The framework-only
  assertion was narrowed to cases Loco itself rejects; this difference is not
  treated as an upstream fix. All five scheduler process tests pass.
- A normal-tier Sol review found that the adapter could validate a different
  scheduler file from the one Loco would execute. A conflicting-file regression
  failed first and passed after rejecting the adapter-only command-line option.
  The review also identified missing in-flight shutdown evidence. A restricted
  worker now passes SIGTERM with a synthetic SMTP job held open. A scratch-image
  scheduler also stayed running after SIGTERM while a subprocess was blocked on
  a disposable database lock, then exited successfully after the lock was
  released and the task wrote exactly one audit event.
- Sol noted that combined server/scheduler startup can remain healthy after a
  scheduler runtime error. The same detached scheduler behaviour exists in the
  1.2 fork; the documented Docker topology uses a separate scheduler process.
  This remains a framework limitation for combined mode.
- Sol's follow-up found no remaining concrete blocker in the configuration and
  shutdown fixes. The rebuilt scratch image passed the restricted server/worker
  smoke check and the scheduled-task and active-shutdown acceptance.
- `rtk task ci` passed on the reconciled candidate: 287 unit, 1,205 integration
  and 68 browser tests passed, alongside the task's other checks. The browser
  screenshots generated by that run were restored after verification.
