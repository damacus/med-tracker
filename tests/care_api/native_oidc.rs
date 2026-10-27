use super::*;
use axum::{
    Json, Router,
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use base64::Engine as _;
use josekit::{jwk::Jwk, jws::JwsHeader};
use med_tracker::models::_entities::{
    accounts, app_settings, households, people, platform_admins, support_access_sessions, users,
};
use sea_orm::sea_query::{Expr, ExprTrait, Query};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone)]
struct Grant {
    nonce: String,
    challenge: String,
    redirect: String,
    subject: String,
    email: String,
}
struct IssuerState {
    origin: String,
    unique: String,
    key: Jwk,
    grants: Mutex<HashMap<String, Grant>>,
    access: Mutex<HashMap<String, Grant>>,
    token_calls: AtomicUsize,
}
struct Issuer {
    state: Arc<IssuerState>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Issuer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn discovery(State(state): State<Arc<IssuerState>>) -> Json<Value> {
    Json(
        json!({"issuer":state.origin,"authorization_endpoint":format!("{}/authorize",state.origin),"token_endpoint":format!("{}/token",state.origin),"userinfo_endpoint":format!("{}/userinfo",state.origin),"jwks_uri":format!("{}/jwks",state.origin),"response_types_supported":["code"],"subject_types_supported":["public"],"id_token_signing_alg_values_supported":["RS256"],"token_endpoint_auth_methods_supported":["client_secret_post","client_secret_basic"]}),
    )
}
async fn jwks(State(state): State<Arc<IssuerState>>) -> Json<Value> {
    let mut key = state.key.to_public_key().unwrap();
    key.set_key_id("native-mock-key");
    Json(json!({"keys":[key]}))
}
async fn token(
    State(state): State<Arc<IssuerState>>,
    Form(input): Form<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    state.token_calls.fetch_add(1, Ordering::SeqCst);
    let grant = input
        .get("code")
        .and_then(|code| state.grants.lock().unwrap().remove(code));
    let Some(grant) = grant else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_grant"})),
        );
    };
    let verifier = input.get("code_verifier").cloned().unwrap_or_default();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    if challenge != grant.challenge || input.get("redirect_uri") != Some(&grant.redirect) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_grant"})),
        );
    }
    let now = chrono::Utc::now().timestamp();
    let nonce = if grant.subject == "wrong-nonce" {
        "invalid-native-nonce"
    } else {
        &grant.nonce
    };
    let claims = json!({"iss":state.origin,"aud":"native-test-client","sub":grant.subject,"nonce":nonce,"email":grant.email,"email_verified":true,"name":"Native provider owner","iat":now,"exp":now+600});
    let mut header = JwsHeader::new();
    header.set_algorithm("RS256");
    header.set_key_id("native-mock-key");
    let id_token = josekit::jws::serialize_compact(
        claims.to_string().as_bytes(),
        &header,
        &josekit::jws::RS256.signer_from_jwk(&state.key).unwrap(),
    )
    .unwrap();
    let access = uuid::Uuid::new_v4().to_string();
    state.access.lock().unwrap().insert(access.clone(), grant);
    (
        StatusCode::OK,
        Json(
            json!({"access_token":access,"token_type":"Bearer","expires_in":600,"id_token":id_token}),
        ),
    )
}
async fn userinfo(
    State(state): State<Arc<IssuerState>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    let access = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let grant = access.and_then(|access| state.access.lock().unwrap().get(access).cloned());
    let Some(grant) = grant else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"invalid_token"})),
        );
    };
    (
        StatusCode::OK,
        Json(
            json!({"id":grant.subject,"sub":grant.subject,"email":grant.email,"email_verified":true,"name":"Native provider owner"}),
        ),
    )
}
impl Issuer {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://localhost:{}", listener.local_addr().unwrap().port());
        let mut key = Jwk::generate_rsa_key(2048).unwrap();
        key.set_key_id("native-mock-key");
        let state = Arc::new(IssuerState {
            origin,
            unique: uuid::Uuid::new_v4().to_string(),
            key,
            grants: Mutex::new(HashMap::new()),
            access: Mutex::new(HashMap::new()),
            token_calls: AtomicUsize::new(0),
        });
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .route("/token", post(token))
            .route("/userinfo", get(userinfo))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self { state, task }
    }
    fn code(&self, authorize: &url::Url, subject: &str, email: &str) -> String {
        let query = authorize
            .query_pairs()
            .into_owned()
            .collect::<HashMap<_, _>>();
        assert_eq!(query["code_challenge_method"], "S256");
        assert!(!query["nonce"].is_empty());
        assert!(!query["state"].is_empty());
        assert!(!query.contains_key("client_secret"));
        assert_eq!(
            url::Url::parse(&query["redirect_uri"]).unwrap().path(),
            "/auth/zitadel/callback"
        );
        let code = uuid::Uuid::new_v4().to_string();
        self.state.grants.lock().unwrap().insert(
            code.clone(),
            Grant {
                nonce: query["nonce"].clone(),
                challenge: query["code_challenge"].clone(),
                redirect: query["redirect_uri"].clone(),
                subject: if subject == "wrong-nonce" {
                    subject.into()
                } else {
                    format!("{}-{subject}", self.state.unique)
                },
                email: email.into(),
            },
        );
        code
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
fn session_cookie(response: &reqwest::Response) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|value| value.starts_with("better-auth.session_token="))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .into()
}
async fn begin(app: &Application, mode: &str, authenticated: &str) -> (url::Url, String) {
    begin_operation(app, mode, authenticated, None).await
}
async fn begin_operation(
    app: &Application,
    mode: &str,
    authenticated: &str,
    operation: Option<&str>,
) -> (url::Url, String) {
    let page_path = match mode {
        "register" => "/auth/zitadel/register",
        "link" => "/account/security/provider/link",
        "operation" => "/account/security",
        _ => "/login",
    };
    let page = app
        .client
        .get(format!("{}{page_path}", app.origin))
        .header("cookie", authenticated)
        .send()
        .await
        .unwrap();
    assert_eq!(page.status().as_u16(), 200);
    let cookie = format!("{}; {}", authenticated, cookies(&page));
    let text = page.text().await.unwrap();
    let csrf = text
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let mut input = vec![("authenticity_token", csrf)];
    if mode == "register" {
        input.extend([
            ("name", "Native provider owner"),
            ("date_of_birth", "1990-01-01"),
        ]);
    }
    if mode == "link" {
        input.push(("link", "true"));
    }
    if let Some(operation) = operation {
        input.push(("operation_id", operation));
    }
    let path = if mode == "register" {
        "/auth/zitadel/register"
    } else {
        "/auth/zitadel"
    };
    let response = super::platform::post_form(app, &cookie, path, &input).await;
    let status = response.status();
    let location = response.headers().get("location").cloned();
    let native_cookie = cookies(&response);
    let text = response.text().await.unwrap();
    assert_eq!(status.as_u16(), 303, "{text}");
    (
        url::Url::parse(location.unwrap().to_str().unwrap())
            .expect("Native OAuth must return an absolute authorization URL"),
        format!("{authenticated}; {native_cookie}"),
    )
}
async fn callback(
    app: &Application,
    authorize: &url::Url,
    code: &str,
    cookie: &str,
    tampered: bool,
) -> reqwest::Response {
    let state = authorize
        .query_pairs()
        .find(|(name, _)| name == "state")
        .unwrap()
        .1
        .into_owned();
    let mut url = url::Url::parse(&app.origin).unwrap();
    url.set_path("/auth/zitadel/callback");
    url.query_pairs_mut()
        .append_pair("code", code)
        .append_pair("state", if tampered { "invalid-state" } else { &state });
    app.client
        .get(url)
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
}
async fn graph(app: &Application) -> Value {
    let mut state = serde_json::Map::new();
    for (name, table) in [
        ("accounts", "accounts"),
        ("people", "people"),
        ("users", "users"),
        ("households", "households"),
        ("memberships", "household_memberships"),
        ("credentials", "auth_credentials"),
        ("sessions", "identity_sessions"),
        ("queue", "pg_loco_queue"),
    ] {
        let query = Query::select()
            .expr_as(Expr::col("id").count(), "count")
            .from(table)
            .to_owned();
        let count: i64 = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "count")
            .unwrap();
        state.insert(name.into(), json!(count));
    }
    Value::Object(state)
}

async fn providers(app: &Application, account: i64) -> Vec<String> {
    let query = Query::select()
        .column("id")
        .from("auth_credentials")
        .and_where(Expr::col("user_id").eq(account))
        .and_where(Expr::col("provider_id").eq("zitadel"))
        .to_owned();
    app.fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get("", "id").unwrap())
        .collect()
}

async fn verification(app: &Application, identifier: &str) -> Option<String> {
    let query = Query::select()
        .column("value")
        .from("identity_verifications")
        .and_where(Expr::col("identifier").eq(identifier))
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .map(|row| row.try_get("", "value").unwrap())
}

#[tokio::test]
async fn native_oidc_registration_signin_link_unlink_and_state_replay_use_mounted_provider() {
    let issuer = Issuer::new().await;
    let settings = issuer.state.origin.clone();
    let mut app=Application::new_configured(Some("synthetic-dose-occurrence-signing-key"),move |value|value["zitadel"]=json!({"issuer":settings,"client_id":"native-test-client","client_secret":"synthetic-native-client-secret"})).await;
    app.client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let registration = super::signup::open_registration(&app).await;
    let owner_email = format!("new-oidc-owner-{}@example.test", uuid::Uuid::new_v4());
    let before = graph(&app).await;
    let (authorize, state_cookie) = begin(&app, "register", "").await;
    let code = issuer.code(
        &authorize,
        "tampered-subject",
        &format!("tampered-oidc-{}@example.test", uuid::Uuid::new_v4()),
    );
    let denied = callback(&app, &authorize, &code, &state_cookie, true).await;
    assert!(denied.status().is_redirection());
    assert_eq!(issuer.state.token_calls.load(Ordering::SeqCst), 0);
    assert_eq!(graph(&app).await, before);
    let (authorize, state_cookie) = begin(&app, "register", "").await;
    let code = issuer.code(
        &authorize,
        "wrong-nonce",
        &format!("wrong-nonce-oidc-{}@example.test", uuid::Uuid::new_v4()),
    );
    let denied = callback(&app, &authorize, &code, &state_cookie, false).await;
    assert!(denied.status().is_redirection());
    assert_eq!(graph(&app).await, before);
    app_settings::Entity::update_many()
        .col_expr(app_settings::Column::InviteOnly, Expr::value(true))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let (authorize, state_cookie) = begin(&app, "register", "").await;
    let code = issuer.code(
        &authorize,
        "closed-registration",
        &format!("closed-oidc-{}@example.test", uuid::Uuid::new_v4()),
    );
    let denied = callback(&app, &authorize, &code, &state_cookie, false).await;
    assert!(denied.status().is_redirection());
    assert_eq!(graph(&app).await, before);
    app_settings::Entity::update_many()
        .col_expr(app_settings::Column::InviteOnly, Expr::value(false))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let (authorize, state_cookie) = begin(&app, "register", "").await;
    let code = issuer.code(&authorize, "native-owner-subject", &owner_email);
    let signed_in = callback(&app, &authorize, &code, &state_cookie, false).await;
    let status = signed_in.status();
    let cookie = session_cookie(&signed_in);
    assert!(status.is_redirection());
    let account = accounts::Entity::find()
        .filter(accounts::Column::Email.eq(&owner_email))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let account_id = account.id;
    let person = people::Entity::find_by_id(account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let user = users::Entity::find()
        .filter(users::Column::PersonId.eq(person.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let membership = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(person.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let household = households::Entity::find_by_id(membership.household_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let native = Query::select()
        .columns(["email_verified", "metadata"])
        .from("accounts")
        .and_where(Expr::col("id").eq(account_id))
        .to_owned();
    let native = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&native))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.status, 2);
    assert_eq!(user.person_id, person.id);
    assert!(native.try_get::<bool>("", "email_verified").unwrap());
    assert_eq!(
        super::signup::legacy_column_count(&app, "users", "password_digest").await,
        0
    );
    assert_eq!(native.try_get::<Value>("", "metadata").unwrap(), json!({}));
    assert_eq!(providers(&app, account_id).await.len(), 1);
    let sessions = graph(&app).await["sessions"].clone();
    let token_calls = issuer.state.token_calls.load(Ordering::SeqCst);
    let replay = callback(&app, &authorize, &code, &state_cookie, false).await;
    assert!(replay.status().is_redirection());
    assert_eq!(graph(&app).await["sessions"], sessions);
    assert_eq!(issuer.state.token_calls.load(Ordering::SeqCst), token_calls);
    let setup = app
        .client
        .get(format!("{}/auth/passkey/setup", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(setup.status().as_u16(), 200);
    let setup = setup.text().await.unwrap();
    assert!(
        setup.contains("data-complete-onboarding"),
        "provider-only setup must expose Continue: {setup}"
    );
    assert!(
        !setup.contains("data-passkey-enrolment"),
        "provider registration must not require another credential"
    );
    let complete = app
        .client
        .post(format!("{}/api/auth/onboarding/complete", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(complete.status().as_u16(), 200);
    let care = app
        .client
        .get(format!(
            "{}/households/{}/medications",
            app.origin, household.slug
        ))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = care.status();
    let body = care.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let request_reset = app
        .client
        .post(format!("{}/api/auth/request-password-reset", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":owner_email,"redirectTo":"/reset-password/confirm"}))
        .send()
        .await
        .unwrap();
    assert_eq!(request_reset.status().as_u16(), 200);
    let messages =
        super::native_password_reset::queued_mail(&app, &owner_email, "Reset your password").await;
    assert_eq!(messages.len(), 1);
    let mail = &messages[0];
    let reset = url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap();
    let reset_token = reset.path_segments().unwrap().next_back().unwrap();
    let reset = app
        .client
        .post(format!("{}/api/auth/reset-password", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"token":reset_token,"newPassword":"Native-oidc-local-passphrase-42"}))
        .send()
        .await
        .unwrap();
    assert_eq!(reset.status().as_u16(), 200);
    let login = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":owner_email,"password":"Native-oidc-local-passphrase-42"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status().as_u16(), 200);
    let cookie = session_cookie(&login);
    let provider_id = providers(&app, account_id)
        .await
        .into_iter()
        .next()
        .unwrap();
    assert!(uuid::Uuid::parse_str(&provider_id).is_ok());
    let unlink = app
        .client
        .post(format!("{}/api/auth/unlink-account", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"accountId":provider_id}))
        .send()
        .await
        .unwrap();
    let status = unlink.status();
    let body = unlink.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let graph_unlinked = graph(&app).await;
    let (authorize, state_cookie) = begin(&app, "signin", "").await;
    let code = issuer.code(&authorize, "native-owner-subject", &owner_email);
    let denied = callback(&app, &authorize, &code, &state_cookie, false).await;
    assert!(denied.status().is_redirection());
    assert_eq!(graph(&app).await, graph_unlinked);
    let (authorize, state_cookie) = begin(&app, "link", &cookie).await;
    let code = issuer.code(&authorize, "native-owner-subject", &owner_email);
    let linked = callback(&app, &authorize, &code, &state_cookie, false).await;
    assert!(linked.status().is_redirection());
    let count = providers(&app, account_id).await.len();
    assert_eq!(count, 1);
    let before = graph(&app).await;
    let (authorize, state_cookie) = begin(&app, "signin", "").await;
    let code = issuer.code(&authorize, "native-owner-subject", &owner_email);
    let signed_in = callback(&app, &authorize, &code, &state_cookie, false).await;
    assert!(signed_in.status().is_redirection());
    let cookie = session_cookie(&signed_in);
    let session = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let body: Value = session.json().await.unwrap();
    assert_eq!(body["user"]["id"].as_str().unwrap(), account_id.to_string());
    let after = graph(&app).await;
    for name in [
        "accounts",
        "people",
        "users",
        "households",
        "memberships",
        "credentials",
    ] {
        assert_eq!(before[name], after[name], "{name}");
    }
    provider_support_retry(&app, &issuer, &cookie, account_id, &owner_email).await;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(household.id),
        person_id: Set(app.fixture.adult.id),
        role: Set("owner".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let operation = app
        .client
        .post(format!("{}/api/auth/security/operation/start", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"action":"close_account"}))
        .send()
        .await
        .unwrap();
    let status = operation.status();
    let operation: Value = operation.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{operation}");
    let operation_id = operation["operation_id"].as_str().unwrap();
    let identifier = format!("password-change:{operation_id}");
    let original = verification(&app, &identifier).await.unwrap();
    let fault = format!("oidc_closure_notice_{}", uuid::Uuid::new_v4().simple());
    // SeaORM has no trigger or PL/pgSQL function builder. Whole-run exclusivity
    // isolates this registration policy and schema fault; objects are removed below.
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic provider closure notice failure'; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject'='Account closed') EXECUTE FUNCTION {fault}()")).await.unwrap();
    let (authorize, state_cookie) =
        begin_operation(&app, "operation", &cookie, Some(operation_id)).await;
    let code = issuer.code(&authorize, "native-owner-subject", &owner_email);
    let proved = callback(&app, &authorize, &code, &state_cookie, false).await;
    let proof_cookie = session_cookie(&proved);
    assert!(proved.status().is_redirection());
    let complete_url = format!(
        "{}/account/security/provider/confirm?operation_id={operation_id}",
        app.origin
    );
    let failed = app
        .client
        .get(&complete_url)
        .header("cookie", &proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(failed.status().as_u16(), 303);
    assert_eq!(failed.headers()["location"], "/account/security");
    assert_eq!(
        accounts::Entity::find_by_id(account_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .status,
        2
    );
    assert_eq!(verification(&app, &identifier).await.unwrap(), original);
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON pg_loco_queue; DROP FUNCTION {fault}()"
        ))
        .await
        .unwrap();
    let closed = app
        .client
        .get(&complete_url)
        .header("cookie", &proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(closed.status().as_u16(), 303);
    assert_eq!(closed.headers()["location"], "/login");
    assert_eq!(
        accounts::Entity::find_by_id(account_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .status,
        3
    );
    assert!(verification(&app, &identifier).await.is_none());
    assert_eq!(
        super::native_password_reset::queued_mail(&app, &owner_email, "Account closed")
            .await
            .len(),
        1
    );
    super::signup::restore_registration(&app, registration).await;
    app.close().await;
}

async fn provider_support_retry(
    app: &Application,
    issuer: &Issuer,
    cookie: &str,
    account_id: i64,
    owner_email: &str,
) {
    let now = chrono::Utc::now().naive_utc();
    let admin = platform_admins::ActiveModel {
        account_id: Set(account_id),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let operation=app.client.post(format!("{}/api/auth/security/operation/start",app.origin)).header("origin",app.context.config.server.full_url()).header("cookie",cookie).json(&json!({"action":"support_request","household_id":app.fixture.household.id,"reason":"Native provider support proof"})).send().await.unwrap();
    let status = operation.status();
    let operation: Value = operation.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{operation}");
    let operation_id = operation["operation_id"].as_str().unwrap();
    let identifier = format!("password-change:{operation_id}");
    let original = verification(app, &identifier).await.unwrap();
    let (authorize, state_cookie) =
        begin_operation(app, "operation", cookie, Some(operation_id)).await;
    let code = issuer.code(&authorize, "native-owner-subject", owner_email);
    let proof = callback(app, &authorize, &code, &state_cookie, false).await;
    assert!(proof.status().is_redirection());
    let proof_cookie = session_cookie(&proof);
    let fault = format!("oidc_support_audit_{}", uuid::Uuid::new_v4().simple());
    // SeaORM has no trigger or PL/pgSQL function builder. The caller reserves
    // whole-run exclusivity; both schema objects are removed before retry.
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic provider support audit failure'; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON versions FOR EACH ROW WHEN(NEW.event='platform/support/requested') EXECUTE FUNCTION {fault}()")).await.unwrap();
    let complete_url = format!(
        "{}/account/security/provider/confirm?operation_id={operation_id}",
        app.origin
    );
    let failed = app
        .client
        .get(&complete_url)
        .header("cookie", &proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(failed.status().as_u16(), 303);
    assert_eq!(failed.headers()["location"], "/account/security");
    assert_eq!(verification(app, &identifier).await.unwrap(), original);
    assert_eq!(support_requests(app, admin.id).await, 0);
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON versions; DROP FUNCTION {fault}()"
        ))
        .await
        .unwrap();
    let retry = app
        .client
        .get(&complete_url)
        .header("cookie", &proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(retry.status().as_u16(), 303);
    assert_eq!(retry.headers()["location"], "/platform/support");
    assert!(verification(app, &identifier).await.is_none());
    assert_eq!(support_requests(app, admin.id).await, 1);
    let replay = app
        .client
        .get(&complete_url)
        .header("cookie", &proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 303);
    assert_eq!(replay.headers()["location"], "/account/security");
    let count = support_requests(app, admin.id).await;
    assert_eq!(count, 1);
    let mut admin = admin.into_active_model();
    admin.status = Set("revoked".into());
    admin.update(&app.fixture.admin).await.unwrap();
}

async fn support_requests(app: &Application, admin: i64) -> u64 {
    support_access_sessions::Entity::find()
        .filter(support_access_sessions::Column::PlatformAdminId.eq(admin))
        .count(&app.fixture.admin)
        .await
        .unwrap()
}
