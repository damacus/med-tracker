use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
CREATE TABLE public.identity_rate_limits (
    bucket text PRIMARY KEY,
    attempts integer NOT NULL CHECK(attempts > 0),
    expires_at timestamptz NOT NULL
);
CREATE INDEX identity_rate_limits_expiry ON public.identity_rate_limits(expires_at);
REVOKE ALL ON public.identity_rate_limits FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.identity_rate_limits TO med_tracker_app;
ALTER TABLE public.identity_rate_limits ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_rate_limits FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_rate_limit_bucket ON public.identity_rate_limits TO med_tracker_app
USING (bucket = current_setting('med_tracker.identity_rate_bucket', true) OR expires_at <= clock_timestamp())
WITH CHECK (bucket = current_setting('med_tracker.identity_rate_bucket', true));
CREATE TABLE public.identity_onboarding (
    account_id bigint PRIMARY KEY REFERENCES public.accounts(id),
    passkey_user_handle uuid NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    recovery_saved_at timestamptz,
    legacy_totp_disabled_at timestamptz,
    legacy_totp_locked_until timestamptz
);
REVOKE ALL ON public.identity_onboarding FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE ON public.identity_onboarding TO med_tracker_app;
ALTER TABLE public.identity_onboarding ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_onboarding FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_onboarding_account ON public.identity_onboarding TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);
CREATE TABLE public.identity_sessions (
    id text PRIMARY KEY,
    account_id bigint NOT NULL REFERENCES public.accounts(id),
    token text NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ip_address text,
    user_agent text,
    impersonated_by bigint REFERENCES public.accounts(id),
    active_household_id bigint REFERENCES public.households(id),
    active boolean NOT NULL DEFAULT true,
    purpose text NOT NULL DEFAULT 'authenticated' CHECK (purpose IN ('authenticated','enrolment')),
    additional_fields jsonb NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX identity_sessions_account_id ON public.identity_sessions(account_id);
CREATE INDEX identity_sessions_expires_at ON public.identity_sessions(expires_at);
CREATE TABLE public.identity_verifications (
    id text PRIMARY KEY,
    account_id bigint REFERENCES public.accounts(id),
    identifier text NOT NULL,
    value text NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX identity_verifications_identifier ON public.identity_verifications(identifier, created_at DESC);
CREATE INDEX identity_verifications_expires_at ON public.identity_verifications(expires_at);
CREATE TABLE public.identity_passkeys (
    id text PRIMARY KEY,
    account_id bigint NOT NULL REFERENCES public.accounts(id),
    name text,
    public_key text NOT NULL,
    credential_id text NOT NULL UNIQUE,
    counter numeric(20,0) NOT NULL CHECK(counter >= 0 AND counter <= 18446744073709551615),
    device_type text NOT NULL,
    backed_up boolean NOT NULL,
    transports text,
    credential text NOT NULL,
    aaguid text,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX identity_passkeys_account_id ON public.identity_passkeys(account_id);
CREATE TABLE public.identity_provider_accounts (
    id text PRIMARY KEY,
    account_id bigint NOT NULL REFERENCES public.accounts(id),
    provider_id text NOT NULL,
    provider_account_id text NOT NULL,
    access_token text,
    refresh_token text,
    id_token text,
    access_token_expires_at timestamptz,
    refresh_token_expires_at timestamptz,
    scope text,
    password text,
    disabled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(provider_id,provider_account_id)
);
CREATE INDEX identity_provider_accounts_account_id ON public.identity_provider_accounts(account_id);
CREATE TABLE public.identity_api_keys (
    id text PRIMARY KEY,
    account_id bigint NOT NULL REFERENCES public.accounts(id),
    key_hash text NOT NULL UNIQUE,
    expires_at timestamptz,
    payload jsonb NOT NULL,
    CHECK(payload->>'id'=id),
    CHECK(payload->>'key'=key_hash),
    CHECK(payload->>'referenceId'=account_id::text)
);
CREATE INDEX identity_api_keys_account_id ON public.identity_api_keys(account_id);
CREATE INDEX identity_api_keys_expiry ON public.identity_api_keys(expires_at);
REVOKE ALL ON public.identity_api_keys FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.identity_api_keys TO med_tracker_app;
ALTER TABLE public.identity_api_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_api_keys FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_api_keys_account ON public.identity_api_keys TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);
CREATE POLICY identity_api_keys_identifier ON public.identity_api_keys TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_api_key_id', true), ''))
WITH CHECK (id = NULLIF(current_setting('med_tracker.identity_api_key_id', true), ''));
CREATE POLICY identity_api_keys_hash ON public.identity_api_keys FOR SELECT TO med_tracker_app
USING (key_hash = NULLIF(current_setting('med_tracker.identity_api_key_hash', true), ''));
CREATE POLICY identity_api_keys_expired_owner ON public.identity_api_keys TO med_tracker_owner
USING (expires_at < CURRENT_TIMESTAMP) WITH CHECK (false);
CREATE FUNCTION public.identity_delete_expired_api_keys() RETURNS bigint
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog
AS 'WITH removed AS (DELETE FROM public.identity_api_keys WHERE expires_at < CURRENT_TIMESTAMP RETURNING 1) SELECT count(*) FROM removed';
REVOKE ALL ON FUNCTION public.identity_delete_expired_api_keys() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.identity_delete_expired_api_keys() TO med_tracker_app;
CREATE TABLE public.identity_device_codes (
    id text PRIMARY KEY,
    device_code text NOT NULL UNIQUE,
    user_code text NOT NULL UNIQUE,
    account_id bigint REFERENCES public.accounts(id),
    expires_at timestamptz NOT NULL,
    status text NOT NULL,
    last_polled_at timestamptz,
    polling_interval bigint,
    client_id text,
    scope text
);
REVOKE ALL ON public.identity_device_codes FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.identity_device_codes TO med_tracker_app;
ALTER TABLE public.identity_device_codes ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_device_codes FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_device_identifier ON public.identity_device_codes TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_device_id', true), '') OR device_code = NULLIF(current_setting('med_tracker.identity_device_code', true), ''))
WITH CHECK (id = NULLIF(current_setting('med_tracker.identity_device_id', true), '') OR device_code = NULLIF(current_setting('med_tracker.identity_device_code', true), ''));
CREATE POLICY identity_device_user_code ON public.identity_device_codes FOR SELECT TO med_tracker_app
USING (user_code = NULLIF(current_setting('med_tracker.identity_device_user_code', true), ''));
CREATE TABLE public.identity_two_factors (
    id text PRIMARY KEY,
    account_id bigint NOT NULL UNIQUE REFERENCES public.accounts(id),
    secret text NOT NULL,
    backup_codes text NOT NULL,
    verified boolean NOT NULL DEFAULT false,
    failed_verification_count bigint NOT NULL DEFAULT 0 CHECK(failed_verification_count>=0),
    locked_until timestamptz,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
REVOKE ALL ON public.identity_provider_accounts, public.identity_two_factors FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.identity_provider_accounts, public.identity_two_factors TO med_tracker_app;
ALTER TABLE public.identity_provider_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_provider_accounts FORCE ROW LEVEL SECURITY;
ALTER TABLE public.identity_two_factors ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_two_factors FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_providers_account ON public.identity_provider_accounts TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);
CREATE POLICY identity_providers_lookup ON public.identity_provider_accounts FOR SELECT TO med_tracker_app
USING (provider_id = NULLIF(current_setting('med_tracker.identity_provider_id', true), '') AND provider_account_id = NULLIF(current_setting('med_tracker.identity_provider_account_id', true), ''));
CREATE POLICY identity_providers_id_lookup ON public.identity_provider_accounts FOR SELECT TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_provider_row_id', true), ''));
CREATE POLICY identity_factors_account ON public.identity_two_factors TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);
CREATE POLICY identity_factors_lookup ON public.identity_two_factors FOR SELECT TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_factor_id', true), ''));
REVOKE ALL ON public.identity_sessions, public.identity_verifications, public.identity_passkeys FROM PUBLIC, med_tracker_app, med_tracker_audit_exporter, med_tracker_audit_verifier;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.identity_sessions, public.identity_verifications, public.identity_passkeys TO med_tracker_app;
ALTER TABLE public.identity_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_sessions FORCE ROW LEVEL SECURITY;
ALTER TABLE public.identity_verifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_verifications FORCE ROW LEVEL SECURITY;
ALTER TABLE public.identity_passkeys ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.identity_passkeys FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_sessions_account ON public.identity_sessions TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);
CREATE POLICY identity_sessions_lookup ON public.identity_sessions FOR SELECT TO med_tracker_app
USING (token = NULLIF(current_setting('med_tracker.identity_session_token', true), ''));
CREATE POLICY identity_passkeys_account ON public.identity_passkeys TO med_tracker_app
USING (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint)
WITH CHECK (account_id = NULLIF(current_setting('med_tracker.current_account_id', true), '')::bigint);
CREATE POLICY identity_passkeys_lookup ON public.identity_passkeys FOR SELECT TO med_tracker_app
USING (credential_id = NULLIF(current_setting('med_tracker.identity_credential_id', true), ''));
CREATE POLICY identity_passkeys_id_lookup ON public.identity_passkeys FOR SELECT TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_passkey_id', true), ''));
CREATE POLICY identity_verifications_identifier ON public.identity_verifications TO med_tracker_app
USING (identifier = NULLIF(current_setting('med_tracker.identity_verification_identifier', true), ''))
WITH CHECK (identifier = NULLIF(current_setting('med_tracker.identity_verification_identifier', true), ''));
CREATE POLICY identity_verifications_id_read ON public.identity_verifications FOR SELECT TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_verification_id', true), ''));
CREATE POLICY identity_verifications_id_delete ON public.identity_verifications FOR DELETE TO med_tracker_app
USING (id = NULLIF(current_setting('med_tracker.identity_verification_id', true), ''));
CREATE POLICY identity_verifications_value ON public.identity_verifications FOR SELECT TO med_tracker_app
USING (value = NULLIF(current_setting('med_tracker.identity_verification_value', true), ''));
CREATE POLICY identity_verifications_expired_owner ON public.identity_verifications TO med_tracker_owner
USING (expires_at <= CURRENT_TIMESTAMP) WITH CHECK (false);
CREATE FUNCTION public.identity_delete_expired_verifications() RETURNS bigint
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog
AS 'WITH removed AS (DELETE FROM public.identity_verifications WHERE expires_at <= CURRENT_TIMESTAMP RETURNING 1) SELECT count(*) FROM removed';
REVOKE ALL ON FUNCTION public.identity_delete_expired_verifications() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.identity_delete_expired_verifications() TO med_tracker_app;
CREATE POLICY identity_sessions_expired_owner ON public.identity_sessions TO med_tracker_owner
USING (expires_at <= CURRENT_TIMESTAMP) WITH CHECK (false);
CREATE FUNCTION public.identity_delete_expired_sessions() RETURNS bigint
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog
AS 'WITH removed AS (DELETE FROM public.identity_sessions WHERE expires_at <= CURRENT_TIMESTAMP RETURNING 1) SELECT count(*) FROM removed';
REVOKE ALL ON FUNCTION public.identity_delete_expired_sessions() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.identity_delete_expired_sessions() TO med_tracker_app;
CREATE POLICY identity_invitation_recipient ON public.household_invitations FOR SELECT TO med_tracker_app, med_tracker_owner
USING (accepted_at IS NULL AND revoked_at IS NULL AND expires_at>CURRENT_TIMESTAMP
AND email = (SELECT a.email FROM public.accounts a WHERE a.id=med_tracker.current_account_id() AND a.status=2 AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP))
AND EXISTS(SELECT 1 FROM public.identity_sessions s WHERE s.account_id=med_tracker.current_account_id() AND s.token=NULLIF(current_setting('med_tracker.identity_session_token',true),'') AND s.active AND s.purpose='authenticated' AND s.expires_at>CURRENT_TIMESTAMP));
CREATE POLICY identity_inviter_projection ON public.household_memberships FOR SELECT TO med_tracker_owner
USING (EXISTS(SELECT 1 FROM public.household_invitations i WHERE i.invited_by_membership_id=household_memberships.id AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at>CURRENT_TIMESTAMP AND i.email=(SELECT a.email FROM public.accounts a WHERE a.id=med_tracker.current_account_id() AND a.status=2)));
CREATE POLICY identity_invitation_session_owner ON public.identity_sessions FOR SELECT TO med_tracker_owner
USING (account_id=med_tracker.current_account_id() AND token=NULLIF(current_setting('med_tracker.identity_session_token',true),''));
CREATE FUNCTION public.identity_invitation_inviter(invitation_id bigint) RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=pg_catalog
AS 'SELECT m.account_id FROM public.household_invitations i JOIN public.household_memberships m ON m.id=i.invited_by_membership_id AND m.household_id=i.household_id WHERE i.id=invitation_id AND i.email=(SELECT a.email FROM public.accounts a WHERE a.id=med_tracker.current_account_id() AND a.status=2)';
REVOKE ALL ON FUNCTION public.identity_invitation_inviter(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.identity_invitation_inviter(bigint) TO med_tracker_app;
"#).await?;
        Ok(())
    }
}
