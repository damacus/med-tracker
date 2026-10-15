use super::*;
use crate::models::ids::{AccountId, PersonId};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use image::{ImageFormat, ImageReader, Limits, imageops::FilterType};
use loco_rs::{Error, app::AppContext, bgworker::BackgroundWorker};
use md5::{Digest, Md5};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, time::Duration};

pub(crate) const MAX_UPLOAD: usize = 5 * 1024 * 1024;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(10);

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
        opendal::install_default();
        let operator = opendal::Operator::new(
            opendal::services::S3::default()
                .bucket("synthetic")
                .endpoint(&format!("http://{address}"))
                .region("us-east-1")
                .access_key_id("synthetic")
                .secret_access_key("synthetic"),
        )
        .unwrap();
        let storage = Storage {
            operator,
            bucket: "synthetic".into(),
        };
        assert!(storage.get("oversized").await.is_err());
        let consumed = observed.load(Ordering::SeqCst);
        assert!(consumed > 0, "Avatar read never reached the stream");
        assert!(
            consumed < 512,
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
    operator: opendal::Operator,
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
        let storage = Storage::configured(&self.ctx)
            .and_then(|storage| storage.at_bucket(&self.ctx, &retired.bucket))
            .map_err(|_| Error::string("Avatar storage unavailable"))?;
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
            "SELECT EXISTS(SELECT 1 FROM public.active_storage_blobs WHERE key=$1) OR med_tracker.avatar_key_retained($1) AS retained",
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
    /// # Errors
    ///
    /// Returns an error when the avatar bucket or configured storage operator is unavailable.
    pub fn configured(ctx: &AppContext) -> Result<Self, OperationError> {
        let bucket = crate::models::settings::text(
            ctx.config.settings.as_ref(),
            &["avatar_storage", "bucket"],
        )
        .map(str::to_owned)
        .ok_or(OperationError::Unavailable)?;
        ctx.storage
            .as_store(crate::models::storage::AVATAR_STORE)
            .ok_or(OperationError::Unavailable)?;
        let operator = ctx
            .shared_store
            .get::<crate::models::storage::Operators>()
            .and_then(|operators| operators.avatar)
            .ok_or(OperationError::Unavailable)?;
        Ok(Self { operator, bucket })
    }

    /// # Errors
    ///
    /// Returns an error when the requested bucket has no configured storage operator.
    pub fn at_bucket(&self, ctx: &AppContext, bucket: &str) -> Result<Self, OperationError> {
        if bucket == self.bucket {
            return Ok(Self {
                operator: self.operator.clone(),
                bucket: self.bucket.clone(),
            });
        }
        let operator =
            crate::models::storage::avatar_operator(ctx.config.settings.as_ref(), Some(bucket))
                .ok_or(OperationError::Unavailable)?;
        Ok(Self {
            operator,
            bucket: bucket.to_owned(),
        })
    }

    /// # Errors
    ///
    /// Returns an error if the storage write fails or exceeds the operation timeout.
    pub async fn put(&self, key: &str, image: &Image) -> Result<(), OperationError> {
        tokio::time::timeout(
            OPERATION_TIMEOUT,
            self.operator
                .write_with(key, image.bytes.clone())
                .content_type(image.content_type),
        )
        .await
        .map_err(|_| OperationError::Unavailable)?
        .map(|_| ())
        .map_err(|_| OperationError::Unavailable)
    }

    /// # Errors
    ///
    /// Returns an error if storage access fails, times out, or the image exceeds the read limit.
    pub async fn get(&self, key: &str) -> Result<Vec<u8>, OperationError> {
        const LIMIT: usize = 8 * 1024 * 1024;
        let reader = tokio::time::timeout(OPERATION_TIMEOUT, self.operator.reader(key))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .map_err(|_| OperationError::Unavailable)?;
        let mut stream = tokio::time::timeout(OPERATION_TIMEOUT, reader.into_bytes_stream(..))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .map_err(|_| OperationError::Unavailable)?;
        let mut bytes = Vec::new();
        let collect = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|_| OperationError::Unavailable)?;
                if bytes.len().saturating_add(chunk.len()) > LIMIT {
                    return Err(OperationError::Unavailable);
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(())
        };
        tokio::time::timeout(OPERATION_TIMEOUT, collect)
            .await
            .map_err(|_| OperationError::Unavailable)??;
        Ok(bytes)
    }

    /// # Errors
    ///
    /// Returns an error if the storage delete fails or exceeds the operation timeout.
    pub async fn delete(&self, key: &str) -> Result<(), OperationError> {
        tokio::time::timeout(OPERATION_TIMEOUT, self.operator.delete(key))
            .await
            .map_err(|_| OperationError::Unavailable)?
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

async fn enqueue_retirement(
    transaction: &sea_orm::DatabaseTransaction,
    bucket: &str,
    key: &str,
) -> Result<(), OperationError> {
    let payload = json!(RetiredAvatar {
        bucket: bucket.to_owned(),
        key: key.to_owned()
    });
    transaction.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)",
        [ulid::Ulid::generate().to_string().into(),payload.into(),AvatarRetirementWorker::class_name().into(),0.into()],
    )).await?;
    Ok(())
}

/// # Errors
///
/// Returns a validation error for empty, oversized, unsupported, mismatched, or invalid image data.
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

/// # Errors
///
/// Returns an error if the attachment query or row decoding fails, or its storage service is unsupported.
pub async fn account_attachment(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: AccountId,
) -> Result<Option<Attachment>, OperationError> {
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT key,content_type,service_name FROM account_avatars WHERE account_id=$1",
            [account_id.get().into()],
        ))
        .await?;
    row.map(|row| {
        let service_name: String = row.try_get("", "service_name")?;
        if service_name != "s3" {
            return Err(OperationError::Unavailable);
        }
        Ok(Attachment {
            key: row.try_get("", "key")?,
            content_type: row.try_get("", "content_type")?,
            service_name,
        })
    })
    .transpose()
}

/// # Errors
///
/// Returns an error if account access is denied or the person's attachment cannot be read.
pub async fn attachment(
    tenant: &TenantTransaction,
    account_id: i64,
) -> Result<Option<Attachment>, OperationError> {
    let current = linked_person(tenant, account_id).await?;
    person_attachment(tenant, current.id).await
}

/// # Errors
///
/// Returns an error if household participation is denied, the query or row decoding fails, or the storage service is unsupported.
pub async fn person_attachment(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<Option<Attachment>, OperationError> {
    crate::models::access::require_participation(tenant, person_id).await?;
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.person_avatar($1)",
            [person_id.into()],
        ))
        .await?;
    row.map(|row| {
        let service_name: String = row.try_get("", "service_name")?;
        if service_name != "s3" {
            return Err(OperationError::Unavailable);
        }
        Ok(Attachment {
            key: row.try_get("", "key")?,
            content_type: row.try_get("", "content_type")?,
            service_name,
        })
    })
    .transpose()
}

async fn audit(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: AccountId,
    person_id: PersonId,
    request_id: &str,
    action: &str,
) -> Result<(), OperationError> {
    transaction.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO account_profile_events(account_id,actor_person_id,event_type,request_id,metadata) VALUES($1,$2,$3,$4,$5)",
        [account_id.get().into(),person_id.get().into(),format!("profile.avatar.{action}").into(),request_id.into(),json!({"person_id":person_id.get()}).into()],
    )).await?;
    Ok(())
}

async fn lock_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: AccountId,
) -> Result<PersonId, OperationError> {
    let account = account::Entity::find_by_id(account_id.get())
        .lock_exclusive()
        .one(transaction)
        .await?
        .ok_or(OperationError::Forbidden)?;
    crate::models::access::verify_account_actor(transaction, account.id).await
}

/// # Errors
///
/// Returns an error if account access is denied or the avatar write, audit, or retirement enqueue fails.
pub async fn replace(
    tenant: &TenantTransaction,
    account_id: i64,
    key: String,
    image: &Image,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    linked_person(tenant, account_id).await?;
    replace_account(
        tenant.transaction(),
        AccountId::from_database(account_id),
        &tenant.scope().request_id,
        key,
        image,
        bucket,
    )
    .await
}

/// # Errors
///
/// Returns an error if account access is denied or the avatar removal, audit, or retirement enqueue fails.
pub async fn remove(
    tenant: &TenantTransaction,
    account_id: i64,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    linked_person(tenant, account_id).await?;
    remove_account(
        tenant.transaction(),
        AccountId::from_database(account_id),
        &tenant.scope().request_id,
        bucket,
    )
    .await
}

/// # Errors
///
/// Returns an error if the account cannot be authorised or an attachment, lock, write, audit, or retirement enqueue fails.
pub async fn replace_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: AccountId,
    request_id: &str,
    key: String,
    image: &Image,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    let person_id = lock_account(transaction, account_id).await?;
    let previous = account_attachment(transaction, account_id).await?;
    let mut keys = vec![key.as_str()];
    if let Some(old) = &previous {
        keys.push(&old.key);
    }
    keys.sort_unstable();
    keys.dedup();
    for retained_key in keys {
        lock_avatar_key(transaction, retained_key).await?;
    }
    transaction.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES($1,$2,$3,'s3',$4,$5) ON CONFLICT(account_id) DO UPDATE SET key=EXCLUDED.key,content_type=EXCLUDED.content_type,service_name=EXCLUDED.service_name,byte_size=EXCLUDED.byte_size,checksum=EXCLUDED.checksum,updated_at=clock_timestamp()",
        [account_id.get().into(),key.into(),image.content_type.into(),(image.bytes.len() as i64).into(),image.checksum.clone().into()],
    )).await?;
    audit(transaction, account_id, person_id, request_id, "updated").await?;
    if let Some(old) = &previous {
        enqueue_retirement(transaction, bucket, &old.key).await?;
    }
    Ok(previous.map(|old| old.key))
}

/// # Errors
///
/// Returns an error if the account cannot be authorised or an attachment, lock, delete, audit, or retirement enqueue fails.
pub async fn remove_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: AccountId,
    request_id: &str,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    let person_id = lock_account(transaction, account_id).await?;
    let previous = account_attachment(transaction, account_id).await?;
    if let Some(old) = &previous {
        lock_avatar_key(transaction, &old.key).await?;
    }
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM account_avatars WHERE account_id=$1",
            [account_id.get().into()],
        ))
        .await?;
    audit(transaction, account_id, person_id, request_id, "removed").await?;
    if let Some(old) = &previous {
        enqueue_retirement(transaction, bucket, &old.key).await?;
    }
    Ok(previous.map(|old| old.key))
}
