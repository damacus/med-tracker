use super::avatar::*;
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use loco_rs::{TestServer, boot::BootResult};
use med_tracker::models::{
    _entities::{
        account_lockouts, accounts, api_sessions, oauth_grants, people, person_access_grants,
    },
    access,
};
use sea_orm::sea_query::{
    Expr, ExprTrait, Func, LockType, OnConflict, Query, extension::postgres::PgExpr,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, IntoActiveModel, QuerySelect};

const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==";
const ENDPOINT: &str = "/api/v1/auth/profile/avatar";

async fn blocker_pid<C: ConnectionTrait>(db: &C) -> i32 {
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

async fn waiting_for_avatar_lock(app: &Application, blocker: i32, pattern: &str) -> bool {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let query = Query::select()
                .expr_as(Func::count(Expr::col("pid")), "waiting")
                .from("pg_stat_activity")
                .and_where(Expr::col("datname").eq(Func::cust("current_database")))
                .and_where(Expr::col("wait_event_type").eq("Lock"))
                .and_where(Expr::col("query").ilike(pattern))
                .and_where(Expr::val(blocker).eq(
                    Func::cust("ANY").arg(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                ))
                .to_owned();
            let row = app
                .fixture
                .admin
                .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<i64>("", "waiting").unwrap() > 0 {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok()
}

#[tokio::test]
async fn household_avatar_permissions_are_rechecked_after_waiting_for_old_key_lock() {
    use sea_orm::TransactionTrait;
    for method in ["DELETE", "PUT"] {
        let (app, boot, server, token) = boot().await;

        let endpoint = format!(
            "/api/v1/households/{}/profile/avatar",
            app.fixture.household.id
        );
        let initial = server
            .put(&endpoint)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        assert_eq!(initial.status_code(), 200, "{}", initial.text());
        let key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
        let blocker = app.fixture.admin.begin().await.unwrap();
        let query = Query::select()
            .expr(
                Func::cust("pg_advisory_xact_lock").arg(Func::cust("hashtextextended").args([
                    Expr::val(format!("medtracker.avatar:{key}")),
                    Expr::val(0_i64),
                ])),
            )
            .to_owned();
        blocker
            .query_one_raw(blocker.get_database_backend().build(&query))
            .await
            .unwrap();
        let pid = blocker_pid(&blocker).await;
        let mutation = if method == "DELETE" {
            server.delete(&endpoint)
        } else {
            server
                .put(&endpoint)
                .content_type("multipart/form-data; boundary=avatar-boundary")
                .bytes(image().into())
        };
        let mutation = mutation.add_header("authorization", format!("Bearer {token}"));
        let revoke = async {
            let waiting = waiting_for_avatar_lock(&app, pid, "%pg_advisory_xact_lock%").await;
            revoke_membership(&app.fixture, &app.fixture.membership).await;
            blocker.commit().await.unwrap();
            waiting
        };
        let (response, waiting) = tokio::join!(mutation, revoke);
        assert!(waiting, "household avatar must reach the held old-key lock");
        assert!(
            response.status_code().as_u16() == 403,
            "{method}: {} {}",
            response.status_code(),
            response.text()
        );
        assert_eq!(
            avatar_key(&app.fixture.admin, app.fixture.account.id).await,
            key
        );
        assert_eq!(
            avatar_audits(&app.fixture.admin, app.fixture.account.id)
                .await
                .len(),
            1
        );
        assert_eq!(
            retirement_jobs(&app.fixture.admin, &bucket(&boot.app_context))
                .await
                .len(),
            0
        );
        let storage =
            med_tracker::models::profile::avatar::Storage::configured(&boot.app_context).unwrap();
        assert_eq!(
            crate::avatar_storage::owned_operator(storage.bucket())
                .key_count()
                .await,
            1
        );
        drop(server);
        if let Some(queue) = boot.app_context.queue_provider.as_ref() {
            queue.shutdown().unwrap();
        }
        boot.app_context.db.close().await.unwrap();
        app.close().await;
    }
}

#[tokio::test]
async fn household_avatar_permissions_are_rechecked_after_waiting_for_account_lock() {
    use sea_orm::TransactionTrait;
    {
        let (app, boot, server, token) = boot().await;
        revoke_grant(&app.fixture).await;
        let endpoint = format!(
            "/api/v1/households/{}/profile/avatar",
            app.fixture.household.id
        );
        let initial = server
            .put(&endpoint)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        assert_eq!(initial.status_code(), 200, "{}", initial.text());
        let blocker = app.fixture.admin.begin().await.unwrap();
        accounts::Entity::find_by_id(app.fixture.account.id)
            .lock(LockType::NoKeyUpdate)
            .one(&blocker)
            .await
            .unwrap()
            .unwrap();
        let pid = blocker_pid(&blocker).await;
        let removal = server
            .delete(&endpoint)
            .add_header("authorization", format!("Bearer {token}"));
        let revoke = async {
            let waiting = waiting_for_avatar_lock(&app, pid, "%accounts%FOR NO KEY UPDATE%").await;
            revoke_membership(&app.fixture, &app.fixture.membership).await;
            blocker.commit().await.unwrap();
            waiting
        };
        let (response, waiting) = tokio::join!(removal, revoke);
        assert!(
            waiting,
            "household avatar mutation must reach the held Account lock"
        );
        assert!(
            response.status_code().as_u16() == 403,
            "{}: {}",
            response.status_code(),
            response.text()
        );
        assert_eq!(
            attachment_count(&app.fixture.admin, app.fixture.account.id).await,
            1
        );
        assert_eq!(
            avatar_audits(&app.fixture.admin, app.fixture.account.id)
                .await
                .len(),
            1
        );
        assert_eq!(
            retirement_jobs(&app.fixture.admin, &bucket(&boot.app_context))
                .await
                .len(),
            0
        );
        drop(server);
        if let Some(queue) = boot.app_context.queue_provider.as_ref() {
            queue.shutdown().unwrap();
        }
        boot.app_context.db.close().await.unwrap();
        app.close().await;
    }
}

#[tokio::test]
async fn account_avatar_rate_limit_and_missing_peer_errors_prevent_sniffing() {
    use axum::http::{HeaderMap, Method};
    use std::{net::IpAddr, sync::Arc};
    let (app, boot, server, _) = boot_with(|config| {
        config.settings.as_mut().unwrap()["api_rate_limit"] =
            json!({"fixed_now_unix_seconds":1000,"trusted_proxy_ips":"127.0.0.1"});
    })
    .await;
    let state = boot
        .app_context
        .shared_store
        .get::<Arc<med_tracker::models::rate_limit::RateLimitState>>()
        .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "198.51.100.71".parse().unwrap());
    for _ in 0..300 {
        assert!(
            state
                .check(
                    "127.0.0.1".parse::<IpAddr>().unwrap(),
                    &headers,
                    &Method::GET,
                    ENDPOINT
                )
                .is_none()
        );
    }
    let rejected = server
        .get(ENDPOINT)
        .add_header("x-forwarded-for", "198.51.100.71")
        .await;
    assert_eq!(rejected.status_code(), 429);
    assert_eq!(rejected.header("ratelimit-limit"), "300");
    assert_eq!(rejected.header("ratelimit-remaining"), "0");
    assert_eq!(rejected.header("retry-after"), "200");
    assert_eq!(rejected.header("cache-control"), "no-store");
    assert_eq!(rejected.header("x-content-type-options"), "nosniff");
    let mock = TestServer::builder()
        .mock_transport()
        .build(boot.router.clone().unwrap())
        .unwrap();
    let unavailable = mock.get(ENDPOINT).await;
    assert_eq!(unavailable.status_code(), 503);
    assert_eq!(unavailable.header("cache-control"), "no-store");
    assert_eq!(unavailable.header("x-content-type-options"), "nosniff");
    drop(mock);
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn account_avatar_stored_credentials_keep_their_membership_validity_rules() {
    use sha2::{Digest, Sha256};
    let (app, boot, server, _) = boot().await;
    let credential = uuid::Uuid::new_v4().to_string();
    let digest = hex::encode(Sha256::digest(credential.as_bytes()));
    let now = chrono::Utc::now().naive_utc();
    api_sessions::ActiveModel {
        account_id: Set(app.fixture.account.id),
        household_membership_id: Set(Some(app.fixture.membership.id)),
        access_token_digest: Set(digest),
        refresh_token_digest: Set(hex::encode(Sha256::digest(uuid::Uuid::new_v4().as_bytes()))),
        permissions_version: Set(app.fixture.membership.permissions_version),
        access_expires_at: Set(now + chrono::Duration::minutes(15)),
        refresh_expires_at: Set(now + chrono::Duration::days(1)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let uploaded = server
        .put(ENDPOINT)
        .add_header("authorization", format!("Bearer {credential}"))
        .content_type("multipart/form-data; boundary=avatar-boundary")
        .bytes(image().into())
        .await;
    assert_eq!(uploaded.status_code(), 200, "{}", uploaded.text());
    let read = server
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {credential}"))
        .await;
    assert_eq!(read.status_code(), 200);
    revoke_membership(&app.fixture, &app.fixture.membership).await;
    let denied = server
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {credential}"))
        .await;
    assert_eq!(denied.status_code(), 401);
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}

async fn boot() -> (Application, BootResult, TestServer, String) {
    boot_with(|_| {}).await
}

async fn boot_with(
    configure: impl FnOnce(&mut Config),
) -> (Application, BootResult, TestServer, String) {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    let bucket = format!("medtracker-fixture-{}", uuid::Uuid::new_v4().simple());
    crate::avatar_storage::create_owned_bucket(&bucket).await;
    config.settings.get_or_insert_with(|| json!({}))["avatar_storage"]["bucket"] = json!(bucket);
    config.database.uri = std::env::var("DATABASE_URL").unwrap();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("Postgres queue required")
    };
    queue.uri = std::env::var("DATABASE_URL").unwrap();
    configure(&mut config);
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    let server = TestServer::new(
        boot.router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap();
    (app, boot, server, token)
}

#[tokio::test]
async fn account_avatar_object_io_rechecks_credentials_and_attachment() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::Notify;
    for (method, mutation, expected) in [
        ("GET", "revoke", 401),
        ("PUT", "revoke", 401),
        ("GET", "remove", 404),
        ("GET", "replace", 404),
    ] {
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let deleted = Arc::new(AtomicUsize::new(0));
        let observed = entered.clone();
        let released = release.clone();
        let cleanup = deleted.clone();
        let router = axum::Router::new().fallback(move |request: Request<Body>| {
            let entered = observed.clone();
            let release = released.clone();
            let deleted = cleanup.clone();
            async move {
                use axum::response::IntoResponse;
                if request.method() == axum::http::Method::DELETE {
                    deleted.fetch_add(1, Ordering::SeqCst);
                    return axum::http::StatusCode::NO_CONTENT.into_response();
                }
                entered.notify_one();
                release.notified().await;
                (
                    axum::http::StatusCode::OK,
                    [("content-type", "image/png")],
                    STANDARD.decode(PNG).unwrap(),
                )
                    .into_response()
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let storage_server =
            tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let (app, boot, server, token) = boot_with(|config| {
            config.settings.as_mut().unwrap()["avatar_storage"]["endpoint"] = json!(endpoint);
        })
        .await;
        if method == "GET" {
            insert_avatar(
                &app.fixture.admin,
                app.fixture.account.id,
                &format!("revocation-read-{}", uuid::Uuid::new_v4()),
            )
            .await;
        }
        let request = if method == "GET" {
            server.get(ENDPOINT)
        } else {
            server
                .put(ENDPOINT)
                .content_type("multipart/form-data; boundary=avatar-boundary")
                .bytes(image().into())
        };
        let request = request.add_header("authorization", format!("Bearer {token}"));
        let revoke = async {
            tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
                .await
                .unwrap();
            match mutation {
                "remove" => remove_avatar(&app.fixture.admin, app.fixture.account.id).await,
                "replace" => {
                    set_avatar(
                        &app.fixture.admin,
                        app.fixture.account.id,
                        AvatarColumn::Key,
                        &format!("newer-attachment-{}", uuid::Uuid::new_v4()),
                    )
                    .await
                }
                _ => {
                    let mut grant = oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
                        .one(&app.fixture.admin)
                        .await
                        .unwrap()
                        .unwrap()
                        .into_active_model();
                    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
                    grant.update(&app.fixture.admin).await.unwrap();
                }
            }
            release.notify_one();
        };
        let (response, ()) = tokio::join!(request, revoke);
        assert_eq!(
            response.status_code(),
            expected,
            "{method}: {}",
            response.text()
        );
        assert_eq!(response.header("cache-control"), "no-store");
        assert_eq!(response.header("x-content-type-options"), "nosniff");
        if method == "PUT" {
            assert_eq!(deleted.load(Ordering::SeqCst), 1);
            assert_eq!(
                attachment_count(&app.fixture.admin, app.fixture.account.id).await,
                0
            );
        }
        storage_server.abort();
        let _ = storage_server.await;
        drop(server);
        if let Some(queue) = boot.app_context.queue_provider.as_ref() {
            queue.shutdown().unwrap();
        }
        boot.app_context.db.close().await.unwrap();
        app.close().await;
    }
}

fn image() -> Vec<u8> {
    multipart_image(&STANDARD.decode(PNG).unwrap(), "image/png")
}

fn multipart_image(bytes: &[u8], content_type: &str) -> Vec<u8> {
    let mut body = format!("--avatar-boundary\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.png\"\r\nContent-Type: {content_type}\r\n\r\n").into_bytes();
    body.extend(bytes);
    body.extend(b"\r\n--avatar-boundary--\r\n");
    body
}

#[tokio::test]
async fn account_avatar_rejects_invalid_uploads_without_replacing_the_attachment() {
    let (app, boot, server, token) = boot().await;
    let uploaded = server
        .put(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .content_type("multipart/form-data; boundary=avatar-boundary")
        .bytes(image().into())
        .await;
    assert_eq!(uploaded.status_code(), 200, "{}", uploaded.text());
    let png = STANDARD.decode(PNG).unwrap();
    for (label, body) in [
        ("non-image bytes", multipart_image(b"not a png", "image/png")),
        ("declared type mismatch", multipart_image(&png, "image/jpeg")),
        ("missing avatar field", b"--avatar-boundary\r\nContent-Disposition: form-data; name=\"other\"; filename=\"other.png\"\r\nContent-Type: image/png\r\n\r\nx\r\n--avatar-boundary--\r\n".to_vec()),
        ("trailing extra field", {
            let mut body = multipart_image(&png, "image/png");
            body.truncate(body.len() - b"--avatar-boundary--\r\n".len());
            body.extend(b"--avatar-boundary\r\nContent-Disposition: form-data; name=\"extra\"\r\n\r\nx\r\n--avatar-boundary--\r\n");
            body
        }),
    ] {
        let rejected = server
            .put(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(body.into())
            .await;
        assert_eq!(rejected.status_code(), 422, "{label}");
        assert_eq!(rejected.header("cache-control"), "no-store");
        assert_eq!(rejected.header("x-content-type-options"), "nosniff");
        let body: Value = rejected.json();
        assert_eq!(body["error"]["code"], "unprocessable_content", "{label}");
        let retained = server
            .get(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(retained.status_code(), 200, "{label}");
        assert_eq!(retained.header("content-type"), "image/png");
    }
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn account_avatar_has_no_household_or_clinical_actor_prerequisite() {
    let (app, boot, server, token) = boot().await;
    let now = chrono::Utc::now().naive_utc();
    let person = people::ActiveModel {
        name: Set("Account-only avatar owner".into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let account = subject_account(&app.fixture, person.id).await;
    let mut grant = oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    grant.account_id = Set(account.id);
    grant.update(&app.fixture.admin).await.unwrap();
    let missing = server
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(missing.status_code(), 404);
    let removed_again = server
        .delete(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(removed_again.status_code(), 204);
    assert_eq!(missing.header("cache-control"), "no-store");
    assert_eq!(missing.header("x-content-type-options"), "nosniff");
    let clinical = server
        .get(&format!(
            "/api/v1/households/{}/people",
            app.fixture.household.id
        ))
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(clinical.status_code(), 403);
    for _ in 0..2 {
        let uploaded = server
            .put(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        assert_eq!(uploaded.status_code(), 200, "{}", uploaded.text());
        assert_eq!(uploaded.json::<Value>()["data"]["avatar_attached"], true);
        let read = server
            .get(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(read.status_code(), 200);
        assert_eq!(read.header("content-type"), "image/png");
        let profile = server
            .get("/api/v1/auth/profile")
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(profile.json::<Value>()["data"]["avatar_attached"], true);
    }
    let removed = server
        .delete(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(removed.status_code(), 204);
    let profile = server
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(profile.json::<Value>()["data"]["avatar_attached"], false);
    let missing = server
        .get(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(missing.status_code(), 404);
    let audits = avatar_audits(&app.fixture.admin, account.id).await;
    assert_eq!(audits.len(), 3);
    assert!(audits.iter().all(|audit| audit.household_id.is_none()
        && audit.whodunnit.is_none()
        && audit.audit_context["actor_account_id"] == account.id
        && audit.audit_context.get("server_trace_id").is_some()));
    let contents = audits
        .iter()
        .map(|audit| {
            format!(
                "{}{}{}",
                audit.object.as_deref().unwrap_or(""),
                audit.object_changes.as_deref().unwrap_or(""),
                audit.audit_context
            )
        })
        .collect::<String>();
    for sensitive in [
        "avatars/",
        "example.test",
        "checksum",
        "image/png",
        "signed",
    ] {
        assert!(!str::contains(&contents, sensitive), "{contents}");
    }
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn account_avatar_audit_and_retirement_failures_roll_back_and_clean_orphans() {
    let (app, boot, server, token) = boot().await;
    let storage =
        med_tracker::models::profile::avatar::Storage::configured(&boot.app_context).unwrap();
    let uploaded = server
        .put(ENDPOINT)
        .add_header("authorization", format!("Bearer {token}"))
        .content_type("multipart/form-data; boundary=avatar-boundary")
        .bytes(image().into())
        .await;
    assert_eq!(uploaded.status_code(), 200);
    for queue_failure in [false, true] {
        let failure =
            FailureTrigger::install(&app.fixture, queue_failure, &bucket(&boot.app_context)).await;
        let failed = server
            .put(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        let removed = server
            .delete(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        failure.remove(&app.fixture).await;
        assert_eq!(failed.status_code(), 503);
        assert!(!str::contains(&failed.text(), "synthetic"));
        assert_eq!(removed.status_code(), 503);
        let retained = server
            .get(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(retained.status_code(), 200);
        assert_eq!(
            crate::avatar_storage::owned_operator(storage.bucket())
                .key_count()
                .await,
            1
        );
        assert_eq!(
            avatar_audits(&app.fixture.admin, app.fixture.account.id)
                .await
                .len(),
            1
        );
    }
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn account_avatar_credentials_and_status_are_revalidated() {
    let (app, boot, server, token) = boot().await;
    for credential in [None, Some("wrong"), Some("medtracker_personal_key")] {
        let request = server.get(ENDPOINT);
        let request = if let Some(value) = credential {
            request.add_header("authorization", format!("Bearer {value}"))
        } else {
            request
        };
        let denied = request.await;
        assert_eq!(denied.status_code(), 401);
        assert_eq!(denied.header("cache-control"), "no-store");
        assert_eq!(denied.header("x-content-type-options"), "nosniff");
    }
    for invalidation in 0..5 {
        let now = chrono::Utc::now().naive_utc();
        let mut grant = oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .into_active_model();
        match invalidation {
            0 => {
                let row = oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
                    .one(&app.fixture.admin)
                    .await
                    .unwrap()
                    .unwrap();
                grant.access_token_scopes = Set(Some("openid".into()));
                grant.access_token_scope_hash = Set(row.token_hash);
            }
            1 => {
                grant.access_token_scopes = Set(Some("medtracker".into()));
                let mut account = app.fixture.account.clone().into_active_model();
                account.status = Set(1);
                account.update(&app.fixture.admin).await.unwrap();
            }
            2 => {
                let mut account = app.fixture.account.clone().into_active_model();
                account.status = Set(2);
                account.update(&app.fixture.admin).await.unwrap();
                account_lockouts::ActiveModel {
                    account_id: Set(app.fixture.account.id),
                    key: Set(uuid::Uuid::new_v4().to_string()),
                    deadline: Set(now + chrono::Duration::hours(1)),
                    ..Default::default()
                }
                .insert(&app.fixture.admin)
                .await
                .unwrap();
            }
            3 => {
                account_lockouts::Entity::delete_by_id(app.fixture.account.id)
                    .exec(&app.fixture.admin)
                    .await
                    .unwrap();
                grant.expires_in = Set(now - chrono::Duration::seconds(1));
            }
            _ => {
                grant.expires_in = Set(now + chrono::Duration::minutes(15));
                grant.revoked_at = Set(Some(now));
            }
        }
        grant.update(&app.fixture.admin).await.unwrap();
        for method in ["GET", "PUT", "DELETE"] {
            let request = match method {
                "PUT" => server
                    .put(ENDPOINT)
                    .content_type("multipart/form-data; boundary=avatar-boundary")
                    .bytes(image().into()),
                "DELETE" => server.delete(ENDPOINT),
                _ => server.get(ENDPOINT),
            };
            let denied = request
                .add_header("authorization", format!("Bearer {token}"))
                .await;
            assert!(matches!(denied.status_code().as_u16(), 401 | 403));
            assert_eq!(denied.header("x-content-type-options"), "nosniff");
        }
    }
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn account_avatar_foreign_rls_requires_current_subject_participation() {
    use sea_orm::TransactionTrait;
    let (app, boot, server, _) = boot().await;
    let foreign = subject_account(&app.fixture, app.fixture.minor.id).await;
    insert_avatar(
        &app.fixture.admin,
        app.fixture.account.id,
        &format!("own-avatar-{}", uuid::Uuid::new_v4()),
    )
    .await;
    insert_avatar(
        &app.fixture.admin,
        foreign.id,
        &format!("foreign-avatar-{}", uuid::Uuid::new_v4()),
    )
    .await;
    let mut access_grant: Option<person_access_grants::Model> = None;
    for (stage, expected) in [(0, 2), (1, 2), (2, 2), (3, 2), (4, 1)] {
        match stage {
            1 => {
                access_grant = Some(view_grant(&app.fixture, app.fixture.minor.id).await);
            }
            2 | 3 => {
                let mut grant = access_grant.as_ref().unwrap().clone().into_active_model();
                grant.expires_at = Set(if stage == 2 {
                    Some(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1))
                } else {
                    None
                });
                grant.revoked_at = Set((stage == 3).then(|| chrono::Utc::now().naive_utc()));
                grant.update(&app.fixture.admin).await.unwrap();
            }
            4 => {
                revoke_membership(&app.fixture, &app.fixture.minor_membership).await;
            }
            _ => {}
        }
        let tenant = access::begin(&app.fixture.runtime, &app.fixture.scope(), None)
            .await
            .unwrap();
        let transaction = tenant.transaction();
        let query = Query::select()
            .column("account_id")
            .from("account_avatars")
            .and_where(Expr::col("account_id").is_in([app.fixture.account.id, foreign.id]))
            .to_owned();
        let rows = transaction
            .query_all_raw(transaction.get_database_backend().build(&query))
            .await
            .unwrap();
        assert_eq!(rows.len(), expected);
        let update = Query::update()
            .table("account_avatars")
            .value("checksum", "foreign-write")
            .and_where(Expr::col("account_id").eq(foreign.id))
            .to_owned();
        let delete = Query::delete()
            .from_table("account_avatars")
            .and_where(Expr::col("account_id").eq(foreign.id))
            .to_owned();
        assert_eq!(
            transaction
                .execute_raw(transaction.get_database_backend().build(&update))
                .await
                .unwrap()
                .rows_affected(),
            0
        );
        assert_eq!(
            transaction
                .execute_raw(transaction.get_database_backend().build(&delete))
                .await
                .unwrap()
                .rows_affected(),
            0
        );
        tenant.rollback().await.unwrap();
    }
    let transaction = app.fixture.runtime.begin().await.unwrap();
    let context = Query::select()
        .expr(Func::cust("set_config").args([
            Expr::val("med_tracker.current_account_id"),
            Expr::val(app.fixture.account.id.to_string()),
            Expr::val(true),
        ]))
        .to_owned();
    transaction
        .query_one_raw(transaction.get_database_backend().build(&context))
        .await
        .unwrap();
    let insert = Query::insert()
        .into_table("account_avatars")
        .columns([
            "account_id",
            "key",
            "content_type",
            "service_name",
            "byte_size",
            "checksum",
        ])
        .values([
            Expr::val(foreign.id),
            Expr::val(format!("guess-{}", uuid::Uuid::new_v4())),
            Expr::val("image/png"),
            Expr::val("s3"),
            Expr::val(1_i64),
            Expr::val("synthetic"),
        ])
        .unwrap()
        .on_conflict(
            OnConflict::column("account_id")
                .update_column("key")
                .to_owned(),
        )
        .to_owned();
    assert!(
        transaction
            .execute_raw(transaction.get_database_backend().build(&insert))
            .await
            .is_err()
    );
    transaction.rollback().await.unwrap();
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}
