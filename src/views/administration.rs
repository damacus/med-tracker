use serde::Serialize;

#[derive(Serialize)]
pub struct HouseholdSettings {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub timezone: String,
    pub subscription_plan: String,
    pub ai_suggestions_enabled: bool,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct MembershipSummary {
    pub id: i64,
    pub account_id: Option<i64>,
    pub email: Option<String>,
    pub person_id: i64,
    pub person_name: Option<String>,
    pub role: String,
    pub status: String,
    pub permissions_version: i32,
    pub joined_at: String,
}

#[derive(Serialize)]
pub struct PersonGrant {
    pub id: i64,
    pub household_membership_id: i64,
    pub person_id: i64,
    pub person_name: Option<String>,
    pub access_level: String,
    pub relationship_type: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}
