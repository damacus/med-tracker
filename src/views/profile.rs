use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Profile {
    pub person_id: String,
    pub account_id: String,
    pub date_of_birth: chrono::NaiveDate,
    pub time_zone: String,
    pub gravatar_enabled: bool,
    pub mobile_shortcuts: Vec<String>,
    pub avatar_attached: bool,
}

#[derive(Serialize)]
pub(crate) struct NotificationPreference {
    pub id: i64,
    pub portable_id: String,
    pub person_id: i64,
    pub person_portable_id: String,
    pub enabled: bool,
    pub dose_due_enabled: bool,
    pub missed_dose_enabled: bool,
    pub low_stock_enabled: bool,
    pub private_text_enabled: bool,
    pub morning_time: Option<String>,
    pub afternoon_time: Option<String>,
    pub evening_time: Option<String>,
    pub night_time: Option<String>,
    pub updated_at: String,
}
