# Fresh Rust rollout notes

Root Loco uses PostgreSQL 18 and a new empty database. Follow
[ADR 0016](../adrs/0016-fresh-rust-without-legacy-compatibility.md): Rails is
read-only reference; there is no Rails data import, credential migration, populated
schema adoption or reverse migration. Never reset or silently adopt an existing
database.

The owner applies the image/version upgrade through the deployment's normal
controller. There is no separate rehearsal, runtime evidence dossier or final
acceptance project. Observe normal startup and application behaviour during the
release, and fix concrete defects through the normal engineering workflow.
This document does not authorise a rollout, shutdown, data change or deployment.

## Image and configuration

- Build runnable root-Loco server and worker images for `linux/amd64` and
  `linux/arm64` under [#2723](https://github.com/damacus/med-tracker/issues/2723)
  and [#2724](https://github.com/damacus/med-tracker/issues/2724). Include TLS trust
  roots, fonts/PDF support, timezone data, templates and static assets. The current
  [publishing workflow](../../.github/workflows/docker-publish.yml) builds
  `rails/`; its image is not the Loco replacement.
- Use the selected version or immutable image reference in the existing deployment
  configuration. Keep server and worker versions compatible with the schema.
  No final root-Loco image reference is supplied by this guide.
- Use [`config/production.yaml`](../../config/production.yaml) as the Rust settings
  reference. Set the public origin with `MEDTRACKER_PUBLIC_HOST`, `PORT` and the
  appropriate `BINDING`; keep production secure cookies enabled. Provision a
  persistent 64-byte Base64 `MEDTRACKER_SESSION_KEY` through the secret store,
  shared across replicas. Do not use fixture keys or copy Rails credentials.
- Keep setup credentials separate from the restricted runtime `DATABASE_URL`.
  The runtime login inherits `med_tracker_app`, has no owner-role membership or
  superuser/`BYPASSRLS` privileges, and serves both application and queue access.
  Keep production automatic migration, truncation and recreation disabled.
- Configure private storage consistently across replicas: avatars use
  `ACTIVE_STORAGE_S3_*`; dm+d uses `S3_ENDPOINT`, `S3_BUCKET`, `AWS_REGION`
  and its access/secret keys. Keep secrets in the deployment secret store and
  preserve existing objects. Configure enabled mail/push delivery and the
  registration, session/token lifetime, timezone and dose-signing settings.
- The root `worker` Task starts a worker **and** scheduler. The `scheduler` Task
  is another scheduler entry point. Choose one scheduler owner; avoid starting
  both unintentionally. Current schedules cover support expiry, dm+d,
  browser reminders and medication-review refresh.

## Rollout

The owner chooses the environment, version and time and uses the deployment's
existing upgrade path. If Flux owns the deployment, change its declared
configuration through the normal GitOps flow; do not bypass it with an invented
Helm or kubectl procedure.

1. Confirm the intended fresh Rust database and storage configuration. Initial
   schema installation uses root `db:migrate` with a separate
   `MIGRATION_DATABASE_URL`; see [database setup](../testing.md#loco-database-setup).
   Stop if the target is populated or unclear. `db:seed`/`db:prepare` load
   synthetic fixtures for development/test, not production users.
2. Keep retained Rails/reference databases and secrets separate. Where an existing
   public service is being replaced, stop or isolate its web, worker and scheduler
   writers as part of the owner's normal switch. Do not point them at the Rust
   database, import their queue or migrate their credentials.
3. Apply the approved image/configuration upgrade. Start the intended server,
   worker and single scheduler through the existing controller. Observe normal
   startup, sign-in and medication operations while releasing. Existing tests and
   CI remain the normal checks for the deliverables being changed; do not create
   a second release-acceptance exercise.

## Stop and recovery

If the release fails, the owner uses the same deployment controller to stop or
revert the affected rollout. Stop the Rust web, worker and scheduler writers
before any separately approved database recovery. Preserve the Rust database,
queue, objects and audit history; do not reset them to make startup pass.

Reverting an image is safe only when it is compatible with the current Rust
schema and configuration. If data recovery is needed, use an approved
Rust-compatible backup/target and the owner's explicit data-loss decision.
Do not restore over active writers or assume a Kubernetes/Helm version rollback
reverses database changes. Rails cannot read Rust-created data through a reverse
migration, and automatic saved-state Rails restoration is not part of this plan.

## Existing scope decisions

The owner accepted [Slice 7](https://github.com/damacus/med-tracker/issues/2654)
for its delivered queue, mail, push, notification, import, refresh, expiry and
scheduling behaviour. Additional speculative acceptance, per-worker stop/restart
and SIGTERM rehearsals are waived. Retain current permissions, atomic mail,
replay and duplicate safety; fix concrete defects if they emerge.

Native delivery, browser offline work, new export products/workers
([#2699](https://github.com/damacus/med-tracker/issues/2699)), canary/demo jobs
([#1780](https://github.com/damacus/med-tracker/issues/1780)), and optional
FHIR/SMART/MCP/provider work remain deferred. Preserve retained Rust restore/sync
and public API contracts. Physical notification checks
([#2559](https://github.com/damacus/med-tracker/issues/2559)) remain deferred until
after deployment; provider acceptance does not prove display. Broader mail/SMTP
capture remains [#2844](https://github.com/damacus/med-tracker/issues/2844)/
[#2845](https://github.com/damacus/med-tracker/issues/2845), and physical scanner
camera checking remains [#2524](https://github.com/damacus/med-tracker/issues/2524).
These are their existing scope decisions, not new release gates.
