use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthRequest, AuthResponse, AuthResult, ResponseBody, middleware::Middleware,
};

pub(super) struct TraceErrors;

#[async_trait]
impl Middleware for TraceErrors {
    fn name(&self) -> &'static str {
        "medtracker-error-trace"
    }
    async fn before_request(&self, _: &AuthRequest) -> AuthResult<Option<AuthResponse>> {
        Ok(None)
    }
    async fn after_request(
        &self,
        _: &AuthRequest,
        response: AuthResponse,
    ) -> AuthResult<AuthResponse> {
        Ok(enrich(response))
    }
}

fn enrich(mut response: AuthResponse) -> AuthResponse {
    if crate::telemetry::ids::current_trace_id().is_none()
        || !(400..600).contains(&response.status)
        || !response.headers.get("content-type").is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return response;
    }
    if let Ok(Some(mut body)) = response.body.json()
        && body.is_object()
    {
        crate::telemetry::ids::enrich_error(&mut body);
        if let Ok(bytes) = serde_json::to_vec(&body) {
            response.body = ResponseBody::Bytes(bytes);
            response.headers.remove("content-length");
        }
    }
    response
}

pub(super) fn materialize(response: AuthResponse) -> axum::response::Response {
    better_auth_core::error::finish_axum_response(response.into_http_response().map(enrich))
}

pub(super) fn error(error: AuthError) -> axum::response::Response {
    better_auth_core::error::finish_axum_response(error.to_http_response().map(enrich))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn typed_error_keeps_status_and_cookies_and_removes_stale_length() {
        let response = AuthResponse::json(
            401,
            &serde_json::json!({"code":"DENIED","message":"Denied"}),
        )
        .unwrap()
        .with_header("set-cookie", "a=b; HttpOnly")
        .with_header("content-length", "1");
        let response = crate::telemetry::ids::hold(
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".into(),
            async { materialize(response) },
        )
        .await;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get("set-cookie").unwrap(),
            "a=b; HttpOnly"
        );
        assert!(response.headers().get("content-length").is_none());
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "DENIED");
        assert_eq!(body["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
    }
}
