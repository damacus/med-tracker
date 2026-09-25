macro_rules! read_entity {
    ($name:ident, $table:literal, { $($field:ident: $kind:ty,)* }) => {
        pub mod $name {
            use sea_orm::entity::prelude::*;

            #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
            #[sea_orm(table_name = $table)]
            pub struct Model {
                #[sea_orm(primary_key)]
                pub id: i64,
                $(pub $field: $kind,)*
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}

            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

read_entity!(person, "people", {
    household_id: i64,
    portable_id: String,
    name: String,
    email: Option<String>,
    date_of_birth: Option<Date>,
    person_type: i32,
    has_capacity: bool,
    updated_at: DateTime,
});

read_entity!(schedule, "schedules", {
    household_id: i64,
    portable_id: String,
    person_id: i64,
    medication_id: i64,
    source_dosage_option_id: Option<i64>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    frequency: Option<String>,
    dose_cycle: Option<i32>,
    start_date: Option<Date>,
    end_date: Option<Date>,
    active: bool,
    notes: Option<String>,
    updated_at: DateTime,
    schedule_type: i32,
    schedule_config: serde_json::Value,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    retired_at: Option<DateTime>,
});

read_entity!(person_medication, "person_medications", {
    household_id: i64,
    portable_id: String,
    person_id: i64,
    medication_id: i64,
    source_dosage_option_id: Option<i64>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    active: bool,
    notes: Option<String>,
    updated_at: DateTime,
    dose_cycle: Option<i32>,
    administration_kind: i32,
    position: i32,
    max_daily_doses: Option<i32>,
    min_hours_between_doses: Option<i32>,
    retired_at: Option<DateTime>,
});

read_entity!(location_membership, "location_memberships", {
    household_id: i64,
    location_id: i64,
    person_id: i64,
});

read_entity!(stock_location, "locations", {
    household_id: i64,
    portable_id: String,
    name: String,
    description: Option<String>,
    updated_at: DateTime,
});

read_entity!(notification_preference, "notification_preferences", {
    household_id: i64,
    person_id: i64,
    portable_id: String,
});

read_entity!(pause_period, "medication_pause_periods", {
    household_id: i64,
    portable_id: String,
    schedule_id: Option<i64>,
    person_medication_id: Option<i64>,
    reason: String,
    note: Option<String>,
    legacy_context: bool,
    recorded_by_membership_id: Option<i64>,
    resumed_by_membership_id: Option<i64>,
    started_at: Option<DateTime>,
    ended_at: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
});
