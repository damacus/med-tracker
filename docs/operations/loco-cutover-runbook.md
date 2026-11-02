# Fresh Rust installation notes

Root Loco uses PostgreSQL 18. Install it against a new, empty database and
create fresh accounts. Follow [ADR 0016](../adrs/0016-fresh-rust-without-legacy-compatibility.md):
PR #3028 removed the Rails application from the current tree; its files remain
in Git history. There is no Rails schema adoption, credential conversion or
reverse migration. These notes describe fresh
installation requirements. They do not define a cutover, live-data transfer or
Rails recovery plan, and they do not authorise a release or deployment.

Rust supports versioned encrypted portable export and import under the
[portable-data contract](../api/portable-data.md). Frozen synthetic Rails
fixtures cover selected serializer examples only; they do not establish
database-backed export selection or completeness for all Rails records. The
known v1/v2 limitations and remaining compatibility evidence are tracked under
[#2702](https://github.com/damacus/med-tracker/issues/2702). That work does not
require planning or performing a Rails cutover for this release.

## Image and configuration

- Root-Loco server, worker and scheduler images have native `linux/amd64` and
  `linux/arm64` build and smoke-check coverage. Follow
  [Loco Linux images](loco-images.md) for the current image workflow. It builds
  and checks the Loco images; PR #3028 archived Rails and removed its separate
  image publisher. Publication is not deployment.
- Use the selected version or immutable image reference in the installation's
  configuration. Keep server and worker versions compatible with the schema.
  This guide does not select a release version or image digest.
- Use [`config/production.yaml`](../../config/production.yaml) as the Rust
  settings reference. Set the public origin with `MEDTRACKER_PUBLIC_HOST`,
  `PORT` and the appropriate `BINDING`; keep production secure cookies enabled.
  Provision a persistent `AUTH_SESSION_SECRET` and 64-byte Base64
  `MEDTRACKER_SESSION_KEY` through the secret store, shared across replicas. Do
  not use fixture keys or copy Rails credentials.
- Keep setup credentials separate from the restricted runtime
  `DATABASE_URL`. The runtime login inherits `med_tracker_app`, has no
  owner-role membership or superuser/`BYPASSRLS` privileges, and serves both
  application and queue access. Keep production automatic migration,
  truncation and recreation disabled.
- Configure private storage consistently across replicas: avatars use
  `ACTIVE_STORAGE_S3_*`; dm+d uses `S3_ENDPOINT`, `S3_BUCKET`, `AWS_REGION` and
  its access/secret keys. Keep secrets in the deployment secret store and
  preserve existing objects. Configure enabled mail/push delivery and the
  registration, session/token lifetime, timezone and dose-signing settings.
- The root `worker` Task starts a worker **and** scheduler. The `scheduler` Task
  is another scheduler entry point. Choose one scheduler owner; avoid starting
  both unintentionally. Current schedules cover support expiry, dm+d, browser
  reminders and medication-review refresh.

## Fresh installation

1. Provision a new, empty PostgreSQL 18 database and the required storage.
   Stop if the target is populated or its ownership is unclear. Initial schema
   installation uses root `db:migrate` with a separate
   `MIGRATION_DATABASE_URL`; see [database setup](../testing.md#loco-database-setup).
   `db:seed` and `db:prepare` load synthetic fixtures for development and test,
   not production users.
2. Configure the selected Loco image, runtime settings, restricted database
   credentials, storage and mail/push services. Create new Better Auth accounts;
   existing Rails credentials and sessions do not authenticate to Rust.
3. Use the installation's normal controller to run the server, worker and one
   scheduler against that database. Observe startup and ordinary sign-in and
   care operations as part of the owner's release process.
4. Where portable transfer is needed, use only the currently supported
   encrypted import/export contract and its documented limits. A frozen fixture
   or successful Rust-native round trip does not prove that every Rails export
   can be restored. Unknown-completeness retained prescription snapshots remain
   blocked from adoption and fresh dose recording under PR #3029; no recovery
   procedure is promised.

## Related scope

Account-owned dose-history reading is delivered (#2982; first-release history
issue [#2685](https://github.com/damacus/med-tracker/issues/2685) is closed).
Person-owned shared treatment plans and cross-household sharing remain
unfinished under [#2684](https://github.com/damacus/med-tracker/issues/2684) and
[#2976](https://github.com/damacus/med-tracker/issues/2976). Household
administration does not grant permission to change a person's plan.

Physical phone-camera acceptance remains follow-up fix-forward work under
[#2524](https://github.com/damacus/med-tracker/issues/2524). Simulated camera
input does not prove a physical phone camera works. Optional medication
locations remain paused for the owner's UI audit. These notes do not turn those
items or Rails-export completeness into a deployment or recovery gate.
