use super::Application;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use med_tracker::models::{
    _entities::{household_memberships, medication_takes, medications, schedules},
    entities::identity_api_key,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
    sea_query::Expr,
};
use serde_json::{Value, json};
use std::io::{Cursor, Write};
use zip::{ZipWriter, write::SimpleFileOptions};

async fn ready(app: &Application, token: &str) -> (String, String) {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in [
        (
            "portable.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice(),
        ),
        (
            "manifest.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json")
                .as_slice(),
        ),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data).unwrap();
    }
    let base = format!(
        "{}/api/v1/households/{}/data_imports",
        app.origin, app.fixture.household.id
    );
    let mut input = json!({"source":"med_tracker05", "archive_base64":STANDARD.encode(zip.finish().unwrap().into_inner()),
        "passphrase":"password", "source_timezone":"Europe/London", "timezone_confirmed":true});
    let inspection = app
        .client
        .post(format!("{base}/inspections"))
        .bearer_auth(token)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(inspection.status().as_u16(), 201);
    let inspection: Value = inspection.json().await.unwrap();
    let source = inspection["data"]["inventory"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["identity"]["kind"] == "person")
        .unwrap()["identity"]
        .clone();
    let omissions: Vec<_> = inspection["data"]["inventory"]["omissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["acknowledgement_id"].clone())
        .collect();
    input["inspection_id"] = inspection["data"]["id"].clone();
    input["mappings"] = json!([{"source":source,"destination":"NewDependent"}]);
    input["declarations"] =
        json!([{"InitialGuardianDeclaration":{"source_person":source,"accepted":true}}]);
    input["recipient_consents"] = json!([]);
    input["acknowledged_omissions"] = json!(omissions);
    let response = app
        .client
        .post(format!("{base}/previews"))
        .bearer_auth(token)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"]["report"]["state"], "ready");
    (
        body["data"]["report"]["id"].as_str().unwrap().to_owned(),
        body["data"]["confirmation"].as_str().unwrap().to_owned(),
    )
}

async fn counts(app: &Application) -> (u64, u64, u64, u64) {
    let household = app.fixture.household.id;
    let db = &app.fixture.admin;
    (
        household_memberships::Entity::find()
            .filter(household_memberships::Column::HouseholdId.eq(household))
            .count(db)
            .await
            .unwrap(),
        medications::Entity::find()
            .filter(medications::Column::HouseholdId.eq(household))
            .count(db)
            .await
            .unwrap(),
        schedules::Entity::find()
            .filter(schedules::Column::HouseholdId.eq(household))
            .count(db)
            .await
            .unwrap(),
        medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(household))
            .count(db)
            .await
            .unwrap(),
    )
}

#[tokio::test]
async fn explicit_confirmation_completes_and_replay_does_not_duplicate_records() {
    let app = Application::new().await;
    let token = app.token().await;
    let (id, confirmation) = ready(&app, &token).await;
    let base = format!(
        "{}/api/v1/households/{}/data_imports",
        app.origin, app.fixture.household.id
    );
    let url = format!("{base}/{id}/confirm");
    let invalid = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .json(&json!({"preview_token":confirmation,"passphrase":"password","actor_id":1}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status().as_u16(), 400);
    let too_large = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("x".repeat(64 * 1024 + 1))
        .send()
        .await
        .unwrap();
    assert_eq!(too_large.status().as_u16(), 422);
    let wrong = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .json(&json!({"preview_token":"wrong","passphrase":"password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status().as_u16(), 403);
    let before = counts(&app).await;
    let request = json!({"preview_token":confirmation,"passphrase":"password"});
    let first = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 200);
    let completed: Value = first.json().await.unwrap();
    assert_eq!(completed["data"]["report"]["state"], "completed");
    let after = counts(&app).await;
    assert_ne!(before, after);
    let detail = app
        .client
        .get(format!("{base}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(detail.status().as_u16(), 200);
    let detail: Value = detail.json().await.unwrap();
    assert_eq!(detail["data"]["report"], completed["data"]["report"]);
    let list = app
        .client
        .get(&base)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(list.status().as_u16(), 200);
    let list: Value = list.json().await.unwrap();
    assert!(
        list["data"]["reports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|report| report["id"] == id && report["state"] == "completed")
    );
    let replay = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .json(&request)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 200);
    let replay: Value = replay.json().await.unwrap();
    assert_eq!(replay["data"]["report"], completed["data"]["report"]);
    assert_eq!(counts(&app).await, after);
    app.close().await;
}

#[tokio::test]
async fn confirmation_denies_stale_destination_read_only_and_revoked_credentials() {
    let app = Application::new().await;
    let token = app.token().await;
    let (id, confirmation) = ready(&app, &token).await;
    let url = format!(
        "{}/api/v1/households/{}/data_imports/{id}/confirm",
        app.origin, app.fixture.household.id
    );
    let body = json!({"preview_token":confirmation,"passphrase":"password"});
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let issue = |permission: &'static str| {
        let service = service.clone();
        let account = app.fixture.account.id;
        let household = app.fixture.household.id;
        async move {
            service
                .api_keys()
                .unwrap()
                .create(
                    &account.to_string(),
                    better_auth::server_api::CreateKeyOptions {
                        config_id: Some("medtracker-personal".into()),
                        name: Some("Confirmation test".into()),
                        expires_in: Some(3600.0),
                        metadata: Some(json!({"households":[household]})),
                        permissions: Some(std::collections::HashMap::from([(
                            "medtracker".into(),
                            vec![permission.into()],
                        )])),
                        ..Default::default()
                    },
                )
                .await
                .unwrap()
                .key
        }
    };
    let read_key = issue("care:read").await;
    let write_key = issue("care:write").await;
    let denied = app
        .client
        .post(&url)
        .bearer_auth(&read_key)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    identity_api_key::Entity::update_many()
        .col_expr(identity_api_key::Column::Enabled, Expr::value(false))
        .filter(identity_api_key::Column::ReferenceId.eq(app.fixture.account.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let revoked = app
        .client
        .post(&url)
        .bearer_auth(&write_key)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(matches!(revoked.status().as_u16(), 401 | 403));
    let before = counts(&app).await;
    let member = app.fixture.scenario.membership.clone();
    let mut changed = member.clone().into_active_model();
    changed.updated_at = Set(member.updated_at + chrono::Duration::seconds(1));
    changed.update(&app.fixture.admin).await.unwrap();
    let stale = app
        .client
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status().as_u16(), 409);
    assert_eq!(counts(&app).await, before);
    app.close().await;
}
