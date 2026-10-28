REVOKE ALL ON public.identity_rate_limits FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.identity_rate_limits TO med_tracker_app;
ALTER TABLE public.identity_rate_limits ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_rate_limits FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_rate_limit_bucket ON public.identity_rate_limits TO med_tracker_app
USING (bucket = current_setting('med_tracker.identity_rate_bucket', true) OR expires_at <= clock_timestamp())
WITH CHECK (bucket = current_setting('med_tracker.identity_rate_bucket', true));
REVOKE ALL ON public.identity_onboarding FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE ON public.identity_onboarding TO med_tracker_app;
ALTER TABLE public.identity_onboarding ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_onboarding FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_onboarding_account ON public.identity_onboarding TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);


CREATE POLICY identity_invitation_recipient ON public.household_invitations FOR SELECT TO med_tracker_app, med_tracker_owner
USING (accepted_at IS NULL AND revoked_at IS NULL AND expires_at>CURRENT_TIMESTAMP
AND lower(email::text) = (SELECT lower(a.email) FROM public.accounts a WHERE a.id=med_tracker.current_account_id() AND a.status=2 AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP))
AND EXISTS(SELECT 1 FROM public.identity_sessions s WHERE s.account_id=med_tracker.current_account_id() AND s.token=NULLIF(current_setting('med_tracker.identity_session_token',true),'') AND s.active AND s.purpose='authenticated' AND s.expires_at>CURRENT_TIMESTAMP));

CREATE POLICY identity_inviter_projection ON public.household_memberships FOR SELECT TO med_tracker_owner
USING (EXISTS(SELECT 1 FROM public.household_invitations i WHERE i.invited_by_membership_id=household_memberships.id AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>CURRENT_TIMESTAMP AND lower(i.email::text)=(SELECT lower(a.email) FROM public.accounts a WHERE a.id=med_tracker.current_account_id() AND a.status=2)));

CREATE FUNCTION public.identity_invitation_inviter(invitation_id bigint) RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog
AS 'SELECT inviter.id FROM public.household_invitations i JOIN public.household_memberships m ON m.id=i.invited_by_membership_id AND m.household_id=i.household_id JOIN public.accounts inviter ON inviter.person_id=m.person_id WHERE i.id=invitation_id AND lower(i.email::text)=(SELECT lower(a.email) FROM public.accounts a WHERE a.id=med_tracker.current_account_id() AND a.status=2)';
REVOKE ALL ON FUNCTION public.identity_invitation_inviter(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.identity_invitation_inviter(bigint) TO med_tracker_app;

REVOKE ALL ON public.auth_credentials, public.identity_sessions, public.identity_verifications,
  public.identity_passkeys, public.identity_two_factors, public.identity_api_keys,
  public.identity_plugin_rate_limits FROM PUBLIC, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.auth_credentials, public.identity_sessions,
  public.identity_verifications, public.identity_passkeys, public.identity_two_factors,
  public.identity_api_keys, public.identity_plugin_rate_limits TO med_tracker_app;
GRANT USAGE, SELECT ON SEQUENCE public.identity_sessions_id_seq,
  public.identity_verifications_id_seq, public.identity_passkeys_id_seq,
  public.identity_two_factors_id_seq, public.identity_api_keys_id_seq TO med_tracker_app;

CREATE FUNCTION public.native_account_verification_status() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$ BEGIN
  IF NEW.email_verified AND NOT OLD.email_verified AND OLD.status = 1 THEN NEW.status := 2; END IF;
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION public.native_account_verification_status() FROM PUBLIC;
CREATE TRIGGER native_account_verification_status BEFORE UPDATE OF email_verified ON public.accounts
FOR EACH ROW EXECUTE FUNCTION public.native_account_verification_status();

CREATE FUNCTION public.native_authentication_audit() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$
DECLARE
  subject_id bigint;
  record_id text;
  resource text;
  mutation text;
BEGIN
  resource := CASE TG_TABLE_NAME WHEN 'accounts' THEN 'account' WHEN 'identity_sessions' THEN 'session' WHEN 'auth_credentials' THEN 'credential' WHEN 'identity_api_keys' THEN 'api_key' ELSE 'passkey' END;
  mutation := CASE TG_OP WHEN 'INSERT' THEN 'created' WHEN 'DELETE' THEN 'deleted' ELSE 'updated' END;
  IF TG_OP = 'DELETE' THEN
    IF TG_TABLE_NAME='auth_credentials' THEN subject_id := OLD.user_id; ELSIF TG_TABLE_NAME='identity_api_keys' THEN subject_id := OLD.reference_id; ELSE subject_id := OLD.account_id; END IF;
    record_id := OLD.id::text;
  ELSE
    record_id := NEW.id::text;
    IF TG_TABLE_NAME='accounts' THEN subject_id:=NEW.id; ELSIF TG_TABLE_NAME='auth_credentials' THEN subject_id:=NEW.user_id; ELSIF TG_TABLE_NAME='identity_api_keys' THEN subject_id:=NEW.reference_id; ELSE subject_id:=NEW.account_id; END IF;
  END IF;
  INSERT INTO public.versions(item_type,item_id,event,object,audit_context,created_at)
  VALUES('NativeAuthentication',subject_id,'native_auth/' || resource || '/' || mutation,
    jsonb_build_object('account_id',subject_id,'resource',resource,'record_id',record_id,'mutation',mutation)::text,
    jsonb_build_object('actor_account_id',NULL,'actor_person_id',NULL,'request_id',NULL),
    timezone('UTC',clock_timestamp()));
  IF TG_OP = 'DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
END $$;
REVOKE ALL ON FUNCTION public.native_authentication_audit() FROM PUBLIC;
CREATE TRIGGER native_account_audit AFTER INSERT OR UPDATE ON public.accounts
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_audit();
CREATE TRIGGER native_session_audit AFTER INSERT OR UPDATE OR DELETE ON public.identity_sessions
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_audit();

CREATE FUNCTION public.native_account_email_changed() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$ BEGIN
  INSERT INTO public.pg_loco_queue(id,task_data,name,run_at,priority)
  VALUES(public.gen_random_uuid()::text,
    jsonb_build_object('from',NULL,'to',OLD.email,'reply_to',NULL,'subject','Email address changed',
      'text','Your MedTracker email address was changed. If this was not you, secure your account immediately.',
      'html','','bcc',NULL,'cc',NULL,'headers',NULL),
    'MailerWorker',clock_timestamp(),100);
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION public.native_account_email_changed() FROM PUBLIC;
CREATE TRIGGER native_account_email_changed AFTER UPDATE OF email ON public.accounts
FOR EACH ROW WHEN (OLD.email IS DISTINCT FROM NEW.email) EXECUTE FUNCTION public.native_account_email_changed();

CREATE TRIGGER native_credential_audit AFTER INSERT OR UPDATE OR DELETE ON public.auth_credentials
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_audit();
CREATE TRIGGER native_passkey_audit AFTER INSERT OR UPDATE OR DELETE ON public.identity_passkeys
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_audit();

CREATE FUNCTION public.native_authentication_notice() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE subject_id bigint; title text; message text; address text;
BEGIN
  IF TG_TABLE_NAME='auth_credentials' THEN
    IF TG_OP='UPDATE' AND NEW.provider_id='credential' AND NEW.password IS DISTINCT FROM OLD.password THEN
      subject_id:=NEW.user_id; title:='Password changed';
      message:='Your MedTracker password was changed. If this was not you, secure your account immediately.';
    ELSIF TG_OP='DELETE' THEN
      subject_id:=OLD.user_id; title:='Sign-in method removed';
      message:='A sign-in method was removed from your MedTracker account. If this was not you, secure your account immediately.';
    END IF;
  ELSIF TG_TABLE_NAME='identity_passkeys' THEN
    IF TG_OP='INSERT' THEN
      subject_id:=NEW.account_id; title:='Passkey added'; message:='A passkey was added to your MedTracker account.';
    ELSIF TG_OP='DELETE' THEN
      subject_id:=OLD.account_id; title:='Sign-in method removed'; message:='A passkey was removed from your MedTracker account.';
    END IF;
  ELSIF TG_TABLE_NAME='identity_api_keys' THEN
    IF TG_OP='INSERT' THEN
      subject_id:=NEW.reference_id; title:='API key created'; message:='A personal API key was created for your MedTracker account. Review its access in account security.';
    ELSIF TG_OP='UPDATE' AND OLD.enabled AND NOT NEW.enabled THEN
      subject_id:=NEW.reference_id; title:='API key revoked'; message:='A personal API key was revoked from your MedTracker account.';
    END IF;
  ELSIF TG_TABLE_NAME='accounts' AND NEW.two_factor_enabled IS DISTINCT FROM OLD.two_factor_enabled THEN
    subject_id:=NEW.id;
    IF NEW.two_factor_enabled THEN title:='Authenticator app enabled'; ELSE title:='Authenticator app disabled'; END IF;
    message:='Your MedTracker authenticator app settings changed. If this was not you, secure your account immediately.';
  END IF;
  IF subject_id IS NOT NULL THEN
    IF TG_TABLE_NAME='identity_api_keys' AND NOT EXISTS(SELECT 1 FROM public.accounts WHERE id=subject_id AND status=2) THEN RETURN NEW; END IF;
    SELECT a.email INTO STRICT address FROM public.accounts a WHERE a.id=subject_id;
    INSERT INTO public.pg_loco_queue(id,task_data,name,run_at,priority)
    VALUES(gen_random_uuid()::text,jsonb_build_object('from',NULL,'to',address,'reply_to',NULL,'subject',title,'text',message,'html','','bcc',NULL,'cc',NULL,'headers',NULL),'MailerWorker',clock_timestamp(),100);
  END IF;
  IF TG_OP='DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
END $$;
REVOKE ALL ON FUNCTION public.native_authentication_notice() FROM PUBLIC;
CREATE TRIGGER native_credential_notice AFTER UPDATE OF password OR DELETE ON public.auth_credentials
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_notice();
CREATE TRIGGER native_passkey_notice AFTER INSERT OR DELETE ON public.identity_passkeys
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_notice();
CREATE TRIGGER native_factor_notice AFTER UPDATE OF two_factor_enabled ON public.accounts
FOR EACH ROW EXECUTE FUNCTION public.native_authentication_notice();

CREATE TRIGGER native_api_key_create_audit AFTER INSERT ON public.identity_api_keys
FOR EACH ROW WHEN (NEW.config_id='medtracker-personal') EXECUTE FUNCTION public.native_authentication_audit();
CREATE TRIGGER native_api_key_delete_audit AFTER DELETE ON public.identity_api_keys
FOR EACH ROW WHEN (OLD.config_id='medtracker-personal') EXECUTE FUNCTION public.native_authentication_audit();
CREATE TRIGGER native_api_key_update_audit AFTER UPDATE OF enabled ON public.identity_api_keys
FOR EACH ROW WHEN (NEW.config_id='medtracker-personal' AND OLD.enabled IS DISTINCT FROM NEW.enabled) EXECUTE FUNCTION public.native_authentication_audit();
CREATE TRIGGER native_api_key_create_notice AFTER INSERT ON public.identity_api_keys
FOR EACH ROW WHEN (NEW.config_id='medtracker-personal') EXECUTE FUNCTION public.native_authentication_notice();
CREATE TRIGGER native_api_key_revoke_notice AFTER UPDATE OF enabled ON public.identity_api_keys
FOR EACH ROW WHEN (NEW.config_id='medtracker-personal' AND OLD.enabled AND NOT NEW.enabled) EXECUTE FUNCTION public.native_authentication_notice();
