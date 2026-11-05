use super::*;

#[tokio::test]
async fn native_auth_json_errors_carry_current_trace_and_preserve_shape() {
    let app = Application::new().await;
    for (body, expected_status, expected_code) in [
        (
            json!({"email": "missing@example.test", "password": "wrong-password"}),
            401,
            "INVALID_EMAIL_OR_PASSWORD",
        ),
        (
            json!({"email": 7, "password": false}),
            400,
            "VALIDATION_ERROR",
        ),
    ] {
        let response = app
            .client
            .post(format!("{}/api/auth/sign-in/email", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let trace = response
            .headers()
            .get("x-medtracker-trace-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status, expected_status, "{body}");
        assert_eq!(body["code"], expected_code, "{body}");
        assert_eq!(body["trace_id"], trace, "{body}");
        assert!(
            body.get("message").is_some() || body.get("code").is_some(),
            "{body}"
        );
    }
    let response = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", "https://untrusted.example.test")
        .json(&json!({"email":"missing@example.test","password":"wrong-password"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_client_error());
    assert_eq!(response.status().as_u16(), 403);
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    let trace = response.headers()["x-medtracker-trace-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["trace_id"], trace, "{body}");
    let missing = app
        .client
        .get(format!("{}/api/auth/no-such-native-route", app.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);
    assert!(missing.headers().get("content-type").is_none());
    assert_eq!(
        missing.headers()["x-medtracker-trace-id"]
            .to_str()
            .unwrap()
            .len(),
        32
    );
    assert!(missing.bytes().await.unwrap().is_empty());
    app.close().await;
}
