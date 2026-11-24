use crate::models::profile::self_account::Shortcut;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct AccountProfile {
    pub account_id: String,
    pub name: Option<String>,
    pub email: String,
    pub time_zone: &'static str,
    pub gravatar_enabled: bool,
    pub avatar_attached: bool,
    pub mobile_shortcuts: Vec<Shortcut>,
}

#[derive(Serialize)]
pub(crate) struct HouseholdList {
    pub account_id: i64,
    pub data: Vec<Household>,
}

#[derive(Serialize)]
pub(crate) struct Household {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub role: String,
    pub membership_id: i64,
}

#[derive(Serialize)]
pub(crate) struct Session {
    pub id: i64,
    pub device_name: Option<String>,
    pub household_id: Option<i64>,
    pub last_used_at: String,
    pub access_token_expires_at: String,
    pub refresh_token_expires_at: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub(crate) struct ErrorResponse<'a> {
    pub error: Error<'a>,
}

#[derive(Serialize)]
pub(crate) struct Error<'a> {
    pub code: &'a str,
    pub message: &'a str,
    pub request_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
