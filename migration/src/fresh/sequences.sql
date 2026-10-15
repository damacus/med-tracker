CREATE SEQUENCE public.account_identities_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_identities_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.account_lockouts_account_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_lockouts_account_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.account_login_failures_account_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_login_failures_account_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.account_otp_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_otp_keys_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.account_webauthn_auth_challenges_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_webauthn_auth_challenges_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.account_webauthn_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_webauthn_keys_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.account_webauthn_user_ids_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.account_webauthn_user_ids_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.accounts_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.accounts_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.active_storage_attachments_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.active_storage_attachments_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.active_storage_blobs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.active_storage_blobs_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.active_storage_variant_records_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.active_storage_variant_records_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.api_app_tokens_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_app_tokens_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.api_change_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_change_events_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.api_household_selection_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_household_selection_grants_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.api_idempotency_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_idempotency_keys_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.api_sessions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_sessions_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.api_tombstones_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.api_tombstones_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.app_settings_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.app_settings_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.audit_chain_heads_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_chain_heads_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.audit_checkpoints_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_checkpoints_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.audit_export_deliveries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_export_deliveries_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.audit_ledger_entries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_ledger_entries_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.audit_signing_keys_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.audit_signing_keys_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.barcode_catalog_entries_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.barcode_catalog_entries_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.carer_relationships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.carer_relationships_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.dosages_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.dosages_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.health_event_medications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.health_event_medications_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.health_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.health_events_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.household_exports_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_exports_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.household_invitation_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_invitation_grants_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.household_invitations_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_invitations_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.household_memberships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_memberships_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.household_purge_runs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_purge_runs_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.household_retention_holds_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.household_retention_holds_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.households_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.households_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.location_memberships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.location_memberships_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.locations_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.locations_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medication_dose_occurrences_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_dose_occurrences_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medication_pause_periods_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_pause_periods_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medication_review_evidence_records_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_review_evidence_records_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medication_review_evidence_refresh_runs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_review_evidence_refresh_runs_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medication_review_prompts_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_review_prompts_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medication_takes_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medication_takes_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.medications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.medications_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.native_device_tokens_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.native_device_tokens_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_amp_trade_families_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_amp_trade_families_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_ampp_relationships_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_ampp_relationships_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_barcodes_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_barcodes_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_imports_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_imports_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_supplementary_releases_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_supplementary_releases_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_trade_families_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_trade_families_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.nhs_dmd_trade_family_groups_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.nhs_dmd_trade_family_groups_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.notification_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.notification_events_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.notification_preferences_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.notification_preferences_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.oauth_applications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.oauth_applications_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.oauth_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.oauth_grants_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.people_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.people_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.person_access_grants_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.person_access_grants_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.person_medications_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.person_medications_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.platform_admins_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.platform_admins_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.push_subscriptions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.push_subscriptions_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.schedules_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.schedules_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.security_audit_events_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.security_audit_events_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.storage_migration_runs_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.storage_migration_runs_id_seq OWNER TO med_tracker_owner;

CREATE SEQUENCE public.support_access_sessions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.support_access_sessions_id_seq OWNER TO med_tracker_owner;




CREATE SEQUENCE public.versions_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.versions_id_seq OWNER TO med_tracker_owner;

ALTER SEQUENCE public.account_identities_id_seq OWNED BY public.account_identities.id;

ALTER SEQUENCE public.account_lockouts_account_id_seq OWNED BY public.account_lockouts.account_id;

ALTER SEQUENCE public.account_login_failures_account_id_seq OWNED BY public.account_login_failures.account_id;

ALTER SEQUENCE public.account_otp_keys_id_seq OWNED BY public.account_otp_keys.id;

ALTER SEQUENCE public.account_webauthn_auth_challenges_id_seq OWNED BY public.account_webauthn_auth_challenges.id;

ALTER SEQUENCE public.account_webauthn_keys_id_seq OWNED BY public.account_webauthn_keys.id;

ALTER SEQUENCE public.account_webauthn_user_ids_id_seq OWNED BY public.account_webauthn_user_ids.id;

ALTER SEQUENCE public.accounts_id_seq OWNED BY public.accounts.id;

ALTER SEQUENCE public.active_storage_attachments_id_seq OWNED BY public.active_storage_attachments.id;

ALTER SEQUENCE public.active_storage_blobs_id_seq OWNED BY public.active_storage_blobs.id;

ALTER SEQUENCE public.active_storage_variant_records_id_seq OWNED BY public.active_storage_variant_records.id;

ALTER SEQUENCE public.api_app_tokens_id_seq OWNED BY public.api_app_tokens.id;

ALTER SEQUENCE public.api_change_events_id_seq OWNED BY public.api_change_events.id;

ALTER SEQUENCE public.api_household_selection_grants_id_seq OWNED BY public.api_household_selection_grants.id;

ALTER SEQUENCE public.api_idempotency_keys_id_seq OWNED BY public.api_idempotency_keys.id;

ALTER SEQUENCE public.api_sessions_id_seq OWNED BY public.api_sessions.id;

ALTER SEQUENCE public.api_tombstones_id_seq OWNED BY public.api_tombstones.id;

ALTER SEQUENCE public.app_settings_id_seq OWNED BY public.app_settings.id;

ALTER SEQUENCE public.audit_chain_heads_id_seq OWNED BY public.audit_chain_heads.id;

ALTER SEQUENCE public.audit_checkpoints_id_seq OWNED BY public.audit_checkpoints.id;

ALTER SEQUENCE public.audit_export_deliveries_id_seq OWNED BY public.audit_export_deliveries.id;

ALTER SEQUENCE public.audit_ledger_entries_id_seq OWNED BY public.audit_ledger_entries.id;

ALTER SEQUENCE public.audit_signing_keys_id_seq OWNED BY public.audit_signing_keys.id;

ALTER SEQUENCE public.barcode_catalog_entries_id_seq OWNED BY public.barcode_catalog_entries.id;

ALTER SEQUENCE public.carer_relationships_id_seq OWNED BY public.carer_relationships.id;

ALTER SEQUENCE public.dosages_id_seq OWNED BY public.dosages.id;

ALTER SEQUENCE public.health_event_medications_id_seq OWNED BY public.health_event_medications.id;

ALTER SEQUENCE public.health_events_id_seq OWNED BY public.health_events.id;

ALTER SEQUENCE public.household_exports_id_seq OWNED BY public.household_exports.id;

ALTER SEQUENCE public.household_invitation_grants_id_seq OWNED BY public.household_invitation_grants.id;

ALTER SEQUENCE public.household_invitations_id_seq OWNED BY public.household_invitations.id;

ALTER SEQUENCE public.household_memberships_id_seq OWNED BY public.household_memberships.id;

ALTER SEQUENCE public.household_purge_runs_id_seq OWNED BY public.household_purge_runs.id;

ALTER SEQUENCE public.household_retention_holds_id_seq OWNED BY public.household_retention_holds.id;

ALTER SEQUENCE public.households_id_seq OWNED BY public.households.id;

ALTER SEQUENCE public.location_memberships_id_seq OWNED BY public.location_memberships.id;

ALTER SEQUENCE public.locations_id_seq OWNED BY public.locations.id;

ALTER SEQUENCE public.medication_dose_occurrences_id_seq OWNED BY public.medication_dose_occurrences.id;

ALTER SEQUENCE public.medication_pause_periods_id_seq OWNED BY public.medication_pause_periods.id;

ALTER SEQUENCE public.medication_review_evidence_records_id_seq OWNED BY public.medication_review_evidence_records.id;

ALTER SEQUENCE public.medication_review_evidence_refresh_runs_id_seq OWNED BY public.medication_review_evidence_refresh_runs.id;

ALTER SEQUENCE public.medication_review_prompts_id_seq OWNED BY public.medication_review_prompts.id;

ALTER SEQUENCE public.medication_takes_id_seq OWNED BY public.medication_takes.id;

ALTER SEQUENCE public.medications_id_seq OWNED BY public.medications.id;

ALTER SEQUENCE public.native_device_tokens_id_seq OWNED BY public.native_device_tokens.id;

ALTER SEQUENCE public.nhs_dmd_amp_trade_families_id_seq OWNED BY public.nhs_dmd_amp_trade_families.id;

ALTER SEQUENCE public.nhs_dmd_ampp_relationships_id_seq OWNED BY public.nhs_dmd_ampp_relationships.id;

ALTER SEQUENCE public.nhs_dmd_barcodes_id_seq OWNED BY public.nhs_dmd_barcodes.id;

ALTER SEQUENCE public.nhs_dmd_imports_id_seq OWNED BY public.nhs_dmd_imports.id;

ALTER SEQUENCE public.nhs_dmd_supplementary_releases_id_seq OWNED BY public.nhs_dmd_supplementary_releases.id;

ALTER SEQUENCE public.nhs_dmd_trade_families_id_seq OWNED BY public.nhs_dmd_trade_families.id;

ALTER SEQUENCE public.nhs_dmd_trade_family_groups_id_seq OWNED BY public.nhs_dmd_trade_family_groups.id;

ALTER SEQUENCE public.notification_events_id_seq OWNED BY public.notification_events.id;

ALTER SEQUENCE public.notification_preferences_id_seq OWNED BY public.notification_preferences.id;

ALTER SEQUENCE public.oauth_applications_id_seq OWNED BY public.oauth_applications.id;

ALTER SEQUENCE public.oauth_grants_id_seq OWNED BY public.oauth_grants.id;

ALTER SEQUENCE public.people_id_seq OWNED BY public.people.id;

ALTER SEQUENCE public.person_access_grants_id_seq OWNED BY public.person_access_grants.id;

ALTER SEQUENCE public.person_medications_id_seq OWNED BY public.person_medications.id;

ALTER SEQUENCE public.platform_admins_id_seq OWNED BY public.platform_admins.id;

ALTER SEQUENCE public.push_subscriptions_id_seq OWNED BY public.push_subscriptions.id;

ALTER SEQUENCE public.schedules_id_seq OWNED BY public.schedules.id;

ALTER SEQUENCE public.security_audit_events_id_seq OWNED BY public.security_audit_events.id;

ALTER SEQUENCE public.storage_migration_runs_id_seq OWNED BY public.storage_migration_runs.id;

ALTER SEQUENCE public.support_access_sessions_id_seq OWNED BY public.support_access_sessions.id;


ALTER SEQUENCE public.versions_id_seq OWNED BY public.versions.id;

ALTER TABLE ONLY public.account_identities ALTER COLUMN id SET DEFAULT nextval('public.account_identities_id_seq'::regclass);

ALTER TABLE ONLY public.account_lockouts ALTER COLUMN account_id SET DEFAULT nextval('public.account_lockouts_account_id_seq'::regclass);

ALTER TABLE ONLY public.account_login_failures ALTER COLUMN account_id SET DEFAULT nextval('public.account_login_failures_account_id_seq'::regclass);

ALTER TABLE ONLY public.account_otp_keys ALTER COLUMN id SET DEFAULT nextval('public.account_otp_keys_id_seq'::regclass);

ALTER TABLE ONLY public.account_webauthn_auth_challenges ALTER COLUMN id SET DEFAULT nextval('public.account_webauthn_auth_challenges_id_seq'::regclass);

ALTER TABLE ONLY public.account_webauthn_keys ALTER COLUMN id SET DEFAULT nextval('public.account_webauthn_keys_id_seq'::regclass);

ALTER TABLE ONLY public.account_webauthn_user_ids ALTER COLUMN id SET DEFAULT nextval('public.account_webauthn_user_ids_id_seq'::regclass);

ALTER TABLE ONLY public.accounts ALTER COLUMN id SET DEFAULT nextval('public.accounts_id_seq'::regclass);

ALTER TABLE ONLY public.active_storage_attachments ALTER COLUMN id SET DEFAULT nextval('public.active_storage_attachments_id_seq'::regclass);

ALTER TABLE ONLY public.active_storage_blobs ALTER COLUMN id SET DEFAULT nextval('public.active_storage_blobs_id_seq'::regclass);

ALTER TABLE ONLY public.active_storage_variant_records ALTER COLUMN id SET DEFAULT nextval('public.active_storage_variant_records_id_seq'::regclass);

ALTER TABLE ONLY public.api_app_tokens ALTER COLUMN id SET DEFAULT nextval('public.api_app_tokens_id_seq'::regclass);

ALTER TABLE ONLY public.api_change_events ALTER COLUMN id SET DEFAULT nextval('public.api_change_events_id_seq'::regclass);

ALTER TABLE ONLY public.api_household_selection_grants ALTER COLUMN id SET DEFAULT nextval('public.api_household_selection_grants_id_seq'::regclass);

ALTER TABLE ONLY public.api_idempotency_keys ALTER COLUMN id SET DEFAULT nextval('public.api_idempotency_keys_id_seq'::regclass);

ALTER TABLE ONLY public.api_sessions ALTER COLUMN id SET DEFAULT nextval('public.api_sessions_id_seq'::regclass);

ALTER TABLE ONLY public.api_tombstones ALTER COLUMN id SET DEFAULT nextval('public.api_tombstones_id_seq'::regclass);

ALTER TABLE ONLY public.app_settings ALTER COLUMN id SET DEFAULT nextval('public.app_settings_id_seq'::regclass);

ALTER TABLE ONLY public.audit_chain_heads ALTER COLUMN id SET DEFAULT nextval('public.audit_chain_heads_id_seq'::regclass);

ALTER TABLE ONLY public.audit_checkpoints ALTER COLUMN id SET DEFAULT nextval('public.audit_checkpoints_id_seq'::regclass);

ALTER TABLE ONLY public.audit_export_deliveries ALTER COLUMN id SET DEFAULT nextval('public.audit_export_deliveries_id_seq'::regclass);

ALTER TABLE ONLY public.audit_ledger_entries ALTER COLUMN id SET DEFAULT nextval('public.audit_ledger_entries_id_seq'::regclass);

ALTER TABLE ONLY public.audit_signing_keys ALTER COLUMN id SET DEFAULT nextval('public.audit_signing_keys_id_seq'::regclass);

ALTER TABLE ONLY public.barcode_catalog_entries ALTER COLUMN id SET DEFAULT nextval('public.barcode_catalog_entries_id_seq'::regclass);

ALTER TABLE ONLY public.carer_relationships ALTER COLUMN id SET DEFAULT nextval('public.carer_relationships_id_seq'::regclass);

ALTER TABLE ONLY public.dosages ALTER COLUMN id SET DEFAULT nextval('public.dosages_id_seq'::regclass);

ALTER TABLE ONLY public.health_event_medications ALTER COLUMN id SET DEFAULT nextval('public.health_event_medications_id_seq'::regclass);

ALTER TABLE ONLY public.health_events ALTER COLUMN id SET DEFAULT nextval('public.health_events_id_seq'::regclass);

ALTER TABLE ONLY public.household_exports ALTER COLUMN id SET DEFAULT nextval('public.household_exports_id_seq'::regclass);

ALTER TABLE ONLY public.household_invitation_grants ALTER COLUMN id SET DEFAULT nextval('public.household_invitation_grants_id_seq'::regclass);

ALTER TABLE ONLY public.household_invitations ALTER COLUMN id SET DEFAULT nextval('public.household_invitations_id_seq'::regclass);

ALTER TABLE ONLY public.household_memberships ALTER COLUMN id SET DEFAULT nextval('public.household_memberships_id_seq'::regclass);

ALTER TABLE ONLY public.household_purge_runs ALTER COLUMN id SET DEFAULT nextval('public.household_purge_runs_id_seq'::regclass);

ALTER TABLE ONLY public.household_retention_holds ALTER COLUMN id SET DEFAULT nextval('public.household_retention_holds_id_seq'::regclass);

ALTER TABLE ONLY public.households ALTER COLUMN id SET DEFAULT nextval('public.households_id_seq'::regclass);

ALTER TABLE ONLY public.location_memberships ALTER COLUMN id SET DEFAULT nextval('public.location_memberships_id_seq'::regclass);

ALTER TABLE ONLY public.locations ALTER COLUMN id SET DEFAULT nextval('public.locations_id_seq'::regclass);

ALTER TABLE ONLY public.medication_dose_occurrences ALTER COLUMN id SET DEFAULT nextval('public.medication_dose_occurrences_id_seq'::regclass);

ALTER TABLE ONLY public.medication_pause_periods ALTER COLUMN id SET DEFAULT nextval('public.medication_pause_periods_id_seq'::regclass);

ALTER TABLE ONLY public.medication_review_evidence_records ALTER COLUMN id SET DEFAULT nextval('public.medication_review_evidence_records_id_seq'::regclass);

ALTER TABLE ONLY public.medication_review_evidence_refresh_runs ALTER COLUMN id SET DEFAULT nextval('public.medication_review_evidence_refresh_runs_id_seq'::regclass);

ALTER TABLE ONLY public.medication_review_prompts ALTER COLUMN id SET DEFAULT nextval('public.medication_review_prompts_id_seq'::regclass);

ALTER TABLE ONLY public.medication_takes ALTER COLUMN id SET DEFAULT nextval('public.medication_takes_id_seq'::regclass);

ALTER TABLE ONLY public.medications ALTER COLUMN id SET DEFAULT nextval('public.medications_id_seq'::regclass);

ALTER TABLE ONLY public.native_device_tokens ALTER COLUMN id SET DEFAULT nextval('public.native_device_tokens_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_amp_trade_families ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_amp_trade_families_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_ampp_relationships ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_ampp_relationships_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_barcodes ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_barcodes_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_imports ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_imports_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_supplementary_releases ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_supplementary_releases_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_trade_families ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_trade_families_id_seq'::regclass);

ALTER TABLE ONLY public.nhs_dmd_trade_family_groups ALTER COLUMN id SET DEFAULT nextval('public.nhs_dmd_trade_family_groups_id_seq'::regclass);

ALTER TABLE ONLY public.notification_events ALTER COLUMN id SET DEFAULT nextval('public.notification_events_id_seq'::regclass);

ALTER TABLE ONLY public.notification_preferences ALTER COLUMN id SET DEFAULT nextval('public.notification_preferences_id_seq'::regclass);

ALTER TABLE ONLY public.oauth_applications ALTER COLUMN id SET DEFAULT nextval('public.oauth_applications_id_seq'::regclass);

ALTER TABLE ONLY public.oauth_grants ALTER COLUMN id SET DEFAULT nextval('public.oauth_grants_id_seq'::regclass);

ALTER TABLE ONLY public.people ALTER COLUMN id SET DEFAULT nextval('public.people_id_seq'::regclass);

ALTER TABLE ONLY public.person_access_grants ALTER COLUMN id SET DEFAULT nextval('public.person_access_grants_id_seq'::regclass);

ALTER TABLE ONLY public.person_medications ALTER COLUMN id SET DEFAULT nextval('public.person_medications_id_seq'::regclass);

ALTER TABLE ONLY public.platform_admins ALTER COLUMN id SET DEFAULT nextval('public.platform_admins_id_seq'::regclass);

ALTER TABLE ONLY public.push_subscriptions ALTER COLUMN id SET DEFAULT nextval('public.push_subscriptions_id_seq'::regclass);

ALTER TABLE ONLY public.schedules ALTER COLUMN id SET DEFAULT nextval('public.schedules_id_seq'::regclass);

ALTER TABLE ONLY public.security_audit_events ALTER COLUMN id SET DEFAULT nextval('public.security_audit_events_id_seq'::regclass);

ALTER TABLE ONLY public.storage_migration_runs ALTER COLUMN id SET DEFAULT nextval('public.storage_migration_runs_id_seq'::regclass);

ALTER TABLE ONLY public.support_access_sessions ALTER COLUMN id SET DEFAULT nextval('public.support_access_sessions_id_seq'::regclass);


ALTER TABLE ONLY public.versions ALTER COLUMN id SET DEFAULT nextval('public.versions_id_seq'::regclass);
