mod ai;
mod facts;
mod lookup;
mod push;

pub(super) use ai::ai_medication_suggestions;
pub(super) use lookup::medication_lookup;
pub(super) use push::test_push_subscription;

fn http_client(timeout_seconds: u64) -> Result<reqwest::Client, crate::ApiError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| crate::ApiError::internal())
}

fn configured_url(value: &str, allow_loopback_http: bool) -> Option<url::Url> {
    let url = url::Url::parse(value).ok()?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return None;
    }
    let loopback =
        allow_loopback_http && url.scheme() == "http" && url.host_str() == Some("127.0.0.1");
    if url.scheme() != "https" && !loopback {
        return None;
    }
    Some(url)
}

async fn bounded_json(mut response: reqwest::Response) -> Option<serde_json::Value> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len().checked_add(chunk.len())? > 100_000 {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).ok()
}
