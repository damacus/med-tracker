use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(better_auth_seaorm::AuthEntity, Clone, Debug, PartialEq, Serialize, DeriveEntityModel)]
#[auth(role = "session")]
#[sea_orm(table_name = "identity_sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub expires_at: DateTimeUtc,
    pub token: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    #[sea_orm(column_name = "account_id")]
    pub user_id: i64,
    pub active_household_id: Option<i64>,
    #[sea_orm(default_value = true)]
    pub active: bool,
    #[sea_orm(column_name = "purpose")]
    pub clinical_session_purpose: String,
    pub authentication_method: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
