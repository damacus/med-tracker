use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};

const AVATAR: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==";

async fn avatar_application() -> Application {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1986-04-17' WHERE id=73001")
        .await
        .unwrap();
    app
}

fn multipart_avatar(bytes: &[u8], content_type: &str) -> Vec<u8> {
    let mut body = b"--avatar-boundary\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"avatar.png\"\r\nContent-Type: ".to_vec();
    body.extend_from_slice(content_type.as_bytes());
    body.extend_from_slice(b"\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--avatar-boundary--\r\n");
    body
}

#[tokio::test]
async fn profile_avatar_api_upload_read_replace_remove_and_recheck_access() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();

    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    if uploaded.status().as_u16() != 200 {
        let status = uploaded.status();
        panic!(
            "avatar upload returned {status}: {}",
            uploaded.text().await.unwrap()
        );
    }
    assert_eq!(uploaded.headers().get("cache-control").unwrap(), "no-store");
    let uploaded: Value = uploaded.json().await.unwrap();
    assert_eq!(uploaded["data"]["avatar_attached"], true);

    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers().get("content-type").unwrap(), "image/png");
    assert_eq!(read.headers().get("cache-control").unwrap(), "no-store");
    assert_eq!(&read.bytes().await.unwrap()[..8], &png[..8]);
    let unauthenticated = app.client.get(&endpoint).send().await.unwrap();
    assert_eq!(unauthenticated.status().as_u16(), 401);

    let replaced = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(replaced.status().as_u16(), 200);
    let attachment_count = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS n FROM active_storage_attachments WHERE record_type='Person' AND record_id=73001 AND name='avatar'"
    )).await.unwrap().unwrap();
    assert_eq!(attachment_count.try_get::<i64>("", "n").unwrap(), 1);

    let removed = app
        .client
        .delete(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(removed.status().as_u16(), 204);
    assert_eq!(removed.headers().get("cache-control").unwrap(), "no-store");
    let missing = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);

    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    for response in [
        app.client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        app.client
            .delete(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        app.client
            .put(&endpoint)
            .bearer_auth(&token)
            .header(
                "content-type",
                "multipart/form-data; boundary=avatar-boundary",
            )
            .body(multipart_avatar(&png, "image/png"))
            .send()
            .await
            .unwrap(),
    ] {
        assert!(matches!(response.status().as_u16(), 403 | 404));
    }
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_rejects_bad_images_without_replacing_existing() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let original = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(original.status().as_u16(), 200);
    for (bytes, content_type) in [
        (b"not a png".to_vec(), "image/png"),
        (png.clone(), "image/jpeg"),
        (vec![0; 5 * 1024 * 1024 + 1], "image/png"),
    ] {
        let rejected = app
            .client
            .put(&endpoint)
            .bearer_auth(&token)
            .header(
                "content-type",
                "multipart/form-data; boundary=avatar-boundary",
            )
            .body(multipart_avatar(&bytes, content_type))
            .send()
            .await
            .unwrap();
        assert_eq!(rejected.status().as_u16(), 422);
        let retained = app
            .client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(retained.status().as_u16(), 200);
        assert_eq!(&retained.bytes().await.unwrap()[..8], &png[..8]);
    }
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_storage_failure_keeps_existing_attachment() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let original = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(original.status().as_u16(), 200);
    let (bad_origin, bad_context, bad_server) = app
        .avatar_bucket_server("medtracker-missing-fixture-bucket")
        .await;
    let bad_endpoint = format!("{bad_origin}/api/v1/households/72001/profile/avatar");
    let failed_upload = app
        .client
        .put(&bad_endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(failed_upload.status().as_u16(), 503);
    let failed_read = app
        .client
        .get(&bad_endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(failed_read.status().as_u16(), 503);
    let retained = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(retained.status().as_u16(), 200);
    assert_eq!(&retained.bytes().await.unwrap()[..8], &png[..8]);
    bad_server.abort();
    let _ = bad_server.await;
    if let Some(queue) = bad_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    bad_context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_audit_failure_rolls_back_metadata_and_cleans_new_object() {
    use crate::avatar_storage::create_owned_bucket;
    let app = avatar_application().await;
    let token = app.token().await;
    let bucket = format!("medtracker-fixture-{}", uuid::Uuid::new_v4().simple());
    let client = create_owned_bucket(&bucket).await;
    let (origin, context, server) = app.avatar_bucket_server(&bucket).await;
    let endpoint = format!("{origin}/api/v1/households/72001/profile/avatar");
    let png = STANDARD.decode(AVATAR).unwrap();
    let original = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(original.status().as_u16(), 200);
    let objects = client
        .list_objects_v2()
        .bucket(&bucket)
        .send()
        .await
        .unwrap();
    assert_eq!(objects.key_count(), Some(1));
    app.fixture.admin.execute_unprepared("CREATE FUNCTION public.fail_profile_avatar_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic avatar audit failure'; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER fail_profile_avatar_audit BEFORE INSERT ON security_audit_events FOR EACH ROW WHEN (NEW.event_type = 'profile.avatar.updated') EXECUTE FUNCTION public.fail_profile_avatar_audit()").await.unwrap();
    let failed = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert!(matches!(failed.status().as_u16(), 500 | 503));
    let objects = client
        .list_objects_v2()
        .bucket(&bucket)
        .send()
        .await
        .unwrap();
    assert_eq!(objects.key_count(), Some(1));
    let rows = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM active_storage_attachments WHERE record_type='Person' AND record_id=73001 AND name='avatar') AS attachments,(SELECT count(*) FROM active_storage_blobs) AS blobs"
    )).await.unwrap().unwrap();
    assert_eq!(rows.try_get::<i64>("", "attachments").unwrap(), 1);
    assert_eq!(rows.try_get::<i64>("", "blobs").unwrap(), 1);
    let retained = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(retained.status().as_u16(), 200);
    server.abort();
    let _ = server.await;
    if let Some(queue) = context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    context.db.close().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_replacement_and_removal_queue_retired_objects_atomically() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    for _ in 0..2 {
        let uploaded = app
            .client
            .put(&endpoint)
            .bearer_auth(&token)
            .header(
                "content-type",
                "multipart/form-data; boundary=avatar-boundary",
            )
            .body(multipart_avatar(&png, "image/png"))
            .send()
            .await
            .unwrap();
        assert_eq!(uploaded.status().as_u16(), 200);
    }
    let retired = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM pg_loco_queue WHERE name='AvatarRetirementWorker'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retired.try_get::<i64>("", "n").unwrap(), 1);
    let removed = app
        .client
        .delete(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(removed.status().as_u16(), 204);
    let retired = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM pg_loco_queue WHERE name='AvatarRetirementWorker'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retired.try_get::<i64>("", "n").unwrap(), 2);
    app.close().await;
}

#[tokio::test]
async fn avatar_retirement_worker_preserves_active_key_and_retries_removed_key_idempotently() {
    use loco_rs::bgworker::BackgroundWorker;
    use med_tracker::models::profile::avatar::{AvatarRetirementWorker, RetiredAvatar};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT key FROM active_storage_blobs ORDER BY id DESC LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    let key: String = row.try_get("", "key").unwrap();
    let retired = RetiredAvatar {
        bucket: "medtracker-fixture".into(),
        key: key.clone(),
    };
    let worker = AvatarRetirementWorker::build(&app.context);
    worker.perform(retired.clone()).await.unwrap();
    assert_eq!(
        app.client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );

    let removed = app
        .client
        .delete(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(removed.status().as_u16(), 204);
    let job = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT id FROM pg_loco_queue WHERE name='AvatarRetirementWorker' ORDER BY run_at DESC LIMIT 1"
    )).await.unwrap().unwrap();
    let job_id: String = job.try_get("", "id").unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE pg_loco_queue SET status='failed' WHERE id='{job_id}'"
        ))
        .await
        .unwrap();
    let queue = app.context.queue_provider.as_ref().unwrap();
    assert_eq!(queue.retry_failed(Some(&job_id)).await.unwrap(), 1);
    worker.perform(retired.clone()).await.unwrap();
    worker.perform(retired).await.unwrap();
    let client = crate::avatar_storage::owned_client();
    assert!(
        client
            .get_object()
            .bucket("medtracker-fixture")
            .key(&key)
            .send()
            .await
            .is_err()
    );
    app.close().await;
}

#[tokio::test]
async fn avatar_retirement_enqueue_failure_preserves_current_attachment() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let first = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 200);
    app.fixture.admin.execute_unprepared("CREATE FUNCTION public.fail_avatar_retirement_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic avatar queue failure'; END $$").await.unwrap();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER fail_avatar_retirement_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN (NEW.name = 'AvatarRetirementWorker') EXECUTE FUNCTION public.fail_avatar_retirement_queue()").await.unwrap();
    let failed = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert!(matches!(failed.status().as_u16(), 500 | 503));
    let retained = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(retained.status().as_u16(), 200);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM active_storage_attachments WHERE record_type='Person' AND record_id=73001 AND name='avatar') AS attachments,(SELECT count(*) FROM active_storage_blobs) AS blobs"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "attachments").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "blobs").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_updates_s3_primary_mirror_without_purging_unmigrated_services() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE active_storage_blobs SET service_name='s3_with_persistent_mirror'",
        )
        .await
        .unwrap();
    assert_eq!(
        app.client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        200
    );
    let replaced = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(replaced.status().as_u16(), 200);
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE active_storage_blobs SET service_name='s3_with_persistent_mirror'",
        )
        .await
        .unwrap();
    assert_eq!(
        app.client
            .delete(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE active_storage_blobs SET service_name='persistent_with_s3_mirror'",
        )
        .await
        .unwrap();
    assert!(matches!(
        app.client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        500 | 503
    ));
    let replacement = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(replacement.status().as_u16(), 500);
    assert_eq!(
        app.client
            .delete(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        500
    );
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id WHERE a.record_id=73001 AND b.service_name='persistent_with_s3_mirror'")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn avatar_failed_reply_cleanup_preserves_a_committed_attachment() {
    use med_tracker::models::profile::avatar::{Storage, discard_upload};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT key FROM active_storage_blobs ORDER BY id DESC LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    let key: String = row.try_get("", "key").unwrap();
    let storage = Storage::configured(&app.context).unwrap();
    let previous = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(previous.status().as_u16(), 200);
    let previous = previous.bytes().await.unwrap();
    discard_upload(&app.context, &storage, &key).await;
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.bytes().await.unwrap(), previous);
    app.close().await;
}

#[tokio::test]
async fn concurrent_avatar_replace_and_remove_leave_one_consistent_attachment_after_retirement() {
    use loco_rs::bgworker::BackgroundWorker;
    use med_tracker::models::profile::avatar::{AvatarRetirementWorker, RetiredAvatar};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let initial = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(initial.status().as_u16(), 200);
    let (replaced, removed) = tokio::join!(
        app.client
            .put(&endpoint)
            .bearer_auth(&token)
            .header(
                "content-type",
                "multipart/form-data; boundary=avatar-boundary"
            )
            .body(multipart_avatar(&png, "image/png"))
            .send(),
        app.client.delete(&endpoint).bearer_auth(&token).send()
    );
    assert_eq!(replaced.unwrap().status().as_u16(), 200);
    assert_eq!(removed.unwrap().status().as_u16(), 204);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS n FROM active_storage_attachments WHERE record_type='Person' AND record_id=73001 AND name='avatar'"
    )).await.unwrap().unwrap();
    let attached: i64 = row.try_get("", "n").unwrap();
    assert!(attached == 0 || attached == 1);
    let jobs = app
        .fixture
        .admin
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT task_data FROM pg_loco_queue WHERE name='AvatarRetirementWorker'",
        ))
        .await
        .unwrap();
    assert!(!jobs.is_empty());
    let worker = AvatarRetirementWorker::build(&app.context);
    for row in jobs {
        let payload: Value = row.try_get("", "task_data").unwrap();
        let retired: RetiredAvatar = serde_json::from_value(payload).unwrap();
        worker.perform(retired.clone()).await.unwrap();
        worker.perform(retired).await.unwrap();
    }
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(
        read.status().as_u16(),
        if attached == 1 { 200 } else { 404 }
    );
    app.close().await;
}

#[tokio::test]
async fn avatar_failed_reply_cleanup_waits_for_pending_attachment_commit() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        profile::avatar::{self, Storage, discard_upload},
    };
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let storage = Storage::configured(&app.context).unwrap();
    let image = avatar::decode(&STANDARD.decode(AVATAR).unwrap(), "image/png").unwrap();
    let key = format!("avatars/73001/{}", uuid::Uuid::new_v4());
    storage.put(&key, &image).await.unwrap();
    let tenant = access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: uuid::Uuid::new_v4().to_string(),
        },
    )
    .await
    .unwrap();
    avatar::replace(&tenant, 71001, key.clone(), &image, storage.bucket())
        .await
        .unwrap();
    let context = app.context.clone();
    let cleanup_key = key.clone();
    let mut cleanup = tokio::spawn(async move {
        let storage = Storage::configured(&context).unwrap();
        discard_upload(&context, &storage, &cleanup_key).await;
    });
    let completed = tokio::time::timeout(std::time::Duration::from_millis(500), &mut cleanup).await;
    tenant.commit().await.unwrap();
    if completed.is_err() {
        tokio::time::timeout(std::time::Duration::from_secs(5), cleanup)
            .await
            .unwrap()
            .unwrap();
    } else {
        completed.unwrap().unwrap();
    }
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.bytes().await.unwrap().as_ref(), image.bytes.as_slice());
    app.close().await;
}
#[tokio::test]
async fn avatar_read_rejects_active_legacy_content_types_without_removing_attachment() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    for content_type in ["image/svg+xml", "text/html", "application/javascript"] {
        app.fixture
            .admin
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE active_storage_blobs SET content_type=$1",
                [content_type.into()],
            ))
            .await
            .unwrap();
        let read = app
            .client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(matches!(read.status().as_u16(), 500 | 503));
        assert_eq!(read.headers().get("cache-control").unwrap(), "no-store");
    }
    app.fixture
        .admin
        .execute_unprepared("UPDATE active_storage_blobs SET content_type='image/png'")
        .await
        .unwrap();
    let retained = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(retained.status().as_u16(), 200);
    app.close().await;
}

#[tokio::test]
async fn avatar_read_prevents_content_sniffing() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers().get("content-type").unwrap(), "image/png");
    assert_eq!(
        read.headers()
            .get("x-content-type-options")
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
    app.close().await;
}

#[tokio::test]
async fn avatar_read_rejects_legacy_non_image_bytes_without_deleting_attachment() {
    use med_tracker::models::profile::avatar::{Image, Storage};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/profile/avatar", app.origin);
    let png = STANDARD.decode(AVATAR).unwrap();
    let uploaded = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header(
            "content-type",
            "multipart/form-data; boundary=avatar-boundary",
        )
        .body(multipart_avatar(&png, "image/png"))
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status().as_u16(), 200);
    let original = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT key FROM active_storage_blobs WHERE key LIKE 'avatars/%'",
        ))
        .await
        .unwrap()
        .unwrap();
    let key: String = row.try_get("", "key").unwrap();
    let storage = Storage::configured(&app.context).unwrap();
    storage
        .put(
            &key,
            &Image {
                bytes: b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>"
                    .to_vec(),
                content_type: "image/png",
                checksum: String::new(),
            },
        )
        .await
        .unwrap();
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert!(matches!(read.status().as_u16(), 500 | 503));
    assert_eq!(read.headers().get("cache-control").unwrap(), "no-store");
    storage
        .put(
            &key,
            &Image {
                bytes: original.to_vec(),
                content_type: "image/png",
                checksum: String::new(),
            },
        )
        .await
        .unwrap();
    let retained = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(retained.status().as_u16(), 200);
    assert_eq!(retained.bytes().await.unwrap(), original);
    app.close().await;
}
