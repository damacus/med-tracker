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
7. Explicit support access end is audited; expiry audit automation remains a hosted hardening gap.

## Export and purge

Hosted beta uses configurable retention with export + purge as the default deployer
policy unless an explicit retention hold exists.

1. Verify the requester is an active owner of the household or an authorized platform admin in support mode.
2. Generate household export from household-scoped queries only.
3. Audit export request, generation, download, and expiry without storing raw medication names or dose notes in the audit metadata.
4. Revoke household memberships, API sessions, API app tokens, push subscriptions, and native device tokens during offboarding.
5. If no retention hold exists, purge tenant-owned rows and tenant-owned attachments.
6. If a retention hold exists, archive the household, disable access, document the hold reason and review date, and keep data out of normal household flows.

## Restore test

1. Restore the latest backup into an isolated environment.
2. Run migrations using the owner-capable migration role.
3. Run the app using `DATABASE_ROLE=med_tracker_app`.
4. Verify RLS default-denies without tenant context.
5. Verify each restored household can only see its own people, medicines, schedules, notifications, audit rows, and attachments.
6. Record restore date, backup identifier, app image/tag, migration version, tester, and outcome.
7. Run combined audit verification and compare restored chain heads with signed WORM checkpoints.
8. Treat a valid but older database chain as restore divergence; do not delete newer WORM evidence.

## Incident response

1. Disable affected credentials or households.
2. Preserve audit rows, support access sessions, logs, and request ids.
3. Export a tenant-scoped evidence bundle for investigation.
4. Patch and verify in an isolated environment.
5. Re-run the go/no-go checklist before re-enabling affected access.
6. Preserve signed manifests, public keys, Object Lock versions, delivery receipts, and verifier JSON output.
