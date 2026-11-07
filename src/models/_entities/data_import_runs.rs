use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "data_import_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub household_id: i64,
    pub created_by_membership_id: i64,
    pub source_timezone: String,
    pub encrypted_prepared: Vec<u8>,
    pub report: Json,
    pub receipt: Json,
    pub mappings: Json,
    pub consents: Json,
    pub acknowledged_omissions: Json,
    pub confirmation: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
