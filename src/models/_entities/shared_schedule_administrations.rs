use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "shared_schedule_administrations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub person_schedule_id: String,
    pub household_id: i64,
    pub medication_take_id: i64,
    pub taken_at: DateTime,
    pub occurrence_starts_on: Option<Date>,
    pub occurrence_position: Option<i32>,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
