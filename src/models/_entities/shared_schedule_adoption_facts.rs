use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "shared_schedule_adoption_facts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub medication_take_id: i64,
    pub person_schedule_id: Option<String>,
    pub source_schedule_id: i64,
    pub source_portable_id: String,
    pub person_id: i64,
    pub household_id: i64,
    pub take_portable_id: String,
    pub original_version_id: i64,
    pub client_uuid: Option<String>,
    pub taken_at: DateTime,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub dose_amount: Decimal,
    pub dose_unit: Option<String>,
    pub taken_from_medication_id: Option<i64>,
    pub taken_from_location_id: Option<i64>,
    pub take_created_at: DateTime,
    pub take_updated_at: DateTime,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub fact_hash: Vec<u8>,
    pub captured_by_account_id: i64,
    pub captured_by_person_id: i64,
    pub captured_by_membership_id: i64,
    pub guardian_declaration_id: Option<i64>,
    pub request_id: String,
    pub authentication_method: Option<String>,
    pub credential_reference: Option<String>,
    pub captured_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
