use super::Application;
use med_tracker::models::_entities::data_import_runs;
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    data_imports::{
        contracts::{
            ConsentInput, ImportState, PersonDestination, PersonId, PersonMapping,
            PreparationReceipt, PreviewResult, PreviewToken,
        },
        persistence::{self, EncryptedPrepared, NewRun, SourceTimeZone},
        reports,
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbErr, EntityTrait, IntoActiveModel, QueryFilter, Set,
};

fn permission_denied(error: &DbErr) -> bool {
    match error {
        DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))
        | DbErr::Query(sea_orm::RuntimeErr::SqlxError(error)) => error
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| code == "42501"),
        _ => false,
    }
}

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
    let mut run = NewRun {
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
    let mut expected_ids = vec![run.report.id.clone()];
    for _ in 0..2 {
        run.report.id = uuid::Uuid::new_v4().to_string();
        expected_ids.push(run.report.id.clone());
        persistence::create(&tenant, &run).await.unwrap();
    }
    expected_ids.sort();
    run.report.id = preview.report.id.clone();
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
    let visible = reports::detail(&tenant, &preview.report.id).await.unwrap();
    assert_eq!(visible.report.id, preview.report.id);
    assert!(visible.confirmation.is_none());
    let page = reports::list(&tenant, None, 2).await.unwrap();
    assert_eq!(page.reports.len(), 2);
    let second_page = reports::list(&tenant, page.next_cursor.as_deref(), 2)
        .await
        .unwrap();
    assert_eq!(second_page.reports.len(), 1);
    let first = persistence::list_reports(&tenant, None, 2).await.unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(
        first.iter().map(|item| &item.id).collect::<Vec<_>>(),
        expected_ids[..2].iter().collect::<Vec<_>>()
    );
    let second = persistence::list_reports(&tenant, Some(&first[1].id), 2)
        .await
        .unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].id, expected_ids[2]);
    assert!(persistence::list_reports(&tenant, None, 0).await.is_err());
    assert!(persistence::list_reports(&tenant, None, 101).await.is_err());
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
    assert!(reports::detail(&other, &preview.report.id).await.is_err());
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
        persistence::list_reports(&other, None, 100)
            .await
            .unwrap()
            .iter()
            .all(|report| report.id != preview.report.id)
    );
    let mut forged = original.clone().into_active_model().reset_all();
    forged.id = Set(uuid::Uuid::new_v4().to_string());
    forged.created_by_membership_id = Set(foreign.membership.id);
    let error = forged.insert(other.transaction()).await.unwrap_err();
    assert!(permission_denied(&error));
    other.rollback().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let mut forged = original.into_active_model().reset_all();
    forged.id = Set(uuid::Uuid::new_v4().to_string());
    forged.created_by_membership_id = Set(app.fixture.minor_membership.id);
    let error = forged.insert(tenant.transaction()).await.unwrap_err();
    assert!(permission_denied(&error));
    tenant.rollback().await.unwrap();

    for column in [
        data_import_runs::Column::EncryptedPrepared,
        data_import_runs::Column::Receipt,
        data_import_runs::Column::Mappings,
        data_import_runs::Column::Consents,
        data_import_runs::Column::AcknowledgedOmissions,
        data_import_runs::Column::SourceTimezone,
        data_import_runs::Column::Id,
        data_import_runs::Column::HouseholdId,
        data_import_runs::Column::CreatedByMembershipId,
    ] {
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        let error = data_import_runs::Entity::update_many()
            .col_expr(column, sea_orm::sea_query::Expr::col(column))
            .filter(data_import_runs::Column::Id.eq(preview.report.id.clone()))
            .exec(tenant.transaction())
            .await
            .unwrap_err();
        assert!(
            permission_denied(&error),
            "immutable column {column:?}: {error}"
        );
        tenant.rollback().await.unwrap();
    }
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let error = data_import_runs::Entity::delete_by_id(preview.report.id.clone())
        .exec(tenant.transaction())
        .await
        .unwrap_err();
    assert!(permission_denied(&error));
    tenant.rollback().await.unwrap();
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
    let wrong_token = PreviewToken("wrong-nonempty-token".into());
    assert!(
        persistence::transition(&tenant, ImportState::Ready, &applying, Some(&wrong_token))
            .await
            .is_err()
    );
    let mut altered = applying.clone();
    altered.totals.expected_create += 1;
    assert!(
        persistence::transition(
            &tenant,
            ImportState::Ready,
            &altered,
            preview.confirmation.as_ref()
        )
        .await
        .is_err()
    );
    let competitor = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    persistence::transition(
        &tenant,
        ImportState::Ready,
        &applying,
        preview.confirmation.as_ref(),
    )
    .await
    .unwrap();
    let competing_report = applying.clone();
    let competing_token = preview.confirmation.clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let mut competing = tokio::spawn(async move {
        let _ = started.send(());
        let result = persistence::transition(
            &competitor,
            ImportState::Ready,
            &competing_report,
            competing_token.as_ref(),
        )
        .await;
        competitor.rollback().await.unwrap();
        result
    });
    ready.await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(150), &mut competing)
            .await
            .is_err()
    );
    tenant.commit().await.unwrap();
    let second = tokio::time::timeout(std::time::Duration::from_secs(10), competing)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        second,
        Err(med_tracker::models::errors::OperationError::Conflict { .. })
    ));
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
