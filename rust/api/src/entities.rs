macro_rules! entity {
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

entity!(api_session, "api_sessions", {
    account_id: i64,
    household_membership_id: Option<i64>,
    access_token_digest: String,
    access_expires_at: DateTime,
    revoked_at: Option<DateTime>,
    permissions_version: i32,
});

entity!(account, "accounts", { status: i32, });

pub mod account_lockout {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "account_lockouts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub account_id: i64,
        pub deadline: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

entity!(household, "households", {
    status: String,
    lifecycle_state: String,
});

entity!(membership, "household_memberships", {
    account_id: i64,
    household_id: i64,
    person_id: Option<i64>,
    permissions_version: i32,
    role: String,
    status: String,
    revoked_at: Option<DateTime>,
});

entity!(user, "users", {
    person_id: i64,
    active: bool,
});

entity!(person, "people", {
    account_id: Option<i64>,
    household_id: i64,
});

entity!(grant, "person_access_grants", {
    household_id: i64,
    household_membership_id: i64,
    person_id: i64,
    access_level: String,
    expires_at: Option<DateTime>,
    revoked_at: Option<DateTime>,
});

entity!(schedule, "schedules", {
    household_id: i64,
    person_id: i64,
    medication_id: i64,
    active: bool,
    start_date: Option<Date>,
    end_date: Option<Date>,
    max_daily_doses: Option<i32>,
    dose_cycle: Option<i32>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
    schedule_type: i32,
    schedule_config: serde_json::Value,
});

entity!(person_medication, "person_medications", {
    household_id: i64,
    person_id: i64,
    medication_id: i64,
    max_daily_doses: Option<i32>,
    dose_amount: Option<Decimal>,
    dose_unit: Option<String>,
});

entity!(location, "locations", {
    household_id: i64,
    portable_id: String,
});

entity!(medication, "medications", {
    household_id: i64,
    created_by_membership_id: Option<i64>,
    portable_id: String,
    name: Option<String>,
    friendly_name: Option<String>,
    category: Option<String>,
    description: Option<String>,
    dose_amount: Option<f64>,
    dose_unit: Option<String>,
    current_supply: Option<Decimal>,
    reorder_threshold: Decimal,
    reorder_status: Option<i32>,
    location_id: i64,
    updated_at: DateTime,
});
