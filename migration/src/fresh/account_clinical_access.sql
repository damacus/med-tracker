ALTER TABLE public.account_clinical_access_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.account_clinical_access_events FORCE ROW LEVEL SECURITY;
CREATE POLICY account_clinical_access_owner_insert ON public.account_clinical_access_events
  FOR INSERT TO med_tracker_owner WITH CHECK (true);
CREATE POLICY account_clinical_access_verifier_select ON public.account_clinical_access_events
  FOR SELECT TO med_tracker_audit_verifier USING (true);
REVOKE ALL ON public.account_clinical_access_events FROM PUBLIC,med_tracker_app,med_tracker_audit_exporter,med_tracker_audit_verifier;
REVOKE ALL ON SEQUENCE public.account_clinical_access_events_id_seq FROM PUBLIC,med_tracker_app,med_tracker_audit_exporter,med_tracker_audit_verifier;
GRANT SELECT ON public.account_clinical_access_events TO med_tracker_audit_verifier;
ALTER TABLE public.account_clinical_access_events ADD CONSTRAINT account_clinical_access_event_kind
  CHECK(event_type IN('self_medication_history.read','self_medication_history.download'));

CREATE FUNCTION med_tracker.account_clinical_credential(p_account bigint,p_method text,p_reference text,p_live boolean)
RETURNS text LANGUAGE plpgsql STABLE SET search_path=pg_catalog AS $$
DECLARE canonical text;
BEGIN
  IF num_nulls(p_account,p_method,p_reference,p_live)>0 OR length(p_reference) NOT BETWEEN 1 AND 128 THEN RETURN NULL; END IF;
  IF p_live AND NOT EXISTS(SELECT 1 FROM public.accounts a WHERE a.id=p_account AND a.status=2 AND a.disabled_at IS NULL
      AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>clock_timestamp())) THEN RETURN NULL; END IF;
  IF p_method='oauth' THEN
    SELECT g.id::text INTO canonical FROM public.oauth_grants g JOIN public.oauth_applications a ON a.id=g.oauth_application_id
    WHERE g.id::text=p_reference AND g.account_id=p_account AND g.client_kind='mobile' AND a.client_kind='mobile'
      AND g.household_membership_id IS NULL AND g.person_id IS NULL AND g.permissions_version IS NULL
      AND (NOT p_live OR (g.revoked_at IS NULL AND g.expires_in>clock_timestamp()
        AND (CASE WHEN g.access_token_scope_hash=g.token_hash THEN g.access_token_scopes ELSE g.scopes END) ~ '(^|[[:space:]])medtracker([[:space:]]|$)'));
  ELSIF p_method='browser_session' THEN
    SELECT s.id::text INTO canonical FROM public.identity_sessions s
    WHERE s.account_id=p_account AND (s.id::text=p_reference OR s.token=p_reference)
      AND (NOT p_live OR (s.active AND s.purpose='authenticated' AND s.expires_at>clock_timestamp()));
  END IF;
  RETURN canonical;
END $$;
REVOKE ALL ON FUNCTION med_tracker.account_clinical_credential(bigint,text,text,boolean) FROM PUBLIC,med_tracker_app;

CREATE FUNCTION med_tracker.append_account_clinical_access(p_action text,p_method text,p_reference text,p_trace uuid)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE actor bigint:=med_tracker.current_account_id(); subject bigint; canonical text;
BEGIN
  canonical:=med_tracker.account_clinical_credential(actor,p_method,p_reference,true);
  SELECT a.person_id INTO subject FROM public.accounts a WHERE a.id=actor;
  IF num_nulls(p_action,p_trace,canonical,subject)>0 OR p_action NOT IN('self_medication_history.read','self_medication_history.download') THEN
    RAISE EXCEPTION 'Account clinical audit attribution is invalid' USING ERRCODE='42501';
  END IF;
  INSERT INTO public.account_clinical_access_events(account_id,person_id,event_type,metadata,audit_context)
  VALUES(actor,subject,p_action,jsonb_build_object('outcome','success','status',200),
    jsonb_build_object('actor_account_id',actor,'actor_person_id',subject,'authentication_method',p_method,
      'session_reference',CASE p_method WHEN 'oauth' THEN 'oauth_grant:' ELSE 'browser_session:' END||canonical,'request_id',p_trace));
END $$;
REVOKE ALL ON FUNCTION med_tracker.append_account_clinical_access(text,text,text,uuid) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.append_account_clinical_access(text,text,text,uuid) TO med_tracker_app;

CREATE FUNCTION med_tracker.append_denied_account_clinical_access(p_account bigint,p_person bigint,p_action text,p_method text,p_reference text,p_trace uuid,p_status integer)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
DECLARE canonical text;
BEGIN
  canonical:=med_tracker.account_clinical_credential(p_account,p_method,p_reference,false);
  IF num_nulls(p_account,p_person,p_action,p_trace,p_status,canonical)>0 OR p_status NOT IN(401,403)
      OR p_action NOT IN('self_medication_history.read','self_medication_history.download')
      OR NOT EXISTS(SELECT 1 FROM public.accounts a WHERE a.id=p_account AND a.person_id=p_person) THEN
    RAISE EXCEPTION 'Account clinical audit attribution is invalid' USING ERRCODE='42501';
  END IF;
  INSERT INTO public.account_clinical_access_events(account_id,person_id,event_type,metadata,audit_context)
  VALUES(p_account,p_person,p_action,jsonb_build_object('outcome','denied','status',p_status),
    jsonb_build_object('actor_account_id',p_account,'actor_person_id',p_person,'authentication_method',p_method,
      'session_reference',CASE p_method WHEN 'oauth' THEN 'oauth_grant:' ELSE 'browser_session:' END||canonical,'request_id',p_trace));
END $$;
REVOKE ALL ON FUNCTION med_tracker.append_denied_account_clinical_access(bigint,bigint,text,text,text,uuid,integer) FROM PUBLIC,med_tracker_app;
GRANT EXECUTE ON FUNCTION med_tracker.append_denied_account_clinical_access(bigint,bigint,text,text,text,uuid,integer) TO med_tracker_app;

CREATE FUNCTION public.audit_capture_account_clinical_access() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
  PERFORM public.audit_append_ledger_entry('account_clinical_access_events',NEW.id,NULL,to_jsonb(NEW),NEW.created_at);
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION public.audit_capture_account_clinical_access() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER account_clinical_access_ledger AFTER INSERT ON public.account_clinical_access_events
FOR EACH ROW EXECUTE FUNCTION public.audit_capture_account_clinical_access();

CREATE FUNCTION med_tracker.immutable_account_clinical_access() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN
  RAISE EXCEPTION 'Account clinical access history is immutable' USING ERRCODE='23514';
END $$;
REVOKE ALL ON FUNCTION med_tracker.immutable_account_clinical_access() FROM PUBLIC,med_tracker_app;
CREATE TRIGGER account_clinical_access_immutable BEFORE UPDATE OR DELETE ON public.account_clinical_access_events
FOR EACH ROW EXECUTE FUNCTION med_tracker.immutable_account_clinical_access();
