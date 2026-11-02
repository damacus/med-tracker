use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "person_household_approvals")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub household_id: i64,
    pub person_id: i64,
    pub invitation_id: i64,
    pub approved_by_account_id: i64,
    pub approved_by_person_id: i64,
    pub guardian_declaration_id: Option<i64>,
    pub statement_version: i16,
    pub approved_at: DateTime,
    pub ended_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
