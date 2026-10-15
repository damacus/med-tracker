use super::contract::{assert_value, contract, resolve};
use super::*;

#[tokio::test]
async fn managed_notifications_require_explicit_opt_in_for_current_people_of_every_type() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        notification_preferences,
    };
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES(73991,'Dependent notification subject',2,false,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74991,72001,73991,'member','active',now(),now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "notification-opt-in".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let people = notification_preferences::managed::read(&tenant).await;
    assert!(
        people.is_ok(),
        "current household participants can be projected without legacy person grants"
    );
    let people = serde_json::to_value(people.unwrap()).unwrap();
    for id in [73002, 73991] {
        let person = people
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .expect("current participant is selectable");
        assert_eq!(
            person["enabled"], false,
            "no automatic child/dependent subscription"
        );
    }
    assert!(
        !people
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == 73001)
    );
    notification_preferences::managed::update(
        &tenant,
        med_tracker::models::ids::AccountId::from_database(71001),
        notification_preferences::Changes::default(),
        &[
            med_tracker::models::ids::PersonId::from_database(73002),
            med_tracker::models::ids::PersonId::from_database(73991),
        ],
    )
    .await
    .expect("explicit opt-in accepts minors and dependent adults");
    tenant.commit().await.unwrap();
    let opted=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM household_notification_subscriptions WHERE account_id=71001 AND household_id=72001 AND enabled")).await.unwrap().unwrap();
    assert_eq!(opted.try_get::<i64>("", "count").unwrap(), 2);
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE notification_subscription_audit_reached; GRANT USAGE ON SEQUENCE notification_subscription_audit_reached TO med_tracker_app; CREATE FUNCTION reject_notification_subscription_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='notification_preferences.managed_people.updated' THEN PERFORM nextval('public.notification_subscription_audit_reached'); RAISE EXCEPTION 'synthetic subscription audit failure' USING ERRCODE='42501'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_notification_subscription_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_notification_subscription_audit()").await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let failed = notification_preferences::managed::update(
        &tenant,
        med_tracker::models::ids::AccountId::from_database(71001),
        notification_preferences::Changes {
            enabled: Some(false),
            ..Default::default()
        },
        &[],
    )
    .await;
    assert!(failed.is_err());
    tenant.rollback().await.unwrap();
    let rollback=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM notification_subscription_audit_reached) AS reached,(SELECT enabled FROM notification_preferences WHERE account_id=71001 AND household_id=72001) AS enabled,(SELECT count(*) FROM household_notification_subscriptions WHERE account_id=71001 AND enabled) AS subscriptions,(SELECT count(*) FROM security_audit_events WHERE event_type='notification_preferences.managed_people.updated') AS audits,(SELECT count(*) FROM versions WHERE item_type='NotificationPreference') AS versions")).await.unwrap().unwrap();
    assert!(rollback.try_get::<bool>("", "reached").unwrap());
    assert!(rollback.try_get::<bool>("", "enabled").unwrap());
    assert_eq!(rollback.try_get::<i64>("", "subscriptions").unwrap(), 2);
    assert_eq!(rollback.try_get::<i64>("", "audits").unwrap(), 1);
    assert_eq!(rollback.try_get::<i64>("", "versions").unwrap(), 1);
    app.fixture.admin.execute_unprepared("DROP TRIGGER reject_notification_subscription_audit ON security_audit_events; DROP FUNCTION reject_notification_subscription_audit()").await.unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74991",
        )
        .await
        .unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let current = serde_json::to_value(
        notification_preferences::managed::read(&tenant)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        !current
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == 73991)
    );
    assert!(
        notification_preferences::managed::update(
            &tenant,
            med_tracker::models::ids::AccountId::from_database(71001),
            notification_preferences::Changes::default(),
            &[med_tracker::models::ids::PersonId::from_database(73991)]
        )
        .await
        .is_err(),
        "departed subject cannot be subscribed"
    );
    tenant.rollback().await.unwrap();
    let retained=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT enabled FROM household_notification_subscriptions WHERE account_id=71001 AND person_id=73002")).await.unwrap().unwrap();
    assert!(retained.try_get::<bool>("", "enabled").unwrap());
    app.close().await;
}

#[tokio::test]
async fn notification_preference_api_creates_defaults_and_preserves_omitted_fields() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);

    let created = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {"enabled": false}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 200);
    let initial_etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let created: Value = created.json().await.unwrap();
    assert_eq!(created["data"]["person_id"], 73001);
    assert_eq!(created["data"]["enabled"], false);
    assert_eq!(created["data"]["dose_due_enabled"], true);
    assert_eq!(created["data"]["private_text_enabled"], false);
    assert_eq!(created["data"]["morning_time"], "08:00:00");
    assert_eq!(created["data"]["night_time"], "22:00:00");

    let changed = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {
            "dose_due_enabled": false, "morning_time": "07:05", "night_time": null
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(changed.status().as_u16(), 200);
    let changed_etag = changed
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_ne!(changed_etag, initial_etag);
    let changed: Value = changed.json().await.unwrap();
    assert_eq!(changed["data"]["enabled"], false);
    assert_eq!(changed["data"]["dose_due_enabled"], false);
    assert_eq!(changed["data"]["morning_time"], "07:05:00");
    assert!(changed["data"]["night_time"].is_null());

    let no_op = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {
            "dose_due_enabled": false, "morning_time": "07:05:00", "night_time": null
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(no_op.status().as_u16(), 200);
    assert_eq!(no_op.headers().get("etag").unwrap(), changed_etag.as_str());
    let no_op: Value = no_op.json().await.unwrap();
    assert_eq!(no_op, changed);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_api_rejects_invalid_input_without_partial_write() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    for body in [
        json!({"notification_preference": {}}),
        json!({"notification_preference": {"enabled": "false"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "24:00"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "23:59:60"}}),
        json!({"notification_preference": {"enabled": true, "morning_time": "07:5"}}),
        json!({"notification_preference": {"enabled": true, "person_id": 73001}}),
    ] {
        let rejected = app
            .client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status().as_u16(), 422);
    }
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_api_keyed_replay_conflict_and_current_membership_denial() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let body = json!({"notification_preference":{"enabled":false}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 200);
    let etag = first.headers().get("etag").unwrap().to_owned();
    let first: Value = first.json().await.unwrap();
    let replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(
        replay.headers().get("idempotency-replayed").unwrap(),
        "true"
    );
    assert_eq!(replay.headers().get("etag").unwrap(), etag);
    let replay: Value = replay.json().await.unwrap();
    assert_eq!(replay, first);
    let conflict = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&json!({"notification_preference":{"enabled":true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict: Value = conflict.json().await.unwrap();
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    let snapshot_sql = "SELECT jsonb_build_object('preferences',(SELECT jsonb_agg(to_jsonb(n)) FROM notification_preferences n),'versions',(SELECT count(*) FROM versions WHERE item_type='NotificationPreference'),'keys',(SELECT count(*) FROM api_idempotency_keys WHERE key='synthetic-notification-key')) AS state";
    let before = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot_sql))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "state")
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001")
        .await
        .unwrap();
    let denied_replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-notification-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(denied_replay.status().as_u16(), 403);
    assert!(
        denied_replay
            .headers()
            .get("idempotency-replayed")
            .is_none()
    );
    let after = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, snapshot_sql))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "state")
        .unwrap();
    assert_eq!(
        before, after,
        "revoked replay leaves preferences, audit and replay cache unchanged"
    );
    app.close().await;
}

#[tokio::test]
async fn notification_audit_versions_record_old_new_pairs_and_omit_unchanged_fields() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let created = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference":{"enabled":false}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 200);
    let created: Value = created.json().await.unwrap();
    let creation = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT object,object_changes::jsonb AS changes FROM versions WHERE item_type='NotificationPreference' AND event='create'"
    )).await.unwrap().unwrap();
    assert!(
        creation
            .try_get::<Option<String>>("", "object")
            .unwrap()
            .is_none()
    );
    let changes = creation.try_get::<Value>("", "changes").unwrap();
    assert_eq!(changes["enabled"], json!([null, false]));
    assert_eq!(changes["morning_time"], json!([null, "08:00:00"]));
    let patch = json!({"notification_preference":{"morning_time":"07:05","night_time":null}});
    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&patch)
        .send()
        .await
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let updated: Value = updated.json().await.unwrap();
    let version = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT object::jsonb AS before,object_changes::jsonb AS changes FROM versions WHERE item_type='NotificationPreference' AND event='update'"
    )).await.unwrap().unwrap();
    assert_eq!(
        version.try_get::<Value>("", "before").unwrap(),
        created["data"]
    );
    assert_eq!(
        version.try_get::<Value>("", "changes").unwrap(),
        json!({
            "morning_time":["08:00:00","07:05:00"],
            "night_time":["22:00:00",null],
            "updated_at":[created["data"]["updated_at"],updated["data"]["updated_at"]]
        })
    );
    let no_op = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&patch)
        .send()
        .await
        .unwrap();
    assert_eq!(no_op.status().as_u16(), 200);
    let count = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS versions FROM versions WHERE item_type='NotificationPreference'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count.try_get::<i64>("", "versions").unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn notification_preference_get_matches_documented_contract() {
    let app = Application::new().await;
    let issued = app.token_pair().await;
    let token = issued["access_token"].as_str().unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/notification_preference",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/notification_preference"]["get"];
    assert_eq!(operation["operationId"], "getNotificationPreference");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    let not_found = resolve(contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        contract,
        &not_found["content"]["application/json"]["schema"],
    );

    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let missing_request_id = missing.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let missing_body: Value = missing.json().await.unwrap();
    assert_eq!(missing_status, 404);
    assert_value(
        contract,
        not_found_schema,
        &missing_body,
        "absent preference",
    );
    assert_eq!(missing_body["error"]["code"], "not_found");
    assert_eq!(missing_body["error"]["request_id"], missing_request_id);

    let created = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"notification_preference": {"evening_time": "19:30"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 200);

    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let content_type = read
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert!(
        content_type
            .as_deref()
            .is_some_and(|value| value.starts_with("application/json"))
    );
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/NotificationPreferenceResponse"
    );
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    let etag_schema = resolve(contract, &operation["responses"]["200"]["headers"]["ETag"]);
    assert_eq!(etag_schema["required"], true);
    assert_eq!(etag_schema["schema"]["type"], "string");
    assert!(etag.as_deref().is_some_and(|value| !value.is_empty()));
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "preference response",
    );
    assert_eq!(body["data"]["person_id"], 73001);
    assert_eq!(body["data"]["evening_time"], "19:30:00");

    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001")
        .await
        .unwrap();
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    assert_value(
        contract,
        resolve(
            contract,
            &forbidden["content"]["application/json"]["schema"],
        ),
        &denied_body,
        "revoked household participation",
    );
    assert_eq!(denied_body["error"]["code"], "forbidden");

    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET status='active',revoked_at=NULL,permissions_version=permissions_version+1 WHERE id=74001")
        .await
        .unwrap();
    let refreshed = app
        .client
        .post(format!("{}/token", app.origin))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(
            serde_urlencoded::to_string([
                ("grant_type", "refresh_token"),
                ("client_id", "native"),
                ("refresh_token", issued["refresh_token"].as_str().unwrap()),
            ])
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(refreshed.status().as_u16(), 200);
    let refreshed: Value = refreshed.json().await.unwrap();
    let restored = app
        .client
        .get(&endpoint)
        .bearer_auth(refreshed["access_token"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(restored.status().as_u16(), 200);
    app.close().await;
}
