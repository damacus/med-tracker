\set ON_ERROR_STOP on

\if :{?runtime_password}
\else
\getenv runtime_password RUNTIME_DATABASE_PASSWORD
\endif
\if :{?runtime_password}
\else
\set runtime_password 'local_runtime_only'
\endif
\if :{?migration_password}
\else
\getenv migration_password MIGRATION_DATABASE_PASSWORD
\endif
\if :{?migration_password}
\else
\set migration_password 'local_migration_only'
\endif
\if :{?auxiliary_password}
\else
\getenv auxiliary_password AUXILIARY_DATABASE_PASSWORD
\endif
\if :{?auxiliary_password}
\else
\set auxiliary_password 'local_auxiliary_only'
\endif

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_owner') THEN
    CREATE ROLE med_tracker_owner NOLOGIN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_app') THEN
    CREATE ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_audit_exporter') THEN
    CREATE ROLE med_tracker_audit_exporter NOLOGIN NOSUPERUSER NOBYPASSRLS;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_audit_exporter') THEN
    CREATE ROLE medtracker_audit_exporter LOGIN PASSWORD 'local_audit_exporter_only';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'med_tracker_audit_verifier') THEN
    CREATE ROLE med_tracker_audit_verifier NOLOGIN NOSUPERUSER NOBYPASSRLS;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_audit_verifier') THEN
    CREATE ROLE medtracker_audit_verifier LOGIN PASSWORD 'local_audit_verifier_only';
  END IF;
END
$$;

SELECT format('CREATE ROLE medtracker_runtime LOGIN PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'runtime_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_runtime') \gexec
SELECT format('CREATE ROLE medtracker_migration LOGIN PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'migration_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_migration') \gexec
SELECT format('CREATE ROLE medtracker_auxiliary LOGIN PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'auxiliary_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_auxiliary') \gexec

ALTER ROLE med_tracker_owner NOLOGIN NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS;
ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS;
SELECT format('ALTER ROLE medtracker_runtime LOGIN PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'runtime_password') \gexec
SELECT format('ALTER ROLE medtracker_migration LOGIN PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'migration_password') \gexec
SELECT format('ALTER ROLE medtracker_auxiliary LOGIN PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'auxiliary_password') \gexec
ALTER ROLE med_tracker_audit_exporter NOLOGIN NOSUPERUSER NOBYPASSRLS;
ALTER ROLE medtracker_audit_exporter LOGIN NOSUPERUSER NOBYPASSRLS;
ALTER ROLE med_tracker_audit_verifier NOLOGIN NOSUPERUSER NOBYPASSRLS;
ALTER ROLE medtracker_audit_verifier LOGIN NOSUPERUSER NOBYPASSRLS;

SELECT format('REVOKE med_tracker_owner, med_tracker_app FROM %I', current_user) \gexec
SELECT format('REVOKE %I FROM %I', granted.rolname, member.rolname)
FROM pg_auth_members memberships
JOIN pg_roles granted ON granted.oid = memberships.roleid
JOIN pg_roles member ON member.oid = memberships.member
WHERE member.rolname IN ('medtracker_auxiliary', 'medtracker_migration', 'medtracker_runtime')
ORDER BY granted.rolname, member.rolname
\gexec
GRANT med_tracker_app TO medtracker_runtime WITH ADMIN FALSE, INHERIT FALSE, SET TRUE;
GRANT med_tracker_owner TO medtracker_migration WITH ADMIN FALSE, INHERIT FALSE, SET TRUE;
GRANT med_tracker_audit_exporter TO medtracker_audit_exporter;
GRANT med_tracker_audit_verifier TO medtracker_audit_verifier;

SELECT format('ALTER DATABASE %I OWNER TO med_tracker_owner', current_database()) \gexec
ALTER SCHEMA public OWNER TO med_tracker_owner;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;

SELECT format('REVOKE CONNECT ON DATABASE %I FROM PUBLIC', current_database()) \gexec
SELECT format('GRANT CONNECT ON DATABASE %I TO med_tracker_owner, med_tracker_app', current_database()) \gexec
SELECT format('GRANT CONNECT ON DATABASE %I TO medtracker_runtime, medtracker_migration', current_database()) \gexec
SELECT format('GRANT CONNECT ON DATABASE %I TO med_tracker_audit_exporter, medtracker_audit_exporter', current_database()) \gexec
SELECT format('GRANT CONNECT ON DATABASE %I TO med_tracker_audit_verifier, medtracker_audit_verifier', current_database()) \gexec
GRANT USAGE, CREATE ON SCHEMA public TO med_tracker_owner;
GRANT USAGE ON SCHEMA public TO med_tracker_app;
ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO med_tracker_app;
ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public
GRANT USAGE, SELECT ON SEQUENCES TO med_tracker_app;
