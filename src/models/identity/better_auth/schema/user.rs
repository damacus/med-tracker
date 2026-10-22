use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(better_auth_seaorm::AuthEntity, Clone, Debug, PartialEq, Serialize, DeriveEntityModel)]
#[auth(role = "user")]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub person_id: i64,
    pub preferences: Json,
    pub status: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub email_verified: bool,
    pub image: Option<String>,
    #[sea_orm(default_value = false)]
    pub two_factor_enabled: bool,
    #[sea_orm(default_expr = "Expr::cust(\"('{}')\")")]
    pub metadata: Json,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
