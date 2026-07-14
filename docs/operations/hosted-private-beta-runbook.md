# Hosted Private Beta Runbook

This runbook covers the hosted private beta target: one Rails application and one
PostgreSQL database serving multiple independent households. The beta must remain
closed until the hosted hardening audit is green.

## Go/No-Go

Before onboarding another household, verify:

- The web process connects with `DATABASE_ROLE=med_tracker_app`.
- Migration/setup processes connect with the owner-capable migration role.
- Existing pre-0.5 databases have completed the
  [pre-0.5 database upgrade](../pre-0-5-database-upgrade.md) bootstrap.
- Hosted admin MFA enforcement is enabled with `HOSTED_ADMIN_MFA_REQUIRED=true`.
- `task rubocop`, `task test`, and `task brakeman` pass on the release branch.
- The hosted hardening audit has no `NO-GO` rows.
- Invite-only registration is pinned by environment or platform-admin policy.
- Backup and Restore test evidence exists for the current deployment.
- Support access is only available through the audited Platform admin flow.
- The audit exporter and verifier use dedicated credentials and cannot read clinical tables or mutate ledger history.
- A signed `legacy-baseline` manifest has been retained outside PostgreSQL with its limitation recorded.
- Full database and WORM verification pass; no delivery is older than five minutes.
- The records manager/DPO has approved the versioned retention schedule and Object Lock mode.

## Tenant/RLS foundation verification

Run these checks against the deployed primary database before enabling hosted
multi-household traffic:

```sql
SELECT rolname, rolsuper, rolbypassrls
FROM pg_roles
WHERE rolname IN ('med_tracker_app', 'med_tracker_owner');

SELECT pg_has_role('med_tracker_app', 'med_tracker_owner', 'member') AS app_inherits_owner,
       has_schema_privilege('med_tracker_app', 'public', 'CREATE') AS app_can_create_public;

SELECT relname
FROM pg_class
WHERE oid = ANY (ARRAY[
  'people'::regclass,
  'locations'::regclass,
  'location_memberships'::regclass,
  'medications'::regclass,
  'dosages'::regclass,
  'schedules'::regclass,
  'person_medications'::regclass,
  'medication_takes'::regclass,
  'notification_preferences'::regclass,
  'household_memberships'::regclass,
  'person_access_grants'::regclass,
  'household_invitations'::regclass,
  'household_invitation_grants'::regclass,
  'security_audit_events'::regclass,
  'active_storage_attachments'::regclass
])
AND NOT (relrowsecurity AND relforcerowsecurity);

SELECT table_name
FROM information_schema.columns
WHERE table_schema = 'public'
  AND column_name = 'household_id'
  AND table_name IN (
    'people',
    'locations',
    'location_memberships',
    'medications',
    'dosages',
    'schedules',
    'person_medications',
    'medication_takes',
    'notification_preferences',
    'household_memberships',
    'person_access_grants',
    'household_invitations',
    'household_invitation_grants',
    'security_audit_events',
    'active_storage_attachments'
  )
  AND is_nullable <> 'NO';

SELECT tablename
FROM pg_policies
WHERE schemaname = 'public'
  AND (
    lower(COALESCE(qual, '')) LIKE '%household_id is null%'
    OR lower(COALESCE(with_check, '')) LIKE '%household_id is null%'
  );
```

The role membership/create checks must return `false`; the forced-RLS,
nullable-column, and null-policy queries must return no rows. Release verification
also runs `task test TEST_FILE=spec/lib/schema_inventory_spec.rb`,
`task test TEST_FILE=spec/models/household_row_level_security_spec.rb`,
`task test TEST_FILE=spec/config/yaml_compose_spec.rb`, and
`task test TEST_FILE=spec/config/database_role_config_spec.rb`.

## Onboarding

1. Confirm the release branch has passed the go/no-go checklist.
2. Create or select the target household through the hosted onboarding flow.
3. Create the first owner by invitation only.
4. Require the owner to configure MFA/passkey before any household admin page is usable.
5. Confirm the household owner can sign in, view only their household dashboard, and invite members.
6. Confirm another household account cannot access the new household by slug, API id, direct record id, or attachment URL.

## Support access

1. Platform admin signs in with MFA/passkey.
2. Platform admin opens a support access session for one household and records a reason.
3. The support session records actor account, target household, request id, IP, reason, MFA proof time, start time, and expiry.
4. The audit event records support-session start/end without raw health data or the free-text reason.
5. Support mode is visually and technically distinct from household membership.
6. Platform admin cannot browse health or medicine data outside an active support access session.
7. Explicit support access end records `support_access_session.ended`; it never doubles as natural expiry evidence.
8. Run `task support-access:expire` on the deployment scheduler and after any scheduler outage. The command uses
   `DATABASE_ROLE=med_tracker_app`, is safe to retry, and records each natural expiry exactly once.
9. Retain the sanitized JSON output containing `event_type`, `outcome`, and `processed_count`. A successful run reports
   `support_access_session.expired`, `success`, and a non-negative count; the corresponding audit metadata contains only
   the support access session identifier, expiry timestamp, and outcome. It excludes the reason, account email, tokens,
   and health data.

## Export and purge

Set `HOUSEHOLD_EXPORT_RETENTION_DAYS` in the deployed web and task environment to
the approved export retention period. The value is clamped to 1-365 days and
defaults to 30 days. Set `HOUSEHOLD_EXPORT_GENERATION_TIMEOUT_MINUTES` to the
maximum expected generation time. It is clamped to 1-1,440 minutes and defaults
to 60 minutes; scheduled cleanup expires attached archives left by generation
that remains incomplete beyond this boundary. Retention holds override both
cleanup paths.

Generate the complete portable export before offboarding. `MEMBERSHIP_ID` must be
an active owner or administrator membership for the target household.
`ACTOR_ACCOUNT_ID` must identify that membership's account, or an active platform
administrator inside an active audited support session for the target household.

```fish
task household-lifecycle:export HOUSEHOLD_ID=123 ACTOR_ACCOUNT_ID=456 MEMBERSHIP_ID=789
```

Retain the sanitized JSON fields `event_type`, `outcome`, `household_id`,
`export_id`, and `attachment_count`. The export lifecycle record and immutable
audit ledger preserve request, generation, ready, download, expiry, and failure
transitions. Attachment entries use identifiers, byte counts, archive paths, and
SHA-256 checksums; verify the artifact checksum before transferring it.

Set `HOUSEHOLD_EXPORT_OUTPUT_ROOT` to the protected persistent directory used for
operator export transfers; it defaults to `/app/storage/exports`. Create any
destination subdirectory before running the command. Download refuses paths whose
resolved parent is outside that root and never overwrites an existing file.

```fish
task household-lifecycle:download HOUSEHOLD_ID=123 ACTOR_ACCOUNT_ID=456 EXPORT_ID=654 DESTINATION=/app/storage/exports/household-123.zip
```

The destination is passed to the container as environment data, never interpolated
into the command. The file is written with mode `0600` only after the authorized
download succeeds and its byte count and SHA-256 checksum match the generated
artifact record. Retain the sanitized `event_type`, `outcome`, `household_id`,
`export_id`, `artifact_byte_size`, and `artifact_checksum_sha256` fields. Command
output never includes the destination or export contents.

Place a retention hold only from an approved records-governance request. The
reason is stored in the protected hold record but excluded from command and audit
metadata:

```fish
task household-lifecycle:hold HOUSEHOLD_ID=123 ACTOR_ACCOUNT_ID=456 REASON="Approved legal preservation" REVIEW_ON=2026-08-13
```

Release the hold only after approval, using the same household context so forced
RLS remains effective:

```fish
task household-lifecycle:release-hold HOLD_ID=321 HOUSEHOLD_ID=123 ACTOR_ACCOUNT_ID=456
```

Offboarding immediately disables normal and support access and revokes household
memberships, browser/API sessions, app tokens, OAuth grants, web push
subscriptions, and native device tokens. The command is safe to retry and emits
one successful offboarding audit event:

```fish
task household-lifecycle:offboard HOUSEHOLD_ID=123 ACTOR_ACCOUNT_ID=456
```

Start or resume purge only after the export has been verified and offboarding has
completed:

```fish
task household-lifecycle:purge HOUSEHOLD_ID=123 ACTOR_ACCOUNT_ID=456
```

Purge refuses an active retention hold before deleting anything. It is safe to
retry after interruption: the durable purge run increments `attempts`, reports
`last_completed_table`, repeats idempotent deletions, and finishes only after every
purgeable `SchemaInventory` tenant table and household-owned attachment is empty.
Immutable `security_audit_events` and `versions` audit history are never updated or
deleted. Tenant security events remain under tenant RLS. Historical actor-membership
identifiers remain in the audit sources after the corresponding access-domain row is
purged. Completion appends one immutable `household.purge.completed` tombstone
containing only the identifiers and status fields listed below. Purge never deletes
another household's attachment or a blob still referenced elsewhere.
Successful evidence contains `event_type`, `outcome`, `household_id`,
`purge_run_id`, `attempts`, and `last_completed_table`. Failed commands emit
`event_type`, `outcome`, and `failure_code`, exit non-zero, and do not claim
completion. Investigate the application exception telemetry, correct the cause,
then run the identical task command again.

Never retain free-text reasons, attachment contents, credentials, or health data in command output.

## Restore test

Restore a current database backup and its matching attachment backup into an isolated,
production-like target using the approved platform recovery procedure. Do not point the
workflow below at a live environment. The repository command verifies an already-restored
target; it does not accept or run a raw restore command.

Before running it, select two households that each contain a representative person,
immutable security audit event, and attachment. Obtain the latest signed WORM checkpoint
for each sample from the independent Object Lock evidence store. Supply the sample ids only
as process inputs; the evidence bundle records them as `sample_a` and `sample_b` and never
records real tenant identifiers.

The production build must receive `APP_IMAGE_REF` as a build argument matching the immutable
tag or digest assigned to that image. The final image records that value in the read-only
`/app/.runtime-image-ref` file outside runtime environment configuration. The operator-provided
`APP_IMAGE` is the expected reference; runtime verification fails unless it exactly matches the
baked image reference. Setting a runtime environment variable cannot override this comparison.

```fish
set -lx DATABASE_BACKUP_ID database-snapshot-2026-07-14T010000Z
set -lx ATTACHMENT_BACKUP_ID attachments-snapshot-2026-07-14T010000Z
set -lx RESTORE_TARGET_ID isolated-restore-2026-q3
set -lx APP_IMAGE ghcr.io/damacus/med-tracker:v0.5.0-rc1
set -lx TESTER restore-operator
set -lx HOUSEHOLD_A_ID 101
set -lx HOUSEHOLD_B_ID 202
set -lx WORM_REFERENCE object-lock-checkpoint-2026-07-14
set -lx WORM_HEADS_JSON '{
  "sample_a": {
    "chain_epoch": "uuid",
    "sequence": 11,
    "entry_hash": "64-hex-characters"
  },
  "sample_b": {
    "chain_epoch": "uuid",
    "sequence": 22,
    "entry_hash": "64-hex-characters"
  }
}'
set -lx EVIDENCE_OUTPUT /approved-mounted-evidence/restore-2026-q3
set -lx EVIDENCE_ROOT /approved-mounted-evidence
task hosted-restore:rehearse
```

The command refuses missing, placeholder, duplicate sample, malformed WORM-head, relative
evidence, and existing evidence destinations before migrations run. It then runs migrations
as `med_tracker_owner`, verifies forced RLS and storage as `med_tracker_app`, and runs combined
database, signed-checkpoint, and Object Lock verification as `med_tracker_audit_verifier`.
Any failed stage stops later stages and produces a failed bundle; partial work can never claim
success. Re-run with the same backup identifiers and a new evidence destination after the
failure is remediated.

Household ids and WORM sequences must be canonical positive decimal integers, checkpoint
epochs must be canonical UUIDs, and the WORM JSON object must contain exactly `sample_a` and
`sample_b` with only `chain_epoch`, `sequence`, and `entry_hash`. The evidence output is
accepted only when its existing parent resolves by realpath below `EVIDENCE_ROOT`. Repository
root and every directory below it, operating-system temporary directories, outside paths, and
symlink aliases or escapes are refused.

The destination must be a pre-approved durable mounted evidence repository, not the source
tree or transient container filesystem. It receives mode-restricted `evidence.json` and
`evidence.md` files plus a checksum-bearing `complete.json` marker. The files are written and
fsynced in a private staging directory before the complete bundle is atomically published;
a partial write is cleaned up and cannot expose final evidence or a completion marker. The
Markdown includes only whitelisted role, schema, baked image, RLS, default-denial, isolation,
storage, audit-count, verified-head, WORM-comparison, and failure-code aggregates. Link the
durable evidence record from the NFR4 row in the hosted
hardening audit only after a second operator has inspected both files and confirmed the final
outcome is `passed`. Never copy secrets, credentials, health data, real tenant identifiers,
attachment contents, sensitive infrastructure paths, or raw command output into that link.
Each command entry records a safe fixed command description and only its whitelisted
structured output. Structured failures are parsed from the Rake task's JSON error record;
raw standard output and standard error are never copied into evidence.

Perform the rehearsal at least quarterly. A new rehearsal is required after changing the
database major version, backup or attachment storage system, encryption or Object Lock
configuration, database roles or RLS policies, migration strategy, application image release
process, tenant schema, audit chain/checkpoint format, or disaster-recovery platform. A failed
backup, missed quarterly rehearsal, unverified evidence link, or any invalidation trigger keeps
#1621 and the hosted launch gate open.

## Incident response

1. Disable affected credentials or households.
2. Preserve audit rows, support access sessions, logs, and request ids.
3. Export a tenant-scoped evidence bundle for investigation.
4. Patch and verify in an isolated environment.
5. Re-run the go/no-go checklist before re-enabling affected access.
6. Preserve signed manifests, public keys, Object Lock versions, delivery receipts, and verifier JSON output.
