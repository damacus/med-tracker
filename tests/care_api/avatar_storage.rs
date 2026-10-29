use std::time::Duration;

#[path = "../support/storage_endpoint.rs"]
pub(super) mod storage_endpoint;

pub struct FixtureBucket {
    operator: opendal::Operator,
}

#[allow(dead_code)]
impl FixtureBucket {
    pub async fn key_count(&self) -> usize {
        self.operator
            .list_with("/")
            .recursive(true)
            .await
            .unwrap()
            .iter()
            .filter(|entry| entry.metadata().is_file())
            .count()
    }

    pub async fn content_type(&self, key: &str) -> String {
        self.operator
            .stat(key)
            .await
            .unwrap()
            .content_type()
            .unwrap()
            .to_owned()
    }

    pub async fn get(&self, key: &str) -> opendal::Result<opendal::Buffer> {
        self.operator.read(key).await
    }

    async fn ready(&self) -> bool {
        self.operator.list("/").await.is_ok()
    }
}

#[tokio::test]
#[ignore]
async fn prepare_owned_bucket() {
    ensure_owned_bucket().await;
}

pub async fn ensure_owned_bucket() {
    static READY: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    READY.get_or_init(prepare).await;
}

pub async fn create_owned_bucket(name: &str) -> FixtureBucket {
    assert!(name.starts_with("medtracker-fixture-"));
    assert!(name[19..].bytes().all(|byte| byte.is_ascii_hexdigit()));
    ensure_owned_bucket().await;
    provision_bucket(name).await;
    let bucket = owned_operator(name);
    wait_ready(&bucket, name).await;
    bucket
}

pub fn owned_operator(bucket: &str) -> FixtureBucket {
    let builder = opendal::services::S3::default()
        .bucket(bucket)
        .endpoint(&storage_endpoint::owned_storage_endpoint())
        .region("us-east-1")
        .access_key_id("storage-smoke-access")
        .secret_access_key("storage-smoke-secret");
    let transport = opendal::HttpTransporter::new(
        opendal_http_transport_reqwest::ReqwestTransport::new(reqwest::Client::new()),
    );
    let operator = opendal::Operator::new(builder)
        .expect("S3 operator requires a bucket")
        .with_context(opendal::OperationContext::new().with_http_transport(transport));
    FixtureBucket { operator }
}

async fn prepare() {
    provision_bucket("medtracker-fixture").await;
    let bucket = owned_operator("medtracker-fixture");
    wait_ready(&bucket, "medtracker-fixture").await;
}

async fn provision_bucket(name: &str) {
    let url = format!("{}/{name}", storage_endpoint::owned_storage_endpoint());
    let status = tokio::task::spawn_blocking(move || {
        std::process::Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--output",
                "/dev/null",
                "--aws-sigv4",
                "aws:amz:us-east-1:s3",
                "--user",
                "storage-smoke-access:storage-smoke-secret",
                "--request",
                "PUT",
                &url,
            ])
            .status()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(status.success(), "curl provisioning of {name} failed");
}

async fn wait_ready(bucket: &FixtureBucket, name: &str) {
    for _ in 0..30 {
        if bucket.ready().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("Owned RustFS bucket {name} did not become ready");
}
