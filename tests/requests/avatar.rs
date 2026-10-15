use super::*;
use axum::{body::Bytes, http::Method};
use base64::{Engine, engine::general_purpose::STANDARD};

const AVATAR: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==";
const PATH: &str = "/households/{household_id}/profile/avatar";

struct Reply {
    status: u16,
    bytes: Vec<u8>,
    content_type: Option<String>,
    cache_control: Option<String>,
    request_id: Option<String>,
    nosniff: Option<String>,
    retry_after: Option<String>,
    limit: Option<String>,
    remaining: Option<String>,
    reset: Option<String>,
}

fn header(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn collect(status: u16, bytes: Vec<u8>, headers: &axum::http::HeaderMap) -> Reply {
    Reply {
        status,
        bytes,
        content_type: header(headers, "content-type"),
        cache_control: header(headers, "cache-control"),
        request_id: header(headers, "x-request-id"),
        nosniff: header(headers, "x-content-type-options"),
        retry_after: header(headers, "retry-after"),
        limit: header(headers, "ratelimit-limit"),
        remaining: header(headers, "ratelimit-remaining"),
        reset: header(headers, "ratelimit-reset"),
    }
}

fn multipart(bytes: &[u8], content_type: &str) -> Bytes {
    let mut body = b"--avatar-boundary\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.png\"\r\nContent-Type: ".to_vec();
    body.extend_from_slice(content_type.as_bytes());
    body.extend_from_slice(b"\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--avatar-boundary--\r\n");
    Bytes::from(body)
}

async fn send(
    server: &TestServer,
    method: Method,
    household: i64,
    bearer: Option<&str>,
    image: Option<(&[u8], &str)>,
) -> Reply {
    let mut request = server.method(
        method,
        &format!("/api/v1/households/{household}/profile/avatar"),
    );
    if let Some(bearer) = bearer {
        request = request.add_header("authorization", format!("Bearer {bearer}"));
    }
    if let Some((bytes, content_type)) = image {
        request = request
            .content_type("multipart/form-data; boundary=avatar-boundary")
            .bytes(multipart(bytes, content_type));
    }
    let response = request.await;
    collect(
        response.status_code().as_u16(),
        response.as_bytes().to_vec(),
        response.headers(),
    )
}

fn json_body(reply: &Reply) -> Value {
    serde_json::from_slice(&reply.bytes).unwrap()
}

fn assert_same_png_pixels(original: &[u8], returned: &[u8]) {
    assert!(returned.starts_with(b"\x89PNG\r\n\x1a\n"));
    let expected = image::load_from_memory(original).unwrap();
    let actual = image::load_from_memory(returned).unwrap();
    assert_eq!(
        (actual.width(), actual.height()),
        (expected.width(), expected.height())
    );
    assert_eq!(actual.to_rgba8(), expected.to_rgba8());
}

fn assert_json(method: &str, reply: &Reply) {
    let document = contract::contract();
    let raw = &document["paths"][PATH][method]["responses"][reply.status.to_string()];
    let described = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    assert!(
        described.is_object(),
        "{method} avatar {} undocumented",
        reply.status
    );
    contract::assert_value(
        document,
        &described["content"]["application/json"]["schema"],
        &json_body(reply),
        &format!("{method} avatar {}", reply.status),
    );
    assert_eq!(reply.content_type.as_deref(), Some("application/json"));
}

fn assert_private(reply: &Reply) {
    assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
    if reply.status != 429 {
        assert_eq!(reply.nosniff.as_deref(), Some("nosniff"));
    }
    assert!(
        reply
            .request_id
            .as_ref()
            .is_some_and(|value| !value.is_empty())
    );
    if reply.content_type.as_deref() == Some("application/json")
        && reply.status != 429
        && json_body(reply).get("error").is_some()
    {
        assert_eq!(
            json_body(reply)["error"]["request_id"],
            reply.request_id.as_deref().unwrap()
        );
    }
}

async fn avatar_state(fixture: &Fixture) -> Value {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM active_storage_attachments WHERE record_type='Person' AND record_id=73001 AND name='avatar') AS attachments,(SELECT count(*) FROM active_storage_blobs) AS blobs,(SELECT count(*) FROM versions WHERE item_type='Person') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Person') AS changes")).await.unwrap().unwrap();
    json!({"attachments":row.try_get::<i64>("","attachments").unwrap(),"blobs":row.try_get::<i64>("","blobs").unwrap(),"versions":row.try_get::<i64>("","versions").unwrap(),"changes":row.try_get::<i64>("","changes").unwrap()})
}

async fn owned_avatar_fixture() -> (Fixture, BootResult) {
    let bucket = format!("medtracker-fixture-{}", uuid::Uuid::new_v4().simple());
    crate::avatar_storage::create_owned_bucket(&bucket).await;
    boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["avatar_storage"]["bucket"] =
            json!(bucket);
    })
    .await
}

#[tokio::test]
async fn avatar_upload_download_delete_use_real_owned_storage_and_private_wire() {
    let (fixture, boot) = owned_avatar_fixture().await;
    profile::seed_credentials(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let mobile = profile::mobile_token(&fixture).await;
    let png = STANDARD.decode(AVATAR).unwrap();
    let server = server(&boot);
    let missing = send(&server, Method::GET, 72001, Some("profile-session"), None).await;
    let mut rotations = Vec::new();
    for (name, bearer) in [
        ("api", "profile-session"),
        ("app", "profile-app"),
        ("mobile", mobile.as_str()),
    ] {
        let uploaded = send(
            &server,
            Method::PUT,
            72001,
            Some(bearer),
            Some((&png, "image/png")),
        )
        .await;
        let after_upload = avatar_state(&fixture).await;
        let download = send(&server, Method::GET, 72001, Some(bearer), None).await;
        let deleted = send(&server, Method::DELETE, 72001, Some(bearer), None).await;
        let after_delete = avatar_state(&fixture).await;
        rotations.push((
            name,
            uploaded,
            after_upload,
            download,
            deleted,
            after_delete,
        ));
    }
    let gone = send(&server, Method::GET, 72001, Some("profile-session"), None).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(missing.status, 404);
    assert_json("get", &missing);
    for (name, uploaded, after_upload, download, deleted, after_delete) in rotations {
        assert_eq!(uploaded.status, 200, "{name} upload");
        assert_json("put", &uploaded);
        assert_eq!(json_body(&uploaded)["data"]["avatar_attached"], true);
        assert_eq!(after_upload["attachments"], 1, "{name} upload");
        assert_eq!(download.status, 200, "{name} download");
        assert_eq!(download.content_type.as_deref(), Some("image/png"));
        assert_same_png_pixels(&png, &download.bytes);
        assert_eq!(deleted.status, 204, "{name} delete");
        assert!(deleted.bytes.is_empty());
        assert!(deleted.content_type.is_none());
        assert_eq!(after_delete["attachments"], 0, "{name} delete");
        for reply in [&uploaded, &download, &deleted] {
            assert_private(reply);
        }
    }
    assert_eq!(gone.status, 404);
    assert_json("get", &gone);
    for reply in [&missing, &gone] {
        assert_private(reply);
    }
}

#[tokio::test]
async fn concurrent_avatar_replace_and_remove_retain_only_the_winning_object() {
    use loco_rs::bgworker::BackgroundWorker;
    use med_tracker::models::profile::avatar::{AvatarRetirementWorker, RetiredAvatar, Storage};

    let (fixture, boot) = owned_avatar_fixture().await;
    profile::seed_credentials(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let png = STANDARD.decode(AVATAR).unwrap();
    let server = server(&boot);
    let initial = send(
        &server,
        Method::PUT,
        72001,
        Some("profile-session"),
        Some((&png, "image/png")),
    )
    .await;
    let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let (replaced, removed, overlapped) = super::overlap_requests_at_row_lock(
        &fixture,
        "SELECT id FROM people WHERE id=73001 FOR UPDATE",
        "%people%FOR UPDATE%",
        async {
            start.wait().await;
            send(
                &server,
                Method::PUT,
                72001,
                Some("profile-session"),
                Some((&png, "image/png")),
            )
            .await
        },
        async {
            start.wait().await;
            send(&server, Method::DELETE, 72001, Some("profile-app"), None).await
        },
    )
    .await;
    let state = avatar_state(&fixture).await;
    let attached_key = fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT b.key FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id WHERE a.record_type='Person' AND a.record_id=73001 AND a.name='avatar'",
    )).await.unwrap().map(|row| row.try_get::<String>("", "key").unwrap());
    let jobs = fixture
        .admin
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT task_data FROM pg_loco_queue WHERE name='AvatarRetirementWorker'",
        ))
        .await
        .unwrap();
    let worker = AvatarRetirementWorker::build(&boot.app_context);
    let storage = Storage::configured(&boot.app_context).unwrap();
    let mut retirements = Vec::new();
    for row in jobs {
        let payload: Value = row.try_get("", "task_data").unwrap();
        let retired: RetiredAvatar = serde_json::from_value(payload).unwrap();
        let first = worker.perform(retired.clone()).await;
        let second = worker.perform(retired.clone()).await;
        let absent = storage.get(&retired.key).await.is_err();
        retirements.push((first.is_ok(), second.is_ok(), absent));
    }
    let active_object = if let Some(key) = attached_key.as_ref() {
        storage.get(key).await.ok()
    } else {
        None
    };
    let read = send(&server, Method::GET, 72001, Some("profile-session"), None).await;
    drop(worker);
    drop(server);
    close(fixture, boot).await;
    assert!(overlapped, "PUT and DELETE reached the same person lock");
    assert_eq!(initial.status, 200);
    assert_eq!(replaced.status, 200);
    assert_eq!(removed.status, 204);
    assert!(removed.bytes.is_empty());
    assert_private(&initial);
    assert_private(&replaced);
    assert_private(&removed);
    assert_json("put", &initial);
    assert_json("put", &replaced);
    assert!(!retirements.is_empty());
    assert!(
        retirements
            .iter()
            .all(|(first, second, absent)| *first && *second && *absent)
    );
    assert_eq!(
        state["attachments"],
        json!(i64::from(attached_key.is_some()))
    );
    assert_eq!(state["blobs"], state["attachments"]);
    if let Some(bytes) = active_object {
        assert_same_png_pixels(&png, &bytes);
        assert_eq!(read.status, 200);
        assert_same_png_pixels(&png, &read.bytes);
    } else {
        assert!(attached_key.is_none());
        assert_eq!(read.status, 404);
        assert_json("get", &read);
    }
    assert_private(&read);
}

#[tokio::test]
async fn avatar_methods_separate_own_foreign_absent_and_invalid_credentials() {
    let (fixture, boot) = owned_avatar_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let png = STANDARD.decode(AVATAR).unwrap();
    let server = server(&boot);
    let before = avatar_state(&fixture).await;
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        for (name, bearer) in [
            ("api", "profile-session"),
            ("app", "profile-app"),
            ("mobile", mobile.as_str()),
        ] {
            for (household, expected) in [(72004, 403), (99999, 404)] {
                let reply = send(
                    &server,
                    method.clone(),
                    household,
                    Some(bearer),
                    (method == Method::PUT).then_some((png.as_slice(), "image/png")),
                )
                .await;
                replies.push((
                    method.as_str().to_lowercase(),
                    name,
                    household,
                    expected,
                    reply,
                ));
            }
        }
        for household in [72001, 99999] {
            let reply = send(
                &server,
                method.clone(),
                household,
                None,
                (method == Method::PUT).then_some((png.as_slice(), "image/png")),
            )
            .await;
            replies.push((
                method.as_str().to_lowercase(),
                "anonymous",
                household,
                401,
                reply,
            ));
        }
    }
    let after = avatar_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, name, household, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method} {name} {household}");
        assert_json(&method, &reply);
        assert_private(&reply);
        assert!(json_body(&reply).get("data").is_none());
    }
}

#[tokio::test]
async fn avatar_methods_reject_wrong_namespace_scope_and_revoked_credentials() {
    let (fixture, boot) = owned_avatar_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let smart = "synthetic-profile-smart";
    profile::seed_disallowed_grants(&fixture, smart).await;
    let png = STANDARD.decode(AVATAR).unwrap();
    let before = avatar_state(&fixture).await;
    let server = server(&boot);
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        for (name, bearer, expected) in [
            ("invalid", "invalid", 401),
            ("smart", smart, 401),
            ("mobile_scope", mobile.as_str(), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                name,
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(bearer),
                    (method == Method::PUT).then_some((png.as_slice(), "image/png")),
                )
                .await,
            ));
        }
    }
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88031; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88032; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        for (name, bearer) in [
            ("api", "profile-session"),
            ("app", "profile-app"),
            ("mobile", mobile.as_str()),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                name,
                401,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(bearer),
                    (method == Method::PUT).then_some((png.as_slice(), "image/png")),
                )
                .await,
            ));
        }
    }
    let after = avatar_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, name, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method} {name}");
        assert_json(&method, &reply);
        assert_private(&reply);
        assert!(json_body(&reply).get("data").is_none());
    }
}

#[tokio::test]
async fn avatar_methods_recheck_stale_permissions_and_membership() {
    let (fixture, boot) = owned_avatar_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let png = STANDARD.decode(AVATAR).unwrap();
    let before = avatar_state(&fixture).await;
    let server = server(&boot);
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET permissions_version=2 WHERE id=74001")
        .await
        .unwrap();
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        for (name, bearer) in [("api", "profile-session"), ("app", "profile-app")] {
            replies.push((
                method.as_str().to_lowercase(),
                name,
                401,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(bearer),
                    (method == Method::PUT).then_some((png.as_slice(), "image/png")),
                )
                .await,
            ));
        }
    }
    fixture.admin.execute_unprepared("UPDATE household_memberships SET permissions_version=1,revoked_at=now() WHERE id=74001").await.unwrap();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        for (name, bearer, expected) in [
            ("api", "profile-session", 401),
            ("app", "profile-app", 401),
            ("mobile", mobile.as_str(), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                name,
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(bearer),
                    (method == Method::PUT).then_some((png.as_slice(), "image/png")),
                )
                .await,
            ));
        }
    }
    let after = avatar_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, name, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method} {name}");
        assert_json(&method, &reply);
        assert_private(&reply);
        assert!(json_body(&reply).get("data").is_none());
    }
}

#[tokio::test]
async fn avatar_malformed_multipart_does_not_replace_current_image() {
    let (fixture, boot) = owned_avatar_fixture().await;
    profile::seed_credentials(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let png = STANDARD.decode(AVATAR).unwrap();
    let server = server(&boot);
    let original = send(
        &server,
        Method::PUT,
        72001,
        Some("profile-session"),
        Some((&png, "image/png")),
    )
    .await;
    let before = avatar_state(&fixture).await;
    let mut invalid = Vec::new();
    for (bytes, content_type) in [
        (b"not-an-image".as_slice(), "image/png"),
        (png.as_slice(), "text/html"),
        (&[][..], "image/png"),
    ] {
        invalid.push(
            send(
                &server,
                Method::PUT,
                72001,
                Some("profile-session"),
                Some((bytes, content_type)),
            )
            .await,
        );
    }
    let after = avatar_state(&fixture).await;
    let retained = send(&server, Method::GET, 72001, Some("profile-session"), None).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(original.status, 200);
    assert_eq!(before, after);
    for reply in invalid {
        assert_eq!(reply.status, 422);
        assert_json("put", &reply);
        assert_private(&reply);
    }
    assert_eq!(retained.status, 200);
    assert_same_png_pixels(&png, &retained.bytes);
    assert_private(&retained);
}

#[tokio::test]
async fn avatar_methods_share_fixed_general_limit_without_storage_effects() {
    let (fixture, boot) = boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] =
            json!({"fixed_now_unix_seconds":1000,"trusted_proxy_ips":""});
    })
    .await;
    profile::seed_credentials(&fixture).await;
    let server = server(&boot);
    for _ in 0..300 {
        assert_eq!(
            server
                .get("/api/v1/capabilities")
                .await
                .status_code()
                .as_u16(),
            200
        );
    }
    let before = avatar_state(&fixture).await;
    let png = STANDARD.decode(AVATAR).unwrap();
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                Some("profile-session"),
                (method == Method::PUT).then_some((png.as_slice(), "image/png")),
            )
            .await,
        ));
    }
    let after = avatar_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(after, before);
    for (method, reply) in replies {
        assert_eq!(reply.status, 429, "{method}");
        assert_eq!(reply.retry_after.as_deref(), Some("200"));
        assert_eq!(reply.limit.as_deref(), Some("300"));
        assert_eq!(reply.remaining.as_deref(), Some("0"));
        assert_eq!(reply.reset.as_deref(), Some("1200"));
        assert_json(&method, &reply);
        assert_private(&reply);
    }
}

#[tokio::test]
async fn avatar_missing_owned_bucket_returns_declared_unavailable_without_attachment() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let bucket = format!("medtracker-fixture-{}", uuid::Uuid::new_v4().simple());
    let (fixture, boot) = boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["avatar_storage"]["bucket"] =
            json!(bucket);
    })
    .await;
    profile::seed_credentials(&fixture).await;
    let server = server(&boot);
    let png = STANDARD.decode(AVATAR).unwrap();
    let before = avatar_state(&fixture).await;
    let failure = send(
        &server,
        Method::PUT,
        72001,
        Some("profile-session"),
        Some((&png, "image/png")),
    )
    .await;
    let after = avatar_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(failure.status, 503);
    assert_eq!(json_body(&failure)["error"]["code"], "avatar_unavailable");
    assert_eq!(before, after);
    assert_json("put", &failure);
    assert_private(&failure);
}
