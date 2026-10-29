use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use med_tracker::models::_entities::{
    accounts, active_storage_attachments, active_storage_blobs, household_memberships,
    person_access_grants, versions,
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query, extension::postgres::PgExpr};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter,
};

pub(super) async fn subject_account(fixture: &Fixture, person_id: i64) -> accounts::Model {
    let now = chrono::Utc::now();
    accounts::ActiveModel {
        person_id: Set(person_id),
        email: Set(format!("avatar-{}@example.test", uuid::Uuid::new_v4())),
        status: Set(2),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap()
}

pub(super) async fn view_grant(fixture: &Fixture, person_id: i64) -> person_access_grants::Model {
    let now = chrono::Utc::now().naive_utc();
    person_access_grants::ActiveModel {
        household_id: Set(fixture.household.id),
        household_membership_id: Set(fixture.membership.id),
        person_id: Set(person_id),
        access_level: Set("view".into()),
        relationship_type: Set("clinician".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap()
}

pub(super) async fn revoke_membership(
    fixture: &Fixture,
    membership: &household_memberships::Model,
) {
    let mut row = membership.clone().into_active_model();
    row.status = Set("revoked".into());
    row.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    row.permissions_version = Set(membership.permissions_version + 1);
    row.update(&fixture.admin).await.unwrap();
}

pub(super) async fn revoke_grant(fixture: &Fixture) {
    let mut grant = fixture.grant.clone().into_active_model();
    grant.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    grant.update(&fixture.admin).await.unwrap();
}

pub(super) async fn avatar_key<C: ConnectionTrait>(db: &C, account_id: i64) -> String {
    let query = Query::select()
        .column("key")
        .from("account_avatars")
        .and_where(Expr::col("account_id").eq(account_id))
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "key")
        .unwrap()
}

pub(super) async fn attachment_count<C: ConnectionTrait>(db: &C, account_id: i64) -> i64 {
    let query = Query::select()
        .expr_as(Func::count(Expr::col("account_id")), "count")
        .from("account_avatars")
        .and_where(Expr::col("account_id").eq(account_id))
        .to_owned();
    db.query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

pub(super) async fn avatar_audits<C: ConnectionTrait>(
    db: &C,
    account_id: i64,
) -> Vec<versions::Model> {
    versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Account"))
        .filter(versions::Column::ItemId.eq(account_id))
        .filter(
            Expr::col(versions::Column::AuditContext)
                .cast_json_field("policy_query")
                .eq("update_own_account_avatar"),
        )
        .all(db)
        .await
        .unwrap()
}

pub(super) async fn set_avatar<C: ConnectionTrait>(
    db: &C,
    account_id: i64,
    column: AvatarColumn,
    value: &str,
) {
    let column = match column {
        AvatarColumn::Key => "key",
        AvatarColumn::Service => "service_name",
        AvatarColumn::ContentType => "content_type",
    };
    let query = Query::update()
        .table("account_avatars")
        .value(column, value)
        .and_where(Expr::col("account_id").eq(account_id))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await
        .unwrap();
}

pub(super) enum AvatarColumn {
    Key,
    Service,
    ContentType,
}

pub(super) async fn insert_avatar<C: ConnectionTrait>(db: &C, account_id: i64, key: &str) {
    let query = Query::insert()
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
            Expr::val(account_id),
            Expr::val(key),
            Expr::val("image/png"),
            Expr::val("s3"),
            Expr::val(1_i64),
            Expr::val("synthetic"),
        ])
        .unwrap()
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await
        .unwrap();
}

pub(super) async fn remove_avatar<C: ConnectionTrait>(db: &C, account_id: i64) {
    let query = Query::delete()
        .from_table("account_avatars")
        .and_where(Expr::col("account_id").eq(account_id))
        .to_owned();
    db.execute_raw(db.get_database_backend().build(&query))
        .await
        .unwrap();
}

pub(super) struct RetirementJob {
    pub id: String,
    pub task_data: Value,
}

pub(super) async fn retirement_jobs<C: ConnectionTrait>(
    db: &C,
    bucket: &str,
) -> Vec<RetirementJob> {
    let query = Query::select()
        .columns(["id", "task_data"])
        .from("pg_loco_queue")
        .and_where(Expr::col("name").eq("AvatarRetirementWorker"))
        .and_where(Expr::col("task_data").cast_json_field("bucket").eq(bucket))
        .to_owned();
    db.query_all_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| RetirementJob {
            id: row.try_get("", "id").unwrap(),
            task_data: row.try_get("", "task_data").unwrap(),
        })
        .collect()
}

pub(super) fn bucket(context: &AppContext) -> String {
    med_tracker::models::profile::avatar::Storage::configured(context)
        .unwrap()
        .bucket()
        .to_owned()
}

pub(super) struct FailureTrigger {
    name: String,
    table: &'static str,
}
impl FailureTrigger {
    pub(super) async fn install(fixture: &Fixture, queue: bool, bucket: &str) -> Self {
        let name = format!("avatar_failure_{}", uuid::Uuid::new_v4().simple());
        let table = if queue { "pg_loco_queue" } else { "versions" };
        // SeaORM Schema/Table builders cannot define PostgreSQL trigger functions.
        // Keep this fault injection isolated to the generated account or owned bucket.
        assert!(bucket.starts_with("medtracker-fixture-"));
        assert!(
            bucket
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        );
        let condition = if queue {
            format!("NEW.name='AvatarRetirementWorker' AND NEW.task_data->>'bucket'='{bucket}'")
        } else {
            format!(
                "NEW.item_type='Account' AND NEW.item_id={} AND NEW.audit_context->>'policy_query'='update_own_account_avatar'",
                fixture.account.id
            )
        };
        fixture.admin.execute_unprepared(&format!(
            "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF {condition} THEN RAISE EXCEPTION 'synthetic avatar failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER {name} BEFORE INSERT ON {table} FOR EACH ROW EXECUTE FUNCTION {name}()"
        )).await.unwrap();
        Self { name, table }
    }

    pub(super) async fn remove(self, fixture: &Fixture) {
        fixture
            .admin
            .execute_unprepared(&format!(
                "DROP TRIGGER {0} ON {1}; DROP FUNCTION {0}()",
                self.name, self.table
            ))
            .await
            .unwrap();
    }
}

const AVATAR: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==";

#[tokio::test]
async fn avatar_cleanup_preserves_legacy_blobs_even_with_unknown_storage_service() {
    use med_tracker::models::profile::avatar::{self, Storage};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let storage = Storage::configured(&app.context).unwrap();
    let key = format!("legacy-retained/{}", uuid::Uuid::new_v4());
    let image = avatar::decode(&STANDARD.decode(AVATAR).unwrap(), "image/png").unwrap();
    storage.put(&key, &image).await.unwrap();
    active_storage_blobs::ActiveModel {
        key: Set(key.clone()),
        filename: Set("legacy.png".into()),
        byte_size: Set(1),
        service_name: Set("persistent_with_s3_mirror".into()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    avatar::discard_upload(&app.context, &storage, &key).await;
    assert!(storage.get(&key).await.is_ok());
    app.close().await;
}

#[tokio::test]
async fn accountless_person_avatar_keeps_legacy_read_and_unknown_service_safeguards() {
    use med_tracker::models::{
        access::{self},
        profile::avatar,
    };
    let app = avatar_application().await;
    view_grant(&app.fixture, app.fixture.minor.id).await;
    let key = format!("legacy-accountless-{}", uuid::Uuid::new_v4());
    let blob = active_storage_blobs::ActiveModel {
        key: Set(key.clone()),
        filename: Set("legacy.png".into()),
        content_type: Set(Some("image/png".into())),
        byte_size: Set(1),
        service_name: Set("s3_with_persistent_mirror".into()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    active_storage_attachments::ActiveModel {
        name: Set("avatar".into()),
        record_type: Set("Person".into()),
        record_id: Set(app.fixture.minor.id),
        blob_id: Set(blob.id),
        household_id: Set(app.fixture.household.id),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let scope = app.fixture.scope();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert_eq!(
        avatar::person_attachment(&tenant, app.fixture.minor.id)
            .await
            .unwrap()
            .unwrap()
            .key,
        key
    );
    tenant.commit().await.unwrap();
    let mut changed = blob.clone().into_active_model();
    changed.service_name = Set("persistent_with_s3_mirror".into());
    changed.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert!(
        avatar::person_attachment(&tenant, app.fixture.minor.id)
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
    assert_eq!(
        active_storage_blobs::Entity::find_by_id(blob.id)
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn gravatar_uses_depicted_person_consent_and_rechecks_access() {
    use med_tracker::models::{
        access::{self},
        profile::gravatar,
    };
    use sha2::Digest;
    let app = avatar_application().await;
    let subject = subject_account(&app.fixture, app.fixture.minor.id).await;
    let email = format!(" Avatar-{}@Example.Test ", uuid::Uuid::new_v4());
    let mut person = app.fixture.minor.clone().into_active_model();
    person.email = Set(Some(email.clone()));
    person.update(&app.fixture.admin).await.unwrap();
    let mut actor = app.fixture.account.clone().into_active_model();
    actor.preferences = Set(json!({"gravatar_enabled":true}));
    actor.update(&app.fixture.admin).await.unwrap();
    let grant = view_grant(&app.fixture, app.fixture.minor.id).await;
    let scope = app.fixture.scope();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert!(
        gravatar::for_person(&tenant, app.fixture.minor.id, false)
            .await
            .unwrap()
            .is_none()
    );
    tenant.commit().await.unwrap();
    let mut target = subject.clone().into_active_model();
    target.preferences = Set(json!({"gravatar_enabled":"1"}));
    target.update(&app.fixture.admin).await.unwrap();
    let mut actor = app.fixture.account.clone().into_active_model();
    actor.preferences = Set(json!({"gravatar_enabled":false}));
    actor.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert_eq!(
        gravatar::for_person(&tenant, app.fixture.minor.id, false)
            .await
            .unwrap()
            .unwrap(),
        format!(
            "https://www.gravatar.com/avatar/{}?s=128&d=404&r=g",
            hex::encode(sha2::Sha256::digest(email.trim().to_lowercase().as_bytes()))
        )
    );
    assert!(
        gravatar::for_person(&tenant, app.fixture.minor.id, true)
            .await
            .unwrap()
            .is_none()
    );
    tenant.commit().await.unwrap();
    let mut person = app.fixture.minor.clone().into_active_model();
    person.email = Set(None);
    person.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let fallback = gravatar::for_person(&tenant, app.fixture.minor.id, false)
        .await
        .unwrap()
        .unwrap();
    assert!(str::contains(
        &fallback,
        &hex::encode(sha2::Sha256::digest(subject.email.as_bytes()))
    ));
    tenant.commit().await.unwrap();
    let mut target = subject.clone().into_active_model();
    target.preferences = Set(json!({"gravatar_enabled":false}));
    target.update(&app.fixture.admin).await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert!(
        gravatar::for_person(&tenant, app.fixture.minor.id, false)
            .await
            .unwrap()
            .is_none()
    );
    tenant.commit().await.unwrap();
    let mut revoked = grant.into_active_model();
    revoked.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    revoked.update(&app.fixture.admin).await.unwrap();
    app.fixture.member().await;
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    for uploaded in [false, true] {
        assert!(
            gravatar::for_person(&tenant, app.fixture.minor.id, uploaded)
                .await
                .is_ok()
        );
    }
    tenant.commit().await.unwrap();
    revoke_membership(&app.fixture, &app.fixture.minor_membership).await;
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    for uploaded in [false, true] {
        assert!(
            gravatar::for_person(&tenant, app.fixture.minor.id, uploaded)
                .await
                .is_err()
        );
    }
    tenant.commit().await.unwrap();
    app.close().await;
}

pub(super) async fn avatar_application() -> Application {
    let bucket = format!("medtracker-fixture-{}", uuid::Uuid::new_v4().simple());
    crate::avatar_storage::create_owned_bucket(&bucket).await;
    let app =
        Application::new_configured(Some("synthetic-dose-occurrence-signing-key"), |settings| {
            settings["avatar_storage"]["bucket"] = json!(bucket);
        })
        .await;
    let mut person = app.fixture.adult.clone().into_active_model();
    person.date_of_birth = Set(chrono::NaiveDate::from_ymd_opt(1986, 4, 17));
    person.update(&app.fixture.admin).await.unwrap();
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
    let object = crate::avatar_storage::owned_client()
        .head_object()
        .bucket(bucket(&app.context))
        .key(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(object.content_type().unwrap(), "image/png");
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
    assert_eq!(
        attachment_count(&app.fixture.admin, app.fixture.account.id).await,
        1
    );

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
    revoke_grant(&app.fixture).await;
    let self_read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(self_read.status().as_u16(), 200);
    revoke_membership(&app.fixture, &app.fixture.membership).await;
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
        assert_eq!(response.status().as_u16(), 403);
    }
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_rejects_bad_images_without_replacing_existing() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let missing_bucket = format!("medtracker-missing-{}", uuid::Uuid::new_v4().simple());
    let (bad_origin, bad_context, bad_server) = app.avatar_bucket_server(&missing_bucket).await;
    let bad_endpoint = format!(
        "{bad_origin}/api/v1/households/{}/profile/avatar",
        app.fixture.household.id
    );
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
    let endpoint = format!(
        "{origin}/api/v1/households/{}/profile/avatar",
        app.fixture.household.id
    );
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
    let original_key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
    let failure = FailureTrigger::install(&app.fixture, false, &bucket).await;
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
    failure.remove(&app.fixture).await;
    assert!(matches!(failed.status().as_u16(), 500 | 503));
    let objects = client
        .list_objects_v2()
        .bucket(&bucket)
        .send()
        .await
        .unwrap();
    assert_eq!(objects.key_count(), Some(1));
    let attached = attachment_count(&app.fixture.admin, app.fixture.account.id).await;
    assert_eq!(attached, 1);
    assert_eq!(
        avatar_key(&app.fixture.admin, app.fixture.account.id).await,
        original_key
    );
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    assert_eq!(
        retirement_jobs(&app.fixture.admin, &bucket(&app.context))
            .await
            .len(),
        1
    );
    let removed = app
        .client
        .delete(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(removed.status().as_u16(), 204);
    assert_eq!(
        retirement_jobs(&app.fixture.admin, &bucket(&app.context))
            .await
            .len(),
        2
    );
    app.close().await;
}

#[tokio::test]
async fn avatar_retirement_worker_preserves_active_key_and_retries_removed_key_idempotently() {
    use loco_rs::bgworker::BackgroundWorker;
    use med_tracker::models::profile::avatar::{AvatarRetirementWorker, RetiredAvatar};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
    let retired = RetiredAvatar {
        bucket: bucket(&app.context),
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
    let jobs = retirement_jobs(&app.fixture.admin, &bucket(&app.context)).await;
    assert_eq!(jobs.len(), 1);
    let job_id = jobs.into_iter().next().unwrap().id;
    revoke_grant(&app.fixture).await;
    let failed = Query::update()
        .table("pg_loco_queue")
        .value("status", "failed")
        .and_where(Expr::col("id").eq(&job_id))
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&failed))
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
            .bucket(bucket(&app.context))
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let original_key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
    let failure = FailureTrigger::install(&app.fixture, true, &bucket(&app.context)).await;
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
    failure.remove(&app.fixture).await;
    assert!(matches!(failed.status().as_u16(), 500 | 503));
    let retained = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(retained.status().as_u16(), 200);
    let attached = attachment_count(&app.fixture.admin, app.fixture.account.id).await;
    assert_eq!(attached, 1);
    assert_eq!(
        avatar_key(&app.fixture.admin, app.fixture.account.id).await,
        original_key
    );
    app.close().await;
}

#[tokio::test]
async fn profile_avatar_api_updates_s3_primary_mirror_without_purging_unmigrated_services() {
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    set_avatar(
        &app.fixture.admin,
        app.fixture.account.id,
        AvatarColumn::Service,
        "s3_with_persistent_mirror",
    )
    .await;
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
    set_avatar(
        &app.fixture.admin,
        app.fixture.account.id,
        AvatarColumn::Service,
        "s3_with_persistent_mirror",
    )
    .await;
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
    set_avatar(
        &app.fixture.admin,
        app.fixture.account.id,
        AvatarColumn::Service,
        "persistent_with_s3_mirror",
    )
    .await;
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
    assert_eq!(replacement.status().as_u16(), 503);
    assert_eq!(
        app.client
            .delete(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        503
    );
    let query = Query::select()
        .expr_as(Func::count(Expr::col("account_id")), "count")
        .from("account_avatars")
        .and_where(Expr::col("account_id").eq(app.fixture.account.id))
        .and_where(Expr::col("service_name").eq("persistent_with_s3_mirror"))
        .to_owned();
    let row = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
}

#[tokio::test]
async fn avatar_failed_reply_cleanup_preserves_a_committed_attachment() {
    use med_tracker::models::profile::avatar::{Storage, discard_upload};
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let attached = attachment_count(&app.fixture.admin, app.fixture.account.id).await;
    assert!(attached == 0 || attached == 1);
    let jobs = retirement_jobs(&app.fixture.admin, &bucket(&app.context)).await;
    assert!(!jobs.is_empty());
    let worker = AvatarRetirementWorker::build(&app.context);
    for row in jobs {
        let payload = row.task_data;
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
        access::{self},
        profile::avatar::{self, Storage, discard_upload},
    };
    crate::avatar_storage::ensure_owned_bucket().await;
    let app = avatar_application().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
    let storage = Storage::configured(&app.context).unwrap();
    let image = avatar::decode(&STANDARD.decode(AVATAR).unwrap(), "image/png").unwrap();
    let key = format!(
        "avatars/{}/{}",
        app.fixture.account.id,
        uuid::Uuid::new_v4()
    );
    storage.put(&key, &image).await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &app.fixture.scope(), None)
        .await
        .unwrap();
    avatar::replace(
        &tenant,
        app.fixture.account.id,
        key.clone(),
        &image,
        storage.bucket(),
    )
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
        set_avatar(
            &app.fixture.admin,
            app.fixture.account.id,
            AvatarColumn::ContentType,
            content_type,
        )
        .await;
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
    set_avatar(
        &app.fixture.admin,
        app.fixture.account.id,
        AvatarColumn::ContentType,
        "image/png",
    )
    .await;
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let endpoint = format!(
        "{}/api/v1/households/{}/profile/avatar",
        app.origin, app.fixture.household.id
    );
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
    let key = avatar_key(&app.fixture.admin, app.fixture.account.id).await;
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
