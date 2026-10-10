use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region, timeout::TimeoutConfig},
};
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn prepare_owned_bucket() {
    ensure_owned_bucket().await;
}

pub async fn ensure_owned_bucket() {
    static READY: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    READY.get_or_init(prepare).await;
}

pub async fn create_owned_bucket(name: &str) -> Client {
    assert!(name.starts_with("medtracker-fixture-"));
    assert!(name[19..].bytes().all(|byte| byte.is_ascii_hexdigit()));
    ensure_owned_bucket().await;
    let client = owned_client();
    client.create_bucket().bucket(name).send().await.unwrap();
    client
}

pub fn owned_client() -> Client {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    let endpoint = std::env::var("ACTIVE_STORAGE_S3_ENDPOINT").unwrap();
    let parsed = reqwest::Url::parse(&endpoint).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert!(parsed.port().is_some());
    assert_eq!(parsed.path(), "/");
    assert!(parsed.query().is_none() && parsed.fragment().is_none());
    assert!(parsed.username().is_empty() && parsed.password().is_none());
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_ACCESS_KEY_ID").unwrap(),
        "storage-smoke-access"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY").unwrap(),
        "storage-smoke-secret"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_REGION").unwrap(),
        "us-east-1"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_FORCE_PATH_STYLE").unwrap(),
        "true"
    );
    let config = aws_sdk_s3::Config::builder()
        .region(Region::new("us-east-1"))
        .credentials_provider(Credentials::new(
            "storage-smoke-access",
            "storage-smoke-secret",
            None,
            None,
            "owned-avatar-fixture",
        ))
        .endpoint_url(endpoint)
        .force_path_style(true)
        .behavior_version(BehaviorVersion::latest())
        .timeout_config(
            TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(4))
                .build(),
        )
        .build();
    Client::from_conf(config)
}

async fn prepare() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
    let endpoint = std::env::var("ACTIVE_STORAGE_S3_ENDPOINT").unwrap();
    let parsed = reqwest::Url::parse(&endpoint).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert!(parsed.port().is_some());
    assert_eq!(parsed.path(), "/");
    assert!(parsed.query().is_none() && parsed.fragment().is_none());
    assert!(parsed.username().is_empty() && parsed.password().is_none());
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_BUCKET").unwrap(),
        "medtracker-fixture"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_ACCESS_KEY_ID").unwrap(),
        "storage-smoke-access"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY").unwrap(),
        "storage-smoke-secret"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_REGION").unwrap(),
        "us-east-1"
    );
    assert_eq!(
        std::env::var("ACTIVE_STORAGE_S3_FORCE_PATH_STYLE").unwrap(),
        "true"
    );

    let client = owned_client();
    for _ in 0..30 {
        if client
            .head_bucket()
            .bucket("medtracker-fixture")
            .send()
            .await
            .is_ok()
            || client
                .create_bucket()
                .bucket("medtracker-fixture")
                .send()
                .await
                .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("Owned RustFS bucket did not become ready");
}
