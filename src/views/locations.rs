use super::PaginationMeta;
use serde::Serialize;

#[derive(Serialize)]
pub struct Location {
    pub id: i64,
    pub portable_id: String,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct LocationResponse {
    pub data: Location,
}

#[derive(Serialize)]
pub struct LocationCollectionResponse {
    pub data: Vec<Location>,
    pub meta: PaginationMeta,
}

#[derive(Serialize)]
pub struct LocationMembership {
    pub id: String,
    pub location_id: String,
    pub location_portable_id: String,
    pub person_id: String,
    pub person_portable_id: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct LocationMembershipResponse {
    pub data: LocationMembership,
}
