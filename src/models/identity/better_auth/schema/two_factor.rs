use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, better_auth_seaorm::AuthEntity)]
#[auth(role = "two_factor")]
#[sea_orm(table_name = "identity_two_factors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub secret: String,
    pub backup_codes: String,
    #[sea_orm(column_name = "account_id")]
    pub user_id: i64,
    pub verified: bool,
    pub failed_verification_count: i64,
    pub locked_until: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
