-- Verifies a database provisioned from db/schema.sql carries the objects that
-- a Rails db:prepare/schema.rb load silently drops. Exits non-zero under
-- psql -v ON_ERROR_STOP=1 when any check fails.
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_trigger t
      JOIN pg_class c ON c.oid = t.tgrelid
      JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE NOT t.tgisinternal
      AND n.nspname = 'public'
      AND c.relname = 'versions'
      AND t.tgname = 'append_versions_to_audit_ledger'
  ) THEN
    RAISE EXCEPTION 'schema verification failed: append_versions_to_audit_ledger trigger missing';
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_trigger t
      JOIN pg_class c ON c.oid = t.tgrelid
      JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE NOT t.tgisinternal
      AND n.nspname = 'public'
      AND c.relname = 'security_audit_events'
      AND t.tgname = 'append_security_events_to_audit_ledger'
  ) THEN
    RAISE EXCEPTION 'schema verification failed: append_security_events_to_audit_ledger trigger missing';
  END IF;

  IF (
    SELECT count(*) FROM pg_proc p
      JOIN pg_namespace n ON n.oid = p.pronamespace
    WHERE n.nspname = 'public'
      AND p.proname IN (
        'audit_append_ledger_entry',
        'audit_capture_source_row',
        'audit_record_signed_checkpoint'
      )
  ) <> 3 THEN
    RAISE EXCEPTION 'schema verification failed: audit ledger functions missing';
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_views
    WHERE schemaname = 'public' AND viewname = 'household_audit_ledger_entries'
  ) THEN
    RAISE EXCEPTION 'schema verification failed: household_audit_ledger_entries view missing';
  END IF;

  IF (
    SELECT count(*) FROM pg_policies
    WHERE schemaname = 'public' AND policyname = 'household_tenant_isolation'
  ) <> 27 THEN
    RAISE EXCEPTION 'schema verification failed: tenant isolation policies missing';
  END IF;

  IF (SELECT count(*) FROM public.schema_migrations) = 0 THEN
    RAISE EXCEPTION 'schema verification failed: schema_migrations is empty';
  END IF;
END
$$;
