macro_rules! entity {
    ($name:ident, $table:literal, { $($field:ident: $kind:ty,)* }) => {
        pub mod $name {
            use sea_orm::entity::prelude::*;

            #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
            #[sea_orm(table_name = $table)]
            pub struct Model {
                #[sea_orm(primary_key)]
                pub id: i64,
                $(pub $field: $kind,)*
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}

            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

entity!(api_session, "api_sessions", {
    account_id: i64,
    household_membership_id: Option<i64>,
    access_token_digest: String,
    access_expires_at: DateTime,
    revoked_at: Option<DateTime>,
    permissions_version: i32,
    refresh_expires_at: DateTime,
    last_used_at: DateTime,
    device_name: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(api_app_token, "api_app_tokens", {
    account_id: i64,
    household_membership_id: i64,
    token_digest: String,
    permissions_version: i32,
    name: String,
    expires_at: DateTime,
    revoked_at: Option<DateTime>,
    last_used_at: DateTime,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(oauth_grant, "oauth_grants", {
    account_id: i64,
    oauth_application_id: i64,
    client_kind: String,
    code: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    redirect_uri: Option<String>,
    token_hash: Option<String>,
    refresh_token_hash: Option<String>,
    expires_in: DateTime,
    revoked_at: Option<DateTime>,
    scopes: String,
    authenticated_at: Option<DateTime>,
    last_used_at: Option<DateTime>,
    device_name: Option<String>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(oauth_application, "oauth_applications", {
    client_id: String,
    client_kind: String,
    name: String,
    redirect_uri: String,
    scopes: String,
    token_endpoint_auth_method: String,
    client_secret: Option<String>,
    client_secret_hash: Option<String>,
});

entity!(otp_key, "account_otp_keys", {});

entity!(webauthn_key, "account_webauthn_keys", {
    account_id: i64,
    webauthn_id: String,
    public_key: String,
    nickname: Option<String>,
    sign_count: i32,
    last_use: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(webauthn_user_id, "account_webauthn_user_ids", {
    account_id: i64,
    webauthn_id: String,
    created_at: DateTime,
    updated_at: DateTime,
});

pub mod recovery_code {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "account_recovery_codes")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: i64,
        #[sea_orm(primary_key, auto_increment = false)]
        pub code: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

entity!(security_audit_event, "security_audit_events", {
    household_id: i64,
    actor_account_id: Option<i64>,
    actor_membership_id: Option<i64>,
    event_type: String,
    request_id: Option<String>,
    ip: Option<String>,
    metadata: serde_json::Value,
    audit_context: serde_json::Value,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(account, "accounts", {
    status: i32,
    email: String,
    password_hash: Option<String>,
    preferences: Json,
    updated_at: DateTime,
});

entity!(active_storage_blob, "active_storage_blobs", {
    key: String,
    filename: String,
    content_type: Option<String>,
    byte_size: i64,
    checksum: Option<String>,
    metadata: Option<String>,
    service_name: String,
    created_at: DateTime,
});

entity!(active_storage_attachment, "active_storage_attachments", {
    name: String,
    record_type: String,
    record_id: i64,
    blob_id: i64,
    household_id: i64,
    created_at: DateTime,
});

entity!(native_device_token, "native_device_tokens", {
    account_id: i64,
    device_token: String,
    platform: String,
    apns_environment: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(push_subscription, "push_subscriptions", {
    account_id: i64,
    endpoint: String,
    p256dh: String,
    auth: String,
    user_agent: Option<String>,
    created_at: DateTime,
    updated_at: DateTime,
});

pub mod active_session_key {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "account_active_session_keys")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub account_id: i64,
        #[sea_orm(primary_key, auto_increment = false)]
        pub session_id: String,
        pub created_at: DateTime,
        pub last_use: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod account_lockout {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "account_lockouts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub account_id: i64,
        pub deadline: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

entity!(household, "households", {
    status: String,
    lifecycle_state: String,
    slug: String,
    name: String,
    timezone: String,
    subscription_plan: String,
    updated_at: DateTime,
});

entity!(household_invitation, "household_invitations", {
    household_id: i64,
    email: String,
    membership_role: String,
    token_digest: String,
    invited_by_membership_id: i64,
    expires_at: DateTime,
    accepted_at: Option<DateTime>,
    revoked_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(household_invitation_grant, "household_invitation_grants", {
    household_id: i64,
    household_invitation_id: i64,
    person_id: i64,
    access_level: String,
    relationship_type: String,
    expires_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(api_idempotency_key, "api_idempotency_keys", {
    household_id: i64,
    account_id: i64,
    api_session_id: Option<i64>,
    api_app_token_id: Option<i64>,
    key: String,
    request_method: String,
    request_path: String,
    request_digest: String,
    response_status: i32,
    response_body: serde_json::Value,
    response_headers: serde_json::Value,
    expires_at: DateTime,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(membership, "household_memberships", {
    account_id: i64,
    household_id: i64,
    person_id: Option<i64>,
    permissions_version: i32,
    role: String,
    status: String,
    revoked_at: Option<DateTime>,
    joined_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(platform_admin, "platform_admins", {
    account_id: i64,
    status: String,
});

entity!(user, "users", {
    person_id: i64,
    active: bool,
    email_address: String,
});

entity!(person, "people", {
    account_id: Option<i64>,
    household_id: i64,
    portable_id: String,
    name: String,
    email: Option<String>,
    person_type: i32,
    date_of_birth: Option<Date>,
    has_capacity: bool,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(health_event, "health_events", {
    household_id: i64,
    person_id: i64,
    portable_id: String,
    event_kind: i32,
    severity: Option<i32>,
    title: String,
    notes: Option<String>,
    started_on: Date,
    ended_on: Option<Date>,
    action_taken: Option<String>,
    medical_help_sought: bool,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(health_event_medication, "health_event_medications", {
    household_id: i64,
    health_event_id: i64,
    medication_id: Option<i64>,
    medication_name: String,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(review_evidence, "medication_review_evidence_records", {
    active_ingredient: Option<String>,
    candidate_terms: Vec<String>,
    interacting_terms: Vec<String>,
    pharmacologic_classes: Vec<String>,
    evidence_text: String,
    label_section: String,
    match_confidence: String,
    match_status: String,
    product_name: String,
    retrieved_on: Date,
    risk_level: String,
    source_effective_on: Option<Date>,
    source_name: String,
    source_record_id: String,
    source_url: String,
    source_version: Option<String>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(review_prompt, "medication_review_prompts", {
    evidence_record_id: i64,
    evidence_source_checked_on: Date,
    evidence_source_effective_on: Date,
    evidence_source_name: String,
    evidence_source_url: String,
    evidence_source_version: String,
    evidence_text: String,
    household_id: i64,
    interacting_medication_id: i64,
    interacting_medication_name: String,
    match_confidence: String,
    match_reason: String,
    match_type: String,
    matched_term: String,
    person_id: i64,
    practitioner_name: Option<String>,
    practitioner_role: Option<String>,
    primary_medication_id: i64,
    primary_medication_name: String,
    review_note: Option<String>,
    reviewed_by_membership_id: Option<i64>,
    reviewed_on: Option<Date>,
    risk_level: String,
    source_instruction: String,
    status: String,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(grant, "person_access_grants", {
    household_id: i64,
    household_membership_id: i64,
    person_id: i64,
    access_level: String,
    expires_at: Option<DateTime>,
    revoked_at: Option<DateTime>,
    relationship_type: String,
    granted_by_membership_id: Option<i64>,
    carer_relationship_id: Option<i64>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(schedule, "schedules", {
    household_id: i64,
    portable_id: String,
    person_id: i64,
    medication_id: i64,
    active: bool,
    retired_at: Option<DateTime>,
    start_date: Option<Date>,
    end_date: Option<Date>,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    source_dosage_option_id: Option<i64>,
    dose_cycle: Option<i32>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    schedule_type: i32,
    schedule_config: serde_json::Value,
    frequency: Option<String>,
    notes: Option<String>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(pause_period, "medication_pause_periods", {
    household_id: i64,
    portable_id: String,
    schedule_id: Option<i64>,
    person_medication_id: Option<i64>,
    reason: String,
    note: Option<String>,
    legacy_context: bool,
    imported_context: bool,
    imported_actor_references: serde_json::Value,
    recorded_by_membership_id: Option<i64>,
    resumed_by_membership_id: Option<i64>,
    started_at: Option<DateTime>,
    ended_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(dose_occurrence, "medication_dose_occurrences", {
    household_id: i64,
    portable_id: String,
    schedule_id: Option<i64>,
    person_medication_id: Option<i64>,
    medication_take_id: Option<i64>,
    resolved_by_membership_id: Option<i64>,
    window_starts_on: Date,
    window_ends_on: Option<Date>,
    position: i32,
    scheduled_at: Option<DateTime>,
    outcome: String,
    reason: Option<String>,
    note: Option<String>,
    resolved_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(person_medication, "person_medications", {
    household_id: i64,
    portable_id: String,
    person_id: i64,
    medication_id: i64,
    active: bool,
    retired_at: Option<DateTime>,
    min_hours_between_doses: Option<i32>,
    administration_kind: i32,
    source_dosage_option_id: Option<i64>,
    dose_cycle: Option<i32>,
    max_daily_doses: Option<i32>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    notes: Option<String>,
    position: i32,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(medication_take, "medication_takes", {
    household_id: i64,
    portable_id: String,
    client_uuid: Option<String>,
    schedule_id: Option<i64>,
    person_medication_id: Option<i64>,
    taken_from_medication_id: Option<i64>,
    taken_from_location_id: Option<i64>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    taken_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(dosage, "dosages", {
    household_id: i64,
    medication_id: i64,
    portable_id: String,
    amount: Decimal,
    unit: String,
    current_supply: Option<Decimal>,
    reorder_threshold: Option<Decimal>,
    frequency: String,
    description: Option<String>,
    default_for_adults: bool,
    default_for_children: bool,
    default_max_daily_doses: i32,
    default_min_hours_between_doses: Decimal,
    default_dose_cycle: i32,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(version, "versions", {
    item_type: String,
    item_id: i64,
    event: String,
    object: Option<String>,
    object_changes: Option<String>,
    whodunnit: Option<String>,
    request_id: Option<String>,
    household_id: Option<i64>,
    actor_membership_id: Option<i64>,
    audit_context: serde_json::Value,
    created_at: Option<DateTime>,
});

entity!(api_change_event, "api_change_events", {
    household_id: i64,
    household_membership_id: Option<i64>,
    account_id: Option<i64>,
    action: String,
    record_type: String,
    record_id: i64,
    record_portable_id: Option<String>,
    request_id: Option<String>,
    metadata: serde_json::Value,
    occurred_at: DateTime,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(api_tombstone, "api_tombstones", {
    household_id: i64,
    household_membership_id: Option<i64>,
    account_id: Option<i64>,
    action: String,
    record_type: String,
    record_portable_id: String,
    metadata: serde_json::Value,
    deleted_at: DateTime,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(location, "locations", {
    household_id: i64,
    portable_id: String,
    name: String,
});

entity!(medication, "medications", {
    household_id: i64,
    created_at: DateTime,
    created_by_membership_id: Option<i64>,
    portable_id: String,
    name: Option<String>,
    friendly_name: Option<String>,
    category: Option<String>,
    description: Option<String>,
    barcode: Option<String>,
    dmd_code: Option<String>,
    dmd_system: Option<String>,
    dmd_concept_class: Option<String>,
    dose_amount: Option<f64>,
    dose_unit: Option<String>,
    current_supply: Option<Decimal>,
    supply_at_last_restock: Option<Decimal>,
    reorder_threshold: Decimal,
    reorder_status: Option<i32>,
    order_supplier: Option<String>,
    order_quantity: Option<Decimal>,
    expected_arrival_on: Option<Date>,
    ordered_at: Option<DateTime>,
    reordered_at: Option<DateTime>,
    warnings: Option<String>,
    default_schedule_type: i32,
    location_id: i64,
    updated_at: DateTime,
});

entity!(nhs_dmd_import, "nhs_dmd_imports", {
    archive_byte_size: Option<i64>,
    archive_checksum: Option<String>,
    archive_key: Option<String>,
    archive_service_name: Option<String>,
    completed_at: Option<DateTime>,
    created_at: DateTime,
    created_count: i32,
    error_message: Option<String>,
    imported_count: i32,
    log: Option<String>,
    processed_records: i32,
    skipped_count: i32,
    skipped_expired_count: i32,
    skipped_invalid_count: i32,
    skipped_missing_name_count: i32,
    started_at: Option<DateTime>,
    status: i32,
    total_records: i32,
    unchanged_count: i32,
    updated_at: DateTime,
    updated_count: i32,
    uploaded_filename: String,
});

entity!(nhs_dmd_barcode, "nhs_dmd_barcodes", {
    amp_code: Option<String>,
    code: String,
    concept_class: Option<String>,
    created_at: DateTime,
    display: String,
    gtin: String,
    system: String,
    updated_at: DateTime,
    vmp_name: Option<String>,
});

entity!(nhs_dmd_ampp_relationship, "nhs_dmd_ampp_relationships", {
    amp_code: String,
    ampp_code: String,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(nhs_dmd_trade_family_group, "nhs_dmd_trade_family_groups", {
    code: String,
    name: String,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(nhs_dmd_trade_family, "nhs_dmd_trade_families", {
    code: String,
    name: String,
    trade_family_group_id: Option<i64>,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(nhs_dmd_amp_trade_family, "nhs_dmd_amp_trade_families", {
    amp_code: String,
    trade_family_id: i64,
    created_at: DateTime,
    updated_at: DateTime,
});

entity!(nhs_dmd_supplementary_release, "nhs_dmd_supplementary_releases", {
    released_on: Date,
    created_at: DateTime,
    updated_at: DateTime,
});
