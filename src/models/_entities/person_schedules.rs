use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "person_schedules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub person_id: i64,
    pub origin_household_id: i64,
    pub origin_schedule_id: i64,
    pub origin_stock_id: i64,
    pub origin_stock_portable_id: String,
    pub medicine_name: Option<String>,
    pub dmd_system: Option<String>,
    pub dmd_code: Option<String>,
    pub dmd_concept_class: Option<String>,
    pub stock_unit: Option<String>,
    pub stock_dose_default: Option<Decimal>,
    pub stock_default_known: bool,
    pub pause_started_at: Option<DateTime>,
    pub pause_instance_id: Option<String>,
    pub pause_reason: Option<String>,
    pub pause_note: Option<String>,
    pub origin_time_zone: String,
    pub timezone_provenance: String,
    pub clinical_revision: i64,
    pub active: bool,
    pub retired_at: Option<DateTime>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))", nullable)]
    pub dose_amount: Option<Decimal>,
    pub dose_unit: Option<String>,
    pub dose_cycle: Option<i32>,
    pub frequency: Option<String>,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    pub max_daily_doses: Option<i32>,
    pub min_hours_between_doses: Option<i32>,
    pub schedule_type: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub schedule_config: Json,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
