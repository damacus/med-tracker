use serde::Serialize;

#[derive(Serialize)]
pub struct Location {
    pub id: i64,
    pub portable_id: String,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: String,
}

pub type LocationResponse = super::DataResponse<Location>;

pub type LocationCollectionResponse = super::PaginatedResponse<Location>;
