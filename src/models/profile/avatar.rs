use super::*;
use crate::models::entities::active_storage_blob;
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
            "SELECT public.avatar_key_retained($1) AS retained",
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

pub async fn enqueue_retirement(
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
    linked_person(tenant, account_id, access).await?;
    account_attachment(
        tenant.transaction(),
        crate::models::ids::AccountId::from_database(account_id),
    )
    .await
}

pub async fn person_attachment(
    tenant: &TenantTransaction,
    person_id: i64,
) -> Result<Option<Attachment>, OperationError> {
    crate::models::access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    let person = person::Entity::find_by_id(person_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if let Some(account_id) = person.account_id {
        return account_attachment(
            tenant.transaction(),
            crate::models::ids::AccountId::from_database(account_id),
        )
        .await;
    }
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

pub(crate) async fn account_attachment(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
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
        if !matches!(service_name.as_str(), "s3" | "s3_with_persistent_mirror") {
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

async fn lock_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
) -> Result<(), OperationError> {
    account::Entity::find_by_id(account_id.get())
        .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
        .one(transaction)
        .await?
        .ok_or(OperationError::Unauthenticated)?;
    crate::models::access::verify_account(transaction, account_id.get()).await
}

async fn audit_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    before: bool,
    after: bool,
) -> Result<(), OperationError> {
    version::ActiveModel {
        item_type: Set("Account".into()),
        item_id: Set(account_id.get()),
        event: Set("update".into()),
        object: Set(Some(json!({"avatar_attached":before}).to_string())),
        object_changes: Set(Some(json!({"avatar_attached":[before,after]}).to_string())),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        audit_context: Set(json!({"actor_account_id":account_id.get(),"request_id":request_id,"server_trace_id":uuid::Uuid::new_v4().to_string(),"policy_query":"update_own_account_avatar"})),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }.insert(transaction).await?;
    Ok(())
}

async fn lock_current(tenant: &TenantTransaction, account_id: i64) -> Result<(), OperationError> {
    let current = linked_person(tenant, account_id, PersonAccess::Manage).await?;
    lock_account(
        tenant.transaction(),
        crate::models::ids::AccountId::from_database(account_id),
    )
    .await?;
    person::Entity::find_by_id(current.id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)?;
    linked_person(tenant, account_id, PersonAccess::Manage).await?;
    Ok(())
}

pub async fn replace(
    tenant: &TenantTransaction,
    account_id: i64,
    key: String,
    image: &Image,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    lock_current(tenant, account_id).await?;
    replace_account_guarded(
        tenant.transaction(),
        crate::models::ids::AccountId::from_database(account_id),
        &tenant.scope().request_id,
        key,
        image,
        bucket,
        Some(tenant),
    )
    .await
}

pub async fn remove(
    tenant: &TenantTransaction,
    account_id: i64,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    lock_current(tenant, account_id).await?;
    remove_account_guarded(
        tenant.transaction(),
        crate::models::ids::AccountId::from_database(account_id),
        &tenant.scope().request_id,
        bucket,
        Some(tenant),
    )
    .await
}

pub(crate) async fn replace_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    key: String,
    image: &Image,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    replace_account_guarded(
        transaction,
        account_id,
        request_id,
        key,
        image,
        bucket,
        None,
    )
    .await
}

async fn recheck_household(
    household: Option<&TenantTransaction>,
    account_id: crate::models::ids::AccountId,
) -> Result<(), OperationError> {
    if let Some(tenant) = household {
        linked_person(tenant, account_id.get(), PersonAccess::Manage).await?;
    }
    Ok(())
}

async fn replace_account_guarded(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    key: String,
    image: &Image,
    bucket: &str,
    household: Option<&TenantTransaction>,
) -> Result<Option<String>, OperationError> {
    lock_account(transaction, account_id).await?;
    let previous = account_attachment(transaction, account_id).await?;
    let mut keys = vec![key.as_str()];
    if let Some(old) = &previous {
        keys.push(&old.key);
    }
    keys.sort_unstable();
    keys.dedup();
    for key in keys {
        lock_avatar_key(transaction, key).await?;
    }
    recheck_household(household, account_id).await?;
    transaction.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO account_avatars(account_id,key,content_type,service_name,byte_size,checksum) VALUES($1,$2,$3,'s3',$4,$5) ON CONFLICT(account_id) DO UPDATE SET key=EXCLUDED.key,content_type=EXCLUDED.content_type,service_name=EXCLUDED.service_name,byte_size=EXCLUDED.byte_size,checksum=EXCLUDED.checksum,updated_at=clock_timestamp()",
        [account_id.get().into(),key.into(),image.content_type.into(),(image.bytes.len() as i64).into(),image.checksum.clone().into()]
    )).await?;
    audit_account(
        transaction,
        account_id,
        request_id,
        previous.is_some(),
        true,
    )
    .await?;
    if let Some(old) = &previous {
        enqueue_retirement(transaction, bucket, &old.key).await?;
    }
    Ok(previous.map(|old| old.key))
}

pub(crate) async fn remove_account(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    bucket: &str,
) -> Result<Option<String>, OperationError> {
    remove_account_guarded(transaction, account_id, request_id, bucket, None).await
}

async fn remove_account_guarded(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    request_id: &str,
    bucket: &str,
    household: Option<&TenantTransaction>,
) -> Result<Option<String>, OperationError> {
    lock_account(transaction, account_id).await?;
    let previous = account_attachment(transaction, account_id).await?;
    if let Some(old) = &previous {
        lock_avatar_key(transaction, &old.key).await?;
    }
    recheck_household(household, account_id).await?;
    if let Some(old) = &previous {
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM account_avatars WHERE account_id=$1",
                [account_id.get().into()],
            ))
            .await?;
        audit_account(transaction, account_id, request_id, true, false).await?;
        enqueue_retirement(transaction, bucket, &old.key).await?;
    }
    Ok(previous.map(|old| old.key))
}
