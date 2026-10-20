use std::path::Path;

use loco_rs::{Error, Result, app::AppContext, environment::Environment};
use migration::MigratorTrait;
use sea_orm::EntityTrait;

use super::_entities::{
    accounts, household_memberships, households, locations, medications, people,
    person_access_grants, person_medications, users,
};

#[allow(
    clippy::result_large_err,
    reason = "Loco's framework error type contains ordered JSON values"
)]
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
    if accounts::Entity::find().one(&ctx.db).await?.is_some() {
        return Err(Error::string(
            "Synthetic seeding requires an empty application database",
        ));
    }
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
    load!(people);
    load!(accounts);
    load!(households);
    load!(users);
    load!(household_memberships);
    load!(person_access_grants);
    load!(locations);
    load!(medications);
    load!(person_medications);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::models::_entities::{
        accounts, household_memberships, households, locations, medications, people,
        person_access_grants, person_medications, users,
    };

    #[test]
    fn development_fixtures_match_application_entities() {
        macro_rules! check {
            ($entity:ident) => {
                let rows: Vec<$entity::Model> = serde_yaml_ng::from_str(include_str!(concat!(
                    "../fixtures/",
                    stringify!($entity),
                    ".yaml"
                )))
                .expect(concat!("valid ", stringify!($entity), " fixtures"));
                assert!(!rows.is_empty());
            };
        }
        check!(people);
        check!(accounts);
        check!(households);
        check!(users);
        check!(household_memberships);
        check!(person_access_grants);
        check!(locations);
        check!(medications);
        check!(person_medications);
        let accounts: Vec<accounts::Model> =
            serde_yaml_ng::from_str(include_str!("../fixtures/accounts.yaml")).unwrap();
        let users: Vec<users::Model> =
            serde_yaml_ng::from_str(include_str!("../fixtures/users.yaml")).unwrap();
        let medications: Vec<medications::Model> =
            serde_yaml_ng::from_str(include_str!("../fixtures/medications.yaml")).unwrap();
        let treatments: Vec<person_medications::Model> =
            serde_yaml_ng::from_str(include_str!("../fixtures/person_medications.yaml")).unwrap();
        for medication in medications {
            assert_eq!(medication.reorder_threshold.to_string(), "10.00");
        }
        for treatment in treatments {
            assert_eq!(treatment.administration_kind, 1);
        }
        for account in accounts {
            assert!(bcrypt::verify("password", account.password_hash.as_deref().unwrap()).unwrap());
        }
        for user in users {
            assert!(bcrypt::verify("password", user.password_digest.as_deref().unwrap()).unwrap());
        }
    }
}
