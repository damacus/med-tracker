ALTER TABLE ONLY public.active_storage_attachments FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.api_change_events FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.api_idempotency_keys FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.api_tombstones FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.carer_relationships FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.dosages FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.health_event_medications FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.health_events FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.household_exports FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.household_invitation_grants FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.household_invitations FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.household_memberships FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.household_retention_holds FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.location_memberships FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.locations FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.medication_dose_occurrences FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.medication_pause_periods FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.medication_review_prompts FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.medication_takes FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.medications FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.notification_events FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.notification_preferences FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.people FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.person_access_grants FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.person_medications FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.schedules FORCE ROW LEVEL SECURITY;
ALTER TABLE ONLY public.security_audit_events FORCE ROW LEVEL SECURITY;

ALTER TABLE public.ar_internal_metadata OWNER TO CURRENT_USER;
ALTER TABLE public.schema_migrations OWNER TO CURRENT_USER;

CREATE VIEW public.household_audit_ledger_entries WITH (security_barrier='true') AS
 SELECT id,
    household_id,
    chain_key,
    chain_epoch,
    epoch_kind,
    sequence,
    previous_hash,
    entry_hash,
    hash_algorithm,
    schema_version,
    source_table,
    source_id,
    envelope,
    occurred_at,
    retention_policy_version,
    retain_until,
    created_at
   FROM public.audit_ledger_entries
  WHERE (household_id = med_tracker.current_household_id());


ALTER VIEW public.household_audit_ledger_entries OWNER TO med_tracker_owner;

CREATE TRIGGER append_security_events_to_audit_ledger AFTER INSERT ON public.security_audit_events FOR EACH ROW EXECUTE FUNCTION public.audit_capture_source_row();

CREATE TRIGGER append_versions_to_audit_ledger AFTER INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.audit_capture_source_row();

ALTER TABLE public.active_storage_attachments ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.api_change_events ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.api_idempotency_keys ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.api_tombstones ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.carer_relationships ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.dosages ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.health_event_medications ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.health_events ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.household_exports ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.household_invitation_grants ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.household_invitations ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.household_memberships ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.household_retention_holds ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.location_memberships ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.locations ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.medication_dose_occurrences ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.medication_pause_periods ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.medication_review_prompts ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.medication_takes ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.medications ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.notification_events ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.notification_preferences ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.people ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.person_access_grants ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.person_medications ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.schedules ENABLE ROW LEVEL SECURITY;

ALTER TABLE public.security_audit_events ENABLE ROW LEVEL SECURITY;

CREATE POLICY audit_verifier_complete_visibility ON public.security_audit_events FOR SELECT TO med_tracker_audit_verifier USING (true);

CREATE POLICY household_tenant_isolation ON public.active_storage_attachments USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.api_change_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.api_idempotency_keys USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.api_tombstones USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.carer_relationships USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.dosages USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.health_event_medications USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.health_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.household_exports USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.household_invitation_grants USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.household_invitations USING (((household_id = med_tracker.current_household_id()) OR (((token_digest)::text = med_tracker.current_invitation_token_digest()) AND (accepted_at IS NULL) AND (revoked_at IS NULL) AND (expires_at > CURRENT_TIMESTAMP)))) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.household_memberships
USING (
  household_id = med_tracker.current_household_id()
  OR person_id = (
    SELECT a.person_id FROM public.accounts a
    WHERE a.id = med_tracker.current_account_id()
  )
)
WITH CHECK (household_id = med_tracker.current_household_id());

CREATE POLICY household_tenant_isolation ON public.household_retention_holds USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.location_memberships USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.locations USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.medication_dose_occurrences USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.medication_pause_periods USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.medication_review_prompts USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.medication_takes USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.medications USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.notification_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.notification_preferences USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY people_own_identity_select ON public.people FOR SELECT
USING (id = (SELECT a.person_id FROM public.accounts a
             WHERE a.id = med_tracker.current_account_id()));

CREATE POLICY people_household_participant_select ON public.people FOR SELECT
USING (
  EXISTS (
    SELECT 1 FROM public.household_memberships subject
    WHERE subject.person_id = people.id
      AND subject.household_id = med_tracker.current_household_id()
      AND subject.status = 'active' AND subject.revoked_at IS NULL
  )
  AND EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    WHERE actor.id = med_tracker.current_membership_id()
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
                      WHERE l.account_id = a.id
                        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  )
);


CREATE POLICY people_own_identity_insert ON public.people FOR INSERT TO med_tracker_app
WITH CHECK (id = (SELECT a.person_id FROM public.accounts a WHERE a.id = med_tracker.current_account_id()));

CREATE POLICY people_household_insert ON public.people FOR INSERT TO med_tracker_app
WITH CHECK (EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    WHERE actor.id = med_tracker.current_membership_id()
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
                      WHERE l.account_id = a.id
                        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  ));

CREATE POLICY people_context_update ON public.people FOR UPDATE TO med_tracker_app
USING ((id = (SELECT a.person_id FROM public.accounts a WHERE a.id = med_tracker.current_account_id())) OR (  EXISTS (
    SELECT 1 FROM public.household_memberships subject
    WHERE subject.person_id = people.id
      AND subject.household_id = med_tracker.current_household_id()
      AND subject.status = 'active' AND subject.revoked_at IS NULL
  )
  AND EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    WHERE actor.id = med_tracker.current_membership_id()
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
                      WHERE l.account_id = a.id
                        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  )))
WITH CHECK ((id = (SELECT a.person_id FROM public.accounts a WHERE a.id = med_tracker.current_account_id())) OR (  EXISTS (
    SELECT 1 FROM public.household_memberships subject
    WHERE subject.person_id = people.id
      AND subject.household_id = med_tracker.current_household_id()
      AND subject.status = 'active' AND subject.revoked_at IS NULL
  )
  AND EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    WHERE actor.id = med_tracker.current_membership_id()
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
                      WHERE l.account_id = a.id
                        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  )));

CREATE POLICY people_household_delete ON public.people FOR DELETE TO med_tracker_app
USING (  EXISTS (
    SELECT 1 FROM public.household_memberships subject
    WHERE subject.person_id = people.id
      AND subject.household_id = med_tracker.current_household_id()
      AND subject.status = 'active' AND subject.revoked_at IS NULL
  )
  AND EXISTS (
    SELECT 1 FROM public.household_memberships actor
    JOIN public.accounts a ON a.person_id = actor.person_id
    JOIN public.households h ON h.id = actor.household_id
    WHERE actor.id = med_tracker.current_membership_id()
      AND a.id = med_tracker.current_account_id() AND a.status = 2
      AND actor.household_id = med_tracker.current_household_id()
      AND actor.status = 'active' AND actor.revoked_at IS NULL
      AND h.status = 'active' AND h.lifecycle_state = 'active'
      AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l
                      WHERE l.account_id = a.id
                        AND l.deadline > pg_catalog.timezone('UTC',pg_catalog.clock_timestamp()))
  ));

CREATE POLICY household_tenant_isolation ON public.person_access_grants USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.person_medications USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.schedules USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));

CREATE POLICY household_tenant_isolation ON public.security_audit_events USING ((household_id = med_tracker.current_household_id())) WITH CHECK ((household_id = med_tracker.current_household_id()));


GRANT USAGE ON SCHEMA med_tracker TO med_tracker_owner;
GRANT USAGE ON SCHEMA med_tracker TO med_tracker_app;

GRANT ALL ON SCHEMA public TO med_tracker_owner;
GRANT USAGE ON SCHEMA public TO med_tracker_app;
GRANT USAGE ON SCHEMA public TO med_tracker_audit_exporter;
GRANT USAGE ON SCHEMA public TO med_tracker_audit_verifier;

GRANT ALL ON FUNCTION med_tracker.current_account_id() TO med_tracker_app;

GRANT ALL ON FUNCTION med_tracker.current_household_id() TO med_tracker_app;

GRANT ALL ON FUNCTION med_tracker.current_invitation_token_digest() TO med_tracker_app;

GRANT ALL ON FUNCTION med_tracker.current_membership_id() TO med_tracker_app;

REVOKE ALL ON FUNCTION med_tracker.purge_medication_takes(p_household_id bigint) FROM PUBLIC;
GRANT ALL ON FUNCTION med_tracker.purge_medication_takes(p_household_id bigint) TO med_tracker_app;

REVOKE ALL ON FUNCTION public.audit_append_ledger_entry(p_source_table text, p_source_id bigint, p_household_id bigint, p_source_payload jsonb, p_occurred_at timestamp with time zone) FROM PUBLIC;

REVOKE ALL ON FUNCTION public.audit_capture_source_row() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text) FROM PUBLIC;
GRANT ALL ON FUNCTION public.audit_record_signed_checkpoint(p_key_id text, p_public_key_base64 text, p_chain_key text, p_chain_epoch uuid, p_sequence bigint, p_entry_hash_hex text, p_signature_base64 text, p_signed_at timestamp with time zone, p_checkpoint_kind text) TO med_tracker_audit_exporter;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_active_session_keys TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_lockouts TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.account_lockouts_account_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_login_change_keys TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_login_failures TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.account_login_failures_account_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_password_reset_keys TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_remember_keys TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_verification_keys TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.account_webauthn_auth_challenges TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.account_webauthn_auth_challenges_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.accounts TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.accounts_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.active_storage_attachments TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.active_storage_attachments_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.active_storage_blobs TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.active_storage_blobs_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.active_storage_variant_records TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.active_storage_variant_records_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_app_tokens TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.api_app_tokens_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_change_events TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.api_change_events_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_household_selection_grants TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.api_household_selection_grants_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_idempotency_keys TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.api_idempotency_keys_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_sessions TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.api_sessions_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.api_tombstones TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.api_tombstones_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.app_settings TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.app_settings_id_seq TO med_tracker_app;

GRANT SELECT ON TABLE public.audit_chain_heads TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_chain_heads TO med_tracker_audit_verifier;

GRANT SELECT ON TABLE public.audit_checkpoints TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_checkpoints TO med_tracker_audit_verifier;

GRANT SELECT,UPDATE ON TABLE public.audit_export_deliveries TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_export_deliveries TO med_tracker_audit_verifier;

GRANT SELECT ON TABLE public.audit_ledger_entries TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_ledger_entries TO med_tracker_audit_verifier;

GRANT SELECT ON TABLE public.audit_signing_keys TO med_tracker_audit_exporter;
GRANT SELECT ON TABLE public.audit_signing_keys TO med_tracker_audit_verifier;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.barcode_catalog_entries TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.barcode_catalog_entries_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.carer_relationships TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.carer_relationships_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.dosages TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.dosages_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.health_event_medications TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.health_event_medications_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.health_events TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.health_events_id_seq TO med_tracker_app;

GRANT SELECT ON TABLE public.household_audit_ledger_entries TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_exports TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.household_exports_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_invitation_grants TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.household_invitation_grants_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_invitations TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.household_invitations_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_memberships TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.household_memberships_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_purge_runs TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.household_purge_runs_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.household_retention_holds TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.household_retention_holds_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.households TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.households_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.location_memberships TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.location_memberships_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.locations TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.locations_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_dose_occurrences TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medication_dose_occurrences_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_pause_periods TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medication_pause_periods_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_review_evidence_records TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medication_review_evidence_records_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_review_evidence_refresh_runs TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medication_review_evidence_refresh_runs_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_review_prompts TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medication_review_prompts_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medication_takes TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medication_takes_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.medications TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.medications_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.native_device_tokens TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.native_device_tokens_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_amp_trade_families TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_amp_trade_families_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_ampp_relationships TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_ampp_relationships_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_barcodes TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_barcodes_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_imports TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_imports_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_supplementary_releases TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_supplementary_releases_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_trade_families TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_trade_families_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.nhs_dmd_trade_family_groups TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.nhs_dmd_trade_family_groups_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.notification_events TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.notification_events_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.notification_preferences TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.notification_preferences_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.oauth_applications TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.oauth_applications_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.oauth_grants TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.oauth_grants_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.people TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.people_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.person_access_grants TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.person_access_grants_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.person_medications TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.person_medications_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.platform_admins TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.platform_admins_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.push_subscriptions TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.push_subscriptions_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.schedules TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.schedules_id_seq TO med_tracker_app;

GRANT SELECT,INSERT ON TABLE public.security_audit_events TO med_tracker_app;
GRANT SELECT ON TABLE public.security_audit_events TO med_tracker_audit_verifier;

GRANT SELECT,USAGE ON SEQUENCE public.security_audit_events_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.storage_migration_runs TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.storage_migration_runs_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.support_access_sessions TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.support_access_sessions_id_seq TO med_tracker_app;

GRANT SELECT,INSERT,DELETE,UPDATE ON TABLE public.users TO med_tracker_app;

GRANT SELECT,USAGE ON SEQUENCE public.users_id_seq TO med_tracker_app;

GRANT SELECT,INSERT ON TABLE public.versions TO med_tracker_app;
GRANT SELECT ON TABLE public.versions TO med_tracker_audit_verifier;

GRANT SELECT,USAGE ON SEQUENCE public.versions_id_seq TO med_tracker_app;

ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA med_tracker GRANT ALL ON FUNCTIONS TO med_tracker_app;

ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public GRANT SELECT,USAGE ON SEQUENCES TO med_tracker_app;

ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public GRANT SELECT,INSERT,DELETE,UPDATE ON TABLES TO med_tracker_app;
