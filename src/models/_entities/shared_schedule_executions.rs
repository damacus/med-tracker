use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "shared_schedule_executions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub schedule_id: i64,
    pub portable_id: String,
    pub household_id: i64,
    pub approval_id: Option<i64>,
    pub person_id: i64,
    pub person_schedule_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
