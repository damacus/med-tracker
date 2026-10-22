use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    better_auth_seaorm::AuthEntity,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    DeriveEntityModel,
)]
#[auth(role = "account")]
#[sea_orm(table_name = "auth_credentials")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub account_id: String,
    pub provider_id: String,
    pub user_id: i64,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub access_token_expires_at: Option<DateTimeUtc>,
    pub refresh_token_expires_at: Option<DateTimeUtc>,
    pub scope: Option<String>,
    pub password: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
