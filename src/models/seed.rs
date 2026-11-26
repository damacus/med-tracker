use std::path::Path;

use loco_rs::{Error, Result, app::AppContext, environment::Environment};
use migration::MigratorTrait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter,
};

use super::_entities::{
    accounts, household_memberships, households, locations, medications, oauth_applications,
    people, person_access_grants, person_medications, platform_admins,
};

pub const LOCAL_ANDROID_CLIENT_ID: &str = "medtracker-android-local";
pub const LOCAL_ANDROID_DEBUG_REDIRECT_URI: &str = "io.damacus.medtracker.debug:/oauth2redirect";
pub const LOCAL_ANDROID_REDIRECT_URIS: &str = "io.damacus.medtracker.debug:/oauth2redirect io.damacus.medtracker.staging:/oauth2redirect io.damacus.medtracker:/oauth2redirect";
pub const LOCAL_ANDROID_SCOPES: &str = "medtracker offline_access";

#[allow(
    clippy::result_large_err,
    reason = "Loco's framework error type contains ordered JSON values"
)]
pub async fn ensure_local_mobile_oauth_client(
    db: &DatabaseConnection,
) -> Result<oauth_applications::Model> {
    if let Some(existing) = oauth_applications::Entity::find()
        .filter(oauth_applications::Column::ClientId.eq(LOCAL_ANDROID_CLIENT_ID))
        .one(db)
        .await?
    {
        return Ok(existing);
    }
    let now = chrono::Utc::now().naive_utc();
    let inserted = oauth_applications::ActiveModel {
        account_id: Set(None),
        client_id: Set(LOCAL_ANDROID_CLIENT_ID.to_owned()),
        client_kind: Set("mobile".to_owned()),
        client_secret: Set(None),
        client_secret_hash: Set(None),
        name: Set("MedTracker Android".to_owned()),
        redirect_uri: Set(LOCAL_ANDROID_REDIRECT_URIS.to_owned()),
        scopes: Set(LOCAL_ANDROID_SCOPES.to_owned()),
        token_endpoint_auth_method: Set("none".to_owned()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(inserted)
}

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
    loco_rs::db::seed::<super::identity::better_auth::schema::account::ActiveModel>(
        &ctx.db,
        &base.join("auth_credentials.yaml").to_string_lossy(),
    )
    .await?;
    ctx.db
        .execute_unprepared("UPDATE public.accounts SET email_verified=(status=2)")
        .await?;
    load!(households);
    load!(platform_admins);
    load!(household_memberships);
    load!(person_access_grants);
    load!(locations);
    load!(medications);
    load!(person_medications);
    ensure_local_mobile_oauth_client(&ctx.db).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::models::_entities::{
        accounts, household_memberships, households, locations, medications, people,
        person_access_grants, person_medications,
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
        check!(household_memberships);
        check!(person_access_grants);
        check!(locations);
        check!(medications);
        check!(person_medications);
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
    }
}
