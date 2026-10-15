use std::{path::Path, sync::Arc, time::Duration};

use loco_rs::{Error, Result, app::AppContext, environment::Environment};
use migration::MigratorTrait;
use sea_orm::{
    ActiveValue::Set, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    IntoActiveModel, Statement, TransactionTrait,
};
use tokio::sync::Semaphore;

use super::{
    _entities::{identity_onboarding, identity_provider_accounts},
    entities::{account, household, location, medication, membership, person, person_medication},
};

#[derive(Clone)]
struct SeedCryptoGate(Arc<Semaphore>);

async fn bounded_seed_blocking<T, F>(gate: SeedCryptoGate, work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = tokio::time::timeout(Duration::from_secs(30), gate.0.acquire_owned())
        .await
        .map_err(|error| Error::string(&error.to_string()))?
        .map_err(|error| Error::string(&error.to_string()))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|error| Error::string(&error.to_string()))
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
    let transaction = ctx.db.begin().await?;
    transaction
        .execute_unprepared("SELECT pg_advisory_xact_lock(734821001)")
        .await?;
    let populated = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT EXISTS (SELECT 1 FROM public.accounts UNION ALL SELECT 1 FROM public.people UNION ALL SELECT 1 FROM public.households UNION ALL SELECT 1 FROM public.household_memberships UNION ALL SELECT 1 FROM public.locations UNION ALL SELECT 1 FROM public.medications UNION ALL SELECT 1 FROM public.person_medications) AS populated",
        ))
        .await?
        .ok_or_else(|| Error::string("Cannot inspect fixture database"))?;
    if populated.try_get::<bool>("", "populated")? {
        return Err(Error::string(
            "Synthetic seeding requires an empty application database",
        ));
    }
    if base == Path::new("tests/fixtures/browser") {
        for source in [
            include_str!("../../tests/fixtures/persistence-records.sql"),
            include_str!("../../tests/fixtures/care-doses.sql"),
            include_str!("../../tests/oauth_server/clients.sql"),
            include_str!("../fixtures/browser.sql"),
        ] {
            transaction.execute_unprepared(source).await?;
        }
        transaction.commit().await?;
        return Ok(());
    }
    let gate = ctx.shared_store.get::<SeedCryptoGate>().unwrap_or_else(|| {
        let gate = SeedCryptoGate(Arc::new(Semaphore::new(1)));
        ctx.shared_store.insert(gate.clone());
        gate
    });
    let handle = tokio::runtime::Handle::current();
    let password = bounded_seed_blocking(gate, move || {
        handle.block_on(better_auth_core::utils::password::hash_password(
            None, "password",
        ))
    })
    .await?
    .map_err(|error| Error::string(&error.to_string()))?;
    let result = seed_default_in(&transaction, base, password).await;
    match result {
        Ok(()) => transaction.commit().await.map_err(Into::into),
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

async fn seed_default_in(
    transaction: &DatabaseTransaction,
    base: &Path,
    password: String,
) -> Result<()> {
    macro_rules! load {
        ($entity:ident, $name:literal) => {{
            let bytes = tokio::fs::read(base.join($name)).await?;
            let models: Vec<$entity::Model> = serde_yaml_ng::from_slice(&bytes)
                .map_err(|error| Error::string(&error.to_string()))?;
            if models.is_empty() {
                return Err(Error::string(concat!(
                    $name,
                    " must contain synthetic rows"
                )));
            }
            $entity::Entity::insert_many(
                models
                    .iter()
                    .cloned()
                    .map(IntoActiveModel::into_active_model),
            )
            .exec(transaction)
            .await?;
            models
        }};
    }
    load!(person, "people.yaml");
    let accounts = load!(account, "accounts.yaml");
    if accounts.len() != 1 {
        return Err(Error::string("Synthetic seed requires exactly one account"));
    }
    let account_id = accounts[0].id;
    load!(household, "households.yaml");
    load!(membership, "household_memberships.yaml");
    load!(location, "locations.yaml");
    load!(medication, "medications.yaml");
    load!(person_medication, "person_medications.yaml");
    identity_onboarding::Entity::insert(identity_onboarding::ActiveModel {
        account_id: Set(account_id),
        recovery_saved_at: Set(Some(chrono::Utc::now().into())),
        ..Default::default()
    })
    .exec(transaction)
    .await?;
    identity_provider_accounts::Entity::insert(identity_provider_accounts::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        account_id: Set(account_id),
        provider_id: Set("credential".to_owned()),
        provider_account_id: Set(account_id.to_string()),
        password: Set(Some(password)),
        ..Default::default()
    })
    .exec(transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_seed_work_holds_its_crypto_permit_until_exit() {
        let gate = SeedCryptoGate(std::sync::Arc::new(tokio::sync::Semaphore::new(1)));
        let (started, starting) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let first_gate = gate.clone();
        let first = tokio::spawn(async move {
            bounded_seed_blocking(first_gate, move || {
                started.send(()).unwrap();
                released.recv().unwrap();
                1
            })
            .await
        });
        starting.await.unwrap();
        first.abort();
        assert!(first.await.is_err());
        let (second_started, mut second_starting) = tokio::sync::oneshot::channel();
        let second = tokio::spawn(async move {
            bounded_seed_blocking(gate, move || {
                second_started.send(()).unwrap();
                2
            })
            .await
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), &mut second_starting)
                .await
                .is_err()
        );
        release.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), second_starting)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.await.unwrap().unwrap(), 2);
    }
}
