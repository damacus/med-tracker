CREATE FUNCTION med_tracker.append_denied_report_request(
  p_account_id bigint, p_person_id bigint, p_membership_id bigint, p_household_id bigint,
  p_role text, p_permissions_version integer, p_method text, p_reference text,
  p_request_id text, p_report_kind text, p_status integer
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE
  v_prefix text;
BEGIN
  IF num_nulls(p_account_id,p_person_id,p_membership_id,p_household_id,p_role,
      p_permissions_version,p_method,p_reference,p_request_id,p_report_kind,p_status)>0
     OR p_status NOT IN (401,403) OR p_report_kind NOT IN ('health','reviews')
     OR p_role NOT IN ('owner','administrator','member') OR p_permissions_version<1
     OR length(p_request_id) NOT BETWEEN 1 AND 128 OR p_request_id !~ '^[A-Za-z0-9_-]+$'
     OR length(p_reference) NOT BETWEEN 1 AND 128
     OR NOT EXISTS (
       SELECT 1 FROM public.accounts a
       JOIN public.household_memberships m ON m.person_id=a.person_id
       JOIN public.households h ON h.id=m.household_id
       WHERE a.id=p_account_id AND a.person_id=p_person_id
         AND m.id=p_membership_id AND h.id=p_household_id
         AND m.permissions_version>=p_permissions_version
     ) THEN
    RAISE EXCEPTION 'Denied report audit attribution is invalid' USING ERRCODE='42501';
  END IF;
  CASE p_method
    WHEN 'api_session' THEN
      IF NOT EXISTS (SELECT 1 FROM public.api_sessions s WHERE s.id::text=p_reference
          AND s.account_id=p_account_id AND s.household_membership_id=p_membership_id
          AND s.permissions_version=p_permissions_version) THEN
        RAISE EXCEPTION 'Denied report audit attribution is invalid' USING ERRCODE='42501';
      END IF;
      v_prefix:='api_session';
    WHEN 'api_app_token' THEN
      IF NOT EXISTS (SELECT 1 FROM public.api_app_tokens s WHERE s.id::text=p_reference
          AND s.account_id=p_account_id AND s.household_membership_id=p_membership_id
          AND s.permissions_version=p_permissions_version) THEN
        RAISE EXCEPTION 'Denied report audit attribution is invalid' USING ERRCODE='42501';
      END IF;
      v_prefix:='api_app_token';
    WHEN 'oauth' THEN
      IF NOT EXISTS (SELECT 1 FROM public.oauth_grants g WHERE g.id::text=p_reference
          AND g.account_id=p_account_id AND g.client_kind='mobile'
          AND g.household_membership_id IS NULL AND g.person_id IS NULL
          AND g.permissions_version IS NULL) THEN
        RAISE EXCEPTION 'Denied report audit attribution is invalid' USING ERRCODE='42501';
      END IF;
      v_prefix:='oauth_grant';
    WHEN 'personal_api_key' THEN
      IF NOT EXISTS (SELECT 1 FROM public.identity_api_keys k WHERE k.id=p_reference
          AND k.account_id=p_account_id) THEN
        RAISE EXCEPTION 'Denied report audit attribution is invalid' USING ERRCODE='42501';
      END IF;
      v_prefix:='personal_api_key';
    ELSE
      RAISE EXCEPTION 'Denied report audit attribution is invalid' USING ERRCODE='42501';
  END CASE;
  INSERT INTO public.security_audit_events(household_id,actor_account_id,actor_membership_id,
      event_type,request_id,metadata,audit_context,created_at,updated_at)
  VALUES(p_household_id,p_account_id,p_membership_id,'api.request',p_request_id,
    jsonb_build_object('http_method','GET','controller',CASE p_report_kind
      WHEN 'health' THEN 'api/v1/health_history_reports' ELSE 'api/v1/medication_review_reports' END,
      'action','show','outcome','failure','status',p_status),
    jsonb_build_object('actor_account_id',p_account_id,'actor_person_id',p_person_id,
      'actor_membership_id',p_membership_id,'household_id',p_household_id,
      'active_role',p_role,'permissions_version',p_permissions_version,
      'authentication_method',p_method,'session_reference',v_prefix||':'||p_reference,
      'request_id',p_request_id),timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()));
END;
$$;
ALTER FUNCTION med_tracker.append_denied_report_request(bigint,bigint,bigint,bigint,text,integer,text,text,text,text,integer) OWNER TO med_tracker_owner;
REVOKE ALL ON FUNCTION med_tracker.append_denied_report_request(bigint,bigint,bigint,bigint,text,integer,text,text,text,text,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION med_tracker.append_denied_report_request(bigint,bigint,bigint,bigint,text,integer,text,text,text,text,integer) TO med_tracker_app;
