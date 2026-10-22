use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(better_auth_seaorm::AuthEntity, Clone, Debug, PartialEq, Serialize, DeriveEntityModel)]
#[auth(role = "verification")]
#[sea_orm(table_name = "identity_verifications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub identifier: String,
    pub value: String,
    pub expires_at: DateTimeUtc,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
