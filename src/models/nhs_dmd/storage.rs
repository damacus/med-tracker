use crate::models::errors::OperationError;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use loco_rs::app::AppContext;
use md5::{Digest, Md5};
use std::time::Duration;

pub const MAX_ARCHIVE_BYTES: usize = 500 * 1024 * 1024;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ArchiveStore {
    operator: opendal::Operator,
}

impl ArchiveStore {
    pub fn configured(ctx: &AppContext) -> Result<Self, OperationError> {
        ctx.storage
            .as_store(crate::models::storage::DMD_STORE)
            .ok_or_else(|| OperationError::unavailable("storage"))?;
        let operator = ctx
            .shared_store
            .get::<crate::models::storage::Operators>()
            .and_then(|operators| operators.dmd)
            .ok_or_else(|| OperationError::unavailable("storage"))?;
        Ok(Self { operator })
    }

    pub async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), OperationError> {
        tokio::time::timeout(
            OPERATION_TIMEOUT,
            self.operator
                .write_with(key, bytes)
                .content_type("application/zip"),
        )
        .await
        .map_err(OperationError::from)?
        .map(|_| ())
        .map_err(OperationError::from)
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, OperationError> {
        let reader = tokio::time::timeout(OPERATION_TIMEOUT, self.operator.reader(key))
            .await
            .map_err(OperationError::from)?
            .map_err(OperationError::from)?;
        let mut stream = tokio::time::timeout(OPERATION_TIMEOUT, reader.into_bytes_stream(..))
            .await
            .map_err(OperationError::from)?
            .map_err(OperationError::from)?;
        let mut bytes = Vec::new();
        let collect = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(OperationError::from)?;
                if bytes.len().saturating_add(chunk.len()) > MAX_ARCHIVE_BYTES {
                    return Err(OperationError::unavailable("storage"));
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(())
        };
        tokio::time::timeout(OPERATION_TIMEOUT, collect)
            .await
            .map_err(OperationError::from)??;
        Ok(bytes)
    }

    pub async fn delete(&self, key: &str) -> Result<(), OperationError> {
        tokio::time::timeout(OPERATION_TIMEOUT, self.operator.delete(key))
            .await
            .map_err(OperationError::from)?
            .map_err(OperationError::from)
    }
}

pub fn checksum(bytes: &[u8]) -> String {
    STANDARD.encode(Md5::digest(bytes))
}
