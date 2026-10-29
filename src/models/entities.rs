pub use super::_entities::account_lockouts as account_lockout;
pub use super::_entities::accounts as account;
pub use super::_entities::active_storage_attachments as active_storage_attachment;
pub use super::_entities::active_storage_blobs as active_storage_blob;
pub use super::_entities::api_app_tokens as api_app_token;
pub use super::_entities::api_change_events as api_change_event;
pub use super::_entities::api_idempotency_keys as api_idempotency_key;
pub use super::_entities::api_sessions as api_session;
pub use super::_entities::api_tombstones as api_tombstone;
pub use super::_entities::barcode_catalog_entries as barcode_catalog_entry;
pub use super::_entities::carer_relationships as carer_relationship;
pub use super::_entities::dosages as dosage;
pub use super::_entities::health_event_medications as health_event_medication;
pub use super::_entities::health_events as health_event;
pub use super::_entities::household_invitation_grants as household_invitation_grant;
pub use super::_entities::household_invitations as household_invitation;
pub use super::_entities::household_memberships as membership;
pub use super::_entities::households as household;
pub use super::_entities::location_memberships as location_membership;
pub use super::_entities::locations as location;
pub use super::_entities::medication_dose_occurrences as dose_occurrence;
pub use super::_entities::medication_pause_periods as pause_period;
pub use super::_entities::medication_review_evidence_records as review_evidence;
pub use super::_entities::medication_review_prompts as review_prompt;
pub use super::_entities::medication_takes as medication_take;
pub use super::_entities::medications as medication;
pub use super::_entities::native_device_tokens as native_device_token;
pub use super::_entities::nhs_dmd_amp_trade_families as nhs_dmd_amp_trade_family;
pub use super::_entities::nhs_dmd_ampp_relationships as nhs_dmd_ampp_relationship;
pub use super::_entities::nhs_dmd_barcodes as nhs_dmd_barcode;
pub use super::_entities::nhs_dmd_imports as nhs_dmd_import;
pub use super::_entities::nhs_dmd_supplementary_releases as nhs_dmd_supplementary_release;
pub use super::_entities::nhs_dmd_trade_families as nhs_dmd_trade_family;
pub use super::_entities::nhs_dmd_trade_family_groups as nhs_dmd_trade_family_group;
pub use super::_entities::notification_preferences as notification_preference;
pub use super::_entities::oauth_applications as oauth_application;
pub use super::_entities::oauth_grants as oauth_grant;
pub use super::_entities::people as person;
pub use super::_entities::person_access_grants as grant;
pub use super::_entities::person_medications as person_medication;
pub use super::_entities::platform_admins as platform_admin;
pub use super::_entities::push_subscriptions as push_subscription;
pub use super::_entities::schedules as schedule;
pub use super::_entities::security_audit_events as security_audit_event;
pub use super::_entities::versions as version;

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

pub mod invitation_mail_attempt {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "invitation_mail_attempts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub household_id: i64,
        pub invitation_id: Option<i64>,
        pub original_invitation_id: i64,
        pub sender_account_id: i64,
        pub sender_membership_id: i64,
        pub sender_permissions_version: i32,
        pub token_digest: String,
        pub status: String,
        pub category: Option<String>,
        pub queued_at: DateTime,
        pub started_at: Option<DateTime>,
        pub finished_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub use super::identity::better_auth::schema::{
    api_key as identity_api_key, passkey as identity_passkey, session as identity_session,
    verification as identity_verification,
};
