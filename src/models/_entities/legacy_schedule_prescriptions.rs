use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "legacy_schedule_prescriptions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub schedule_id: i64,
    pub source_portable_id: String,
    pub person_id: i64,
    pub household_id: i64,
    pub origin_stock_id: i64,
    pub origin_stock_portable_id: String,
    pub medicine_name: Option<String>,
    pub dmd_system: Option<String>,
    pub dmd_code: Option<String>,
    pub dmd_concept_class: Option<String>,
    pub stock_unit: Option<String>,
    pub stock_dose_default: Option<Decimal>,
    pub stock_default_known: bool,
    pub active: bool,
    pub retired_at: Option<DateTime>,
    pub dose_amount: Option<Decimal>,
    pub dose_unit: Option<String>,
    pub dose_cycle: Option<i32>,
    pub frequency: Option<String>,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub max_daily_doses: Option<i32>,
    pub min_hours_between_doses: Option<i32>,
    pub schedule_type: i32,
    pub schedule_config: Json,
    pub origin_time_zone: Option<String>,
    pub pause_started_at: Option<DateTime>,
    pub pause_instance_id: Option<String>,
    pub source_created_at: DateTime,
    pub frozen_at: DateTime,
    pub source_ended_at: DateTime,
    pub source_closed_active: bool,
    pub captured_by_account_id: i64,
    pub captured_by_membership_id: i64,
    pub request_id: String,
    pub adopted_person_schedule_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
