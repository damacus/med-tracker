use crate::models::errors::OperationError;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region},
    primitives::ByteStream,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use md5::{Digest, Md5};

pub const MAX_ARCHIVE_BYTES: usize = 500 * 1024 * 1024;

pub struct ArchiveStore {
    client: Client,
    bucket: String,
}

impl ArchiveStore {
    pub fn configured(settings: Option<&serde_json::Value>) -> Result<Self, OperationError> {
        let required = |key: &str| {
            crate::models::settings::text(settings, &["dmd_storage", key])
                .map(str::to_owned)
                .ok_or(OperationError::Unavailable)
        };
        let endpoint = required("endpoint")?;
        let bucket = required("bucket")?;
        let key = required("access_key_id")?;
        let secret = required("secret_access_key")?;
        let region = crate::models::settings::text(settings, &["dmd_storage", "region"])
            .unwrap_or("us-east-1")
            .to_owned();
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(endpoint)
            .force_path_style(true)
            .region(Region::new(region))
            .credentials_provider(Credentials::new(
                key,
                secret,
                None,
                None,
                "medtracker-archive",
            ))
            .build();
        Ok(Self {
            client: Client::from_conf(config),
            bucket,
        })
    }

    pub async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), OperationError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type("application/zip")
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, OperationError> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        if response
            .content_length()
            .is_some_and(|length| length < 0 || length as usize > MAX_ARCHIVE_BYTES)
        {
            return Err(OperationError::Unavailable);
        }
        let mut stream = response.body;
        let mut bytes = Vec::new();
        while let Some(chunk) = stream
            .try_next()
            .await
            .map_err(|_| OperationError::Unavailable)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_ARCHIVE_BYTES {
                return Err(OperationError::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
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
            .map_err(|_| OperationError::Unavailable)?;
        Ok(())
    }
}

pub fn checksum(bytes: &[u8]) -> String {
    STANDARD.encode(Md5::digest(bytes))
}
