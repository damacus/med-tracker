use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_allocations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub household_id: i64,
    pub medication_id: i64,
    pub dosage_id: Option<i64>,
    pub person_id: i64,
    pub actor_account_id: i64,
    pub submission_id: Uuid,
    pub stock_unit: String,
    pub quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
