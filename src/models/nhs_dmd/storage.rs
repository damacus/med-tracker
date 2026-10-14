use crate::models::errors::OperationError;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use loco_rs::{app::AppContext, storage::drivers::StoreDriver};
use md5::{Digest, Md5};
use std::{path::Path, sync::Arc, time::Duration};

pub const MAX_ARCHIVE_BYTES: usize = 500 * 1024 * 1024;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ArchiveStore {
    inner: Arc<loco_rs::storage::Storage>,
}

impl ArchiveStore {
    pub fn configured(ctx: &AppContext) -> Result<Self, OperationError> {
        ctx.storage
            .as_store(crate::models::storage::DMD_STORE)
            .ok_or(OperationError::Unavailable)?;
        Ok(Self {
            inner: ctx.storage.clone(),
        })
    }

    fn driver(&self) -> Result<&dyn StoreDriver, OperationError> {
        self.inner
            .as_store_err(crate::models::storage::DMD_STORE)
            .map_err(|_| OperationError::Unavailable)
    }

    pub async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), OperationError> {
        tokio::time::timeout(
            OPERATION_TIMEOUT,
            self.driver()?.upload(Path::new(key), &bytes.into()),
        )
        .await
        .map_err(|_| OperationError::Unavailable)?
        .map(|_| ())
        .map_err(|_| OperationError::Unavailable)
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, OperationError> {
        let mut stream =
            tokio::time::timeout(OPERATION_TIMEOUT, self.driver()?.get_stream(Path::new(key)))
                .await
                .map_err(|_| OperationError::Unavailable)?
                .map_err(|_| OperationError::Unavailable)?;
        let mut bytes = Vec::new();
        let collect = async {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|_| OperationError::Unavailable)?;
                if bytes.len().saturating_add(chunk.len()) > MAX_ARCHIVE_BYTES {
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
        tokio::time::timeout(OPERATION_TIMEOUT, self.driver()?.delete(Path::new(key)))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .map_err(|_| OperationError::Unavailable)
    }
}

pub fn checksum(bytes: &[u8]) -> String {
    STANDARD.encode(Md5::digest(bytes))
}
