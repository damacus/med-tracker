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
- The 0.5 release will run migrations with `DATABASE_ROLE=med_tracker_owner`.

Do not run the web process as the migration role. The app runtime should use
`DATABASE_ROLE=med_tracker_app`.

## One-time bootstrap

Run this SQL as a database administrator, superuser, or a role allowed to create
and manage PostgreSQL roles. Replace `<database_name>` and `<app_login_role>`
with the actual database and login role names. Quote names that contain hyphens.

```sql
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_owner') THEN
    CREATE ROLE med_tracker_owner NOLOGIN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app') THEN
    CREATE ROLE med_tracker_app NOLOGIN;
  END IF;
END
$$;

ALTER ROLE med_tracker_owner NOLOGIN;
ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;

GRANT med_tracker_owner TO <app_login_role>;
GRANT med_tracker_app TO <app_login_role>;

ALTER DATABASE <database_name> OWNER TO med_tracker_owner;
ALTER SCHEMA public OWNER TO med_tracker_owner;

GRANT CONNECT ON DATABASE <database_name> TO med_tracker_owner, med_tracker_app;
GRANT USAGE, CREATE ON SCHEMA public TO med_tracker_owner;
GRANT USAGE ON SCHEMA public TO med_tracker_app;

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'med_tracker') THEN
    GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner, med_tracker_app;
  END IF;
END
$$;
```

The app login should remain a normal non-superuser role. Do not grant it
`CREATEROLE`, `BYPASSRLS`, or admin option on the runtime roles for normal
upgrades.

## Preflight check

After the bootstrap SQL and before running migrations, run the read-only Rails
preflight from a Rails container using the same database login the deployment
uses:

```bash
rails med_tracker:pre_0_5_database_upgrade_preflight
```

The preflight checks that both runtime roles exist, are `NOLOGIN`, are not
superusers, do not have `BYPASSRLS`, and that the app login is a member of both
roles. If it fails, fix the bootstrap state before running `db:prepare`.

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
      DATABASE_ROLE: med_tracker_owner

containers:
  app:
    env:
      DATABASE_ROLE: med_tracker_app
```

With Flux or another GitOps controller, commit those values to the source repo
before reconciling the release.

## Verification

After the migration and app startup complete, verify the role state:

```sql
SELECT rolname, rolcreaterole, rolsuper, rolbypassrls
FROM pg_roles
WHERE rolname IN ('med_tracker_owner', 'med_tracker_app', '<app_login_role>')
ORDER BY rolname;

SELECT m.roleid::regrole AS granted_role,
       m.member::regrole AS member_role,
       m.admin_option
FROM pg_auth_members m
WHERE m.roleid IN ('med_tracker_owner'::regrole, 'med_tracker_app'::regrole)
ORDER BY 1::text, 2::text;
```

Expected result:

- `med_tracker_owner` is `NOLOGIN`, not superuser, and not `BYPASSRLS`.
- `med_tracker_app` is `NOLOGIN`, not superuser, and not `BYPASSRLS`.
- The app login is a member of both runtime roles.
- `admin_option` is `false` for the app login memberships.

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

## If an earlier 0.5 attempt failed

If a 0.5 migration already enabled forced row-level security and then failed,
upgrade to a release containing this runbook before retrying. The strict
household migration temporarily removes `FORCE ROW LEVEL SECURITY` while it
backfills legacy rows, then restores forced RLS before it completes.

This avoids the emergency-only workaround of granting `BYPASSRLS` to the
migration role.
