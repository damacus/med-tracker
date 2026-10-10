use super::*;
use crate::models::entities::{active_storage_blob, security_audit_event};
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region, timeout::TimeoutConfig},
    primitives::ByteStream,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageFormat, ImageReader, Limits, imageops::FilterType};
use loco_rs::{Error, app::AppContext, bgworker::BackgroundWorker};
use md5::{Digest, Md5};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, time::Duration};
use tokio::io::AsyncReadExt;

const MAX_UPLOAD: usize = 5 * 1024 * 1024;

#[cfg(test)]
mod bounded_read_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn avatar_read_stops_an_oversized_stream_before_consuming_it() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let sent = Arc::new(AtomicUsize::new(0));
        let observed = sent.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(socket.read_u8().await.unwrap());
            }
            socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: image/png\r\nConnection: close\r\n\r\n").await.unwrap();
            let chunk = vec![0_u8; 65536];
            for _ in 0..1024 {
                if socket.write_all(b"10000\r\n").await.is_err()
                    || socket.write_all(&chunk).await.is_err()
                    || socket.write_all(b"\r\n").await.is_err()
                {
                    return;
                }
                sent.fetch_add(1, Ordering::SeqCst);
            }
            let _ = socket.write_all(b"0\r\n\r\n").await;
        });
        let config = aws_sdk_s3::Config::builder()
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "synthetic",
                "synthetic",
                None,
                None,
                "avatar-bound-test",
            ))
            .endpoint_url(format!("http://{address}"))
            .force_path_style(true)
            .behavior_version(BehaviorVersion::latest())
            .build();
        let storage = Storage {
            client: Client::from_conf(config),
            bucket: "synthetic".into(),
        };
        assert!(storage.get("oversized").await.is_err());
        assert!(
            observed.load(Ordering::SeqCst) < 512,
            "Oversized avatar response was consumed in full"
        );
        server.abort();
    }
}

pub struct Image {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub checksum: String,
}

pub struct Attachment {
    pub key: String,
    pub content_type: String,
    pub service_name: String,
}

pub struct Storage {
    client: Client,
    bucket: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RetiredAvatar {
    pub bucket: String,
    pub key: String,
}

pub struct AvatarRetirementWorker {
    ctx: AppContext,
}

#[async_trait::async_trait]
impl BackgroundWorker<RetiredAvatar> for AvatarRetirementWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, retired: RetiredAvatar) -> loco_rs::Result<()> {
        let mut storage = Storage::configured(&self.ctx)
            .map_err(|_| Error::string("Avatar storage unavailable"))?;
        storage.bucket = retired.bucket;
        delete_unretained(&self.ctx, &storage, &retired.key)
            .await
            .map_err(|_| Error::string("Avatar retirement failed"))
    }
}

async fn lock_avatar_key<C: ConnectionTrait>(db: &C, key: &str) -> Result<(), sea_orm::DbErr> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        [format!("medtracker.avatar:{key}").into()],
    ))
    .await?;
    Ok(())
}

async fn delete_unretained(
    ctx: &AppContext,
    storage: &Storage,
    key: &str,
) -> Result<(), OperationError> {
    let transaction = ctx.db.begin().await?;
    lock_avatar_key(&transaction, key).await?;
    let retained = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM public.active_storage_blobs WHERE key=$1) AS retained",
            [key.into()],
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    if !retained.try_get::<bool>("", "retained")? {
        storage
            .delete(key)
            .await
            .map_err(|_| OperationError::Unavailable)?;
    }
    transaction.commit().await?;
    Ok(())
}

impl Storage {
    pub fn configured(ctx: &AppContext) -> Result<Self, OperationError> {
        let endpoint =
            std::env::var("ACTIVE_STORAGE_S3_ENDPOINT").map_err(|_| OperationError::Unavailable)?;
        let bucket = ctx
            .config
            .settings
            .as_ref()
            .and_then(|settings| settings.get("avatar_storage"))
            .and_then(|avatar| avatar.get("bucket"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| std::env::var("ACTIVE_STORAGE_S3_BUCKET").ok())
            .ok_or(OperationError::Unavailable)?;
        let region =
            std::env::var("ACTIVE_STORAGE_S3_REGION").map_err(|_| OperationError::Unavailable)?;
        let access_key = std::env::var("ACTIVE_STORAGE_S3_ACCESS_KEY_ID")
            .map_err(|_| OperationError::Unavailable)?;
        let secret_key = std::env::var("ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY")
            .map_err(|_| OperationError::Unavailable)?;
        let path_style = std::env::var("ACTIVE_STORAGE_S3_FORCE_PATH_STYLE")
            .map_or(true, |value| value == "true");
        if bucket.is_empty() || region.is_empty() || access_key.is_empty() || secret_key.is_empty()
        {
            return Err(OperationError::Unavailable);
        }
        let config = aws_sdk_s3::Config::builder()
            .region(Region::new(region))
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "medtracker-avatar-storage",
            ))
            .endpoint_url(endpoint)
            .force_path_style(path_style)
            .behavior_version(BehaviorVersion::latest())
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(10))
                    .build(),
            )
            .build();
        Ok(Self {
            client: Client::from_conf(config),
            bucket,
        })
    }

    pub async fn put(&self, key: &str, image: &Image) -> Result<(), OperationError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(image.content_type)
            .body(ByteStream::from(image.bytes.clone()))
            .send()
            .await
            .map(|_| ())
            .map_err(|_| OperationError::Unavailable)
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, OperationError> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        if object
            .content_length()
            .is_some_and(|length| length > 8 * 1024 * 1024)
        {
            return Err(OperationError::Unavailable);
        }
        let mut reader = object.body.into_async_read().take(8 * 1024 * 1024 + 1);
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), reader.read_to_end(&mut bytes))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .map_err(|_| OperationError::Unavailable)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(OperationError::Unavailable);
        }
        Ok(bytes)
    }

    pub async fn delete(&self, key: &str) -> Result<(), OperationError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|_| OperationError::Unavailable)
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}

pub async fn discard_upload(ctx: &AppContext, storage: &Storage, key: &str) {
    if delete_unretained(ctx, storage, key).await.is_err() {
        tracing::error!("Avatar upload cleanup failed; queuing retry");
        let retired = RetiredAvatar {
            bucket: storage.bucket().to_owned(),
            key: key.to_owned(),
        };
        if AvatarRetirementWorker::perform_later(ctx, retired)
            .await
            .is_err()
        {
            tracing::error!("Avatar upload cleanup could not be queued");
        }
    }
}

pub async fn enqueue_retirement(
    tenant: &TenantTransaction,
    bucket: &str,
    key: &str,
) -> Result<(), OperationError> {
    let payload = json!(RetiredAvatar {
        bucket: bucket.to_owned(),
        key: key.to_owned()
    });
    tenant.transaction().execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)",
        [ulid::Ulid::new().to_string().into(),payload.into(),AvatarRetirementWorker::class_name().into(),0.into()],
    )).await?;
    Ok(())
}

pub fn decode(bytes: &[u8], claimed_type: &str) -> Result<Image, OperationError> {
    if bytes.is_empty() || bytes.len() > MAX_UPLOAD {
        return Err(validation(
            "avatar",
            "must be a PNG, JPEG or WebP image under 5 MiB",
        ));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| validation("avatar", "is invalid"))?;
    let expected = match claimed_type {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/webp" => ImageFormat::WebP,
        _ => return Err(validation("avatar", "must be a PNG, JPEG or WebP image")),
    };
    if reader.format() != Some(expected) {
        return Err(validation("avatar", "content type does not match image"));
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| validation("avatar", "is invalid"))?;
    let decoded = if decoded.width() > 1024 || decoded.height() > 1024 {
        decoded.resize(1024, 1024, FilterType::Lanczos3)
    } else {
        decoded
    };
    let mut output = Cursor::new(Vec::new());
    decoded
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| validation("avatar", "is invalid"))?;
    let bytes = output.into_inner();
    let checksum = STANDARD.encode(Md5::digest(&bytes));
    Ok(Image {
        bytes,
        content_type: "image/png",
        checksum,
    })
}

pub async fn attachment(
    tenant: &TenantTransaction,
    account_id: i64,
    access: PersonAccess,
) -> Result<Option<Attachment>, OperationError> {
    let current = linked_person(tenant, account_id, access).await?;
    person_attachment(tenant, current.id).await
}

pub async fn person_attachment(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<Option<Attachment>, OperationError> {
    crate::models::access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    let attached = active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::RecordId.eq(person_id))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .one(tenant.transaction())
        .await?;
    let Some(attached) = attached else {
        return Ok(None);
    };
    let blob = active_storage_blob::Entity::find_by_id(attached.blob_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Unavailable)?;
    if !matches!(
        blob.service_name.as_str(),
        "s3" | "s3_with_persistent_mirror"
    ) {
        return Err(OperationError::Unavailable);
    }
    Ok(Some(Attachment {
        key: blob.key,
        content_type: blob.content_type.unwrap_or_else(|| "image/png".into()),
        service_name: blob.service_name,
    }))
}

async fn audit(
    tenant: &TenantTransaction,
    current: &person::Model,
    action: &str,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(current.household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set(format!("profile.avatar.{action}")),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"person_id":current.id})),
        audit_context: Set(json!({"actor_account_id":tenant.scope().actor.account_id,"actor_membership_id":tenant.membership().id,"household_id":current.household_id,"request_id":tenant.scope().request_id})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}

async fn lock_current(
    tenant: &TenantTransaction,
    account_id: i64,
) -> Result<person::Model, OperationError> {
    let current = linked_person(tenant, account_id, PersonAccess::Manage).await?;
    person::Entity::find_by_id(current.id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    linked_person(tenant, account_id, PersonAccess::Manage).await
}

pub async fn replace(
    tenant: &TenantTransaction,
    account_id: i64,
    key: String,
    image: &Image,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    let current = lock_current(tenant, account_id).await?;
    lock_avatar_key(tenant.transaction(), &key).await?;
    let previous = attachment(tenant, account_id, PersonAccess::Manage).await?;
    if previous
        .as_ref()
        .is_some_and(|old| old.service_name != "s3")
    {
        return Err(OperationError::Unavailable);
    }
    let now = Utc::now().naive_utc();
    let blob = active_storage_blob::ActiveModel {
        key: Set(key),
        filename: Set("avatar.png".into()),
        content_type: Set(Some(image.content_type.into())),
        byte_size: Set(image.bytes.len() as i64),
        checksum: Set(Some(image.checksum.clone())),
        metadata: Set(Some("{}".into())),
        service_name: Set("s3".into()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    if previous.is_some() {
        let attached = active_storage_attachment::Entity::find()
            .filter(active_storage_attachment::Column::RecordType.eq("Person"))
            .filter(active_storage_attachment::Column::RecordId.eq(current.id))
            .filter(active_storage_attachment::Column::Name.eq("avatar"))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Unavailable)?;
        let old_blob = attached.blob_id;
        active_storage_attachment::Entity::delete_by_id(attached.id)
            .exec(tenant.transaction())
            .await?;
        active_storage_blob::Entity::delete_by_id(old_blob)
            .exec(tenant.transaction())
            .await?;
    }
    active_storage_attachment::ActiveModel {
        name: Set("avatar".into()),
        record_type: Set("Person".into()),
        record_id: Set(current.id),
        blob_id: Set(blob.id),
        household_id: Set(current.household_id),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    audit(tenant, &current, "updated").await?;
    if let Some(old) = &previous {
        enqueue_retirement(tenant, bucket, &old.key).await?;
    }
    Ok(previous.map(|old| old.key))
}

pub async fn remove(
    tenant: &TenantTransaction,
    account_id: i64,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    let current = lock_current(tenant, account_id).await?;
    let previous = attachment(tenant, account_id, PersonAccess::Manage).await?;
    if previous
        .as_ref()
        .is_some_and(|old| old.service_name != "s3")
    {
        return Err(OperationError::Unavailable);
    }
    if previous.is_some() {
        let attached = active_storage_attachment::Entity::find()
            .filter(active_storage_attachment::Column::RecordType.eq("Person"))
            .filter(active_storage_attachment::Column::RecordId.eq(current.id))
            .filter(active_storage_attachment::Column::Name.eq("avatar"))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Unavailable)?;
        let old_blob = attached.blob_id;
        active_storage_attachment::Entity::delete_by_id(attached.id)
            .exec(tenant.transaction())
            .await?;
        active_storage_blob::Entity::delete_by_id(old_blob)
            .exec(tenant.transaction())
            .await?;
    }
    audit(tenant, &current, "removed").await?;
    if let Some(old) = &previous {
        enqueue_retirement(tenant, bucket, &old.key).await?;
    }
    Ok(previous.map(|old| old.key))
}
