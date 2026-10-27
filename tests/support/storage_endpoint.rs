pub fn is_owned_storage_endpoint(uri: &url::Url, owns_storage: bool) -> bool {
    let host = match uri.host_str() {
        Some("127.0.0.1") => uri.port().is_some_and(|port| port != 0),
        Some("rustfs") => uri.port() == Some(9000),
        _ => false,
    };
    owns_storage
        && host
        && uri.scheme() == "http"
        && uri.path() == "/"
        && uri.query().is_none()
        && uri.fragment().is_none()
        && uri.username().is_empty()
        && uri.password().is_none()
}

pub fn owned_storage_endpoint() -> String {
    crate::template::require_owned_database();
    let endpoint = std::env::var("ACTIVE_STORAGE_S3_ENDPOINT").unwrap();
    assert!(is_owned_storage_endpoint(
        &url::Url::parse(&endpoint).unwrap(),
        std::env::var("MEDTRACKER_OWNED_STORAGE").as_deref() == Ok("1")
    ));
    for (key, value) in [
        ("ACTIVE_STORAGE_S3_BUCKET", "medtracker-fixture"),
        ("ACTIVE_STORAGE_S3_ACCESS_KEY_ID", "storage-smoke-access"),
        (
            "ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY",
            "storage-smoke-secret",
        ),
        ("ACTIVE_STORAGE_S3_REGION", "us-east-1"),
        ("ACTIVE_STORAGE_S3_FORCE_PATH_STYLE", "true"),
    ] {
        assert_eq!(std::env::var(key).as_deref(), Ok(value), "{key}");
    }
    endpoint
}
