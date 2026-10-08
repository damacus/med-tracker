use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::doses::{self, Command, Outcome, Take},
    errors::OperationError,
};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

fn owned_url() -> String {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    let uri = std::env::var("CUTOVER_DATABASE_URL").unwrap();
    let url = url::Url::parse(&uri).unwrap();
    assert_eq!(url.scheme(), "postgres");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.path(), "/medtracker_reference");
    uri
}

async fn take(
    runtime: &sea_orm::DatabaseConnection,
    account_id: i64,
    household_id: i64,
) -> Result<Outcome, OperationError> {
    let scope = HouseholdScope {
        actor: Actor { account_id },
        household_id,
        request_id: "cutover-dose".into(),
    };
    let tenant = access::begin(runtime, &scope).await?;
    let result = doses::execute(
        &tenant,
        Command::Take(Take {
            client_uuid: Some("d87cc65d-b3c3-49d4-9236-87da302d3465".into()),
            source_type: "person_medication".into(),
            source_id: "81001".into(),
            taken_at: "2026-10-06T10:00:00Z".into(),
            dose_amount: Some("2".into()),
            dose_unit: Some("tablet".into()),
            taken_from_medication_id: Some(80001),
            expected_effective_amount: None,
            expected_effective_unit: None,
        }),
    )
    .await;
    match result {
        Ok(outcome) => {
            tenant.commit().await?;
            Ok(outcome)
        }
        Err(error) => {
            tenant.rollback().await?;
            Err(error)
        }
    }
}

#[tokio::test]
async fn populated_loco_write_preserves_history_and_rejects_cross_household_access() {
    let uri = owned_url();
    let admin = Database::connect(&uri).await.unwrap();
    let row = admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM accounts WHERE id=71001 AND password_hash IS NOT NULL)::bigint AS accounts, (SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='cutover_fixture/preserved')::bigint AS historical_audits",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "historical_audits").unwrap(), 1);
    let mut runtime_url = url::Url::parse(&uri).unwrap();
    runtime_url
        .set_username("medtracker_browser_runtime")
        .unwrap();
    runtime_url.set_password(Some("password")).unwrap();
    let runtime = Database::connect(runtime_url.as_str()).await.unwrap();
    assert!(matches!(
        take(&runtime, 71001, 72001).await,
        Ok(Outcome::Created(_))
    ));
    assert!(matches!(
        take(&runtime, 71001, 72001).await,
        Ok(Outcome::Replayed(_))
    ));
    assert!(matches!(
        take(&runtime, 71001, 72002).await,
        Err(OperationError::Forbidden)
    ));
    let row = admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM medication_takes WHERE taken_from_medication_id=80001)::bigint AS takes, (SELECT current_supply::text FROM medications WHERE id=80001) AS supply, (SELECT count(*) FROM versions WHERE item_type='MedicationTake' AND event='create')::bigint AS audits",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "takes").unwrap(), 1);
    assert_eq!(row.try_get::<String>("", "supply").unwrap(), "8.00");
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
    runtime.close().await.unwrap();
    admin.close().await.unwrap();
}
