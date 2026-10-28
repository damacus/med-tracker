use super::*;
use chrono::{Duration, Utc};
use med_tracker::models::_entities::{account_lockouts, accounts};
use med_tracker::models::entities::identity_session;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, sea_query::Expr};
use sea_orm::{
    QueryOrder, QuerySelect,
    sea_query::{ExprTrait, Func, Query},
};
use serde_json::{Value, json};

async fn factor_field<T: sea_orm::TryGetable>(app: &Application, table: &str, column: &str) -> T {
    let query = Query::select()
        .column(sea_orm::sea_query::Alias::new(column))
        .from(sea_orm::sea_query::Alias::new(table))
        .and_where(
            Expr::col(if table == "accounts" {
                "id"
            } else {
                "account_id"
            })
            .eq(app.fixture.account.id),
        )
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", column)
        .unwrap()
}

async fn factor_count(app: &Application) -> i64 {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("identity_two_factors")
        .and_where(Expr::col("account_id").eq(app.fixture.account.id))
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

async fn lockout(app: &Application, locked: bool) {
    if locked {
        account_lockouts::ActiveModel {
            account_id: Set(app.fixture.account.id),
            key: Set(uuid::Uuid::new_v4().to_string()),
            deadline: Set(Utc::now().naive_utc() + Duration::hours(1)),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    } else {
        account_lockouts::Entity::delete_by_id(app.fixture.account.id)
            .exec(&app.fixture.admin)
            .await
            .unwrap();
    }
}

fn cookies(response: &reqwest::Response) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ")
}

fn code(uri: &str) -> String {
    let uri = url::Url::parse(uri).unwrap();
    let secret = uri
        .query_pairs()
        .find(|(key, _)| key == "secret")
        .unwrap()
        .1
        .into_owned();
    let secret = String::from_utf8(
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret).unwrap(),
    )
    .unwrap();
    better_auth::plugins::TwoFactorPlugin::new()
        .generate_totp(&secret)
        .unwrap()
}

async fn login(app: &Application) -> reqwest::Response {
    app.client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":app.fixture.account.email,"password":"password"}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn public_authenticator_setup_replaces_pending_factor_and_finishes_once() {
    let app = Application::new().await;
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let selected = identity_session::Entity::update_many()
        .col_expr(
            identity_session::Column::ActiveHouseholdId,
            Expr::value(app.fixture.household.id),
        )
        .filter(identity_session::Column::UserId.eq(app.fixture.account.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(selected.rows_affected, 1);
    let mut pending = Vec::new();
    for _ in 0..2 {
        let response = app
            .client
            .post(format!("{}/api/auth/security/totp/start", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&json!({"password":"password"}))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        assert_eq!(body["backupCodes"].as_array().unwrap().len(), 10);
        assert!(body["totpURI"].as_str().unwrap().contains("MedTracker"));
        pending.push(body);
    }
    let current_code = code(pending[1]["totpURI"].as_str().unwrap());
    let rejected = app
        .client
        .post(format!("{}/api/auth/security/totp/finish", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"code":"invalid"}))
        .send()
        .await
        .unwrap();
    assert!(rejected.status().is_client_error());
    assert!(!factor_field::<bool>(&app, "accounts", "two_factor_enabled").await);
    let enrolled = app
        .client
        .post(format!("{}/api/auth/security/totp/finish", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"code":current_code}))
        .send()
        .await
        .unwrap();
    let status = enrolled.status();
    let enrolled_cookie = cookies(&enrolled);
    let body: Value = enrolled.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    assert!(factor_field::<bool>(&app, "accounts", "two_factor_enabled").await);
    assert!(factor_field::<bool>(&app, "identity_two_factors", "verified").await);
    let rotated = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(app.fixture.account.id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(rotated.len(), 1);
    assert_eq!(
        rotated[0].active_household_id,
        Some(app.fixture.household.id)
    );
    let current = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(current.status().as_u16(), 200);
    let current: Value = current.json().await.unwrap();
    assert_eq!(
        current["session"]["active_household_id"],
        app.fixture.household.id.to_string()
    );
    assert!(
        current["session"]
            .get("activeOrganizationId")
            .is_none_or(Value::is_null)
    );
    let home = app
        .client
        .get(format!("{}/", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(home.status().as_u16(), 200);
    assert_eq!(
        home.url().path(),
        format!("/households/{}/dashboard", app.fixture.household.slug)
    );
    let old = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(Value::is_null(&old.json::<Value>().await.unwrap()));
    let first_totp = login(&app).await;
    let totp_cookie = cookies(&first_totp);
    let challenge: Value = first_totp.json().await.unwrap();
    assert_eq!(challenge["twoFactorRedirect"], true);
    let totp_login = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", totp_cookie)
        .json(&json!({"code":current_code}))
        .send()
        .await
        .unwrap();
    let status = totp_login.status();
    let totp_session = cookies(&totp_login);
    let body: Value = totp_login.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let live = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", totp_session)
        .send()
        .await
        .unwrap();
    assert_eq!(live.status().as_u16(), 200);
    assert!(live.json::<Value>().await.unwrap()["session"].is_object());
    let first = login(&app).await;
    assert_eq!(first.status().as_u16(), 200);
    let pending_cookie = cookies(&first);
    let body: Value = first.json().await.unwrap();
    assert_eq!(body["twoFactorRedirect"], true, "{body}");
    let backup = pending[1]["backupCodes"][0].as_str().unwrap();
    let proved = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &pending_cookie)
        .json(&json!({"backup":true,"code":backup}))
        .send()
        .await
        .unwrap();
    let status = proved.status();
    let proof_cookie = cookies(&proved);
    let body: Value = proved.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let authenticated = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(authenticated.status().as_u16(), 200);
    assert_eq!(
        authenticated.json::<Value>().await.unwrap()["user"]["twoFactorEnabled"],
        true
    );
    let replay_challenge = login(&app).await;
    let replay_cookie = cookies(&replay_challenge);
    let replay = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", replay_cookie)
        .json(&json!({"backup":true,"code":backup}))
        .send()
        .await
        .unwrap();
    assert!(replay.status().is_client_error());
    let before = factor_field::<String>(&app, "identity_two_factors", "backup_codes").await;
    lockout(&app, true).await;
    for path in ["/two-factor/generate-backup-codes", "/two-factor/disable"] {
        let denied = app
            .client
            .post(format!("{}/api/auth{path}", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &enrolled_cookie)
            .json(&json!({"password":"password"}))
            .send()
            .await
            .unwrap();
        assert!(denied.status().is_client_error());
    }
    let after = factor_field::<String>(&app, "identity_two_factors", "backup_codes").await;
    assert_eq!(before, after);
    lockout(&app, false).await;
    let disabled = app
        .client
        .post(format!(
            "{}/api/auth/security/totp/disable/confirm",
            app.origin
        ))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &enrolled_cookie)
        .json(&json!({"password":"password"}))
        .send()
        .await
        .unwrap();
    let status = disabled.status();
    let disabled_cookie = cookies(&disabled);
    let body: Value = disabled.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    assert!(!factor_field::<bool>(&app, "accounts", "two_factor_enabled").await);
    assert_eq!(factor_count(&app).await, 0);
    let current = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", disabled_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(
        current.json::<Value>().await.unwrap()["user"]["twoFactorEnabled"],
        false
    );
    let old = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap();
    assert!(Value::is_null(&old.json::<Value>().await.unwrap()));
    app.close().await;
}

#[tokio::test]
async fn zero_quota_personal_key_public_replay_is_denied_with_redacted_errors() {
    let app = Application::new().await;
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let issued = service
        .api_keys()
        .unwrap()
        .create(
            &app.fixture.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Exhausted synthetic key".into()),
                expires_in: Some(Duration::days(1).num_seconds() as f64),
                remaining: Some(0.0),
                metadata: Some(json!({"households":[app.fixture.household.id]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let secret = issued.key;
    let mut first = None;
    for _ in 0..2 {
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{}/profile",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        let mut body: Value = response.json().await.unwrap();
        assert!(!body.to_string().contains(&secret));
        let request_id = body["error"]["request_id"].as_str().unwrap().to_owned();
        assert!(!request_id.is_empty());
        if let Some(first) = &first {
            assert_ne!(&request_id, first);
        }
        first = Some(request_id);
        body["error"].as_object_mut().unwrap().remove("request_id");
        assert_eq!(
            body,
            json!({"error":{"code":"unauthorized","message":"Authentication required"}})
        );
    }
    assert!(
        issued
            .api_key
            .expires_at
            .typed()
            .unwrap()
            .as_ref()
            .unwrap()
            .to_datetime()
            .unwrap()
            .unwrap()
            > Utc::now()
    );
    app.close().await;
}

#[tokio::test]
async fn native_personal_key_scoped_reads_writes_and_live_membership_denial() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let issued = service
        .api_keys()
        .unwrap()
        .create(
            &app.fixture.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Scoped synthetic key".into()),
                expires_in: Some(Duration::days(1).num_seconds() as f64),
                metadata: Some(json!({"households":[app.fixture.household.id]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into(), "care:write".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let secret = issued.key;
    let read = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/profile",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    let status = read.status();
    let body: Value = read.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    assert!(!body.to_string().contains(&secret));
    let write = app
        .client
        .patch(format!(
            "{}/api/v1/households/{}/profile",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&secret)
        .header("idempotency-key", uuid::Uuid::new_v4().to_string())
        .json(&json!({"profile":{"time_zone":"Europe/London"}}))
        .send()
        .await
        .unwrap();
    let status = write.status();
    let body: Value = write.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let account = accounts::Entity::find_by_id(app.fixture.account.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stored = account.preferences["time_zone"].as_str().unwrap();
    assert_eq!(stored, "Europe/London");
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/profile",
            app.origin, other.household.id
        ))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 403);
    let mut member = app.fixture.membership.clone().into_active_model();
    member.status = Set("revoked".into());
    member.revoked_at = Set(Some(Utc::now().naive_utc()));
    let revoked = member.update(&app.fixture.admin).await.unwrap();
    assert_eq!(revoked.id, app.fixture.membership.id);
    let denied = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/profile",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    other.close().await;
    app.close().await;
}

async fn key_lifecycle_state(app: &Application) -> Value {
    let query = Query::select()
        .expr_as(Func::cust("to_jsonb").arg(Expr::col("k")), "record")
        .from_as("identity_api_keys", "k")
        .and_where(Expr::col(("k", "reference_id")).eq(app.fixture.account.id))
        .order_by(("k", "id"), sea_orm::Order::Asc)
        .to_owned();
    let keys = app
        .fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get::<Value>("", "record").unwrap())
        .collect::<Vec<_>>();
    let audits = versions::Entity::find()
        .filter(versions::Column::ItemId.eq(app.fixture.account.id))
        .filter(versions::Column::Event.starts_with("native_auth/api_key/"))
        .order_by_asc(versions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let mut notices = super::native_password_reset::queued_mail(
        app,
        &app.fixture.account.email,
        "API key created",
    )
    .await;
    notices.extend(
        super::native_password_reset::queued_mail(
            app,
            &app.fixture.account.email,
            "API key revoked",
        )
        .await,
    );
    json!({
        "keys": if keys.is_empty() {Value::Null} else {json!(keys)},
        "audits": if audits.is_empty() {Value::Null} else {json!(audits)},
        "notices": if notices.is_empty() {Value::Null} else {json!(notices)}
    })
}

async fn create_lifecycle_key(app: &Application, cookie: &str) -> reqwest::Response {
    app.client.post(format!("{}/api/auth/security/api-key/create",app.origin))
        .header("origin",app.context.config.server.full_url()).header("cookie",cookie)
        .json(&json!({"name":"Atomic lifecycle key","households":[app.fixture.household.id],"permissions":["care:read"],"expires_days":30})).send().await.unwrap()
}

#[tokio::test]
async fn native_personal_key_lifecycle_rolls_back_notice_and_audit() {
    let app = Application::new().await;
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let before = key_lifecycle_state(&app).await;
    let fault = format!("reject_key_notice_{}", uuid::Uuid::new_v4().simple());
    let marker = format!("{fault}_reached");
    // SeaORM has no trigger, PL/pgSQL function or GRANT/REVOKE builder. These
    // faults require whole-run exclusivity; schema and grants are restored below.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE {marker}; GRANT USAGE,SELECT ON SEQUENCE {marker} TO med_tracker_app; CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('public.{marker}'); RAISE EXCEPTION 'synthetic key notice failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject' IN('API key created','API key revoked')) EXECUTE FUNCTION {fault}()")).await.unwrap();
    let rejected = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(rejected.status().as_u16(), 500);
    let probe = Query::select()
        .column("is_called")
        .from(sea_orm::sea_query::Alias::new(marker.clone()))
        .to_owned();
    assert!(
        app.fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&probe))
            .await
            .unwrap()
            .unwrap()
            .try_get::<bool>("", "is_called")
            .unwrap()
    );
    assert_eq!(
        key_lifecycle_state(&app).await,
        before,
        "key, notice and audit must all roll back"
    );
    app.fixture
        .admin
        .execute_unprepared(&format!("DROP TRIGGER {fault} ON pg_loco_queue"))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let audit_rejected = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(audit_rejected.status().as_u16(), 500);
    assert_eq!(
        key_lifecycle_state(&app).await,
        before,
        "failed key audit must roll back native insertion"
    );
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let issued = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(issued.status().as_u16(), 200);
    let body: Value = issued.json().await.unwrap();
    let secret = body["apiKey"].as_str().unwrap();
    assert!(secret.starts_with("medtracker_"));
    let created = key_lifecycle_state(&app).await;
    assert_eq!(created["keys"].as_array().unwrap().len(), 1);
    assert_eq!(created["audits"].as_array().unwrap().len(), 1);
    assert_eq!(created["audits"][0]["event"], "native_auth/api_key/created");
    assert_eq!(created["notices"].as_array().unwrap().len(), 1);
    assert!(!serde_json::to_string(&created).unwrap().contains(secret));
    assert_eq!(
        created["keys"][0]["metadata"],
        serde_json::to_string(&json!({"households":[app.fixture.household.id]})).unwrap()
    );
    let key_id = created["keys"][0]["id"].as_i64().unwrap().to_string();
    app.fixture.admin.execute_unprepared(&format!("CREATE TRIGGER {fault} BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject'='API key revoked') EXECUTE FUNCTION {fault}()")).await.unwrap();
    let revoke = || {
        app.client
            .post(format!("{}/api/auth/security/api-key/revoke", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&json!({"key_id":key_id}))
            .send()
    };
    assert_eq!(revoke().await.unwrap().status().as_u16(), 500);
    assert_eq!(
        key_lifecycle_state(&app).await,
        created,
        "failed revocation notice must preserve enabled key and audit"
    );
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON pg_loco_queue; DROP FUNCTION {fault}(); DROP SEQUENCE {marker}"
        ))
        .await
        .unwrap();
    assert_eq!(revoke().await.unwrap().status().as_u16(), 200);
    let revoked = key_lifecycle_state(&app).await;
    assert_eq!(revoked["keys"][0]["enabled"], false);
    assert_eq!(revoked["audits"].as_array().unwrap().len(), 2);
    assert_eq!(revoked["notices"].as_array().unwrap().len(), 2);
    assert_eq!(revoked["audits"][1]["event"], "native_auth/api_key/updated");
    assert_eq!(revoke().await.unwrap().status().as_u16(), 200);
    assert_eq!(
        key_lifecycle_state(&app).await,
        revoked,
        "idempotent revoke must not repeat lifecycle side effects"
    );
    app.close().await;
}

#[tokio::test]
async fn native_personal_key_create_retains_authoritative_admission_and_typed_scopes() {
    let app = Application::new().await;
    let other = app.fixture.foreign_household().await;
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let input = json!({"name":"Admission key","households":[app.fixture.household.id],"permissions":["care:read"],"expires_days":30});
    let before = key_lifecycle_state(&app).await;
    let anonymous = app
        .client
        .post(format!("{}/api/auth/security/api-key/create", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status().as_u16(), 401);
    let csrf = app
        .client
        .post(format!("{}/api/auth/security/api-key/create", app.origin))
        .header("origin", "https://foreign.example.test")
        .header("cookie", &cookie)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(csrf.status().as_u16(), 403);
    for injected in [
        json!({"userId":other.account.id.to_string()}),
        json!({"configId":"other"}),
        json!({"metadata":{"households":[other.household.id]}}),
        json!({"remaining":100}),
    ] {
        let mut body = input.clone();
        body.as_object_mut()
            .unwrap()
            .extend(injected.as_object().unwrap().clone());
        let denied = app
            .client
            .post(format!("{}/api/auth/security/api-key/create", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(denied.status().as_u16(), 400);
    }
    for (field, value) in [
        ("households", json!([other.household.id])),
        ("permissions", json!(["unsupported:write"])),
        ("expires_days", json!(366)),
    ] {
        let mut body = input.clone();
        body[field] = value;
        let denied = app
            .client
            .post(format!("{}/api/auth/security/api-key/create", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(denied.status().is_client_error());
    }
    let bypass = app
        .client
        .post(format!("{}/api/auth/api-key/create", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"name":"Bypass key"}))
        .send()
        .await
        .unwrap();
    assert_eq!(bypass.status().as_u16(), 403);
    for kind in 0..4 {
        set_admission(&app, kind, false).await;
        let denied = app
            .client
            .post(format!("{}/api/auth/security/api-key/create", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&input)
            .send()
            .await
            .unwrap();
        assert!(denied.status().is_client_error(), "{}", denied.status());
        set_admission(&app, kind, true).await;
    }
    assert_eq!(key_lifecycle_state(&app).await, before);
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let allowed = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(allowed.status().as_u16(), 200);
    let body: Value = allowed.json().await.unwrap();
    let secret = body["apiKey"].as_str().unwrap();
    let before_use = key_lifecycle_state(&app).await;
    let clinical = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/profile",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(secret)
        .send()
        .await
        .unwrap();
    assert_eq!(clinical.status().as_u16(), 200);
    let after_use = key_lifecycle_state(&app).await;
    assert_eq!(after_use["audits"], before_use["audits"]);
    assert_eq!(after_use["notices"], before_use["notices"]);
    assert!(
        !serde_json::to_string(&after_use["audits"])
            .unwrap()
            .contains(secret)
    );
    other.close().await;
    app.close().await;
}

#[tokio::test]
async fn native_personal_key_create_waits_for_account_closure_and_rechecks_status() {
    use sea_orm::TransactionTrait;
    let app = Application::new().await;
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let transaction = app.fixture.admin.begin().await.unwrap();
    accounts::Entity::find_by_id(app.fixture.account.id)
        .lock_exclusive()
        .one(&transaction)
        .await
        .unwrap();
    let pid_query = Query::select()
        .expr_as(Func::cust("pg_backend_pid"), "pid")
        .to_owned();
    let blocker: i32 = transaction
        .query_one_raw(transaction.get_database_backend().build(&pid_query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let client = app.client.clone();
    let origin = app.origin.clone();
    let public = app.context.config.server.full_url();
    let household_id = app.fixture.household.id;
    let create = tokio::spawn(async move {
        client.post(format!("{origin}/api/auth/security/api-key/create")).header("origin",public).header("cookie",cookie).json(&json!({"name":"Concurrent close key","households":[household_id],"permissions":["care:read"],"expires_days":30})).send().await.unwrap()
    });
    let waited = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let query = Query::select()
                .expr_as(
                    Func::cust("to_jsonb")
                        .arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
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
            if rows.iter().any(|row| {
                let pids: Vec<i32> =
                    serde_json::from_value(row.try_get::<Value>("", "blockers").unwrap()).unwrap();
                pids.contains(&blocker)
            }) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    let mut account = app.fixture.account.clone().into_active_model();
    account.status = Set(3);
    account.update(&transaction).await.unwrap();
    transaction.commit().await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), create)
        .await
        .unwrap()
        .unwrap();
    assert!(
        waited.is_ok(),
        "key creation must serialise with account closure"
    );
    assert_eq!(response.status().as_u16(), 401);
    let state = key_lifecycle_state(&app).await;
    assert!(Value::is_null(&state["keys"]));
    assert!(Value::is_null(&state["notices"]));
    assert!(Value::is_null(&state["audits"]));
    app.close().await;
}

async fn set_admission(app: &Application, kind: u8, allowed: bool) {
    match kind {
        0 => {
            identity_session::Entity::update_many()
                .col_expr(
                    identity_session::Column::ClinicalSessionPurpose,
                    Expr::value(if allowed {
                        "authenticated"
                    } else {
                        "enrolment"
                    }),
                )
                .filter(identity_session::Column::UserId.eq(app.fixture.account.id))
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
        1 => {
            let mut account = app.fixture.account.clone().into_active_model();
            account.status = Set(if allowed { 2 } else { 3 });
            account.update(&app.fixture.admin).await.unwrap();
        }
        2 => lockout(app, !allowed).await,
        _ => {
            identity_session::Entity::update_many()
                .col_expr(
                    identity_session::Column::ExpiresAt,
                    Expr::value(if allowed {
                        Utc::now() + Duration::days(1)
                    } else {
                        Utc::now() - Duration::minutes(1)
                    }),
                )
                .filter(identity_session::Column::UserId.eq(app.fixture.account.id))
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
}
