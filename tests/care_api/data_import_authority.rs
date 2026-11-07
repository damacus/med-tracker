use super::Application;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use med_tracker::models::{
    _entities::{accounts, data_import_runs, household_memberships, people},
    access::{self, Actor, HouseholdScope},
    care::administration::recovery::{
        self, RecoveryAction, RecoveryCommand, RecoveryGuardianId, RecoveryOutcome,
        RecoverySubjectId,
    },
    data_imports::{
        consents::{self, ConsentBinding, Decision},
        contracts::{PersonId, SourceIdentity},
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
    sea_query::{Expr, ExprTrait, Func, Query},
};
use serde_json::{Value, json};
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

fn archive_for(kind: &str) -> String {
    let mut payload: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.json"
    ))
    .unwrap();
    if kind == "adult" {
        payload["records"]["people"][0]["person_type"] = json!("adult");
        payload["records"]["people"][0]["has_capacity"] = json!(true);
    }
    if kind == "minor" {
        payload["records"]["people"][0]["person_type"] = json!("minor");
        payload["records"]["people"][0]["date_of_birth"] = json!("2020-01-01");
    }
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("portable.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&serde_json::to_vec(&payload).unwrap())
        .unwrap();
    zip.start_file("manifest.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(include_bytes!(
        "../fixtures/portable_rails/prepared-history-v2.manifest.json"
    ))
    .unwrap();
    STANDARD.encode(zip.finish().unwrap().into_inner())
}

async fn inspect(app: &Application, token: &str, archive: &str) -> Value {
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{}/data_imports/inspections",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .json(&json!({"source":"med_tracker05", "archive_base64":archive,
            "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true}))
        .send()
        .await
        .unwrap();
    Application::assert_status(response, 201).await["data"].clone()
}

async fn preview(
    app: &Application,
    token: &str,
    archive: &str,
    inspected: &Value,
    person: i64,
    declarations: Value,
    recipients: Value,
) -> Value {
    let source = inspected["inventory"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["identity"]["kind"] == "person")
        .unwrap()["identity"]
        .clone();
    let omissions: Vec<Value> = inspected["inventory"]["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["acknowledgement_id"].clone())
        .collect();
    let response = app.client.post(format!("{}/api/v1/households/{}/data_imports/previews", app.origin, app.fixture.household.id))
        .bearer_auth(token).json(&json!({"inspection_id":inspected["id"], "source":"med_tracker05", "archive_base64":archive,
            "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true,
            "mappings":[{"source":source, "destination":{"Existing":person}}], "declarations":declarations,
            "recipient_consents":recipients, "acknowledged_omissions":omissions}))
        .send().await.unwrap();
    Application::assert_status(response, 201).await["data"].clone()
}

#[tokio::test]
async fn missing_and_pending_adult_consent_return_source_only_preview() {
    let app = Application::new().await;
    let token = app.token().await;
    let recipient = app.fixture.foreign_household().await;
    let mut person = people::Entity::find_by_id(recipient.account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    person.name = Set("private destination sentinel".into());
    person.date_of_birth = Set(Some("1990-01-15".parse().unwrap()));
    person.person_type = Set(0);
    person.has_capacity = Set(true);
    person.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(recipient.account.person_id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let archive = archive_for("adult");
    let inspected = inspect(&app, &token, &archive).await;
    let source: SourceIdentity = serde_json::from_value(
        inspected["inventory"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["identity"]["kind"] == "person")
            .unwrap()["identity"]
            .clone(),
    )
    .unwrap();
    let scope = app.fixture.scenario.scope();
    let binding = ConsentBinding {
        source_sha256: inspected["archive_sha256"].as_str().unwrap().into(),
        source_person: source.clone(),
        target_person: PersonId(recipient.account.person_id),
    };
    let tenant =
        consents::begin_for_target(&app.fixture.runtime, &scope, None, binding.target_person)
            .await
            .unwrap();
    let consent_id = consents::request(&tenant, binding).await.unwrap();
    tenant.commit().await.unwrap();
    for recipients in [
        json!([]),
        json!([{"source_person": source, "request_id": consent_id}]),
    ] {
        let result = preview(
            &app,
            &token,
            &archive,
            &inspected,
            recipient.account.person_id,
            json!([]),
            recipients,
        )
        .await;
        assert_eq!(result["report"]["state"], "blocked");
        assert!(result["confirmation"].is_null());
        assert!(
            result["report"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| issue["reason"] == "consent_required")
        );
        assert!(!result.to_string().contains("private destination sentinel"));
        assert!(result["report"]["people"][0]["person"]["destination"].is_null());
        let id = result["report"]["id"].as_str().unwrap();
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{}/data_imports/{id}",
                app.origin, scope.household_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 404);
    }
    recipient.close().await;
    app.close().await;
}

#[tokio::test]
async fn dependent_adult_self_preview_succeeds_but_other_adult_consent_does_not_grant_authority() {
    let app = Application::new().await;
    let token = app.token().await;
    let actor_id = app.fixture.account.person_id;
    let mut actor = people::Entity::find_by_id(actor_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    actor.person_type = Set(2);
    actor.has_capacity = Set(false);
    actor.date_of_birth = Set(Some("1990-01-15".parse().unwrap()));
    actor.update(&app.fixture.admin).await.unwrap();

    let self_archive = archive_for("dependent_adult");
    let self_inspection = inspect(&app, &token, &self_archive).await;
    let own = preview(
        &app,
        &token,
        &self_archive,
        &self_inspection,
        actor_id,
        json!([{"ActingPerson":{"subject":actor_id,"accepted":true}}]),
        json!([]),
    )
    .await;
    assert_eq!(own["report"]["state"], "ready", "{own}");
    assert!(own["confirmation"].is_string());
    let own_person = &own["report"]["people"][0];
    assert_eq!(
        own_person["person"]["source"]["id"],
        "synthetic-rails-person-prepared-history-v2"
    );
    assert_eq!(own_person["person"]["expected"], "match");
    assert!(own_person["person"]["actual"].is_null());
    assert_eq!(own["report"]["totals"]["created"], 0);
    assert!(
        own_person["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |record| record["source"]["id"] == "synthetic-rails-schedule-prepared-history-v2"
                    && record["expected"] == "create"
            )
    );
    assert!(
        own_person["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |record| record["source"]["id"] == "synthetic-rails-take-prepared-history-v2"
                    && record["expected"] == "create"
            )
    );

    let recipient = app.fixture.foreign_household().await;
    let mut person = people::Entity::find_by_id(recipient.account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    person.person_type = Set(0);
    person.has_capacity = Set(true);
    person.date_of_birth = Set(Some("1990-01-15".parse().unwrap()));
    person.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(recipient.account.person_id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let other_archive = archive_for("adult");
    let other_inspection = inspect(&app, &token, &other_archive).await;
    let source: SourceIdentity = serde_json::from_value(
        other_inspection["inventory"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["identity"]["kind"] == "person")
            .unwrap()["identity"]
            .clone(),
    )
    .unwrap();
    let binding = ConsentBinding {
        source_sha256: other_inspection["archive_sha256"].as_str().unwrap().into(),
        source_person: source.clone(),
        target_person: PersonId(recipient.account.person_id),
    };
    let requester_scope = app.fixture.scenario.scope();
    let requester = consents::begin_for_target(
        &app.fixture.runtime,
        &requester_scope,
        None,
        binding.target_person,
    )
    .await
    .unwrap();
    let request_id = consents::request(&requester, binding.clone())
        .await
        .unwrap();
    requester.commit().await.unwrap();
    let recipient_scope = HouseholdScope {
        actor: Actor {
            account_id: recipient.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    let recipient_tenant = consents::begin_for_actor(&app.fixture.runtime, &recipient_scope, None)
        .await
        .unwrap();
    consents::decide(&recipient_tenant, &request_id, &binding, Decision::Approved)
        .await
        .unwrap();
    recipient_tenant.commit().await.unwrap();
    let consent_check = consents::begin_for_requests(
        &app.fixture.runtime,
        &requester_scope,
        None,
        std::slice::from_ref(&request_id),
    )
    .await
    .unwrap();
    assert!(
        consents::current(&consent_check, &request_id, &binding)
            .await
            .unwrap()
    );
    consent_check.rollback().await.unwrap();
    let other = preview(
        &app,
        &token,
        &other_archive,
        &other_inspection,
        recipient.account.person_id,
        json!([]),
        json!([{"source_person":source,"request_id":request_id}]),
    )
    .await;
    assert_eq!(other["report"]["state"], "blocked", "{other}");
    assert!(other["confirmation"].is_null());
    let person = &other["report"]["people"][0];
    assert_eq!(
        person["person"]["source"]["id"],
        "synthetic-rails-person-prepared-history-v2"
    );
    assert!(person["person"]["actual"].is_null());
    assert!(
        other["report"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["reason"] == "insufficient_authority")
    );
    assert!(
        person["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |record| record["source"]["id"] == "synthetic-rails-schedule-prepared-history-v2"
                    && record["reason"] == "insufficient_authority"
            )
    );
    let denied = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/data_imports/{}",
            app.origin,
            app.fixture.household.id,
            other["report"]["id"].as_str().unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 404);
    app.close().await;
}

async fn pid(db: &impl ConnectionTrait) -> i32 {
    let query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap()
}

async fn wait_for_lock(app: &Application, waiter: Option<i32>, blocker: i32) -> bool {
    let mut query = Query::select();
    query
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(
            Expr::expr(Func::cust("array_position").args([
                Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                Expr::val(blocker),
            ]))
            .is_not_null(),
        );
    if let Some(waiter) = waiter {
        query.and_where(Expr::col("pid").eq(waiter));
    }
    let query = query.to_owned();
    for _ in 0..40 {
        if app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .is_some()
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    false
}

async fn account_lock_waiter(app: &Application, blocker: i32) -> Option<i32> {
    let query = Query::select()
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(Expr::col("query").like("%\"accounts\"%FOR UPDATE%"))
        .and_where(
            Expr::expr(Func::cust("array_position").args([
                Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                Expr::val(blocker),
            ]))
            .is_not_null(),
        )
        .to_owned();
    for _ in 0..140 {
        if let Some(row) = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
        {
            return Some(row.try_get("", "pid").unwrap());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    None
}

async fn import_run_count(app: &Application) -> u64 {
    data_import_runs::Entity::find()
        .filter(data_import_runs::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap()
}

#[tokio::test]
async fn self_preview_denies_membership_revoked_during_account_lock_wait() {
    let app = Application::new().await;
    let token = app.token().await;
    let actor_id = app.fixture.account.person_id;
    let mut person = people::Entity::find_by_id(actor_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    person.person_type = Set(2);
    person.has_capacity = Set(false);
    person.date_of_birth = Set(Some("1990-01-15".parse().unwrap()));
    person.update(&app.fixture.admin).await.unwrap();
    let archive = archive_for("dependent_adult");
    let inspected = inspect(&app, &token, &archive).await;
    let control = preview(
        &app,
        &token,
        &archive,
        &inspected,
        actor_id,
        json!([{"ActingPerson":{"subject":actor_id,"accepted":true}}]),
        json!([]),
    )
    .await;
    assert_eq!(control["report"]["state"], "ready", "{control}");
    assert!(control["confirmation"].is_string());
    assert_eq!(
        control["report"]["people"][0]["person"]["source"]["id"],
        "synthetic-rails-person-prepared-history-v2"
    );
    assert_eq!(
        control["report"]["people"][0]["person"]["expected"],
        "match"
    );
    let before = import_run_count(&app).await;
    assert!(before > 0, "the ready control must have been persisted");

    let source = inspected["inventory"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["identity"]["kind"] == "person")
        .unwrap()["identity"]
        .clone();
    let omissions: Vec<Value> = inspected["inventory"]["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["acknowledgement_id"].clone())
        .collect();
    let request = json!({"inspection_id":inspected["id"], "source":"med_tracker05", "archive_base64":archive,
        "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true,
        "mappings":[{"source":source,"destination":{"Existing":actor_id}}],
        "declarations":[{"ActingPerson":{"subject":actor_id,"accepted":true}}],
        "recipient_consents":[], "acknowledged_omissions":omissions});

    let blocker = app.fixture.admin.begin().await.unwrap();
    let blocker_pid = pid(&blocker).await;
    accounts::Entity::find_by_id(app.fixture.account.id)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let client = app.client.clone();
    let url = format!(
        "{}/api/v1/households/{}/data_imports/previews",
        app.origin, app.fixture.household.id
    );
    let attempt = tokio::spawn(async move {
        client
            .post(url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
    });
    let waiter = account_lock_waiter(&app, blocker_pid).await;
    if waiter.is_none() {
        blocker.rollback().await.unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(8), attempt).await;
        panic!("preview did not wait on the fixture actor account lock");
    }
    assert_ne!(waiter.unwrap(), blocker_pid);
    let revoked = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::RevokedAt,
                Expr::value(Some(chrono::Utc::now().naive_utc())),
            )
            .filter(household_memberships::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(household_memberships::Column::PersonId.eq(actor_id))
            .exec(&app.fixture.admin),
    )
    .await
    .expect("membership revocation must commit without the account lock")
    .unwrap();
    assert_eq!(revoked.rows_affected, 1);
    blocker.commit().await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(8), attempt)
        .await
        .expect("preview did not finish after releasing the account lock")
        .unwrap()
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    assert!(
        matches!(status, 403 | 404),
        "revoked membership must refuse preview: {status}: {body}"
    );
    assert!(body["data"]["confirmation"].is_null());
    assert_eq!(
        import_run_count(&app).await,
        before,
        "a denied preview must not create an import run"
    );
    app.close().await;
}

#[tokio::test]
async fn preview_and_recovery_preserve_declaration_before_person_lock_order() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app.client.post(format!("{}/api/v1/households/{}/people", app.origin, app.fixture.household.id))
        .bearer_auth(&token).header("idempotency-key", uuid::Uuid::new_v4().to_string())
        .json(&json!({"person":{"name":"Synthetic recovery subject", "date_of_birth":"2020-01-01", "person_type":"minor", "has_capacity":false, "initial_guardian_declaration":true}}))
        .send().await.unwrap();
    let created = Application::assert_status(response, 201).await;
    let subject = created["data"]["id"]
        .as_i64()
        .or_else(|| {
            created["data"]["id"]
                .as_str()
                .and_then(|id| id.parse().ok())
        })
        .unwrap();
    let foreign = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(foreign.household.id),
        person_id: Set(subject),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut old_guardian = people::Entity::find_by_id(app.fixture.account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    old_guardian.has_capacity = Set(false);
    old_guardian.person_type = Set(2);
    old_guardian.date_of_birth = Set(Some("1990-01-15".parse().unwrap()));
    old_guardian.update(&app.fixture.admin).await.unwrap();
    let archive = archive_for("minor");
    let inspected = inspect(&app, &token, &archive).await;
    let source = inspected["inventory"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["identity"]["kind"] == "person")
        .unwrap()["identity"]
        .clone();
    let omissions: Vec<Value> = inspected["inventory"]["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["acknowledgement_id"].clone())
        .collect();

    let recovery_scope = foreign.scope();
    let tenant = access::begin(&app.fixture.runtime, &recovery_scope, None)
        .await
        .unwrap();
    let selected = recovery::options(&tenant)
        .await
        .unwrap()
        .subjects
        .into_iter()
        .find(|item| item.id.get() == subject)
        .expect("recovery must offer the orphaned subject");
    let recovery_pid = pid(tenant.transaction()).await;
    let blocker = app.fixture.admin.begin().await.unwrap();
    let blocker_pid = pid(&blocker).await;
    people::Entity::find_by_id(subject)
        .lock_exclusive()
        .one(&blocker)
        .await
        .unwrap()
        .unwrap();
    let guardian_id = foreign.account.person_id;
    let recovery = tokio::spawn(async move {
        let result = recovery::execute(
            &tenant,
            RecoveryCommand {
                subject: RecoverySubjectId::parse(subject).unwrap(),
                guardian: RecoveryGuardianId::parse(guardian_id).unwrap(),
                expected: selected.snapshot,
                action: RecoveryAction::Appoint,
                reason: "Synthetic cross-household authority recovery".into(),
            },
        )
        .await;
        if result.is_ok() {
            tenant.commit().await.unwrap();
        } else {
            tenant.rollback().await.unwrap();
        }
        result
            .map(|result| result.outcome)
            .map_err(|error| format!("{error:?}"))
    });
    assert!(
        wait_for_lock(&app, Some(recovery_pid), blocker_pid).await,
        "recovery must hold its declaration lock while waiting for the subject"
    );
    let client = app.client.clone();
    let url = format!(
        "{}/api/v1/households/{}/data_imports/previews",
        app.origin, app.fixture.household.id
    );
    let request = json!({"inspection_id":inspected["id"], "source":"med_tracker05", "archive_base64":archive,
        "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true,
        "mappings":[{"source":source,"destination":{"Existing":subject}}],
        "declarations":[{"ActingGuardian":{"subject":subject,"accepted":true}}], "recipient_consents":[], "acknowledged_omissions":omissions});
    let preview = tokio::spawn(async move {
        client
            .post(url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
    });
    let preview_waited = wait_for_lock(&app, None, recovery_pid).await;
    if preview_waited {
        let revoked = household_memberships::Entity::update_many()
            .col_expr(
                household_memberships::Column::RevokedAt,
                Expr::value(Some(chrono::Utc::now().naive_utc())),
            )
            .filter(household_memberships::Column::HouseholdId.eq(app.fixture.household.id))
            .filter(household_memberships::Column::PersonId.eq(app.fixture.account.person_id))
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        assert_eq!(revoked.rows_affected, 1);
    }
    blocker.commit().await.unwrap();
    let recovery_result = tokio::time::timeout(std::time::Duration::from_secs(8), recovery)
        .await
        .expect("recovery deadlocked")
        .unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(8), preview)
        .await
        .expect("preview deadlocked")
        .unwrap()
        .unwrap();
    assert!(
        preview_waited,
        "preview must wait for the recovery declaration lock before acquiring the person lock"
    );
    assert_eq!(recovery_result.unwrap(), RecoveryOutcome::Appointed);
    let code = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    if code == 201 {
        assert_eq!(body["data"]["report"]["state"], "blocked", "{body}");
        assert!(body["data"]["confirmation"].is_null());
    } else {
        assert!(matches!(code, 403 | 404), "{code}: {body}");
    }
    let verify = access::begin(&app.fixture.runtime, &recovery_scope, None)
        .await
        .unwrap();
    assert!(
        recovery::options(&verify)
            .await
            .unwrap()
            .subjects
            .into_iter()
            .all(|item| item.id.get() != subject)
    );
    verify.rollback().await.unwrap();
    app.close().await;
}
