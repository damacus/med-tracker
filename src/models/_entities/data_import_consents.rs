use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "data_import_consents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub household_id: i64,
    pub requester_account_id: i64,
    pub requester_membership_id: i64,
    pub recipient_account_id: i64,
    pub recipient_membership_id: i64,
    pub target_person_id: i64,
    pub source_sha256: String,
    pub source_person: Json,
    pub decision: String,
    pub created_at: DateTime,
    pub decided_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
