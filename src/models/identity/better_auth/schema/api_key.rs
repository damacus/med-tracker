use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, better_auth_seaorm::AuthEntity)]
#[auth(role = "api_key")]
#[sea_orm(table_name = "identity_api_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: Option<String>,
    pub start: Option<String>,
    pub prefix: Option<String>,
    #[sea_orm(column_name = "key")]
    pub key_hash: String,
    pub reference_id: i64,
    pub config_id: String,
    pub refill_interval: Option<f64>,
    pub refill_amount: Option<f64>,
    pub last_refill_at: Option<DateTimeUtc>,
    pub enabled: bool,
    pub rate_limit_enabled: bool,
    pub rate_limit_time_window: Option<f64>,
    pub rate_limit_max: Option<f64>,
    pub request_count: Option<f64>,
    pub remaining: Option<f64>,
    pub last_request: Option<DateTimeUtc>,
    pub expires_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub permissions: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
