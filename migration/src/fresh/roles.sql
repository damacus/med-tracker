DO $$
DECLARE role_name text;
BEGIN
  FOREACH role_name IN ARRAY ARRAY['med_tracker_owner','med_tracker_app','med_tracker_audit_exporter','med_tracker_audit_verifier'] LOOP
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = role_name) THEN
      BEGIN
        EXECUTE format('CREATE ROLE %I NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS', role_name);
      EXCEPTION WHEN duplicate_object OR unique_violation THEN
        NULL;
      END;
    END IF;
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = role_name AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls))
      OR EXISTS (SELECT FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.member WHERE r.rolname = role_name) THEN
      RAISE EXCEPTION 'Incompatible privileges on role %', role_name;
    END IF;
  END LOOP;
END $$;
CREATE SCHEMA med_tracker;



CREATE EXTENSION IF NOT EXISTS citext WITH SCHEMA public;

CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;
GRANT USAGE, CREATE ON SCHEMA public TO med_tracker_owner;
GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner;
ALTER TABLE public.seaql_migrations OWNER TO med_tracker_owner;
REVOKE ALL ON public.seaql_migrations FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
