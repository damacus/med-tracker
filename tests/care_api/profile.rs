use super::contract::{assert_value, contract, resolve};
use super::*;
use med_tracker::models::_entities::{
    accounts, api_change_events, api_idempotency_keys, carer_relationships,
    notification_preferences as preference_rows, people,
};
use med_tracker::models::entities::identity_api_key;
use sea_orm::{
    Condition,
    sea_query::{Expr, ExprTrait, Func, Query},
};

async fn account(app: &Application) -> accounts::Model {
    accounts::Entity::find_by_id(app.fixture.account.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

async fn set_preferences(app: &Application, preferences: Value) {
    let mut row = account(app).await.into_active_model();
    row.preferences = Set(preferences);
    row.update(&app.fixture.admin).await.unwrap();
}

async fn revoke_self_grant(app: &Application) {
    let mut grant = app.fixture.grant.clone().into_active_model();
    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    grant.update(&app.fixture.admin).await.unwrap();
}

async fn membership_state(app: &Application, active: bool, increment: bool) {
    let current = household_memberships::Entity::find_by_id(app.fixture.membership.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut row = current.clone().into_active_model();
    row.status = Set(if active { "active" } else { "revoked" }.into());
    row.revoked_at = Set((!active).then(|| chrono::Utc::now().naive_utc()));
    if increment {
        row.permissions_version = Set(current.permissions_version + 1);
    }
    row.update(&app.fixture.admin).await.unwrap();
}

struct ProfileEffects {
    versions: u64,
    preferences: u64,
    keys: u64,
    changes: u64,
}

async fn profile_effects(app: &Application) -> ProfileEffects {
    let versions = versions::Entity::find()
        .filter(versions::Column::ItemType.is_in(["Account", "Person", "NotificationPreference"]))
        .filter(
            Condition::any()
                .add(versions::Column::HouseholdId.eq(app.fixture.household.id))
                .add(
                    Condition::all()
                        .add(versions::Column::ItemType.eq("Account"))
                        .add(versions::Column::ItemId.eq(app.fixture.account.id)),
                ),
        )
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let preferences = preference_rows::Entity::find()
        .filter(preference_rows::Column::PersonId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let keys = api_idempotency_keys::Entity::find()
        .filter(api_idempotency_keys::Column::HouseholdId.eq(app.fixture.household.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(api_change_events::Column::RecordType.is_in(["Person", "NotificationPreference"]))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    ProfileEffects {
        versions,
        preferences,
        keys,
        changes,
    }
}

async fn blocked_by(app: &Application, blocker: i32) -> usize {
    let query = Query::select()
        .column("pid")
        .expr_as(
            Func::cust("to_jsonb").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
            "blockers",
        )
        .from("pg_stat_activity")
        .and_where(Expr::col("datname").eq(Func::cust("current_database")))
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .to_owned();
    let rows = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap();
    let graph = rows
        .iter()
        .map(|row| {
            (
                row.try_get::<i32>("", "pid").unwrap(),
                serde_json::from_value::<Vec<i32>>(row.try_get::<Value>("", "blockers").unwrap())
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let mut blocked = std::collections::HashSet::from([blocker]);
    loop {
        let before = blocked.len();
        for (pid, blockers) in &graph {
            if blockers.iter().any(|pid| blocked.contains(pid)) {
                blocked.insert(*pid);
            }
        }
        if blocked.len() == before {
            break;
        }
    }
    blocked.len() - 1
}

#[tokio::test]
async fn age_validation_ignores_existing_and_submitted_account_timezones() {
    use chrono::TimeZone;
    use med_tracker::controllers::api::care::AgeValidationClock;

    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    set_preferences(&app, json!({"time_zone":"Pacific/Kiritimati"})).await;
    let mut patient = app.fixture.adult.clone().into_active_model();
    patient.person_type = Set(2);
    patient.has_capacity = Set(false);
    patient.update(&app.fixture.admin).await.unwrap();
    let mut carer = app.fixture.minor.clone().into_active_model();
    carer.person_type = Set(0);
    carer.has_capacity = Set(true);
    carer.date_of_birth = Set(Some("1980-01-01".parse().unwrap()));
    carer.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    carer_relationships::ActiveModel {
        household_id: Set(household_id),
        carer_id: Set(app.fixture.minor.id),
        patient_id: Set(app.fixture.adult.id),
        relationship_type: Set(Some("family_member".into())),
        active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
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
            set_preferences(&app, json!({"time_zone":"Pacific/Kiritimati"})).await;
            let current = people::Entity::find_by_id(app.fixture.adult.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap();
            let mut person = current.into_active_model();
            person.date_of_birth = Set(Some("1985-02-03".parse().unwrap()));
            person.update(&app.fixture.admin).await.unwrap();
        }
    }
    app.close().await;
}
use sea_orm::TransactionTrait;

#[tokio::test]
async fn current_person_profile_api_saves_nonsecurity_fields_and_preserves_omitted_values() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    let original: Value = read.json().await.unwrap();
    assert_eq!(
        original["data"]["person_id"],
        app.fixture.adult.id.to_string()
    );
    assert_eq!(
        original["data"]["account_id"],
        app.fixture.account.id.to_string()
    );

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
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Person"))
        .filter(versions::Column::ItemId.eq(app.fixture.adult.id))
        .filter(versions::Column::Event.eq("update"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(household_id))
        .filter(api_change_events::Column::RecordType.eq("Person"))
        .filter(api_change_events::Column::RecordId.eq(app.fixture.adult.id))
        .filter(api_change_events::Column::Action.eq("update"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(audit, 1);
    assert_eq!(changes, 1);

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
async fn profile_api_rejects_invalid_and_security_fields_atomically_and_rechecks_membership() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let other = app.fixture.foreign_household().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
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
        .get(format!(
            "{}/api/v1/households/{}/profile",
            app.origin, other.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 403);
    revoke_self_grant(&app).await;
    let canonical_self = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(canonical_self.status().as_u16(), 200);
    membership_state(&app, false, true).await;
    let revoked = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status().as_u16(), 403);
    other.close().await;
    app.close().await;
}

#[tokio::test]
async fn profile_get_matches_documented_contract() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/profile"]["get"];
    assert_eq!(operation["operationId"], "getHouseholdProfile");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
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
    assert_eq!(body["data"]["person_id"], app.fixture.adult.id.to_string());
    assert_eq!(
        body["data"]["account_id"],
        app.fixture.account.id.to_string()
    );
    assert_eq!(body["data"]["date_of_birth"], "1985-02-03");

    revoke_self_grant(&app).await;
    let canonical_self = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(canonical_self.status().as_u16(), 200);
    membership_state(&app, false, true).await;
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
    assert_eq!(denied_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &denied_body,
        "revoked household membership",
    );
    assert_eq!(denied_body["error"]["code"], "forbidden");
    assert_eq!(denied_body["error"]["request_id"], denied_request_id);

    membership_state(&app, true, true).await;
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
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
    let mut preferences = account(&app).await.preferences;
    preferences["gravatar_enabled"] = json!("1");
    set_preferences(&app, preferences).await;
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
    assert_eq!(account(&app).await.preferences["time_zone"], "");
    app.close().await;
}

#[tokio::test]
async fn simultaneous_profile_preferences_updates_preserve_both_supplied_fields() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
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
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
    let body = json!({"profile":{"time_zone":"London"}});
    let replay_key = uuid::Uuid::new_v4().to_string();
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
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
        .header("idempotency-key", &replay_key)
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
        .header("idempotency-key", &replay_key)
        .json(&json!({"profile":{"time_zone":"UTC"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict: Value = conflict.json().await.unwrap();
    assert_eq!(conflict["error"]["code"], "idempotency_key_reused");
    let versions = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Account"))
        .filter(versions::Column::ItemId.eq(app.fixture.account.id))
        .filter(versions::Column::Event.eq("update"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(versions, 1);
    revoke_self_grant(&app).await;
    let canonical_self = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(canonical_self.status().as_u16(), 200);
    membership_state(&app, false, true).await;
    let denied_replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", &replay_key)
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
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_apis_reject_malformed_requests_without_writes() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    for resource in ["profile", "notification_preference"] {
        let endpoint = format!("{}/api/v1/households/{household_id}/{resource}", app.origin);
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
    let effects = profile_effects(&app).await;
    assert_eq!(effects.versions, 0);
    assert_eq!(effects.preferences, 0);
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_apis_require_the_account_linked_membership_person() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let other = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    let other_membership = household_memberships::ActiveModel {
        household_id: Set(other.household.id),
        person_id: Set(app.fixture.minor.id),
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
    for resource in ["profile", "notification_preference"] {
        let endpoint = format!(
            "{}/api/v1/households/{}/{resource}",
            app.origin, other.household.id
        );
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
            assert_eq!(response.status().as_u16(), 403);
        }
    }
    let canonical = account(&app).await.person_id;
    let original = household_memberships::Entity::find_by_id(app.fixture.membership.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let other_link = household_memberships::Entity::find_by_id(other_membership.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(canonical, app.fixture.adult.id);
    assert_eq!(original.person_id, app.fixture.adult.id);
    assert_eq!(original.household_id, household_id);
    assert_eq!(other_link.person_id, app.fixture.minor.id);
    assert_eq!(profile_effects(&app).await.versions, 0);
    other.close().await;
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_apis_roll_back_mutations_when_request_audit_fails() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let profile = format!("{}/api/v1/households/{household_id}/profile", app.origin);
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
    let fault = format!("profile_request_audit_{}", uuid::Uuid::new_v4().simple());
    let marker = format!("{fault}_reached");
    // SeaORM has no trigger, PL/pgSQL function or GRANT builder. This schema
    // fault requires whole-run exclusivity; all objects are removed below.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE {marker}; GRANT USAGE,SELECT ON SEQUENCE {marker} TO med_tracker_app; CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller' IN ('api/v1/profiles','api/v1/notification_preferences') AND NEW.metadata->>'action'='update' THEN PERFORM nextval('{marker}'); RAISE EXCEPTION 'Synthetic profile request audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION {fault}()")).await.unwrap();
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
            .patch(format!(
                "{}/api/v1/households/{household_id}/{resource}",
                app.origin
            ))
            .bearer_auth(&token)
            .header(
                "idempotency-key",
                format!("{}-synthetic-audit-{resource}", app.fixture.client_uuid),
            )
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
    let query = Query::select()
        .column("is_called")
        .from(sea_orm::sea_query::Alias::new(marker.clone()))
        .to_owned();
    let row = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    let reached: bool = row.try_get("", "is_called").unwrap();
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {fault} ON security_audit_events; DROP FUNCTION {fault}(); DROP SEQUENCE {marker}")).await.unwrap();
    let effects = profile_effects(&app).await;
    assert!(reached);
    assert_eq!(
        [
            effects.versions,
            effects.preferences,
            effects.keys,
            effects.changes
        ],
        [0; 4]
    );
    app.close().await;
}

#[tokio::test]
async fn profile_and_notification_keyed_replays_reject_revoked_oauth_credentials() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
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
            .patch(format!(
                "{}/api/v1/households/{household_id}/{resource}",
                app.origin
            ))
            .bearer_auth(&token)
            .header(
                "idempotency-key",
                format!("{}-synthetic-revoke-{resource}", app.fixture.client_uuid),
            )
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
    }
    let mut grant = app.fixture.authorization.clone().into_active_model();
    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    grant.update(&app.fixture.admin).await.unwrap();
    for (resource, body) in [
        ("profile", json!({"profile":{"time_zone":"London"}})),
        (
            "notification_preference",
            json!({"notification_preference":{"enabled":false}}),
        ),
    ] {
        let endpoint = format!("{}/api/v1/households/{household_id}/{resource}", app.origin);
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
            .header(
                "idempotency-key",
                format!("{}-synthetic-revoke-{resource}", app.fixture.client_uuid),
            )
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
        ("accounts", app.fixture.account.id)
    } else {
        ("households", app.fixture.household.id)
    };
    let household_id = app.fixture.household.id;
    let blocker = app.fixture.admin.begin().await.unwrap();
    let query = Query::select()
        .column("id")
        .from(table)
        .and_where(Expr::col("id").eq(id))
        .lock(sea_orm::sea_query::LockType::Update)
        .to_owned();
    blocker
        .query_one_raw(blocker.get_database_backend().build(&query))
        .await
        .unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let blocker_pid: i32 = blocker
        .query_one_raw(blocker.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let endpoint = format!("{}/api/v1/households/{household_id}/{resource}", app.origin);
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(token)
        .header(
            "idempotency-key",
            format!("{}-{key}", app.fixture.client_uuid),
        )
        .json(&bodies[0])
        .send();
    let second = app
        .client
        .patch(&endpoint)
        .bearer_auth(token)
        .header(
            "idempotency-key",
            format!("{}-{key}", app.fixture.client_uuid),
        )
        .json(&bodies[1])
        .send();
    let release = async {
        let mut reached = false;
        for _ in 0..200 {
            if blocked_by(app, blocker_pid).await >= 2 {
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
    let household_id = app.fixture.household.id;
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
    let creates = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .filter(versions::Column::ItemType.eq("NotificationPreference"))
        .filter(versions::Column::Event.eq("create"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(creates, 1);
    assert_eq!(profile_effects(&app).await.keys, 4);
    app.close().await;
}

pub(super) async fn issue_profile_personal_key(app: &Application, name: &str) -> String {
    use better_auth_core::{AuthRequest, HttpMethod};
    use med_tracker::models::identity::better_auth::{IdentityService, dispatch};

    let service = app.context.shared_store.get::<IdentityService>().unwrap();
    let origin = url::Url::parse(service.context().config.base_url.as_static().unwrap())
        .unwrap()
        .origin()
        .ascii_serialization();
    let mut login = AuthRequest::new(HttpMethod::Post, "/sign-in/email");
    login
        .headers
        .insert("content-type".into(), "application/json".into());
    login.body = Some(
        serde_json::to_vec(&json!({"email":app.fixture.account.email,"password":"password"}))
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
    let mut create = AuthRequest::new(HttpMethod::Post, "/security/api-key/create");
    create
        .headers
        .insert("content-type".into(), "application/json".into());
    create.headers.insert("cookie".into(), cookie);
    create.headers.insert("origin".into(), origin);
    create.body=Some(serde_json::to_vec(&json!({"name":name,"households":[app.fixture.household.id],"permissions":["care:read","care:write"],"expires_days":1})).unwrap());
    let issued = dispatch(&service, create, "synthetic-profile-personal-create".into())
        .await
        .unwrap();
    assert_eq!(issued.status, 200, "native personal-key creation failed");
    let issued: Value = issued.body.json().unwrap().unwrap();
    issued["apiKey"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn personal_key_profile_and_preferences_overlap_and_revoked_keys_cannot_replay() {
    let app = profile_application().await;
    let household_id = app.fixture.household.id;
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
            .patch(format!(
                "{}/api/v1/households/{household_id}/profile",
                app.origin
            ))
            .bearer_auth(&first)
            .header(
                "idempotency-key",
                format!(
                    "{}-synthetic-personal-profile-{index}",
                    app.fixture.client_uuid
                ),
            )
            .json(&json!({"profile":{"time_zone":zone}}))
            .send();
        let preferences = app
            .client
            .patch(format!(
                "{}/api/v1/households/{household_id}/notification_preference",
                app.origin
            ))
            .bearer_auth(&second)
            .header(
                "idempotency-key",
                format!(
                    "{}-synthetic-personal-pref-{index}",
                    app.fixture.client_uuid
                ),
            )
            .json(&json!({"notification_preference":{"enabled":enabled}}))
            .send();
        let (profile, preferences) = tokio::join!(profile, preferences);
        assert_eq!(profile.unwrap().status().as_u16(), 200);
        assert_eq!(preferences.unwrap().status().as_u16(), 200);
    }
    identity_api_key::Entity::update_many()
        .col_expr(identity_api_key::Column::Enabled, Expr::value(false))
        .filter(identity_api_key::Column::ReferenceId.eq(app.fixture.account.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
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
            .patch(format!(
                "{}/api/v1/households/{household_id}/{resource}",
                app.origin
            ))
            .bearer_auth(token)
            .header(
                "idempotency-key",
                format!("{}-{key}", app.fixture.client_uuid),
            )
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
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let tenant = access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor {
                account_id: app.fixture.account.id,
            },
            household_id: app.fixture.household.id,
            request_id: "synthetic-opposite-household-writer".into(),
        },
        None,
    )
    .await
    .unwrap();
    locations::authorize(&tenant).await.unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let blocker: i32 = tenant
        .transaction()
        .query_one_raw(
            tenant
                .transaction()
                .get_database_backend()
                .build(&pid_query),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let client = app.client.clone();
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
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
        if blocked_by(&app, blocker).await > 0 {
            waiting = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        waiting,
        "Profile write must reach the household lock before the actor audit"
    );
    let now = chrono::Utc::now().naive_utc();
    let inserted = security_audit_events::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(app.fixture.account.id)),
        actor_membership_id: Set(Some(app.fixture.membership.id)),
        event_type: Set("synthetic.opposite_writer".into()),
        request_id: Set(Some(uuid::Uuid::new_v4().to_string())),
        metadata: Set(json!({})),
        audit_context: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await;
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
    Application::new().await
}

#[tokio::test]
async fn profile_requires_a_date_of_birth_and_rejects_incomplete_records_without_writes() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let mut person = app.fixture.adult.clone().into_active_model();
    person.date_of_birth = Set(None);
    person.update(&app.fixture.admin).await.unwrap();
    let endpoint = format!("{}/api/v1/households/{household_id}/profile", app.origin);
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
    assert_eq!(account(&app).await.preferences, json!({}));
    assert_eq!(profile_effects(&app).await.versions, 0);
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
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let initial = app
        .client
        .put(format!(
            "{}/api/v1/households/{household_id}/notification_preference",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"notification_preference":{"enabled":true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(initial.status().as_u16(), 200);
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-midread-membership".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    membership_state(&app, true, true).await;
    let snapshot = profile::read(&tenant, app.fixture.account.id).await;
    let preferences = notification_preferences::read(&tenant, app.fixture.account.id).await;
    tenant.rollback().await.unwrap();
    revoke_self_grant(&app).await;
    let current = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let current_profile = profile::read(&current, app.fixture.account.id).await;
    let current_preferences =
        notification_preferences::read(&current, app.fixture.account.id).await;
    membership_state(&app, false, true).await;
    let revoked_profile = profile::read(&current, app.fixture.account.id).await;
    let revoked_preferences =
        notification_preferences::read(&current, app.fixture.account.id).await;
    current.rollback().await.unwrap();
    let fresh = access::begin(&app.fixture.runtime, &scope, None).await;
    let fresh_denied = matches!(fresh, Err(OperationError::Forbidden));
    app.close().await;
    assert_eq!(snapshot.err(), Some(OperationError::Forbidden));
    assert_eq!(preferences.err(), Some(OperationError::Forbidden));
    assert!(current_profile.is_ok());
    assert!(current_preferences.is_ok());
    assert_eq!(revoked_profile.err(), Some(OperationError::Forbidden));
    assert_eq!(revoked_preferences.err(), Some(OperationError::Forbidden));
    assert!(fresh_denied);
}
