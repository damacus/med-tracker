DO $$
DECLARE tables text;
BEGIN
  IF current_database() <> 'medtracker_reference' OR current_user <> 'medtracker'
     OR NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'medtracker_browser_runtime') THEN
    RAISE EXCEPTION 'Browser reset requires the owned reference database';
  END IF;
  ALTER TABLE public.versions DROP CONSTRAINT IF EXISTS synthetic_order_audit_failure;
  ALTER TABLE public.versions DROP CONSTRAINT IF EXISTS synthetic_treatment_audit_failure;
  ALTER TABLE public.versions DROP CONSTRAINT IF EXISTS browser_oauth_audit_failure;
  ALTER TABLE public.security_audit_events DROP CONSTRAINT IF EXISTS browser_oauth_audit_failure;
  DROP TRIGGER IF EXISTS browser_revoke_grant_on_prompt ON public.medication_review_prompts;
  DROP TRIGGER IF EXISTS browser_revoke_session_on_prompt ON public.medication_review_prompts;
  DROP FUNCTION IF EXISTS browser_revoke_grant_on_prompt();
  DROP FUNCTION IF EXISTS browser_revoke_session_on_prompt();
  SELECT string_agg(format('%I.%I', schemaname, tablename), ',') INTO tables
    FROM pg_tables WHERE schemaname = 'public' AND tablename <> 'seaql_migrations';
  EXECUTE 'TRUNCATE ' || tables || ' RESTART IDENTITY';
END $$;
