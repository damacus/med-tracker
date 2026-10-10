use super::contract::{assert_value, contract, resolve};
use super::*;

#[tokio::test]
async fn age_validation_ignores_existing_and_submitted_account_timezones() {
    use chrono::TimeZone;
    use med_tracker::controllers::api::care::AgeValidationClock;

    let app = profile_application().await;
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences='{\"time_zone\":\"Pacific/Kiritimati\"}' WHERE id=71001; UPDATE people SET person_type=2,has_capacity=false WHERE id=73001; UPDATE people SET person_type=0,has_capacity=true,date_of_birth='1980-01-01' WHERE id=73002; INSERT INTO carer_relationships(household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(72001,73002,73001,'family_member',true,now(),now())").await.unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    for method in [reqwest::Method::PATCH, reqwest::Method::PUT] {
        for (hour, minute, second, expected) in [(23, 59, 59, 422), (0, 0, 0, 200)] {
            let day = if hour == 23 { 30 } else { 1 };
            let month = if hour == 23 { 6 } else { 7 };
            app.context.shared_store.insert(AgeValidationClock::fixed(
                chrono::Utc
                    .with_ymd_and_hms(2026, month, day, hour, minute, second)
                    .unwrap(),
                chrono_tz::UTC,
            ));
            let response = app.client.request(method.clone(), &endpoint).bearer_auth(&token)
                .json(&json!({"profile":{"date_of_birth":"2008-07-01","time_zone":"America/Los_Angeles"}})).send().await.unwrap();
            assert_eq!(response.status().as_u16(), expected);
            let saved = app
                .client
                .get(&endpoint)
                .bearer_auth(&token)
                .send()
                .await
                .unwrap()
                .json::<Value>()
                .await
                .unwrap();
            assert_eq!(
                saved["data"]["date_of_birth"],
                if expected == 200 {
                    "2008-07-01"
                } else {
                    "1985-02-03"
                }
            );
            assert_eq!(
                saved["data"]["time_zone"],
                if expected == 200 {
                    "America/Los_Angeles"
                } else {
                    "Pacific/Kiritimati"
                }
            );
            app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences='{\"time_zone\":\"Pacific/Kiritimati\"}' WHERE id=71001; UPDATE people SET date_of_birth='1985-02-03' WHERE id=73001").await.unwrap();
        }
    }
    app.close().await;
}
use sea_orm::TransactionTrait;

#[tokio::test]
async fn current_person_profile_api_saves_nonsecurity_fields_and_preserves_omitted_values() {
    let app = profile_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    let original: Value = read.json().await.unwrap();
    assert_eq!(original["data"]["person_id"], "73001");
    assert_eq!(original["data"]["account_id"], "71001");

    let updated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"profile": {
            "date_of_birth": "1986-04-17",
            "time_zone": "London",
            "gravatar_enabled": true,
            "mobile_shortcuts": ["dashboard", "inventory"]
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let updated: Value = updated.json().await.unwrap();
    assert_eq!(updated["data"]["date_of_birth"], "1986-04-17");
    assert_eq!(updated["data"]["time_zone"], "London");
    assert_eq!(updated["data"]["gravatar_enabled"], true);
    assert_eq!(
        updated["data"]["mobile_shortcuts"],
        json!(["dashboard", "inventory"])
    );
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=73001 AND event='update') AS versions, (SELECT count(*) FROM api_change_events WHERE record_type='Person' AND record_id=73001 AND action='update') AS changes"
    )).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<i64>("", "versions").unwrap(), 1);
    assert_eq!(audit.try_get::<i64>("", "changes").unwrap(), 1);

    let partial = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"profile": {
            "gravatar_enabled": false
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(partial.status().as_u16(), 200);
    let partial: Value = partial.json().await.unwrap();
    assert_eq!(partial["data"]["gravatar_enabled"], false);
    assert_eq!(partial["data"]["date_of_birth"], "1986-04-17");
    assert_eq!(partial["data"]["time_zone"], "London");
    assert_eq!(
        partial["data"]["mobile_shortcuts"],
        json!(["dashboard", "inventory"])
    );
    app.close().await;
}

#[tokio::test]
async fn profile_api_rejects_invalid_and_security_fields_atomically_and_rechecks_grants() {
    let app = profile_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let initial = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(initial.status().as_u16(), 200);
    let initial: Value = initial.json().await.unwrap();
    for body in [
        json!({"profile": {"date_of_birth": "1986-04-17", "mobile_shortcuts": ["dashboard", "dashboard"]}}),
        json!({"profile": {"date_of_birth": "1986-04-17", "time_zone": "Mars/Canal"}}),
        json!({"profile": {"date_of_birth": "1986-04-17", "email": "takeover@example.test"}}),
        json!({"profile": {"date_of_birth": "1986-04-17", "role": "owner"}}),
        json!({"profile": {"date_of_birth": null}}),
        json!({"profile": {"date_of_birth": "1986-4-7"}}),
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
        let current: Value = app
            .client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(current, initial);
    }
    let foreign = app
        .client
        .get(format!("{}/api/v1/households/72002/profile", app.origin))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 403);
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let revoked = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert!(matches!(revoked.status().as_u16(), 403 | 404));
    app.close().await;
}

#[tokio::test]
async fn profile_get_matches_documented_contract() {
    let app = profile_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/profile"]["get"];
    assert_eq!(operation["operationId"], "getHouseholdProfile");
    assert_eq!(
        operation["responses"]["404"]["$ref"],
        "#/components/responses/NotFound"
    );
    let not_found = resolve(contract, &operation["responses"]["404"]);
    let not_found_schema = resolve(
        contract,
        &not_found["content"]["application/json"]["schema"],
    );

    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let cache_control = read
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert_eq!(
        operation["responses"]["200"]["headers"]["Cache-Control"]["$ref"],
        "#/components/headers/no_store"
    );
    let no_store_header = resolve(
        contract,
        &operation["responses"]["200"]["headers"]["Cache-Control"],
    );
    assert_eq!(no_store_header["schema"]["enum"], json!(["no-store"]));
    assert_eq!(cache_control.as_deref(), Some("no-store"));
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/HouseholdProfileResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "profile response",
    );
    assert_eq!(body["data"]["person_id"], "73001");
    assert_eq!(body["data"]["account_id"], "71001");
    assert_eq!(body["data"]["date_of_birth"], "1985-02-03");

    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
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
    let denied_request_id = denied.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 404);
    assert_value(
        contract,
        not_found_schema,
        &denied_body,
        "outside view scope",
    );
    assert_eq!(denied_body["error"]["code"], "not_found");
    assert_eq!(denied_body["error"]["request_id"], denied_request_id);

    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=NULL WHERE id=78001")
        .await
        .unwrap();
    let restored = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(restored.status().as_u16(), 200);
    app.close().await;
}

#[tokio::test]
async fn profile_api_clears_stored_timezone_and_reads_legacy_gravatar_without_cache() {
    let app = profile_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    app.fixture.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_set(preferences,'{gravatar_enabled}','\"1\"') WHERE id=71001").await.unwrap();
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers().get("cache-control").unwrap(), "no-store");
    let read: Value = read.json().await.unwrap();
    assert_eq!(read["data"]["gravatar_enabled"], true);
    let cleared = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"profile":{"time_zone":""}}))
        .send()
        .await
        .unwrap();
    assert_eq!(cleared.status().as_u16(), 200);
    assert_eq!(cleared.headers().get("cache-control").unwrap(), "no-store");
    let cleared: Value = cleared.json().await.unwrap();
    assert_eq!(
        cleared["data"]["time_zone"],
        std::env::var("TZ").unwrap_or_else(|_| "UTC".into())
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT preferences->>'time_zone' AS time_zone FROM accounts WHERE id=71001",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "time_zone").unwrap(), "");
    app.close().await;
}

#[tokio::test]
async fn simultaneous_profile_preferences_updates_preserve_both_supplied_fields() {
    let app = profile_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let zone = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"profile":{"time_zone":"Europe/London"}}))
        .send();
    let gravatar = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"profile":{"gravatar_enabled":true}}))
        .send();
    let (zone, gravatar) = tokio::join!(zone, gravatar);
    assert_eq!(zone.unwrap().status().as_u16(), 200);
    assert_eq!(gravatar.unwrap().status().as_u16(), 200);
    let read: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(read["data"]["time_zone"], "Europe/London");
    assert_eq!(read["data"]["gravatar_enabled"], true);
    app.close().await;
}

#[tokio::test]
async fn keyed_profile_update_replays_without_a_second_account_version() {
    let app = profile_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let body = json!({"profile":{"time_zone":"London"}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-profile-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 200);
    let first: Value = first.json().await.unwrap();
    let replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-profile-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(
        replay.headers().get("idempotency-replayed").unwrap(),
        "true"
    );
    let replay: Value = replay.json().await.unwrap();
    assert_eq!(replay, first);
    let conflict = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-profile-key")
        .json(&json!({"profile":{"time_zone":"UTC"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict: Value = conflict.json().await.unwrap();
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS n FROM versions WHERE item_type='Account' AND item_id=71001 AND event='update'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "n").unwrap(), 1);
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let denied_replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-profile-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(matches!(denied_replay.status().as_u16(), 403 | 404));
    assert!(
        denied_replay
            .headers()
            .get("idempotency-replayed")
            .is_none()
    );
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_apis_reject_malformed_requests_without_writes() {
    let app = profile_application().await;
    let token = app.token().await;
    for resource in ["profile", "notification_preference"] {
        let endpoint = format!("{}/api/v1/households/72001/{resource}", app.origin);
        for method in [reqwest::Method::PATCH, reqwest::Method::PUT] {
            let malformed = app
                .client
                .request(method.clone(), &endpoint)
                .bearer_auth(&token)
                .header("content-type", "application/json")
                .body("{invalid")
                .send()
                .await
                .unwrap();
            assert_eq!(malformed.status().as_u16(), 400);
            let absent = app
                .client
                .request(method.clone(), &endpoint)
                .bearer_auth(&token)
                .json(&json!({}))
                .send()
                .await
                .unwrap();
            assert_eq!(absent.status().as_u16(), 400);
            let unauthenticated = app
                .client
                .request(method, &endpoint)
                .json(&json!({resource: {}}))
                .send()
                .await
                .unwrap();
            assert_eq!(unauthenticated.status().as_u16(), 401);
        }
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Account','Person','NotificationPreference')) AS versions, (SELECT count(*) FROM notification_preferences WHERE person_id=73001) AS preferences"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "versions").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "preferences").unwrap(), 0);
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_apis_require_the_account_linked_membership_person() {
    let app = profile_application().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET person_id=73002 WHERE id=74001")
        .await
        .unwrap();
    for resource in ["profile", "notification_preference"] {
        let endpoint = format!("{}/api/v1/households/72001/{resource}", app.origin);
        for method in [
            reqwest::Method::GET,
            reqwest::Method::PATCH,
            reqwest::Method::PUT,
        ] {
            let response = app
                .client
                .request(method, &endpoint)
                .bearer_auth(&token)
                .json(&if resource == "profile" {
                    json!({"profile":{"time_zone":"London"}})
                } else {
                    json!({"notification_preference":{"enabled":false}})
                })
                .send()
                .await
                .unwrap();
            assert!(matches!(response.status().as_u16(), 403 | 404));
        }
    }
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_apis_roll_back_mutations_when_request_audit_fails() {
    let app = profile_application().await;
    let token = app.token().await;
    let profile = format!("{}/api/v1/households/72001/profile", app.origin);
    let initial: Value = app
        .client
        .get(&profile)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE profile_request_audit_reached; GRANT USAGE,SELECT ON SEQUENCE profile_request_audit_reached TO med_tracker_app; CREATE FUNCTION reject_profile_request_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller' IN ('api/v1/profiles','api/v1/notification_preferences') AND NEW.metadata->>'action'='update' THEN PERFORM nextval('profile_request_audit_reached'); RAISE EXCEPTION 'Synthetic profile request audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_request_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_request_audit()").await.unwrap();
    for (resource, body) in [
        (
            "profile",
            json!({"profile":{"date_of_birth":"1986-04-17","time_zone":"Europe/London"}}),
        ),
        (
            "notification_preference",
            json!({"notification_preference":{"enabled":false}}),
        ),
    ] {
        let response = app
            .client
            .patch(format!("{}/api/v1/households/72001/{resource}", app.origin))
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-audit-{resource}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 500);
    }
    let current: Value = app
        .client
        .get(&profile)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current, initial);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT is_called FROM profile_request_audit_reached) AS reached, (SELECT count(*) FROM versions WHERE item_type IN ('Account','Person','NotificationPreference')) AS versions, (SELECT count(*) FROM notification_preferences WHERE person_id=73001) AS preferences, (SELECT count(*) FROM api_idempotency_keys WHERE key LIKE 'synthetic-audit-%') AS keys, (SELECT count(*) FROM api_change_events WHERE record_type IN ('Person','NotificationPreference')) AS changes"
    )).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "reached").unwrap());
    for column in ["versions", "preferences", "keys", "changes"] {
        assert_eq!(row.try_get::<i64>("", column).unwrap(), 0);
    }
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_keyed_replays_reject_revoked_oauth_credentials() {
    let app = profile_application().await;
    let token = app.token().await;
    for (resource, body) in [
        ("profile", json!({"profile":{"time_zone":"London"}})),
        (
            "notification_preference",
            json!({"notification_preference":{"enabled":false}}),
        ),
    ] {
        let response = app
            .client
            .patch(format!("{}/api/v1/households/72001/{resource}", app.origin))
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-revoke-{resource}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
    }
    app.fixture
        .admin
        .execute_unprepared("UPDATE oauth_grants SET revoked_at=now() WHERE id=76001")
        .await
        .unwrap();
    for (resource, body) in [
        ("profile", json!({"profile":{"time_zone":"London"}})),
        (
            "notification_preference",
            json!({"notification_preference":{"enabled":false}}),
        ),
    ] {
        let endpoint = format!("{}/api/v1/households/72001/{resource}", app.origin);
        let read = app
            .client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(read.status().as_u16(), 401);
        let response = app
            .client
            .patch(&endpoint)
            .bearer_auth(&token)
            .header("idempotency-key", format!("synthetic-revoke-{resource}"))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        assert!(response.headers().get("idempotency-replayed").is_none());
    }
    app.close().await;
}

async fn overlapping_profile_requests(
    app: &Application,
    token: &str,
    resource: &str,
    key: &str,
    bodies: [Value; 2],
) -> [reqwest::Response; 2] {
    let (table, id) = if resource == "profile" {
        ("accounts", 71001)
    } else {
        ("households", 72001)
    };
    let blocker = app.fixture.admin.begin().await.unwrap();
    blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT id FROM {table} WHERE id={id} FOR UPDATE"),
        ))
        .await
        .unwrap();
    let endpoint = format!("{}/api/v1/households/72001/{resource}", app.origin);
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(token)
        .header("idempotency-key", key)
        .json(&bodies[0])
        .send();
    let second = app
        .client
        .patch(&endpoint)
        .bearer_auth(token)
        .header("idempotency-key", key)
        .json(&bodies[1])
        .send();
    let release = async {
        let mut reached = false;
        for _ in 0..200 {
            let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                "SELECT count(*) AS waiting FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND (query LIKE '%FOR UPDATE%' OR query LIKE '%FOR NO KEY UPDATE%')"
            )).await.unwrap().unwrap();
            if row.try_get::<i64>("", "waiting").unwrap() >= 2 {
                reached = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        blocker.commit().await.unwrap();
        assert!(
            reached,
            "Both mutations must reach the serialization lock before releasing it"
        );
    };
    let (first, second, ()) = tokio::join!(first, second, release);
    [first.unwrap(), second.unwrap()]
}

#[tokio::test]
async fn overlapping_keyed_profile_and_notification_requests_replay_once_and_conflict() {
    let app = profile_application().await;
    let token = app.token().await;
    for (resource, body, conflicting) in [
        (
            "profile",
            json!({"profile":{"time_zone":"London"}}),
            json!({"profile":{"time_zone":"UTC"}}),
        ),
        (
            "notification_preference",
            json!({"notification_preference":{"enabled":false}}),
            json!({"notification_preference":{"enabled":true}}),
        ),
    ] {
        let replies = overlapping_profile_requests(
            &app,
            &token,
            resource,
            &format!("synthetic-overlap-{resource}"),
            [body.clone(), body.clone()],
        )
        .await;
        let statuses = replies.each_ref().map(|reply| reply.status().as_u16());
        assert_eq!(
            statuses,
            [200, 200],
            "Identical overlapping {resource} requests must replay"
        );
        assert_eq!(
            replies
                .iter()
                .filter(|reply| reply.headers().get("idempotency-replayed").is_some())
                .count(),
            1
        );
        let [first, second] = replies;
        assert_eq!(
            first.json::<Value>().await.unwrap(),
            second.json::<Value>().await.unwrap()
        );
        let replies = overlapping_profile_requests(
            &app,
            &token,
            resource,
            &format!("synthetic-overlap-conflict-{resource}"),
            [body, conflicting],
        )
        .await;
        let mut statuses = replies.each_ref().map(|reply| reply.status().as_u16());
        statuses.sort();
        assert_eq!(statuses, [200, 409]);
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM versions WHERE item_type='NotificationPreference' AND event='create') AS preference_creates, (SELECT count(*) FROM api_idempotency_keys WHERE key LIKE 'synthetic-overlap-%') AS keys"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "preference_creates").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "keys").unwrap(), 4);
    app.close().await;
}

async fn issue_profile_personal_key(app: &Application, name: &str) -> String {
    use better_auth_core::{AuthRequest, HttpMethod};
    use med_tracker::models::identity::better_auth::{IdentityService, dispatch};
    app.fixture.admin.execute_unprepared(
        "INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=EXCLUDED.recovery_saved_at",
    ).await.unwrap();
    let service = app.context.shared_store.get::<IdentityService>().unwrap();
    let origin = url::Url::parse(&service.context().config.base_url)
        .unwrap()
        .origin()
        .ascii_serialization();
    let mut login = AuthRequest::new(HttpMethod::Post, "/sign-in/email");
    login
        .headers
        .insert("content-type".into(), "application/json".into());
    login.body = Some(
        serde_json::to_vec(&json!({"email":"persistence@example.test","password":"password"}))
            .unwrap(),
    );
    let signed_in = dispatch(&service, login, "synthetic-profile-personal-login".into())
        .await
        .unwrap();
    assert_eq!(signed_in.status, 200);
    let cookie = signed_in
        .headers
        .get_all("set-cookie")
        .filter_map(|value| value.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    assert!(!cookie.is_empty());
    let mut start = AuthRequest::new(HttpMethod::Post, "/security/operation/start");
    start
        .headers
        .insert("content-type".into(), "application/json".into());
    start.headers.insert("cookie".into(), cookie.clone());
    start.headers.insert("origin".into(), origin.clone());
    start.body = Some(serde_json::to_vec(&json!({"action":"create_api_key","key":{"name":name,"households":[72001],"permissions":["care:read","care:write"],"expires_days":1}})).unwrap());
    let started = dispatch(&service, start, "synthetic-profile-personal-start".into())
        .await
        .unwrap();
    if started.status != 200 {
        let error: Value = serde_json::from_slice(&started.body).unwrap();
        panic!(
            "Personal key operation setup rejected: status={}, code={}, message={}",
            started.status, error["code"], error["message"]
        );
    }
    let started: Value = serde_json::from_slice(&started.body).unwrap();
    let mut confirm = AuthRequest::new(HttpMethod::Post, "/security/password/confirm");
    confirm
        .headers
        .insert("content-type".into(), "application/json".into());
    confirm.headers.insert("cookie".into(), cookie);
    confirm.headers.insert("origin".into(), origin);
    confirm.body = Some(
        serde_json::to_vec(&json!({"operation_id":started["operation_id"],"password":"password"}))
            .unwrap(),
    );
    let issued = dispatch(
        &service,
        confirm,
        "synthetic-profile-personal-confirm".into(),
    )
    .await
    .unwrap();
    if issued.status != 200 {
        let error: Value = serde_json::from_slice(&issued.body).unwrap();
        panic!(
            "Personal key confirmation setup rejected: status={}, code={}, message={}",
            issued.status, error["code"], error["message"]
        );
    }
    let issued: Value = serde_json::from_slice(&issued.body).unwrap();
    issued["apiKey"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn personal_key_profile_and_preferences_overlap_and_revoked_keys_cannot_replay() {
    let app = profile_application().await;
    let first = issue_profile_personal_key(&app, "Synthetic profile writer one").await;
    let second = issue_profile_personal_key(&app, "Synthetic profile writer two").await;
    for index in 0..10 {
        let zone = if index % 2 == 0 {
            "Europe/London"
        } else {
            "UTC"
        };
        let enabled = index % 2 == 0;
        let profile = app
            .client
            .patch(format!("{}/api/v1/households/72001/profile", app.origin))
            .bearer_auth(&first)
            .header(
                "idempotency-key",
                format!("synthetic-personal-profile-{index}"),
            )
            .json(&json!({"profile":{"time_zone":zone}}))
            .send();
        let preferences = app
            .client
            .patch(format!(
                "{}/api/v1/households/72001/notification_preference",
                app.origin
            ))
            .bearer_auth(&second)
            .header(
                "idempotency-key",
                format!("synthetic-personal-pref-{index}"),
            )
            .json(&json!({"notification_preference":{"enabled":enabled}}))
            .send();
        let (profile, preferences) = tokio::join!(profile, preferences);
        assert_eq!(profile.unwrap().status().as_u16(), 200);
        assert_eq!(preferences.unwrap().status().as_u16(), 200);
    }
    app.fixture.admin.execute_unprepared("UPDATE identity_api_keys SET payload=jsonb_set(payload,'{enabled}','false') WHERE account_id=71001").await.unwrap();
    for (resource, body, key, token) in [
        (
            "profile",
            json!({"profile":{"time_zone":"UTC"}}),
            "synthetic-personal-profile-9",
            &first,
        ),
        (
            "notification_preference",
            json!({"notification_preference":{"enabled":false}}),
            "synthetic-personal-pref-9",
            &second,
        ),
    ] {
        let rejected = app
            .client
            .patch(format!("{}/api/v1/households/72001/{resource}", app.origin))
            .bearer_auth(token)
            .header("idempotency-key", key)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status().as_u16(), 401);
        assert!(rejected.headers().get("idempotency-replayed").is_none());
    }
    app.close().await;
}

#[tokio::test]
async fn profile_write_does_not_deadlock_a_household_writer_recording_actor_audit() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::locations,
    };
    let app = profile_application().await;
    let token = app.token().await;
    let tenant = access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "synthetic-opposite-household-writer".into(),
        },
    )
    .await
    .unwrap();
    locations::authorize(&tenant).await.unwrap();
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let profile = tokio::spawn(async move {
        client
            .patch(endpoint)
            .bearer_auth(token)
            .json(&json!({"profile":{"date_of_birth":"1986-04-17","time_zone":"Europe/London"}}))
            .send()
            .await
            .unwrap()
    });
    let mut waiting = false;
    for _ in 0..200 {
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS waiting FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%households%' AND query LIKE '%FOR UPDATE%'"
        )).await.unwrap().unwrap();
        if row.try_get::<i64>("", "waiting").unwrap() > 0 {
            waiting = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        waiting,
        "Profile write must reach the household lock before the actor audit"
    );
    let inserted = tenant.transaction().execute_unprepared("INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(72001,71001,74001,'synthetic.opposite_writer','synthetic-opposite-household-writer','{}','{}',now(),now())").await;
    if inserted.is_ok() {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let response = profile.await.unwrap();
    assert!(
        inserted.is_ok(),
        "Household writer must retain account FK access: {inserted:?}"
    );
    assert_eq!(response.status().as_u16(), 200);
    app.close().await;
}

async fn profile_application() -> Application {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1985-02-03' WHERE id=73001")
        .await
        .unwrap();
    app
}

#[tokio::test]
async fn profile_requires_a_date_of_birth_and_rejects_incomplete_records_without_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile", app.origin);
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 422);
    for method in [reqwest::Method::PATCH, reqwest::Method::PUT] {
        let denied = app
            .client
            .request(method, &endpoint)
            .bearer_auth(&token)
            .json(&json!({"profile":{"time_zone":"Europe/London"}}))
            .send()
            .await
            .unwrap();
        assert_eq!(denied.status().as_u16(), 422);
    }
    let unchanged = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT preferences FROM accounts WHERE id=71001) AS preferences, (SELECT count(*) FROM versions WHERE item_type IN ('Account','Person')) AS versions"
    )).await.unwrap().unwrap();
    assert_eq!(
        unchanged.try_get::<Value>("", "preferences").unwrap(),
        json!({})
    );
    assert_eq!(unchanged.try_get::<i64>("", "versions").unwrap(), 0);
    let repaired = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"profile":{"date_of_birth":"1986-04-17","time_zone":"Europe/London"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(repaired.status().as_u16(), 200);
    let repaired: Value = repaired.json().await.unwrap();
    assert_eq!(repaired["data"]["date_of_birth"], "1986-04-17");
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(
        read.json::<Value>().await.unwrap()["data"]["date_of_birth"],
        "1986-04-17"
    );
    app.close().await;
}

#[tokio::test]
async fn membership_change_during_read_reports_forbidden_not_missing_profile() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        errors::OperationError,
        notification_preferences, profile,
    };
    let app = profile_application().await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-midread-membership".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET permissions_version=permissions_version+1 WHERE id=74001",
        )
        .await
        .unwrap();
    let snapshot = profile::read(&tenant, 71001).await;
    let preferences = notification_preferences::read(&tenant, 71001).await;
    tenant.rollback().await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let unscoped_tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let unscoped = profile::read(&unscoped_tenant, 71001).await;
    unscoped_tenant.rollback().await.unwrap();
    app.close().await;
    assert_eq!(snapshot.err(), Some(OperationError::Forbidden));
    assert_eq!(preferences.err(), Some(OperationError::Forbidden));
    assert_eq!(unscoped.err(), Some(OperationError::NotFound));
}
