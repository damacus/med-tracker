use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, better_auth_seaorm::AuthEntity)]
#[auth(role = "passkey")]
#[sea_orm(table_name = "identity_passkeys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: Option<String>,
    pub public_key: String,
    #[sea_orm(column_name = "account_id")]
    pub user_id: i64,
    pub credential_id: String,
    pub counter: i64,
    pub device_type: String,
    pub backed_up: bool,
    pub transports: Option<String>,
    pub credential: String,
    pub aaguid: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
