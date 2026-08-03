## Context

See [proposal.md](proposal.md) for motivation and [the capability specification](specs/canary-demo-reset/spec.md) for required behavior.

Canary runs the production Rails environment and Solid Queue inside Puma. Its previous CNPG cluster bootstrapped from the production Barman source, archived its own WAL to `s3://cnpg-med-tracker-canary/`, and took daily canary backups. The restored database copied production accounts, care data, and notification destinations even though Kubernetes resources were isolated. On 2026-08-03 the canary Flux Kustomizations were suspended and its application, database, volumes, backup objects, ObjectStore, and bucket were purged. Canary remains offline and suspended until this change supplies a synthetic replacement.

Development seeding already loads realistic Rails fixtures through `SpecFixtureLoader` and `FixtureHouseholdSetup`, but `db/seeds.rb` deliberately restricts that behavior to local Rails environments. Those test-support files change for test needs and are not an appropriate direct deployment contract for a production-mode demo.

The reset crosses two repositories. MedTracker owns the reset behavior and synthetic dataset. `damacus/home-ops` owns the canary schedule, database bootstrap, storage, and one-time destructive transition. The current bjw-s app-template 5.0.1 supports multiple controllers, including CronJobs, in one release and can mount an existing claim across selected controllers.

## Goals / Non-Goals

**Goals:**

- Make demo mode an explicit application configuration with a visible user-facing identity.
- Make committed synthetic data the only source of canary application state.
- Provide one guarded, repeatable reset operation for scheduled and manual use.
- Replace database state atomically and remove unreachable uploaded files afterward.
- Keep the reset runner independent of Solid Queue state that it deletes.
- Preserve intentional notification testing for demo users.
- Make the initial production-data removal explicit, auditable, and limited to canary resources.

**Non-Goals:**

- Provide a general-purpose production database reset command.
- Preserve canary records, audit history, uploads, subscriptions, or sessions across resets.
- Use canary as a production backup-restore rehearsal environment.
- Change normal household deletion, immutable medication-take, audit-retention, or production backup behavior.
- Duplicate the complete RSpec fixture corpus in the demo environment.

## Decisions

### 1. Make demo mode an application-owned boundary

Add one application configuration, `DEMO_MODE`, which defaults to disabled and is enabled only by the canary deployment. A small application boundary exposes whether demo mode is active, the human-readable reset schedule, and the safety assertion used by the operator command. Demo mode does not replace the exact target checks described below.

When active, the shared authenticated layout renders a persistent accessible notice that identifies the environment as synthetic and disposable and says that data resets every Sunday at 04:15 Europe/London. The notice uses the existing component system and remains visible at mobile and desktop widths. Demo mode does not weaken authentication, authorization, household isolation, medication rules, audit behavior, or notification delivery.

**Rejected:** Infer demo behavior from `RAILS_ENV=production`, the hostname alone, or the presence of demo accounts. Canary deliberately runs the production Rails environment, and implicit detection is too weak a boundary for destructive behavior.

**Rejected:** Keep demo mode solely in Kubernetes scripting. That would prevent the application from presenting an honest demo identity and would split reset safety from the code that owns the data contract.

### 2. Promote an allow-listed fixture subset into a dedicated demo baseline

Create a committed demo dataset under `db/demo/` containing fictional `example.com` accounts and representative household, role, medication, schedule, PRN, and current-date take scenarios. Reuse the fixture-loading technique and established household/grant setup behavior, but expose a demo-specific loader whose input list is explicit and contains no delivery registrations, device tokens, API credentials, uploads, or production identifiers.

The baseline will be tested as product-owned seed data: it must load into an empty migrated PostgreSQL database, satisfy model and tenancy constraints, authenticate the documented demo users, represent scheduled and as-needed medication behavior, and contain zero notification destinations before a tester registers one.

**Rejected:** Load `spec/fixtures` wholesale. Test fixture changes could silently change deployed demo state, and the current loader is intentionally named and scoped as test support.

The versioned dataset is the recovery baseline as well as the weekly-reset baseline. Rebuilding canary means creating a blank database, running the deployed migrations, and loading this dataset. No database dump is generated or retained.

**Rejected:** Maintain a physical or logical database dump. Dumps bind the baseline to a schema version, obscure review of synthetic content, and make candidate migration testing less deterministic.

### 3. Put destructive behavior behind a demo-mode canary reset command

Add a non-HTTP operator command, exposed through the repository's Task/Rake boundary, that orchestrates preflight, database replacement, storage cleanup, and verification. No controller, background job, or user permission will invoke it.

Preflight requires all of the following:

- an explicit `DEMO_MODE=true` application setting;
- the exact canary application host;
- the exact canary database host from the effective Active Record configuration;
- the persistent storage root expected by the canary deployment; and
- an owner-capable database connection dedicated to the reset runner.

The command logs only stages, counts, durations, and synthetic fixture identifiers. It never logs database URLs, credentials, push endpoints, device tokens, health record values, or filenames.

**Rejected:** Accept a generic `FORCE=true` switch. A single switch is too easy to reproduce in production and does not prove target identity.

**Rejected:** Expose an admin reset endpoint. This unnecessarily makes cluster-wide destructive behavior reachable through the application authorization surface.

### 4. Replace database state in one transaction and clean storage after commit

The reset runner acquires a PostgreSQL advisory lock before making changes. It enumerates the deployed application's primary tables from Active Record, excludes only schema/migration metadata, and truncates runtime tables with identity restart and foreign-key cascading inside a database transaction. The same transaction loads and validates the demo baseline. A load failure rolls the primary database back to its pre-reset state.

PostgreSQL cannot provide one transaction across the separate primary, queue, cache, and cable databases. After the primary commit, the runner truncates every non-schema table in each configured auxiliary database. An auxiliary cleanup failure leaves the command failed and is safely retryable because the primary already contains the deterministic baseline.

After database and auxiliary cleanup, the runner removes all objects below the verified canary storage root. At that point the new baseline contains no Active Storage records, so a storage-cleanup failure can leave only unreachable orphan files, not broken baseline attachments. The command remains failed until cleanup and post-reset verification succeed; retrying is safe.

This is an environment reset boundary, not a domain deletion path. It intentionally removes immutable takes and audit records only because every record in this isolated environment is disposable synthetic demo state.

**Rejected:** Delete records model by model. That would run application callbacks, be slow, risk missing new global tables, and make a partial reset more likely.

**Rejected:** Drop and recreate the database every week. The reset runner should not need cluster-administration privileges, and repeated database recreation would couple routine demo cleanup to CNPG reconciliation.

### 5. Schedule reset as a second controller in the canary HelmRelease

Add a CronJob controller to the existing app-template HelmRelease. It reuses the exact candidate application image tag, canary secret, security context, and canary storage claim. It does not start Puma or Solid Queue; it runs only the reset command.

Schedule it for Sunday 04:15 with `Europe/London` as the explicit Kubernetes timezone. Use `Forbid` concurrency, one active execution, a bounded runtime, bounded retry history, and `restartPolicy: Never`. The application-level PostgreSQL advisory lock also prevents overlap between scheduled and manually triggered resets.

Keeping the controller in the same HelmRelease allows YAML anchors and isolation checks to prove that the web and reset runners use the same image and `DEMO_MODE` setting. The existing `task kubernetes:med-tracker-canary-isolation` contract will be expanded to validate the schedule, demo mode, image identity, canary database secret, and storage mount.

**Rejected:** Schedule the reset in `config/recurring.yml`. Reset deletes Solid Queue tables, including the scheduler state needed to execute and record the reset.

**Rejected:** Use a standalone image or script container. That can drift from the application schema and baseline loader being exercised.

### 6. Use a minimal, backup-free canary CNPG cluster

Change the canary CNPG manifest to one instance bootstrapped with clean `initdb`, owned by the canary application role. Retain only the database cluster, normal storage, and the monitoring required to operate it. Disable superuser access because the reset runs as the database owner. Remove its `externalClusters` production-backup reference, Barman WAL-archiver plugin, ScheduledBackup, ObjectStore, and canary RustFS credential/bootstrap resources.

The deployed image's migration init container prepares the blank schema. The application reset command then loads the versioned baseline. This is also the disaster-recovery procedure: CNPG recreates blank storage, and the application reconstructs all intended data. Because canary is reproducible and disposable, it has no physical backup, logical dump, WAL archive, Backup or ScheduledBackup custom resource, object-store destination, or backup credential.

**Rejected:** Continue restoring production and scrub selected tables. A deny-list can miss new sensitive tables or external delivery credentials, and every refresh would recreate the original incident class.

**Rejected:** Keep a single synthetic baseline backup. The committed application dataset already provides that baseline without schema coupling, can retain no tester-entered data, and is exercised by every reset.

### 7. Keep notifications enabled for intentional test registrations

The committed baseline contains no push subscription or native device token. Test users can register their own device and exercise reminder behavior normally. The next reset removes that registration along with other mutable demo state.

Remove the misleading canary `PUSH_NOTIFICATIONS_ENABLED=false` value rather than implementing a suppression feature that conflicts with the demo requirement. Mail delivery remains a separate operational decision and is not changed by this capability.

### 8. Verify reset through public and persistence boundaries

Post-reset verification checks deterministic baseline counts and synthetic identifiers, successful demo authentication setup, representative scheduled and PRN records, zero baseline delivery registrations, tenancy/grant integrity, an empty upload root, and the canary health endpoint. The reset exits non-zero for any mismatch.

RSpec covers the demo-mode boundary and notice as well as the command's public result and persisted state, including safety refusal, transaction rollback, repeated execution, notification-registration removal, and PHI-safe failures. Browser verification covers the notice at mobile and desktop widths. Home-ops validation renders both repositories' deployment contract and checks the CronJob and production-free, backup-free CNPG configuration.

## Risks / Trade-offs

- **[Table discovery misses a non-Active Record persistence surface]** → Inventory all configured primary, cache, queue, and cable databases in tests and fail verification when an unexpected runtime table survives.
- **[A reset overlaps an application request]** → Use transactional database replacement; requests either observe pre-reset state or the committed baseline. Existing sessions become invalid and users sign in again.
- **[Storage cleanup fails after database commit]** → Baseline contains no attachment records, so files are unreachable; report failure and make retry clear the remaining files idempotently.
- **[Known demo passwords are reachable externally]** → Retain the existing edge authentication boundary for canary and use only synthetic data. The demo credentials are not valid anywhere else.
- **[App and home-ops changes deploy out of order]** → Land and publish the reset-capable app image first, then change home-ops. Keep the purged canary suspended until both the image and production-free manifests are available.
- **[A future manifest reintroduces production recovery]** → Expand the canary-isolation task to reject `bootstrap.recovery`, production Barman references, canary backup resources, and mismatched reset targets.
- **[No canary backup limits forensic history]** → Treat CronJob logs and bounded operational metrics as reset evidence; canary data itself is deliberately non-retained.
- **[The deployed image and intended baseline drift]** → Version baseline data with application code, use the exact web image for reset, and verify the baseline identity after every load.
- **[Demo mode is accidentally enabled outside canary]** → Default it off, require exact canary target assertions before reset, and add deployment-isolation tests that reject it in production manifests.

## Migration Plan

1. Preserve the completed containment evidence: Flux is suspended, canary data-bearing resources and backups are absent, and production retains its expected identity and healthy state.
2. Implement and verify the MedTracker demo-mode boundary and notice, demo baseline, guarded reset command, and tests using Red-Green-Refactor.
3. Publish a candidate image containing the reset capability.
4. In a fresh `home-ops` worktree, enable `DEMO_MODE` only for canary and update its HelmRelease and isolation task; reduce CNPG to one `initdb` instance; remove production recovery, WAL archiving, every database backup resource, ObjectStore, and canary backup credentials.
5. Render and validate the app, database, storage, and monitoring trees without changing the live cluster.
6. Reconcile the database Kustomization first and allow CNPG to create a blank database. Reconcile the reset-capable application only after the database is healthy.
7. Run migrations, then trigger the CronJob once manually to load and verify the baseline before enabling canary traffic.
8. Start canary, sign in as a demo user, verify scheduled and PRN scenarios, register a test device, and verify one canary notification.
9. Leave the CronJob enabled for the next Sunday 04:15 Europe/London run and verify its first scheduled completion.

The old canary state is intentionally unrecoverable. Rollback means redeploying a known-good reset-capable image and regenerating the synthetic baseline. Production resources are never rollback inputs or targets.

## Open Questions

None.
