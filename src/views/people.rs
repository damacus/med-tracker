use crate::models::care::people::PersonKind;
use serde::Serialize;

#[derive(Serialize)]
pub struct Person {
    pub id: i64,
    pub portable_id: String,
    pub updated_at: String,
    pub name: String,
    pub email: Option<String>,
    pub date_of_birth: Option<String>,
    pub person_type: PersonKind,
    pub has_capacity: bool,
    pub age: Option<i32>,
    pub location_ids: Vec<i64>,
    pub location_portable_ids: Vec<String>,
    pub notification_preference_id: Option<i64>,
    pub notification_preference_portable_id: Option<String>,
}

pub type PersonResponse = super::DataResponse<Person>;

pub type PersonCollectionResponse = super::PaginatedResponse<Person>;

pub(crate) type MeResponse = super::DataResponse<Me>;

#[derive(Serialize)]
pub(crate) struct Me {
    pub(crate) id: i64,
    pub(crate) email_address: String,
    pub(crate) membership_role: String,
    pub(crate) active: bool,
    pub(crate) person: Person,
    pub(crate) account: MeAccount,
}

#[derive(Serialize)]
pub(crate) struct MeAccount {
    pub(crate) id: i64,
    pub(crate) email: String,
    pub(crate) status: MeAccountStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MeAccountStatus {
    Unverified,
    Verified,
    Closed,
}
