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

entity!(account, "accounts", { status: i32, email: String, password_hash: Option<String>, });

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
});

entity!(membership, "household_memberships", {
    account_id: i64,
    household_id: i64,
    person_id: Option<i64>,
    permissions_version: i32,
    role: String,
    status: String,
    revoked_at: Option<DateTime>,
    updated_at: DateTime,
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
    min_hours_between_doses: Option<Decimal>,
    source_dosage_option_id: Option<i64>,
    dose_cycle: Option<i32>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    schedule_type: i32,
    schedule_config: serde_json::Value,
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
