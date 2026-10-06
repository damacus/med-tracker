use std::path::Path;

use loco_rs::{Error, Result, app::AppContext, environment::Environment};
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

macro_rules! fixture_entity {
    ($name:ident, $table:literal, { $($field:ident: $kind:ty,)* }) => {
        mod $name {
            use sea_orm::entity::prelude::*;
            #[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
            #[sea_orm(table_name = $table)]
            pub struct Model {
                #[sea_orm(primary_key)]
                pub id: i64,
                pub created_at: DateTime,
                pub updated_at: DateTime,
                $(pub $field: $kind,)*
            }
            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}
            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

fixture_entity!(accounts, "accounts", { email: String, status: i32, });
fixture_entity!(households, "households", { created_by_account_id: i64, name: String, slug: String, timezone: String, });
fixture_entity!(people, "people", { account_id: Option<i64>, household_id: i64, name: String, person_type: i32, has_capacity: bool, });
fixture_entity!(users, "users", { person_id: i64, email_address: String, });
fixture_entity!(household_memberships, "household_memberships", { account_id: i64, household_id: i64, person_id: i64, role: String, joined_at: DateTime, });
fixture_entity!(person_access_grants, "person_access_grants", { household_id: i64, household_membership_id: i64, person_id: i64, access_level: String, relationship_type: String, });
fixture_entity!(locations, "locations", { household_id: i64, name: String, });
fixture_entity!(medications, "medications", { household_id: i64, location_id: i64, name: String, current_supply: Decimal, dose_amount: f64, dose_unit: String, });
fixture_entity!(person_medications, "person_medications", { household_id: i64, person_id: i64, medication_id: i64, dose_amount: Decimal, dose_unit: String, position: i32, });

pub async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
    if !matches!(
        ctx.environment,
        Environment::Development | Environment::Test
    ) {
        return Err(Error::string(
            "Synthetic fixtures are limited to development and test",
        ));
    }
    migration::Migrator::up(&ctx.db, None).await?;
    let populated = ctx
        .db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT EXISTS (SELECT 1 FROM accounts) AS populated",
        ))
        .await?
        .ok_or_else(|| Error::string("Cannot inspect fixture database"))?;
    if populated.try_get::<bool>("", "populated")? {
        return Err(Error::string(
            "Synthetic seeding requires an empty application database",
        ));
    }
    let password = tokio::task::spawn_blocking(|| bcrypt::hash("password", 12))
        .await
        .map_err(|error| Error::string(&error.to_string()))?
        .map_err(|error| Error::string(&error.to_string()))?;
    macro_rules! load {
        ($entity:ident) => {
            loco_rs::db::seed::<$entity::ActiveModel>(
                &ctx.db,
                &base
                    .join(concat!(stringify!($entity), ".yaml"))
                    .to_string_lossy(),
            )
            .await?;
        };
    }
    load!(accounts);
    load!(households);
    load!(people);
    load!(users);
    load!(household_memberships);
    load!(person_access_grants);
    load!(locations);
    load!(medications);
    load!(person_medications);
    ctx.db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE accounts SET password_hash = $1 WHERE id = 71001",
            [password.clone().into()],
        ))
        .await?;
    ctx.db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE users SET password_digest = $1 WHERE id = 77001",
            [password.into()],
        ))
        .await?;
    Ok(())
}
