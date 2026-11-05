use std::time::Duration;

use reqwest::header::RETRY_AFTER;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::USER_AGENT;
use crate::error::{ApiError, ErrorEnvelope};
use crate::models::DataEnvelope;

#[derive(Clone)]
pub struct ApiClient {
    base_url: Url,
    bearer_token: Option<String>,
    http: reqwest::Client,
}

fn optional_header(
    request: reqwest::RequestBuilder,
    header: (&str, Option<&str>),
) -> Result<reqwest::RequestBuilder, ApiError> {
    match header.1 {
        Some(value) => {
            let mut value = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
                .map_err(|_| ApiError::InvalidRequestHeader)?;
            value.set_sensitive(true);
            Ok(request.header(header.0, value))
        }
        None => Ok(request),
    }
}

impl ApiClient {
    #[doc = "# Errors\nThe URL is invalid or has no HTTP server origin."]
    pub fn server_origin(base_url: &str) -> Result<String, ApiError> {
        let parsed = Url::parse(base_url)?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(ApiError::InvalidServerOrigin);
        }
        Ok(parsed.origin().ascii_serialization())
    }

    #[doc = "# Errors\nEither URL cannot be parsed.\nCompares normalised HTTP server origins without paths."]
    pub fn same_origin(left: &str, right: &str) -> Result<bool, ApiError> {
        let left = Url::parse(left)?;
        let right = Url::parse(right)?;
        Ok(matches!(left.scheme(), "http" | "https")
            && matches!(right.scheme(), "http" | "https")
            && left.origin() == right.origin())
    }

    #[doc = "# Errors\nThe base URL is invalid or the HTTP client cannot be built."]
    pub fn new(base_url: impl AsRef<str>, bearer_token: Option<String>) -> Result<Self, ApiError> {
        let mut parsed = Url::parse(base_url.as_ref())?;
        if !parsed.path().ends_with('/') {
            parsed.set_path(&format!("{}/", parsed.path().trim_end_matches('/')));
        }

        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self {
            base_url: parsed,
            bearer_token,
            http,
        })
    }

    #[doc = "# Errors\nThe URL is invalid, the request fails or is rejected, or JSON decoding fails."]
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let request = self.authorize(self.http.get(self.url(path)?));
        self.send_json(request).await
    }

    #[doc = "# Errors\nThe URL is invalid, the request fails or is rejected, or JSON decoding fails."]
    pub async fn post_json<B, T>(&self, path: &str, body: &B) -> Result<T, ApiError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let request = self.authorize(self.http.post(self.url(path)?).json(body));
        self.send_json(request).await
    }

    #[doc = "# Errors\nPosting or decoding the response envelope fails."]
    pub async fn post_json_data<B, T>(&self, path: &str, body: &B) -> Result<T, ApiError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let envelope: DataEnvelope<T> = self.post_json(path, body).await?;
        Ok(envelope.data)
    }

    pub(crate) async fn post_data_with_header<B, T>(
        &self,
        path: &str,
        body: &B,
        header: (&str, Option<&str>),
    ) -> Result<T, ApiError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let request = optional_header(
            self.authorize(self.http.post(self.url(path)?).json(body)),
            header,
        )?;
        let envelope: DataEnvelope<T> = self.send_json(request).await?;
        Ok(envelope.data)
    }

    #[doc = "# Errors\nThe URL is invalid, the request fails, or the server rejects it."]
    pub async fn delete_empty(&self, path: &str) -> Result<(), ApiError> {
        let request = self.authorize(self.http.delete(self.url(path)?));
        let response = request.send().await?;
        if response.status().is_success() {
            return Ok(());
        }

        let retry_after = retry_after_header(&response);
        Err(self
            .decode_error(response.status().as_u16(), retry_after, response)
            .await)
    }

    #[doc = "# Errors\nFetching or decoding the response envelope fails."]
    pub async fn get_data<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let envelope: DataEnvelope<T> = self.get_json(path).await?;
        Ok(envelope.data)
    }

    pub(crate) async fn get_data_with_query<T>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let mut url = self.url(path)?;
        url.query_pairs_mut().extend_pairs(query);
        let request = self.authorize(self.http.get(url));
        let envelope: DataEnvelope<T> = self.send_json(request).await?;
        Ok(envelope.data)
    }

    fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.bearer_token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn url(&self, path: &str) -> Result<Url, ApiError> {
        Ok(self.base_url.join(path.trim_start_matches('/'))?)
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, ApiError> {
        let response = request.send().await?;
        let status = response.status();
        if status.is_success() {
            return Ok(response.json::<T>().await?);
        }

        let retry_after = retry_after_header(&response);
        Err(self
            .decode_error(status.as_u16(), retry_after, response)
            .await)
    }

    async fn decode_error(
        &self,
        status: u16,
        retry_after: Option<String>,
        response: reqwest::Response,
    ) -> ApiError {
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(error) => return ApiError::Http(error),
        };
        if let Ok(envelope) = serde_json::from_slice::<ErrorEnvelope>(&body) {
            ApiError::Server {
                status,
                code: envelope.error.code,
                message: envelope.error.message,
                request_id: envelope.error.request_id,
                trace_id: envelope.error.trace_id,
                retry_after,
            }
        } else {
            let value: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            let text = |key: &str| {
                value
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            };
            let scalar = text("error");
            let oauth_code = scalar
                .as_ref()
                .filter(|error| {
                    matches!(
                        error.as_str(),
                        "invalid_request"
                            | "invalid_client"
                            | "invalid_grant"
                            | "temporarily_unavailable"
                    )
                })
                .cloned();
            let flat = text("code").zip(text("message"));
            let recognised = flat.is_some() || scalar.is_some();
            ApiError::Server {
                status,
                code: flat
                    .as_ref()
                    .map(|(code, _)| code.clone())
                    .or(oauth_code)
                    .unwrap_or_else(|| format!("http_{status}")),
                message: flat
                    .map(|(_, message)| message)
                    .or_else(|| recognised.then(|| text("error_description")).flatten())
                    .unwrap_or_else(|| format!("Request failed (HTTP {status}).")),
                request_id: recognised.then(|| text("request_id")).flatten(),
                trace_id: recognised.then(|| text("trace_id")).flatten(),
                retry_after,
            }
        }
    }
}

fn retry_after_header(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_origin_normalises_host_and_default_port() -> Result<(), ApiError> {
        assert!(ApiClient::same_origin(
            "https://EXAMPLE.com:443/prefix/",
            "https://example.com"
        )?);
        for other in [
            "http://example.com",
            "https://example.com:444",
            "https://other.example",
            "file:///tmp/config",
        ] {
            assert!(!ApiClient::same_origin("https://example.com", other)?);
        }
        assert!(ApiClient::same_origin("https://example.com", "invalid").is_err());
        Ok(())
    }

    #[test]
    fn guardian_import_passphrase_header_is_sensitive() -> Result<(), Box<dyn std::error::Error>> {
        let secret = "synthetic-sensitive-portable-password";
        let request = optional_header(
            reqwest::Client::new().post("http://localhost/"),
            ("x-medtracker-portable-passphrase", Some(secret)),
        )?
        .build()?;
        let sensitive = request
            .headers()
            .get("x-medtracker-portable-passphrase")
            .is_some_and(reqwest::header::HeaderValue::is_sensitive);
        let diagnostics_redacted = !format!("{request:?}").contains(secret);
        assert!(
            sensitive && diagnostics_redacted,
            "Passphrase request safety: sensitive={sensitive}, diagnostics_redacted={diagnostics_redacted}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn guardian_import_invalid_header_is_safe_without_network()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = wiremock::MockServer::start().await;
        let client = ApiClient::new(server.uri(), None)?;
        let result = client
            .portable_import(
                "72001",
                serde_json::json!({"format":"synthetic"}),
                Some("synthetic-sensitive-portable-password\r\ninvalid".to_owned()),
                crate::models::PortableImportMode::Apply,
            )
            .await;
        let error = result.err().ok_or("Invalid passphrase header must fail")?;
        assert_eq!(error.to_string(), "invalid request header");
        assert!(!format!("{error:?}").contains("synthetic-sensitive-portable-password"));
        let requests = server
            .received_requests()
            .await
            .ok_or("Expected captured requests")?;
        assert!(requests.is_empty());
        Ok(())
    }
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn portable_redirects_never_forward_passphrase_or_bundle()
    -> Result<(), Box<dyn std::error::Error>> {
        for status in [307, 308] {
            let source = MockServer::start().await;
            let destination = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})),
                )
                .mount(&destination)
                .await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(status)
                    .insert_header("Location", destination.uri())
                    .set_body_json(serde_json::json!({"error": {"code": "redirect", "message": "API moved", "request_id": "synthetic-redirect"}})))
                .mount(&source).await;
            let result = ApiClient::new(source.uri(), Some("synthetic-token".into()))?
                .portable_import(
                    "1",
                    serde_json::json!({"bundle": "synthetic-private-bundle"}),
                    Some("synthetic-passphrase".into()),
                    crate::PortableImportMode::Apply,
                )
                .await;
            assert!(
                destination
                    .received_requests()
                    .await
                    .ok_or("capture disabled")?
                    .is_empty(),
                "redirect {status} forwarded sensitive request"
            );
            assert!(
                matches!(result, Err(ApiError::Server { status: actual, .. }) if actual == status)
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_json_sends_bearer_and_decodes_data() -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/capabilities"))
            .and(header("authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "format": "medtracker.api.capabilities.v1" }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), Some("test-token".to_string()))?;
        let value: serde_json::Value = client.get_json("/api/v1/capabilities").await?;

        assert_eq!(
            value.pointer("/data/format"),
            Some(&serde_json::json!("medtracker.api.capabilities.v1"))
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_json_decodes_api_error_with_request_id() -> Result<(), Box<dyn std::error::Error>>
    {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/households/1/me"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "code": "unauthorized",
                    "message": "Authentication required",
                    "request_id": "req-test"
                }
            })))
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), None)?;
        let result = client
            .get_json::<serde_json::Value>("/api/v1/households/1/me")
            .await;

        assert!(matches!(
            result,
            Err(
            ApiError::Server {
                status: 401,
                ref code,
                ref request_id,
                ..
            }) if code == "unauthorized" && request_id.as_deref() == Some("req-test")
        ));
        Ok(())
    }

    #[tokio::test]
    async fn error_trace_ids_survive_nested_flat_and_old_server_responses()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        let trace = "4bf92f3577b34da6a3ce929d0e0e4736";
        for (path, body) in [
            (
                "/nested",
                serde_json::json!({"error":{"code":"forbidden","message":"Denied","trace_id":trace}}),
            ),
            (
                "/flat",
                serde_json::json!({"code":"FORBIDDEN","message":"Denied","trace_id":trace}),
            ),
            (
                "/scalar",
                serde_json::json!({"error":"invalid_grant","trace_id":trace}),
            ),
            (
                "/throttle",
                serde_json::json!({"error":"Too many authentication attempts. Try again later.","trace_id":trace}),
            ),
            (
                "/lookup",
                serde_json::json!({"results":[],"error":"Medication search is temporarily unavailable.","trace_id":trace}),
            ),
            (
                "/old",
                serde_json::json!({"error":{"code":"forbidden","message":"Denied"}}),
            ),
            (
                "/unknown",
                serde_json::json!({"private":"sentinel","trace_id":trace}),
            ),
        ] {
            Mock::given(method("GET"))
                .and(wiremock::matchers::path(path))
                .respond_with(ResponseTemplate::new(403).set_body_json(body))
                .mount(&server)
                .await;
        }
        Mock::given(method("GET"))
            .and(wiremock::matchers::path("/malformed"))
            .respond_with(ResponseTemplate::new(403).set_body_string("private non-json"))
            .mount(&server)
            .await;
        let client = ApiClient::new(server.uri(), None)?;
        for path in ["/nested", "/flat", "/scalar", "/throttle", "/lookup"] {
            let result = client.get_json::<serde_json::Value>(path).await;
            assert!(
                matches!(result, Err(ApiError::Server { trace_id: Some(ref id), .. }) if id == trace)
            );
        }
        for path in ["/throttle", "/lookup"] {
            let result = client.get_json::<serde_json::Value>(path).await;
            assert!(
                matches!(result, Err(ApiError::Server { ref code, ref message, .. })
                if code == "http_403" && message == "Request failed (HTTP 403).")
            );
        }
        for path in ["/old", "/unknown", "/malformed"] {
            let result = client.get_json::<serde_json::Value>(path).await;
            assert!(matches!(
                result,
                Err(ApiError::Server { trace_id: None, .. })
            ));
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_json_surfaces_retry_after() -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/households/1/me"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "60")
                    .set_body_json(serde_json::json!({
                        "error": {
                            "code": "rate_limited",
                            "message": "Slow down",
                            "request_id": "req-limited"
                        }
                    })),
            )
            .mount(&server)
            .await;

        let client = ApiClient::new(server.uri(), None)?;
        let result = client
            .get_json::<serde_json::Value>("/api/v1/households/1/me")
            .await;

        assert!(matches!(
            result,
            Err(
            ApiError::Server {
                status: 429,
                ref retry_after,
                ..
            }) if retry_after.as_deref() == Some("60")
        ));
        Ok(())
    }
}
