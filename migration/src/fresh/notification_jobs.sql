CREATE POLICY reminder_owner_read ON public.notification_preferences FOR SELECT TO med_tracker_owner USING (true);
CREATE POLICY reminder_owner_read ON public.notification_deliveries FOR SELECT TO med_tracker_owner USING (true);
CREATE POLICY reminder_owner_terminal ON public.notification_deliveries FOR UPDATE TO med_tracker_owner
USING (status IN ('pending','sending')) WITH CHECK (status IN ('invalid','uncertain'));
GRANT SELECT,UPDATE ON public.notification_deliveries TO med_tracker_owner;

CREATE FUNCTION med_tracker.reminder_recipients(after_household bigint,after_account bigint,page_size integer)
RETURNS TABLE(account_id bigint,household_id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
  IF after_household IS NULL OR after_account IS NULL OR page_size IS NULL OR page_size<1 OR page_size>100 THEN
    RAISE EXCEPTION 'Invalid reminder page' USING ERRCODE='MT002';
  END IF;
  RETURN QUERY SELECT a.id,h.id FROM public.accounts a
  JOIN public.household_memberships m ON m.person_id=a.person_id
  JOIN public.households h ON h.id=m.household_id
  LEFT JOIN public.notification_preferences p ON p.account_id=a.id AND p.household_id=h.id
  WHERE (h.id,a.id)>(after_household,after_account)
    AND a.status=2 AND a.disabled_at IS NULL
    AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)
    AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active'
    AND (p.enabled OR EXISTS(SELECT 1 FROM public.notification_deliveries d WHERE d.account_id=a.id
      AND d.household_id=h.id AND d.status IN ('pending','sending')))
  ORDER BY h.id,a.id LIMIT page_size;
END $$;
ALTER FUNCTION med_tracker.reminder_recipients(bigint,bigint,integer) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.reminder_recipients(bigint,bigint,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.reminder_recipients(bigint,bigint,integer) TO med_tracker_app;

CREATE FUNCTION med_tracker.cleanup_reminders(target_household bigint,target_account bigint)
RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE affected bigint;
BEGIN
  WITH candidates AS (
    SELECT d.id FROM public.notification_deliveries d
    WHERE (target_household IS NULL OR d.household_id=target_household)
      AND (target_account IS NULL OR d.account_id=target_account)
      AND ((d.status='sending' AND d.claimed_at<=timezone('UTC',clock_timestamp())-interval '120 seconds')
        OR (d.status='pending' AND NOT EXISTS(
          SELECT 1 FROM public.accounts a JOIN public.household_memberships m ON m.person_id=a.person_id
          JOIN public.households h ON h.id=m.household_id
          WHERE a.id=d.account_id AND h.id=d.household_id AND a.status=2 AND a.disabled_at IS NULL
            AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)
            AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active')))
    ORDER BY d.id LIMIT 100 FOR UPDATE OF d SKIP LOCKED
  ), updated AS (
    UPDATE public.notification_deliveries d SET
      status=CASE WHEN d.status='sending' THEN 'uncertain' ELSE 'invalid' END,
      outcome=CASE WHEN d.status='sending' THEN 'abandoned_claim' ELSE 'recipient_revoked' END,
      claimed_at=NULL,updated_at=timezone('UTC',clock_timestamp())
    FROM candidates c WHERE d.id=c.id RETURNING d.id,d.household_id,d.status,d.outcome
  ) INSERT INTO public.security_audit_events(household_id,event_type,metadata,audit_context,created_at,updated_at)
    SELECT household_id,'browser_push.reminder.'||status,jsonb_build_object('delivery_id',id,'reason',outcome),
      jsonb_build_object('household_id',household_id),timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp())
    FROM updated;
  GET DIAGNOSTICS affected = ROW_COUNT;
  RETURN affected;
END $$;
ALTER FUNCTION med_tracker.cleanup_reminders(bigint,bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.cleanup_reminders(bigint,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.cleanup_reminders(bigint,bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.record_low_stock_notifications(target_medication bigint,target_take bigint,trace_id text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE target_household bigint := med_tracker.current_household_id(); actor_membership bigint; affected bigint;
BEGIN
  IF target_household IS NULL THEN
    RAISE EXCEPTION 'Household authority unavailable' USING ERRCODE='MT001';
  END IF;
  PERFORM med_tracker.lock_current_household();
  SELECT m.id INTO actor_membership FROM public.household_memberships m JOIN public.accounts a ON a.person_id=m.person_id
    WHERE a.id=med_tracker.current_account_id() AND m.household_id=target_household AND m.status='active' AND m.revoked_at IS NULL;
  IF trace_id IS NULL OR trace_id='' OR NOT EXISTS(
    SELECT 1 FROM public.medication_takes t JOIN public.medications m ON m.id=t.taken_from_medication_id AND m.household_id=t.household_id
    WHERE t.id=target_take AND t.household_id=target_household AND m.id=target_medication
      AND m.current_supply<=m.reorder_threshold) THEN
    RAISE EXCEPTION 'Stock intent unavailable' USING ERRCODE='MT002';
  END IF;
  INSERT INTO public.notification_events(household_id,person_id,event_type,event_key,metadata,created_at,updated_at)
  SELECT m.household_id,a.person_id,'browser_low_stock_pending',
    'low-stock:'||target_medication::text||':'||target_take::text||':'||a.person_id::text,
    jsonb_build_object('account_id',a.id,'medication_id',target_medication,'take_id',target_take),
    timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp())
  FROM public.household_memberships m JOIN public.accounts a ON a.person_id=m.person_id
  JOIN public.notification_preferences p ON p.account_id=a.id AND p.household_id=m.household_id
  WHERE m.household_id=target_household AND m.status='active' AND m.revoked_at IS NULL
    AND a.status=2 AND a.disabled_at IS NULL
    AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)
    AND p.enabled AND p.low_stock_enabled
    AND (EXISTS(SELECT 1 FROM public.person_medications pm WHERE pm.household_id=m.household_id
      AND pm.person_id=a.person_id AND pm.medication_id=target_medication AND pm.active)
      OR EXISTS(SELECT 1 FROM public.schedules s WHERE s.household_id=m.household_id
        AND s.person_id=a.person_id AND s.medication_id=target_medication AND s.active))
  ON CONFLICT DO NOTHING;
  GET DIAGNOSTICS affected = ROW_COUNT;
  IF affected>0 THEN
    INSERT INTO public.security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at)
    VALUES(target_household,med_tracker.current_account_id(),actor_membership,'browser_push.low_stock.queued',trace_id,
      jsonb_build_object('medication_id',target_medication,'take_id',target_take,'recipient_count',affected),
      jsonb_build_object('household_id',target_household,'actor_account_id',med_tracker.current_account_id(),'actor_membership_id',actor_membership,'request_id',trace_id),
      timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()));
  END IF;
END $$;
ALTER FUNCTION med_tracker.record_low_stock_notifications(bigint,bigint,text) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.record_low_stock_notifications(bigint,bigint,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.record_low_stock_notifications(bigint,bigint,text) TO med_tracker_app;
