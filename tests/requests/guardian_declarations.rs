use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "guardian_declarations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub subject_person_id: i64,
    pub guardian_person_id: i64,
    pub declared_by_account_id: i64,
    pub origin_household_id: i64,
    pub statement_version: i16,
    pub revision: i64,
    pub withdrawn_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
