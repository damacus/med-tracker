# Pre-0.5 database upgrade

MedTracker 0.5 adds PostgreSQL runtime roles and strict household row-level
security. Existing databases created before 0.5 need a one-time database-admin
bootstrap before the application migration runs.

New empty installs that run `compose/init-roles.sql` before Rails starts do not
need this manual bootstrap.

## Who needs this

Run this bootstrap when all of these are true:

- The database was created by a MedTracker release before 0.5.
- The deployment uses PostgreSQL roles without app-superuser privileges.
- The release will use distinct migration and runtime login credentials.

Do not give the runtime process the migration credential or the bootstrap
credential.

## One-time bootstrap

Stop web and job workloads. From the matching release checkout, run the shared
bootstrap artifact as a database administrator or another role allowed to create
and manage PostgreSQL roles:

```bash
psql "$BOOTSTRAP_DATABASE_URL" \
  --set=runtime_password="$RUNTIME_DATABASE_PASSWORD" \
  --set=migration_password="$MIGRATION_DATABASE_PASSWORD" \
  --set=auxiliary_password="$AUXILIARY_DATABASE_PASSWORD" \
  --file compose/init-roles.sql
```

The SQL is idempotent and applies to the database named by
`BOOTSTRAP_DATABASE_URL`. It creates `medtracker_migration`,
`medtracker_runtime`, and `medtracker_auxiliary` as non-superuser logins. The
memberships are deliberately narrow:

```sql
GRANT med_tracker_owner TO medtracker_migration WITH INHERIT FALSE, SET TRUE;
GRANT med_tracker_app TO medtracker_runtime WITH INHERIT FALSE, SET TRUE;
```

Provision separate databases owned by `medtracker_auxiliary` for Solid Queue,
Solid Cache, and Solid Cable. Revoke public connection access to those databases
and grant it only to the auxiliary login. Do not grant the auxiliary login any
role on the primary database.

## Preflight check

After the bootstrap SQL and before running migrations, run the read-only Rails
preflight through `MIGRATION_DATABASE_URL` with
`DATABASE_ROLE=med_tracker_owner`:

```bash
rails med_tracker:pre_0_5_database_upgrade_preflight
```

The preflight checks that the group and login roles have safe attributes, that
the migration login can set only `med_tracker_owner`, and that the runtime login
can set only `med_tracker_app`. If it fails, fix the bootstrap state before
running `db:migrate`.

## Account access bootstrap

The 0.5 household cutover replaces legacy `users.role` authorization with
household memberships and person access grants. Upgrades from pre-0.5 databases
must preserve three pieces of access state:

- every account-linked person needs an active `household_membership`;
- every active membership linked to a person needs a self `manage`
  `person_access_grant`;
- active `carer_relationships` need matching grants for the patient person.

MedTracker includes an idempotent Rails migration to backfill that state. It
promotes one active membership to `owner` only when a household has no active
owner, preferring an active `platform_admin` account when one exists. Do not
manually deactivate accounts to recover access; missing membership/grant rows
are the usual failure mode.

## Kubernetes and CNPG

For CloudNativePG, run the bootstrap against the primary PostgreSQL pod. Example:

```bash
kubectl exec -n <namespace> <primary-postgres-pod> -c postgres -- \
  psql -d <database_name> -v ON_ERROR_STOP=1
```

Paste the bootstrap SQL into that `psql` session, then update the workload
configuration:

```yaml
initContainers:
  migrate:
    env:
      DATABASE_URL: ${MIGRATION_DATABASE_URL}
      DATABASE_ROLE: med_tracker_owner

containers:
  app:
    env:
      DATABASE_URL: ${RUNTIME_DATABASE_URL}
      DATABASE_ROLE: med_tracker_app
```

Do not add `BOOTSTRAP_DATABASE_URL` to either workload. Store the bootstrap
credential only in the operator-controlled process that runs
`compose/init-roles.sql`, then remove it after the bootstrap succeeds.

With Flux or another GitOps controller, commit those values to the source repo
before reconciling the release.

## Verification

After the migration and app startup complete, verify the role state:

```sql
SELECT rolname, rolcreaterole, rolcreatedb, rolreplication, rolsuper, rolbypassrls
FROM pg_roles
WHERE rolname IN (
  'med_tracker_owner',
  'med_tracker_app',
  'medtracker_migration',
  'medtracker_runtime',
  'medtracker_auxiliary'
)
ORDER BY rolname;

SELECT m.roleid::regrole AS granted_role,
       m.member::regrole AS member_role,
       m.inherit_option,
       m.set_option,
       m.admin_option
FROM pg_auth_members m
WHERE m.roleid IN ('med_tracker_owner'::regrole, 'med_tracker_app'::regrole)
ORDER BY 1::text, 2::text;
```

Expected result:

- `med_tracker_owner` is `NOLOGIN`, not superuser, and cannot create roles,
  create databases, replicate, or bypass RLS.
- `med_tracker_app` is `NOLOGIN`, not superuser, and cannot create roles,
  create databases, replicate, or bypass RLS.
- `medtracker_migration`, `medtracker_runtime`, and `medtracker_auxiliary` are
  login roles, but are not superusers and cannot create roles, create databases,
  replicate, or bypass RLS.
- `medtracker_migration` has only the `med_tracker_owner` membership.
- `medtracker_runtime` has only the `med_tracker_app` membership.
- Both memberships have `inherit_option=false`, `set_option=true`, and
  `admin_option=false`.
- `medtracker_auxiliary` has neither membership and cannot connect to the primary
  database.

Connect through `RUNTIME_DATABASE_URL` and verify `SET ROLE med_tracker_app`
succeeds while `SET ROLE med_tracker_owner` is denied. Connect through
`MIGRATION_DATABASE_URL` and verify the inverse. Do not test these credentials
against production from an unapproved network path.

Verify household backfill and migration completion:

```sql
SET row_security = off;

SELECT COUNT(*) AS households
FROM households;

SELECT 'people' AS table_name, COUNT(*) FILTER (WHERE household_id IS NULL) AS null_household_id FROM people
UNION ALL SELECT 'locations', COUNT(*) FILTER (WHERE household_id IS NULL) FROM locations
UNION ALL SELECT 'location_memberships', COUNT(*) FILTER (WHERE household_id IS NULL) FROM location_memberships
UNION ALL SELECT 'medications', COUNT(*) FILTER (WHERE household_id IS NULL) FROM medications
UNION ALL SELECT 'dosages', COUNT(*) FILTER (WHERE household_id IS NULL) FROM dosages
UNION ALL SELECT 'schedules', COUNT(*) FILTER (WHERE household_id IS NULL) FROM schedules
UNION ALL SELECT 'person_medications', COUNT(*) FILTER (WHERE household_id IS NULL) FROM person_medications
UNION ALL SELECT 'medication_takes', COUNT(*) FILTER (WHERE household_id IS NULL) FROM medication_takes
UNION ALL SELECT 'notification_preferences', COUNT(*) FILTER (WHERE household_id IS NULL) FROM notification_preferences
ORDER BY table_name;
```

Every `null_household_id` count should be `0`.

Verify the account access bootstrap:

```sql
SET row_security = off;

SELECT COUNT(*) AS account_people_without_membership
FROM people
LEFT JOIN household_memberships
  ON household_memberships.household_id = people.household_id
 AND household_memberships.account_id = people.account_id
 AND household_memberships.status = 'active'
WHERE people.household_id IS NOT NULL
  AND people.account_id IS NOT NULL
  AND household_memberships.id IS NULL;

SELECT COUNT(*) AS active_memberships_without_self_grant
FROM household_memberships
LEFT JOIN person_access_grants
  ON person_access_grants.household_membership_id = household_memberships.id
 AND person_access_grants.person_id = household_memberships.person_id
 AND person_access_grants.relationship_type = 'self'
 AND person_access_grants.access_level = 'manage'
 AND person_access_grants.revoked_at IS NULL
WHERE household_memberships.status = 'active'
  AND household_memberships.person_id IS NOT NULL
  AND person_access_grants.id IS NULL;

SELECT COUNT(*) AS ownerless_households
FROM households
WHERE NOT EXISTS (
  SELECT 1
  FROM household_memberships
  WHERE household_memberships.household_id = households.id
    AND household_memberships.role = 'owner'
    AND household_memberships.status = 'active'
);
```

All three counts should be `0`. If any count is non-zero, deploy a release that
contains the account access backfill migration and rerun migrations with
`DATABASE_ROLE=med_tracker_owner`.

## If an earlier 0.5 attempt failed

If a 0.5 migration already enabled forced row-level security and then failed,
upgrade to a release containing this runbook before retrying. The strict
household migration temporarily removes `FORCE ROW LEVEL SECURITY` while it
backfills legacy rows, then restores forced RLS before it completes.

This avoids the emergency-only workaround of granting `BYPASSRLS` to the
migration role.

If users can sign in but the app redirects to login, reports that their account
is deactivated, or shows "You are not authorized to perform this action" for
every account after a failed cutover, check the account access bootstrap queries
above before changing account statuses. That symptom usually means the deployed
app cannot see an active household membership for the signing-in account.
