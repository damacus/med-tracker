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
END
$$;

ALTER ROLE med_tracker_owner NOLOGIN;
ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;
ALTER ROLE med_tracker_audit_exporter NOLOGIN NOSUPERUSER NOBYPASSRLS;
ALTER ROLE medtracker_audit_exporter LOGIN NOSUPERUSER NOBYPASSRLS;

GRANT med_tracker_owner TO medtracker;
GRANT med_tracker_app TO medtracker;
GRANT med_tracker_audit_exporter TO medtracker_audit_exporter;

ALTER DATABASE medtracker OWNER TO med_tracker_owner;
ALTER SCHEMA public OWNER TO med_tracker_owner;

GRANT CONNECT ON DATABASE medtracker TO med_tracker_owner, med_tracker_app;
GRANT CONNECT ON DATABASE medtracker TO med_tracker_audit_exporter;
GRANT USAGE, CREATE ON SCHEMA public TO med_tracker_owner;
GRANT USAGE ON SCHEMA public TO med_tracker_app;
