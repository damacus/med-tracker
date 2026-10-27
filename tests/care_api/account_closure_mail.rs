use super::platform::{operation_start, sign_in};
use super::*;
use med_tracker::models::_entities::{
    accounts, api_app_tokens, api_sessions, medications, native_device_tokens, oauth_grants,
    people, person_access_grants, push_subscriptions,
};
use med_tracker::models::entities::{active_session_key, identity_api_key, identity_session};
use sea_orm::{
    QueryOrder,
    sea_query::{Expr, ExprTrait, Func, Query},
};

struct Prepared {
    cookie: String,
    operation: String,
    other: crate::scenario::Scenario,
    second_membership: household_memberships::Model,
    second_grant: person_access_grants::Model,
}

async fn prepare(app: &Application) -> Prepared {
    super::account_sessions::stored_credentials(app).await;
    let other = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    let mut memberships = Vec::new();
    for (household, person) in [
        (app.fixture.household.id, other.account.person_id),
        (other.household.id, app.fixture.adult.id),
    ] {
        memberships.push(
            household_memberships::ActiveModel {
                household_id: Set(household),
                person_id: Set(person),
                role: Set("member".into()),
                status: Set("active".into()),
                joined_at: Set(now),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap(),
        );
    }
    let mut grants = Vec::new();
    for (membership, person, level, relationship, revoked) in [
        (
            &memberships[0],
            other.account.person_id,
            "manage",
            "self",
            false,
        ),
        (
            &memberships[1],
            app.fixture.adult.id,
            "manage",
            "self",
            false,
        ),
        (
            &memberships[1],
            other.account.person_id,
            "view",
            "family_member",
            true,
        ),
    ] {
        grants.push(
            person_access_grants::ActiveModel {
                household_id: Set(membership.household_id),
                household_membership_id: Set(membership.id),
                person_id: Set(person),
                access_level: Set(level.into()),
                relationship_type: Set(relationship.into()),
                revoked_at: Set(revoked.then_some(now - chrono::Duration::days(1))),
                created_at: Set(now - chrono::Duration::days(2)),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap(),
        );
    }
    native_device_tokens::ActiveModel {
        account_id: Set(app.fixture.account.id),
        device_token: Set(uuid::Uuid::new_v4().to_string()),
        platform: Set("ios".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    push_subscriptions::ActiveModel {
        account_id: Set(app.fixture.account.id),
        endpoint: Set(format!("https://example.test/{}", uuid::Uuid::new_v4())),
        p256dh: Set("synthetic-key".into()),
        auth: Set("synthetic-auth".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    identity_api_key::ActiveModel {
        reference_id: Set(app.fixture.account.id),
        key_hash: Set(uuid::Uuid::new_v4().to_string()),
        config_id: Set("medtracker-personal".into()),
        enabled: Set(true),
        rate_limit_enabled: Set(false),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    active_session_key::ActiveModel {
        account_id: Set(app.fixture.account.id),
        session_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(now),
        last_use: Set(now),
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let cookie = sign_in(app, app.fixture.account.id, &app.fixture.account.email).await;
    assert_bearers(app, reqwest::StatusCode::OK).await;
    let (status, body) = operation_start(app, &cookie, json!({"action":"close_account"})).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    Prepared {
        cookie,
        operation: body["operation_id"].as_str().unwrap().to_owned(),
        other,
        second_membership: memberships[1].clone(),
        second_grant: grants[1].clone(),
    }
}

async fn confirm(app: &Application, cookie: &str, operation: &str) -> reqwest::Response {
    app.client
        .post(format!("{}/api/auth/security/password/confirm", app.origin))
        .header("cookie", cookie)
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"operation_id":operation,"password":"password"}))
        .send()
        .await
        .unwrap()
}

async fn verification_state(app: &Application, operation: &str) -> Value {
    let query = Query::select()
        .expr_as(Func::cust("to_jsonb").arg(Expr::col("v")), "record")
        .from_as("identity_verifications", "v")
        .and_where(Expr::col(("v", "identifier")).eq(format!("password-change:{operation}")))
        .order_by(("v", "id"), sea_orm::Order::Asc)
        .to_owned();
    let rows = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<Value>("", "record").unwrap())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        Value::Null
    } else {
        json!(rows)
    }
}

async fn state(app: &Application, prepared: &Prepared) -> Value {
    let account = accounts::Entity::find_by_id(app.fixture.account.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let user = med_tracker::models::_entities::users::Entity::find_by_id(app.fixture.user.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let memberships = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(app.fixture.adult.id))
        .order_by_asc(household_memberships::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let grants = person_access_grants::Entity::find()
        .filter(
            person_access_grants::Column::HouseholdId
                .is_in([app.fixture.household.id, prepared.other.household.id]),
        )
        .order_by_asc(person_access_grants::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let sessions = api_sessions::Entity::find()
        .filter(api_sessions::Column::AccountId.eq(account.id))
        .order_by_asc(api_sessions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let tokens = api_app_tokens::Entity::find()
        .filter(api_app_tokens::Column::AccountId.eq(account.id))
        .order_by_asc(api_app_tokens::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let oauth = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(account.id))
        .order_by_asc(oauth_grants::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let key_query = Query::select()
        .expr_as(Func::cust("to_jsonb").arg(Expr::col("k")), "record")
        .from_as("identity_api_keys", "k")
        .and_where(Expr::col(("k", "reference_id")).eq(account.id))
        .order_by(("k", "id"), sea_orm::Order::Asc)
        .to_owned();
    let keys = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&key_query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<Value>("", "record").unwrap())
        .collect::<Vec<_>>();
    let identity_sessions = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(account.id))
        .order_by_asc(identity_session::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let registry = active_session_key::Entity::find()
        .filter(active_session_key::Column::AccountId.eq(account.id))
        .order_by_asc(active_session_key::Column::SessionId)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let push = push_subscriptions::Entity::find()
        .filter(push_subscriptions::Column::AccountId.eq(account.id))
        .order_by_asc(push_subscriptions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let devices = native_device_tokens::Entity::find()
        .filter(native_device_tokens::Column::AccountId.eq(account.id))
        .order_by_asc(native_device_tokens::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    json!({
        "account":account.status, "user":user.active, "operations":verification_state(app, &prepared.operation).await,
        "memberships":memberships, "grants":grants, "api_sessions":sessions, "app_tokens":tokens,
        "oauth":oauth.iter().map(|row| json!([row.id,row.revoked_at])).collect::<Vec<_>>(),
        "keys":keys,
        "sessions":identity_sessions.iter().map(|row| json!([row.id,row.active])).collect::<Vec<_>>(),
        "registry":if registry.is_empty() { Value::Null } else { json!(registry.iter().map(|row| &row.session_id).collect::<Vec<_>>()) },
        "push":if push.is_empty() { Value::Null } else { json!(push) },
        "devices":if devices.is_empty() { Value::Null } else { json!(devices) },
    })
}

async fn clinical_records(app: &Application, prepared: &Prepared) -> Value {
    let people = people::Entity::find()
        .filter(people::Column::Id.is_in([
            app.fixture.adult.id,
            app.fixture.minor.id,
            app.fixture.dependent.id,
            prepared.other.account.person_id,
        ]))
        .order_by_asc(people::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let medications = medications::Entity::find()
        .filter(
            medications::Column::HouseholdId
                .is_in([app.fixture.household.id, prepared.other.household.id]),
        )
        .order_by_asc(medications::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    json!({"people":people,"medications":medications})
}

async fn assert_bearers(app: &Application, expected: reqwest::StatusCode) {
    for label in ["current", "app"] {
        let response = app
            .client
            .get(format!("{}/api/v1/auth/sessions", app.origin))
            .bearer_auth(super::account_sessions::credential_token(app, label))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

async fn notices_and_audits(app: &Application) -> (i64, i64) {
    let notices = super::native_password_reset::queued_mail(
        app,
        &app.fixture.account.email,
        "Account closed",
    )
    .await;
    let audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("AuthenticationToken"))
        .filter(versions::Column::ItemId.eq(app.fixture.account.id))
        .filter(versions::Column::Event.eq("auth_token/account/closed"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    for audit in &audits {
        assert_eq!(
            audit.audit_context["actor_account_id"]
                .to_string()
                .trim_matches('"'),
            app.fixture.account.id.to_string()
        );
    }
    (
        i64::try_from(notices.len()).unwrap(),
        i64::try_from(audits.len()).unwrap(),
    )
}

async fn rollback_case(deferred: bool, enqueue: bool) {
    let app = Application::new().await;
    let prepared = prepare(&app).await;
    let cookie = prepared.cookie.clone();
    let operation = prepared.operation.clone();
    let before = state(&app, &prepared).await;
    assert_eq!(notices_and_audits(&app).await, (0, 0));
    let fault = format!("closure_failure_{}", uuid::Uuid::new_v4().simple());
    let marker = format!("{fault}_reached");
    let account_id = app.fixture.account.id;
    let trigger = if enqueue {
        format!(
            "CREATE TRIGGER {fault} BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN (NEW.task_data->>'subject'='Account closed') EXECUTE FUNCTION {fault}()"
        )
    } else if deferred {
        format!(
            "CREATE CONSTRAINT TRIGGER {fault} AFTER UPDATE ON accounts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.id={account_id} AND NEW.status=3) EXECUTE FUNCTION {fault}()"
        )
    } else {
        format!(
            "CREATE TRIGGER {fault} AFTER UPDATE ON accounts FOR EACH ROW WHEN (NEW.id={account_id} AND NEW.status=3) EXECUTE FUNCTION {fault}()"
        )
    };
    let queued = if enqueue {
        ""
    } else {
        "IF NOT EXISTS(SELECT 1 FROM pg_loco_queue WHERE task_data->>'subject'='Account closed' AND task_data->>'to'=(SELECT email FROM accounts WHERE id=NEW.id)) THEN RAISE EXCEPTION 'closure notice was not reached'; END IF;"
    };
    // SeaORM has no trigger, PL/pgSQL function or GRANT builder. These schema
    // faults require whole-run exclusivity; all objects are removed before retry.
    app.fixture.admin.execute_unprepared(&format!(
        "CREATE SEQUENCE {marker};
        GRANT USAGE,SELECT ON SEQUENCE {marker} TO med_tracker_app;
        CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN {queued} PERFORM nextval('{marker}'); RAISE EXCEPTION 'synthetic closure failure' USING ERRCODE='23514'; END $$;
        {trigger}"
    )).await.unwrap();
    let response = confirm(&app, &cookie, &operation).await;
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(!response.headers().contains_key("set-cookie"));
    let query = Query::select()
        .column("is_called")
        .from(sea_orm::sea_query::Alias::new(marker.clone()))
        .to_owned();
    let reached = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    assert!(
        reached.try_get::<bool>("", "is_called").unwrap(),
        "injected failure must be reached"
    );
    assert_eq!(
        state(&app, &prepared).await,
        before,
        "all closure effects must roll back"
    );
    assert_eq!(notices_and_audits(&app).await, (0, 0));
    assert_bearers(&app, reqwest::StatusCode::OK).await;
    let table = if enqueue { "pg_loco_queue" } else { "accounts" };
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON {table}; DROP FUNCTION {fault}(); DROP SEQUENCE {marker}"
        ))
        .await
        .unwrap();
    assert_eq!(
        confirm(&app, &cookie, &operation).await.status(),
        reqwest::StatusCode::OK
    );
    assert_eq!(notices_and_audits(&app).await, (1, 1));
    assert_bearers(&app, reqwest::StatusCode::UNAUTHORIZED).await;
    prepared.other.close().await;
    app.close().await;
}

#[tokio::test]
async fn account_closure_mail_late_failure_rolls_back() {
    rollback_case(false, false).await;
}

#[tokio::test]
async fn account_closure_mail_commit_failure_rolls_back() {
    rollback_case(true, false).await;
}

#[tokio::test]
async fn account_closure_mail_enqueue_failure_rolls_back() {
    rollback_case(false, true).await;
}

#[tokio::test]
async fn account_closure_mail_success_and_replay() {
    let app = Application::new().await;
    let prepared = prepare(&app).await;
    let cookie = prepared.cookie.clone();
    let operation = prepared.operation.clone();
    let before = state(&app, &prepared).await;
    assert_eq!(before["grants"].as_array().unwrap().len(), 4);
    let records = clinical_records(&app, &prepared).await;
    let response = confirm(&app, &cookie, &operation).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(response.headers().contains_key("set-cookie"));
    assert_eq!(notices_and_audits(&app).await, (1, 1));
    let key_notices = super::native_password_reset::queued_mail(
        &app,
        &app.fixture.account.email,
        "API key revoked",
    )
    .await
    .len();
    assert_eq!(
        key_notices, 0,
        "account closure sends one account notice rather than per-key revocation notices"
    );
    let after = state(&app, &prepared).await;
    assert_bearers(&app, reqwest::StatusCode::UNAUTHORIZED).await;
    assert_eq!(after["account"], 3);
    assert_eq!(after["user"], false);
    for membership in after["memberships"].as_array().unwrap() {
        assert_eq!(membership["status"], "revoked");
        assert!(!Value::is_null(&membership["revoked_at"]));
    }
    let grants = after["grants"].as_array().unwrap();
    assert_eq!(grants.len(), 4);
    for prior in before["grants"].as_array().unwrap() {
        let grant = grants
            .iter()
            .find(|grant| grant["id"] == prior["id"])
            .unwrap();
        if prior["id"] == app.fixture.grant.id || prior["id"] == prepared.second_grant.id {
            assert!(Value::is_null(&prior["revoked_at"]));
            assert!(
                !Value::is_null(&grant["revoked_at"]),
                "active grants in both households must be revoked"
            );
        } else {
            assert_eq!(
                grant, prior,
                "another member's grant and an already revoked grant must be unchanged"
            );
        }
    }
    for kind in ["api_sessions", "app_tokens"] {
        for credential in after[kind].as_array().unwrap() {
            assert!(!Value::is_null(&credential["revoked_at"]));
        }
    }
    for credential in after["oauth"].as_array().unwrap() {
        assert!(!Value::is_null(&credential[1]));
    }
    assert_eq!(after["keys"][0]["enabled"], false);
    for session in after["sessions"].as_array().unwrap() {
        assert_eq!(session[1], false);
    }
    for kind in ["registry", "push", "devices"] {
        assert!(Value::is_null(&after[kind]), "{kind} must be removed");
    }
    let preserved = clinical_records(&app, &prepared).await;
    for kind in ["people", "medications"] {
        assert_eq!(records[kind], preserved[kind]);
    }
    assert!(
        !confirm(&app, &cookie, &operation)
            .await
            .status()
            .is_success()
    );
    assert_eq!(notices_and_audits(&app).await, (1, 1));
    prepared.other.close().await;
    app.close().await;
}

#[tokio::test]
async fn account_closure_mail_later_household_owner_denial_rolls_back() {
    let app = Application::new().await;
    let prepared = prepare(&app).await;
    let cookie = prepared.cookie.clone();
    let operation = prepared.operation.clone();
    let mut member = prepared.other.membership.clone().into_active_model();
    member.role = Set("member".into());
    member.update(&app.fixture.admin).await.unwrap();
    let mut owner = prepared.second_membership.clone().into_active_model();
    owner.role = Set("owner".into());
    owner.update(&app.fixture.admin).await.unwrap();
    let fault = format!("closure_first_household_{}", uuid::Uuid::new_v4().simple());
    let marker = format!("{fault}_reached");
    let grant_id = app.fixture.grant.id;
    // SeaORM has no trigger, PL/pgSQL function or GRANT builder. Whole-run
    // exclusivity isolates this ordering probe; all objects are removed below.
    app.fixture.admin.execute_unprepared(&format!(
        "CREATE SEQUENCE {marker};
         GRANT USAGE,SELECT ON SEQUENCE {marker} TO med_tracker_app;
         CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('{marker}'); RETURN NEW; END $$;
         CREATE TRIGGER {fault} AFTER UPDATE ON person_access_grants FOR EACH ROW WHEN (NEW.id={grant_id} AND NEW.revoked_at IS NOT NULL) EXECUTE FUNCTION {fault}()"
    )).await.unwrap();
    let before = state(&app, &prepared).await;
    let response = confirm(&app, &cookie, &operation).await;
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(!response.headers().contains_key("set-cookie"));
    let query = Query::select()
        .column("is_called")
        .from(sea_orm::sea_query::Alias::new(marker.clone()))
        .to_owned();
    let reached = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    assert!(
        reached.try_get::<bool>("", "is_called").unwrap(),
        "first household grant must have been changed before second household denial"
    );
    assert_eq!(state(&app, &prepared).await, before);
    assert_eq!(notices_and_audits(&app).await, (0, 0));
    assert_bearers(&app, reqwest::StatusCode::OK).await;
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {fault} ON person_access_grants; DROP FUNCTION {fault}(); DROP SEQUENCE {marker}")).await.unwrap();
    prepared.other.close().await;
    app.close().await;
}
