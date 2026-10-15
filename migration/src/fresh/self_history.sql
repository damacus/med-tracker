CREATE FUNCTION med_tracker.self_medication_history(after_time timestamptz,after_entry bigint,starts_at timestamptz,ends_at timestamptz,page_size integer)
RETURNS TABLE(history_entry_id bigint,occurred_at timestamptz,recorded_at timestamp,stock_name text,id bigint,portable_id text,client_uuid text,updated_at timestamp,taken_at timestamp,
  dose_amount numeric,dose_unit text,household_id bigint,person_id bigint,person_portable_id text,subject_name text,
  schedule_id bigint,schedule_portable_id text,person_medication_id bigint,person_medication_portable_id text,
  medication_id bigint,medication_portable_id text,medication_name text,administration text,
  taken_from_medication_id bigint,taken_from_medication_portable_id text,taken_from_location_id bigint,
  taken_from_location_portable_id text,location_name text,actor_kind text,actor_person_id bigint,actor_membership_id bigint,actor_name text)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE subject bigint;
BEGIN
  SELECT a.person_id INTO subject FROM public.accounts a WHERE a.id=med_tracker.current_account_id()
    AND a.status=2 AND a.disabled_at IS NULL AND NOT EXISTS(
      SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>clock_timestamp());
  IF subject IS NULL THEN RAISE EXCEPTION 'Account history access denied' USING ERRCODE='42501'; END IF;
  IF ((after_time IS NULL)<>(after_entry IS NULL)) OR after_entry<=0 OR page_size IS NULL OR page_size NOT BETWEEN 1 AND 101
      OR (starts_at IS NOT NULL AND ends_at IS NOT NULL AND starts_at>=ends_at) THEN
    RAISE EXCEPTION 'History page is invalid' USING ERRCODE='23514';
  END IF;
  RETURN QUERY SELECT h.id,h.occurred_at,(h.snapshot->'event'->>'created_at')::timestamp,h.snapshot->'stock'->>'name',h.take_id,h.snapshot->'event'->>'portable_id',h.snapshot->'event'->>'client_uuid',
    (h.snapshot->'event'->>'updated_at')::timestamp,(h.snapshot->'event'->>'taken_at')::timestamp,
    (h.snapshot->'event'->>'dose_amount')::numeric,h.snapshot->'event'->>'dose_unit',h.origin_household_id,
    h.person_id,h.snapshot->'subject'->>'portable_id',h.snapshot->'subject'->>'name',
    (h.snapshot->'event'->>'schedule_id')::bigint,CASE WHEN h.snapshot->'event'->>'schedule_id' IS NOT NULL THEN h.snapshot->'source'->>'portable_id' END,
    (h.snapshot->'event'->>'person_medication_id')::bigint,CASE WHEN h.snapshot->'event'->>'person_medication_id' IS NOT NULL THEN h.snapshot->'source'->>'portable_id' END,
    (h.snapshot->'medicine'->>'id')::bigint,h.snapshot->'medicine'->>'portable_id',h.snapshot->'medicine'->>'name',h.snapshot->'source'->>'administration',
    (h.snapshot->'event'->>'taken_from_medication_id')::bigint,h.snapshot->'stock'->>'portable_id',
    (h.snapshot->'event'->>'taken_from_location_id')::bigint,h.snapshot->'location'->>'portable_id',h.snapshot->'location'->>'name',
    h.snapshot->'actor'->>'kind',(h.snapshot->'actor'->>'person_id')::bigint,(h.snapshot->'actor'->>'membership_id')::bigint,h.snapshot->'actor'->>'name'
  FROM public.clinical_history_entries h
  WHERE h.person_id=subject AND h.event_revision=1 AND (after_time IS NULL OR (h.occurred_at,h.id)>(after_time,after_entry))
    AND (starts_at IS NULL OR h.occurred_at>=starts_at) AND (ends_at IS NULL OR h.occurred_at<ends_at)
  ORDER BY h.occurred_at,h.id LIMIT page_size;
END $$;
REVOKE ALL ON FUNCTION med_tracker.self_medication_history(timestamptz,bigint,timestamptz,timestamptz,integer) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.self_medication_history(timestamptz,bigint,timestamptz,timestamptz,integer) TO med_tracker_app;
