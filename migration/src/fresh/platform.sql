CREATE FUNCTION med_tracker.platform_accounts(search_pattern text, requested_page bigint)
RETURNS TABLE(total bigint,id bigint,name text,email text,status integer,sign_in_enabled boolean,administrator text)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=pg_catalog
AS $$
BEGIN
  IF NOT EXISTS(SELECT 1 FROM public.platform_admins p JOIN public.accounts a ON a.id=p.account_id
    WHERE a.id=med_tracker.current_account_id() AND p.status='active' AND a.status=2 AND a.disabled_at IS NULL
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)) THEN
    RAISE EXCEPTION 'Platform authority unavailable' USING ERRCODE='MT001';
  END IF;
  RETURN QUERY
    WITH matching AS MATERIALIZED (
      SELECT a.id,a.name,a.email,a.status,a.disabled_at,p.status AS administrator
      FROM public.accounts a LEFT JOIN public.platform_admins p ON p.account_id=a.id
      WHERE a.email ILIKE search_pattern ESCAPE '\'
    ), totals AS (SELECT count(*) AS total FROM matching)
    SELECT totals.total,entry.id,entry.name::text,entry.email::text,entry.status,
      entry.status=2 AND entry.disabled_at IS NULL,entry.administrator::text
    FROM totals LEFT JOIN LATERAL (
      SELECT m.* FROM matching m ORDER BY m.id LIMIT 25
      OFFSET (least(greatest(COALESCE(requested_page,1),1),greatest((totals.total+24)/25,1))-1)*25
    ) entry ON true ORDER BY entry.id;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.platform_accounts(text,bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.platform_accounts(text,bigint) TO med_tracker_app;

CREATE POLICY platform_session_revoke_read ON public.identity_sessions FOR SELECT TO med_tracker_owner
USING (EXISTS(SELECT 1 FROM med_tracker.platform_account_state(account_id)));
CREATE POLICY platform_session_revoke_write ON public.identity_sessions FOR UPDATE TO med_tracker_owner
USING (EXISTS(SELECT 1 FROM med_tracker.platform_account_state(account_id)))
WITH CHECK (NOT active AND EXISTS(SELECT 1 FROM med_tracker.platform_account_state(account_id)));
CREATE POLICY platform_key_revoke_read ON public.identity_api_keys FOR SELECT TO med_tracker_owner
USING (EXISTS(SELECT 1 FROM med_tracker.platform_account_state(account_id)));
CREATE POLICY platform_key_revoke_write ON public.identity_api_keys FOR UPDATE TO med_tracker_owner
USING (EXISTS(SELECT 1 FROM med_tracker.platform_account_state(account_id)))
WITH CHECK (payload->>'enabled'='false' AND EXISTS(SELECT 1 FROM med_tracker.platform_account_state(account_id)));

CREATE FUNCTION med_tracker.set_platform_account_active(target_account bigint, enabled boolean, session_reference text, trace_id text, actor_ip text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog
AS $$
DECLARE actor_person bigint; target_status integer; event_name text; actor_context jsonb;
BEGIN
  PERFORM pg_advisory_xact_lock(1920296809,3);
  SELECT a.person_id INTO actor_person FROM public.accounts a
    JOIN public.platform_admins p ON p.account_id=a.id
    WHERE a.id=med_tracker.current_account_id() AND a.status=2 AND a.disabled_at IS NULL AND p.status='active'
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP);
  IF NOT FOUND THEN RAISE EXCEPTION 'Platform authority unavailable' USING ERRCODE='MT001'; END IF;
  IF enabled IS NULL THEN RAISE EXCEPTION 'Platform availability required' USING ERRCODE='MT002'; END IF;
  SELECT a.status INTO target_status FROM public.accounts a WHERE a.id=target_account FOR UPDATE;
  IF NOT FOUND OR (enabled AND target_status<>2) THEN
    RAISE EXCEPTION 'Platform target unavailable' USING ERRCODE='MT002';
  END IF;
  IF NOT enabled AND EXISTS(SELECT 1 FROM public.platform_admins p JOIN public.accounts a ON a.id=p.account_id
      WHERE p.account_id=target_account AND p.status='active' AND a.status=2 AND a.disabled_at IS NULL
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP))
    AND (SELECT count(*) FROM public.platform_admins p JOIN public.accounts a ON a.id=p.account_id
      WHERE p.status='active' AND a.status=2 AND a.disabled_at IS NULL
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP))<=1 THEN
    RAISE EXCEPTION 'Last usable platform administrator' USING ERRCODE='MT003';
  END IF;
  IF NOT enabled THEN
    UPDATE public.identity_sessions SET active=false,updated_at=clock_timestamp() WHERE account_id=target_account AND active;
    UPDATE public.api_sessions SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=target_account AND revoked_at IS NULL;
    UPDATE public.api_app_tokens SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=target_account AND revoked_at IS NULL;
    UPDATE public.oauth_grants SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=target_account AND revoked_at IS NULL;
    UPDATE public.identity_api_keys SET payload=jsonb_set(payload,'{enabled}','false'::jsonb) WHERE account_id=target_account;
    DELETE FROM public.account_active_session_keys WHERE account_id=target_account;
  END IF;
  UPDATE public.accounts SET disabled_at=CASE WHEN enabled THEN NULL ELSE COALESCE(disabled_at,timezone('UTC',clock_timestamp())) END,
    updated_at=timezone('UTC',clock_timestamp()) WHERE id=target_account;
  event_name := CASE WHEN enabled THEN 'platform/user/activated' ELSE 'platform/user/deactivated' END;
  actor_context := jsonb_build_object('actor_account_id',med_tracker.current_account_id(),'actor_person_id',actor_person,
    'active_role','platform_admin','authentication_method','browser_session','session_reference',session_reference,'request_id',trace_id,'ip',actor_ip);
  INSERT INTO public.versions(item_type,item_id,event,object,whodunnit,request_id,audit_context,household_id,created_at)
    VALUES('Account',target_account,event_name,jsonb_build_object('account_id',target_account,'active',enabled)::text,
      actor_person::text,trace_id,actor_context,NULL,timezone('UTC',clock_timestamp()));
END
$$;
REVOKE ALL ON FUNCTION med_tracker.set_platform_account_active(bigint,boolean,text,text,text) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.set_platform_account_active(bigint,boolean,text,text,text) TO med_tracker_app;

CREATE FUNCTION med_tracker.platform_recovery_households(search_pattern text, selected_household bigint)
RETURNS TABLE(id bigint,name text,slug text,has_owner boolean)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=pg_catalog
AS $$
BEGIN
  IF NOT EXISTS(SELECT 1 FROM med_tracker.platform_account_state(med_tracker.current_account_id())) THEN
    RAISE EXCEPTION 'Platform authority unavailable' USING ERRCODE='MT001';
  END IF;
  RETURN QUERY SELECT h.id,h.name::text,h.slug::text,
    CASE WHEN selected_household IS NULL THEN false ELSE EXISTS(
      SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.person_id=m.person_id
      JOIN public.people p ON p.id=a.person_id
      WHERE m.household_id=h.id AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL
        AND a.status=2 AND a.disabled_at IS NULL AND p.person_type=0 AND p.has_capacity
        AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)
    ) END
    FROM public.households h WHERE h.status='active' AND h.lifecycle_state='active'
      AND (selected_household IS NULL OR h.id=selected_household)
      AND (h.name ILIKE search_pattern ESCAPE '\' OR h.slug ILIKE search_pattern ESCAPE '\')
    ORDER BY h.name,h.id LIMIT 50;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.platform_recovery_households(text,bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.platform_recovery_households(text,bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.platform_recovery_members(selected_household bigint, requested_page bigint DEFAULT 1)
RETURNS TABLE(total bigint,id bigint,name text,email text)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=pg_catalog
AS $$
BEGIN
  IF NOT EXISTS(SELECT 1 FROM med_tracker.platform_account_state(med_tracker.current_account_id())) THEN
    RAISE EXCEPTION 'Platform authority unavailable' USING ERRCODE='MT001';
  END IF;
  RETURN QUERY WITH matching AS MATERIALIZED (
    SELECT m.id,a.name,a.email FROM public.household_memberships m
    JOIN public.accounts a ON a.person_id=m.person_id JOIN public.people p ON p.id=a.person_id
    WHERE m.household_id=selected_household AND m.role<>'owner' AND m.status='active' AND m.revoked_at IS NULL
      AND a.id<>med_tracker.current_account_id() AND a.status=2 AND a.disabled_at IS NULL
      AND p.person_type=0 AND p.has_capacity
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)
      AND EXISTS(SELECT 1 FROM med_tracker.platform_recovery_households('%',selected_household) h WHERE NOT h.has_owner)
  ), totals AS (SELECT count(*) AS total FROM matching)
  SELECT totals.total,entry.id,entry.name::text,entry.email::text FROM totals LEFT JOIN LATERAL (
    SELECT m.* FROM matching m ORDER BY m.id LIMIT 50
    OFFSET (least(greatest(COALESCE(requested_page,1),1),greatest((totals.total+49)/50,1))-1)*50
  ) entry ON true ORDER BY entry.id;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.platform_recovery_members(bigint,bigint) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.platform_recovery_members(bigint,bigint) TO med_tracker_app;

CREATE FUNCTION med_tracker.check_household_owner_recovery(selected_household bigint, target_membership bigint, recovery_reason text)
RETURNS void LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=pg_catalog
AS $$
BEGIN
  IF NOT EXISTS(SELECT 1 FROM med_tracker.platform_account_state(med_tracker.current_account_id())) THEN
    RAISE EXCEPTION 'Platform authority unavailable' USING ERRCODE='MT001';
  END IF;
  IF recovery_reason IS NULL OR char_length(btrim(recovery_reason)) NOT BETWEEN 1 AND 2000 THEN
    RAISE EXCEPTION 'Recovery reason required' USING ERRCODE='MT002';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM public.households h WHERE h.id=selected_household AND h.status='active' AND h.lifecycle_state='active') THEN
    RAISE EXCEPTION 'Household unavailable' USING ERRCODE='MT004';
  END IF;
  IF EXISTS(SELECT 1 FROM med_tracker.platform_recovery_households('%',selected_household) h WHERE h.has_owner) THEN
    RAISE EXCEPTION 'Household already has a usable owner' USING ERRCODE='MT003';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.id=target_membership AND m.household_id=selected_household) THEN
    RAISE EXCEPTION 'Participation unavailable' USING ERRCODE='MT004';
  END IF;
  IF EXISTS(SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.person_id=m.person_id
    WHERE m.id=target_membership AND a.id=med_tracker.current_account_id()) THEN
    RAISE EXCEPTION 'Self recovery unavailable' USING ERRCODE='MT001';
  END IF;
  IF NOT EXISTS(SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.person_id=m.person_id
    JOIN public.people p ON p.id=a.person_id
    WHERE m.id=target_membership AND m.household_id=selected_household AND m.role<>'owner'
      AND m.status='active' AND m.revoked_at IS NULL AND a.status=2 AND a.disabled_at IS NULL
      AND p.person_type=0 AND p.has_capacity
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)) THEN
    RAISE EXCEPTION 'Recovery target ineligible' USING ERRCODE='MT002';
  END IF;
END
$$;
REVOKE ALL ON FUNCTION med_tracker.check_household_owner_recovery(bigint,bigint,text) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.check_household_owner_recovery(bigint,bigint,text) TO med_tracker_app;

CREATE FUNCTION med_tracker.recover_household_owner(selected_household bigint, target_membership bigint, recovery_reason text, session_reference text, trace_id text, actor_ip text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog
AS $$
DECLARE actor_person bigint; target_account bigint; actor_context jsonb;
BEGIN
  PERFORM pg_advisory_xact_lock(1920296809,3);
  PERFORM med_tracker.check_household_owner_recovery(selected_household,target_membership,recovery_reason);
  SELECT a.person_id INTO actor_person FROM public.accounts a WHERE a.id=med_tracker.current_account_id() FOR UPDATE;
  PERFORM 1 FROM public.platform_admins p WHERE p.account_id=med_tracker.current_account_id() FOR SHARE;
  SELECT a.id INTO target_account FROM public.accounts a JOIN public.household_memberships m ON m.person_id=a.person_id
    WHERE m.id=target_membership;
  PERFORM 1 FROM public.accounts a WHERE a.id=target_account FOR UPDATE;
  PERFORM 1 FROM public.people p JOIN public.accounts a ON a.person_id=p.id WHERE a.id=target_account FOR SHARE OF p;
  PERFORM 1 FROM public.households h WHERE h.id=selected_household FOR UPDATE;
  PERFORM 1 FROM public.household_memberships m WHERE m.household_id=selected_household ORDER BY m.id FOR UPDATE;
  PERFORM med_tracker.check_household_owner_recovery(selected_household,target_membership,recovery_reason);
  UPDATE public.household_memberships SET role='owner',permissions_version=permissions_version+1,
    updated_at=timezone('UTC',clock_timestamp()) WHERE id=target_membership AND household_id=selected_household;
  actor_context := jsonb_build_object('actor_account_id',med_tracker.current_account_id(),'actor_person_id',actor_person,
    'active_role','platform_admin','authentication_method','browser_session','session_reference',session_reference,
    'request_id',trace_id,'ip',actor_ip,'household_id',selected_household);
  INSERT INTO public.versions(item_type,item_id,event,object,whodunnit,request_id,audit_context,household_id,created_at)
    VALUES('HouseholdMembership',target_membership,'platform/owner_recovered',
      jsonb_build_object('household_id',selected_household,'membership_id',target_membership,
        'account_id',med_tracker.current_account_id(),'reason',btrim(recovery_reason))::text,
      actor_person::text,trace_id,actor_context,selected_household,timezone('UTC',clock_timestamp()));
  INSERT INTO public.api_change_events(household_id,account_id,action,record_type,record_id,request_id,metadata,occurred_at,created_at,updated_at)
    VALUES(selected_household,med_tracker.current_account_id(),'update','HouseholdMembership',target_membership,trace_id,
      actor_context,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
END
$$;
REVOKE ALL ON FUNCTION med_tracker.recover_household_owner(bigint,bigint,text,text,text,text) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.recover_household_owner(bigint,bigint,text,text,text,text) TO med_tracker_app;
