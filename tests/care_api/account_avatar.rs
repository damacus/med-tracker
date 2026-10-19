use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use loco_rs::{TestServer, boot::BootResult};

const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==";
const ENDPOINT: &str = "/api/v1/auth/profile/avatar";

async fn waiting_for_avatar_lock(app: &Application, pattern: &str) -> bool {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query ILIKE $1 AND cardinality(pg_blocking_pids(pid))>0) AS waiting", [pattern.into()]
            )).await.unwrap().unwrap();
            if row.try_get::<bool>("", "waiting").unwrap() { return; }
            tokio::task::yield_now().await;
        }
    }).await.is_ok()
}

#[tokio::test]
async fn household_avatar_permissions_are_rechecked_after_waiting_for_old_key_lock() {
    use sea_orm::TransactionTrait;
    for method in ["DELETE", "PUT"] {
        let (app, boot, server, token) = boot().await;
        app.fixture
            .admin
            .execute_unprepared("UPDATE people SET date_of_birth='1986-04-17' WHERE id=73001")
            .await
            .unwrap();
        let endpoint = "/api/v1/households/72001/profile/avatar";
        let initial = server
            .put(endpoint)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        assert_eq!(initial.status_code(), 200, "{}", initial.text());
        let key: String = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT key FROM account_avatars WHERE account_id=71001",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "key")
            .unwrap();
        let blocker = app.fixture.admin.begin().await.unwrap();
        blocker
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
                [format!("medtracker.avatar:{key}").into()],
            ))
            .await
            .unwrap();
        let mutation = if method == "DELETE" {
            server.delete(endpoint)
        } else {
            server
                .put(endpoint)
                .content_type("multipart/form-data; boundary=avatar-boundary")
                .bytes(image().into())
        };
        let mutation = mutation.add_header("authorization", format!("Bearer {token}"));
        let revoke = async {
            let waiting = waiting_for_avatar_lock(&app, "%pg_advisory_xact_lock%").await;
            app.fixture
                .admin
                .execute_unprepared(
                    "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001",
                )
                .await
                .unwrap();
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
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT key,(SELECT count(*) FROM versions WHERE audit_context->>'policy_query'='update_own_account_avatar') AS audits,(SELECT count(*) FROM pg_loco_queue WHERE name='AvatarRetirementWorker') AS retired FROM account_avatars WHERE account_id=71001"
        )).await.unwrap().unwrap();
        assert_eq!(row.try_get::<String>("", "key").unwrap(), key);
        assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
        assert_eq!(row.try_get::<i64>("", "retired").unwrap(), 0);
        let storage =
            med_tracker::models::profile::avatar::Storage::configured(&boot.app_context).unwrap();
        let objects = crate::avatar_storage::owned_client()
            .list_objects_v2()
            .bucket(storage.bucket())
            .send()
            .await
            .unwrap();
        assert_eq!(objects.key_count(), Some(1));
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
        let sql = "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001";
        let (app, boot, server, token) = boot().await;
        app.fixture
            .admin
            .execute_unprepared("UPDATE people SET date_of_birth='1986-04-17' WHERE id=73001; UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
            .await
            .unwrap();
        let endpoint = "/api/v1/households/72001/profile/avatar";
        let initial = server
            .put(endpoint)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        assert_eq!(initial.status_code(), 200, "{}", initial.text());
        let blocker = app.fixture.admin.begin().await.unwrap();
        blocker
            .execute_unprepared("SELECT id FROM accounts WHERE id=71001 FOR NO KEY UPDATE")
            .await
            .unwrap();
        let removal = server
            .delete(endpoint)
            .add_header("authorization", format!("Bearer {token}"));
        let revoke = async {
            let waiting = waiting_for_avatar_lock(&app, "%accounts%FOR NO KEY UPDATE%").await;
            app.fixture.admin.execute_unprepared(sql).await.unwrap();
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
        let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM account_avatars WHERE account_id=71001) AS attached,(SELECT count(*) FROM versions WHERE audit_context->>'policy_query'='update_own_account_avatar') AS audits,(SELECT count(*) FROM pg_loco_queue WHERE name='AvatarRetirementWorker') AS retired"
        )).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "attached").unwrap(), 1);
        assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
        assert_eq!(row.try_get::<i64>("", "retired").unwrap(), 0);
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
    let digest = hex::encode(Sha256::digest(b"avatar-stored-session"));
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(71001,74001,$1,'avatar-session-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())", [digest.into()]
    )).await.unwrap();
    let uploaded = server
        .put(ENDPOINT)
        .add_header("authorization", "Bearer avatar-stored-session")
        .content_type("multipart/form-data; boundary=avatar-boundary")
        .bytes(image().into())
        .await;
    assert_eq!(uploaded.status_code(), 200, "{}", uploaded.text());
    let read = server
        .get(ENDPOINT)
        .add_header("authorization", "Bearer avatar-stored-session")
        .await;
    assert_eq!(read.status_code(), 200);
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    let denied = server
        .get(ENDPOINT)
        .add_header("authorization", "Bearer avatar-stored-session")
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
    let app = Application::new().await;
    let token = app.token().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    let bucket = format!("medtracker-fixture-{}", uuid::Uuid::new_v4().simple());
    crate::avatar_storage::create_owned_bucket(&bucket).await;
    config.settings.get_or_insert_with(|| json!({}))["avatar_storage"]["bucket"] = json!(bucket);
    config.database.uri = app.fixture.runtime_uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("Postgres queue required")
    };
    queue.uri = app.fixture.runtime_uri.clone();
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
            app.fixture.admin.execute_unprepared("INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(71001,'revocation-read','image/png','s3',1,'synthetic')").await.unwrap();
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
            let sql = match mutation {
                "remove" => "DELETE FROM account_avatars WHERE account_id=71001",
                "replace" => {
                    "UPDATE account_avatars SET key='newer-attachment' WHERE account_id=71001"
                }
                _ => "UPDATE oauth_grants SET revoked_at=now() WHERE account_id=71001",
            };
            app.fixture.admin.execute_unprepared(sql).await.unwrap();
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
            let rows = app
                .fixture
                .admin
                .query_all_raw(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT account_id FROM account_avatars",
                ))
                .await
                .unwrap();
            assert!(rows.is_empty());
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
    let mut body = b"--avatar-boundary\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.png\"\r\nContent-Type: image/png\r\n\r\n".to_vec();
    body.extend(STANDARD.decode(PNG).unwrap());
    body.extend(b"\r\n--avatar-boundary--\r\n");
    body
}

#[tokio::test]
async fn account_avatar_has_no_household_or_clinical_actor_prerequisite() {
    let (app, boot, server, token) = boot().await;
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,'Account-only avatar owner',0,true,now(),now()); INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(71002,73004,'Account-only avatar owner','account-only-avatar@example.test',2,now(),now()); UPDATE oauth_grants SET account_id=71002 WHERE account_id=71001 AND client_kind='mobile'").await.unwrap();
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
        .get("/api/v1/households/72001/people")
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
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS total,bool_and(household_id IS NULL AND whodunnit IS NULL AND audit_context->>'actor_account_id'='71002' AND audit_context ? 'server_trace_id') AS attributed,string_agg(coalesce(object,'') || coalesce(object_changes,'') || audit_context::text,'') AS contents FROM versions WHERE item_type='Account' AND audit_context->>'policy_query'='update_own_account_avatar'"
    )).await.unwrap().unwrap();
    assert_eq!(audit.try_get::<i64>("", "total").unwrap(), 3);
    assert!(audit.try_get::<bool>("", "attributed").unwrap());
    let contents: String = audit.try_get("", "contents").unwrap();
    for sensitive in [
        "avatars/",
        "example.test",
        "checksum",
        "image/png",
        "signed",
    ] {
        assert!(!contents.contains(sensitive), "{contents}");
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
    for sql in [
        "CREATE FUNCTION reject_avatar_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.audit_context->>'policy_query'='update_own_account_avatar' THEN RAISE EXCEPTION 'synthetic failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_avatar_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_avatar_audit()",
        "DROP TRIGGER reject_avatar_audit ON versions; CREATE FUNCTION reject_avatar_retirement() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END $$; CREATE TRIGGER reject_avatar_retirement BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_avatar_retirement()",
    ] {
        app.fixture.admin.execute_unprepared(sql).await.unwrap();
        let failed = server
            .put(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(image().into())
            .await;
        assert_eq!(failed.status_code(), 503);
        assert!(!failed.text().contains("synthetic"));
        let removed = server
            .delete(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(removed.status_code(), 503);
        let retained = server
            .get(ENDPOINT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(retained.status_code(), 200);
        let objects = crate::avatar_storage::owned_client()
            .list_objects_v2()
            .bucket(storage.bucket())
            .send()
            .await
            .unwrap();
        assert_eq!(objects.key_count(), Some(1));
        let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS total FROM versions WHERE audit_context->>'policy_query'='update_own_account_avatar'"
        )).await.unwrap().unwrap();
        assert_eq!(audit.try_get::<i64>("", "total").unwrap(), 1);
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
    for sql in [
        "UPDATE oauth_grants SET access_token_scopes='openid',access_token_scope_hash=token_hash WHERE account_id=71001",
        "UPDATE oauth_grants SET access_token_scopes='medtracker' WHERE account_id=71001; UPDATE accounts SET status=1 WHERE id=71001",
        "UPDATE accounts SET status=2 WHERE id=71001; INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-avatar-lock',now()+interval '1 hour')",
        "DELETE FROM account_lockouts; UPDATE oauth_grants SET expires_in=now()-interval '1 second' WHERE account_id=71001",
        "UPDATE oauth_grants SET expires_in=now()+interval '15 minutes',revoked_at=now() WHERE account_id=71001",
    ] {
        app.fixture.admin.execute_unprepared(sql).await.unwrap();
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
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71002,73002,'foreign-avatar@example.test',2,now(),now()); INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(71001,'own-avatar','image/png','s3',1,'synthetic'),(71002,'foreign-avatar','image/png','s3',1,'synthetic')").await.unwrap();
    for (mutation, expected) in [
        ("SELECT 1", 2),
        (
            "INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78002,72001,74001,73002,'view','clinician',now(),now())",
            2,
        ),
        (
            "UPDATE person_access_grants SET expires_at=now()-interval '1 second' WHERE id=78002",
            2,
        ),
        (
            "UPDATE person_access_grants SET expires_at=NULL,revoked_at=now() WHERE id=78002",
            2,
        ),
        (
            "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74980",
            1,
        ),
    ] {
        app.fixture
            .admin
            .execute_unprepared(mutation)
            .await
            .unwrap();
        let transaction = app.fixture.runtime.begin().await.unwrap();
        transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71001',true),set_config('med_tracker.current_household_id','72001',true),set_config('med_tracker.current_membership_id','74001',true)").await.unwrap();
        let rows = transaction
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT account_id FROM account_avatars",
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), expected);
        for sql in [
            "UPDATE account_avatars SET checksum='foreign-write' WHERE account_id=71002",
            "DELETE FROM account_avatars WHERE account_id=71002",
        ] {
            assert_eq!(
                transaction
                    .execute_unprepared(sql)
                    .await
                    .unwrap()
                    .rows_affected(),
                0
            );
        }
        transaction.rollback().await.unwrap();
    }
    let transaction = app.fixture.runtime.begin().await.unwrap();
    transaction.execute_unprepared("SET LOCAL ROLE med_tracker_app; SELECT set_config('med_tracker.current_account_id','71001',true)").await.unwrap();
    assert!(transaction.execute_unprepared("INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES(71002,'guess','image/png','s3',1,'synthetic') ON CONFLICT(account_id) DO UPDATE SET key='guess'").await.is_err());
    transaction.rollback().await.unwrap();
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
}
