use super::Application;
use med_tracker::models::_entities::data_import_runs;
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    data_imports::{
        contracts::{
            ConsentInput, ImportState, PersonDestination, PersonId, PersonMapping,
            PreparationReceipt, PreviewResult,
        },
        persistence::{self, EncryptedPrepared, NewRun, SourceTimeZone},
    },
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};

#[tokio::test]
async fn data_import_runs_persist_snapshot_and_isolate_households() {
    let app = Application::new().await;
    let foreign = app.fixture.foreign_household().await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.scenario.account.id,
        },
        household_id: app.fixture.scenario.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let mut preview: PreviewResult = serde_json::from_str(include_str!(
        "../../src/models/data_imports/examples/preview.json"
    ))
    .unwrap();
    preview.report.id = uuid::Uuid::new_v4().to_string();
    let run = NewRun {
        report: preview.report.clone(),
        receipt: PreparationReceipt {
            payload_sha256: "payload".into(),
            manifest_sha256: "manifest".into(),
            target_sha256: "target".into(),
        },
        encrypted_prepared: EncryptedPrepared::encrypt(
            &serde_json::json!({"synthetic":"prepared"}),
            "password",
        )
        .unwrap(),
        source_timezone: SourceTimeZone::parse("Europe/London").unwrap(),
        mappings: vec![PersonMapping {
            source: preview.report.people[0].person.source.clone(),
            destination: PersonDestination::Existing(PersonId(app.fixture.adult.id)),
        }],
        consents: vec![ConsentInput::ActingPerson {
            subject: PersonId(app.fixture.adult.id),
            accepted: true,
        }],
        acknowledged_omissions: vec!["omission-1".into()],
        confirmation: preview.confirmation.clone(),
    };
    persistence::create(&tenant, &run).await.unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let stored = persistence::find(&tenant, &preview.report.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.created_by_membership_id, tenant.membership().id);
    let saved = stored.snapshot;
    assert_eq!(saved.report, preview.report);
    assert_eq!(
        saved.encrypted_prepared.decrypt("password").unwrap(),
        serde_json::json!({"synthetic":"prepared"})
    );
    assert!(saved.encrypted_prepared.decrypt("wrong").is_err());
    assert_eq!(saved.receipt, run.receipt);
    assert_eq!(saved.mappings, run.mappings);
    assert_eq!(saved.consents, run.consents);
    assert_eq!(saved.acknowledged_omissions, vec!["omission-1"]);
    assert_eq!(saved.source_timezone.name(), "Europe/London");
    assert!(saved.confirmation == preview.confirmation);
    assert!(
        persistence::list_reports(&tenant)
            .await
            .unwrap()
            .iter()
            .any(|report| report.id == preview.report.id)
    );
    tenant.rollback().await.unwrap();
    let original = data_import_runs::Entity::find_by_id(preview.report.id.clone())
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let foreign_scope = HouseholdScope {
        actor: Actor {
            account_id: foreign.account.id,
        },
        household_id: foreign.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let other = access::begin(&foreign.runtime, &foreign_scope, None)
        .await
        .unwrap();
    assert!(
        persistence::find(&other, &preview.report.id)
            .await
            .unwrap()
            .is_none()
    );
    let changed = data_import_runs::Entity::update_many()
        .col_expr(
            data_import_runs::Column::Confirmation,
            sea_orm::sea_query::Expr::value("forged"),
        )
        .filter(data_import_runs::Column::Id.eq(preview.report.id.clone()))
        .exec(other.transaction())
        .await
        .unwrap();
    assert_eq!(changed.rows_affected, 0);
    assert!(
        persistence::list_reports(&other)
            .await
            .unwrap()
            .iter()
            .all(|report| report.id != preview.report.id)
    );
    let mut forged = original.into_active_model();
    forged.id = Set(uuid::Uuid::new_v4().to_string());
    forged.created_by_membership_id = Set(foreign.membership.id);
    assert!(forged.insert(other.transaction()).await.is_err());
    other.rollback().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let mut applying = preview.report.clone();
    applying.state = ImportState::Applying;
    assert!(
        persistence::transition(&tenant, ImportState::Ready, &applying, None)
            .await
            .is_err()
    );
    persistence::transition(
        &tenant,
        ImportState::Ready,
        &applying,
        preview.confirmation.as_ref(),
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let saved = persistence::find(&tenant, &preview.report.id)
        .await
        .unwrap()
        .unwrap()
        .snapshot;
    assert_eq!(saved.report.state, ImportState::Applying);
    assert!(saved.confirmation.is_none());
    assert!(
        persistence::transition(
            &tenant,
            ImportState::Ready,
            &applying,
            preview.confirmation.as_ref()
        )
        .await
        .is_err()
    );
    tenant.rollback().await.unwrap();
    assert!(SourceTimeZone::parse("Not/A_Zone").is_err());
    foreign.close().await;
    app.close().await;
}
