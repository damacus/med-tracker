use serde::Serialize;

#[derive(Serialize)]
pub struct InvitationSummary {
    pub id: i64,
    pub email: String,
    pub membership_role: String,
    pub pending: bool,
    pub accepted_at: Option<String>,
    pub revoked_at: Option<String>,
    pub expires_at: String,
}

#[derive(Serialize)]
pub struct ResentInvitation {
    pub invitation_id: String,
    pub expires_at: String,
    pub delivery_status: DeliveryStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Queued,
}

#[derive(Serialize)]
pub(crate) struct InvitationAcceptance {
    pub(crate) household_id: String,
    pub(crate) membership_id: String,
    pub(crate) person_id: String,
    pub(crate) role: String,
}
