CREATE POLICY support_expiry_audit_insert ON public.security_audit_events
FOR INSERT TO med_tracker_owner WITH CHECK (
  event_type = 'support_access_session.expired'
  AND actor_membership_id IS NULL
  AND request_id IS NULL AND ip IS NULL
  AND audit_context->>'active_role' = 'support_expiry_task'
  AND audit_context->>'authentication_method' = 'system'
  AND EXISTS (
    SELECT 1 FROM public.support_access_sessions AS s
    JOIN public.platform_admins AS a ON a.id = s.platform_admin_id
    WHERE s.id::text = metadata->>'support_access_session_id'
      AND s.household_id = security_audit_events.household_id
      AND s.expired_at IS NOT NULL AND s.ended_at IS NULL
      AND a.account_id = security_audit_events.actor_account_id
      AND a.account_id::text = audit_context->>'actor_account_id'
  )
);

CREATE FUNCTION med_tracker.due_support_access_sessions(p_after bigint, p_limit integer)
RETURNS TABLE(id bigint)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog SET TimeZone = 'UTC' AS $$
BEGIN
  IF p_after IS NULL OR p_after < 0 OR p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 500 THEN
    RAISE EXCEPTION USING ERRCODE='22023', MESSAGE='Invalid support expiry page';
  END IF;
  RETURN QUERY SELECT s.id FROM public.support_access_sessions AS s
    WHERE s.id > p_after AND s.ended_at IS NULL AND s.expired_at IS NULL
      AND s.expires_at <= pg_catalog.clock_timestamp()
    ORDER BY s.id LIMIT p_limit;
END;
$$;
ALTER FUNCTION med_tracker.due_support_access_sessions(bigint,integer) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.due_support_access_sessions(bigint,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.due_support_access_sessions(bigint,integer) TO med_tracker_app;

CREATE FUNCTION med_tracker.expire_support_access_session(p_id bigint)
RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog SET TimeZone = 'UTC' AS $$
DECLARE
  v_session public.support_access_sessions%ROWTYPE;
  v_actor bigint;
  v_context jsonb;
BEGIN
  PERFORM s.id FROM public.support_access_sessions AS s
    WHERE s.id=p_id AND s.ended_at IS NULL AND s.expired_at IS NULL
      AND s.expires_at <= pg_catalog.clock_timestamp()
    FOR UPDATE SKIP LOCKED;
  IF NOT FOUND THEN RETURN false; END IF;
  UPDATE public.support_access_sessions AS s
    SET expired_at=pg_catalog.clock_timestamp(), updated_at=pg_catalog.clock_timestamp()
    WHERE s.id=p_id AND s.ended_at IS NULL AND s.expired_at IS NULL
      AND s.expires_at <= pg_catalog.clock_timestamp()
    RETURNING s.* INTO v_session;
  IF NOT FOUND THEN RETURN false; END IF;
  SELECT a.account_id INTO STRICT v_actor FROM public.platform_admins AS a
    WHERE a.id=v_session.platform_admin_id;
  v_context := pg_catalog.jsonb_build_object(
    'actor_account_id', v_actor, 'actor_user_id', NULL,
    'active_role', 'support_expiry_task', 'authentication_method', 'system');
  INSERT INTO public.versions(item_type,item_id,event,object,whodunnit,request_id,
    audit_context,household_id,created_at)
  VALUES('SupportAccessSession',p_id,'platform/support/expired',
    pg_catalog.jsonb_build_object('household_id',v_session.household_id)::text,
    NULL,NULL,v_context,v_session.household_id,pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()));
  INSERT INTO public.security_audit_events(household_id,actor_account_id,actor_membership_id,
    event_type,metadata,audit_context,request_id,ip,created_at,updated_at)
  VALUES(v_session.household_id,v_actor,NULL,'support_access_session.expired',
    pg_catalog.jsonb_build_object('support_access_session_id',p_id,
      'expired_at',pg_catalog.to_char(v_session.expires_at,'YYYY-MM-DD"T"HH24:MI:SS"Z"'),
      'outcome','expired'),
    v_context,NULL,NULL,pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()),
    pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()));
  RETURN true;
END;
$$;
ALTER FUNCTION med_tracker.expire_support_access_session(bigint) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.expire_support_access_session(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.expire_support_access_session(bigint) TO med_tracker_app;
