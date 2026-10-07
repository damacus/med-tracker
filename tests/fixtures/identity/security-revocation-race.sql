BEGIN;
SET LOCAL lock_timeout='5s';
SET LOCAL statement_timeout='10s';
SELECT id FROM public.accounts WHERE id=71001 FOR UPDATE;
SELECT set_config('application_name','security-revocation-race',true);
DO $$
DECLARE deadline timestamptz := clock_timestamp()+interval '5 seconds';
BEGIN
  LOOP
    EXIT WHEN EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND pg_backend_pid()=ANY(pg_blocking_pids(pid)));
    IF clock_timestamp()>=deadline THEN
      RAISE EXCEPTION 'Security operation did not reach the held account lock';
    END IF;
    PERFORM pg_sleep(0.02);
  END LOOP;
  DELETE FROM public.identity_sessions WHERE account_id=71001;
END;
$$;
COMMIT;
