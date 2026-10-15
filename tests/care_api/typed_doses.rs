use super::{Application, take_body};
use serde_json::{Value, json};

fn attribute(body: &mut Value, field: &str, value: Value) -> Result<(), &'static str> {
    body.get_mut("medication_take")
        .and_then(Value::as_object_mut)
        .ok_or("Missing medication take fixture")?
        .insert(field.into(), value);
    Ok(())
}

fn code(body: &Value) -> Option<&str> {
    body.get("error")?.get("code")?.as_str()
}

#[tokio::test]
async fn typed_dose_http_validation_replay_and_conflict_keep_wire_and_effects()
-> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medication_takes", app.origin);
    let empty = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(empty.status().as_u16(), 400);
    assert_eq!(code(&empty.json::<Value>().await?), Some("bad_request"));
    for (field, value, status, expected_code) in [
        ("dose_amount", json!(0.1), 422, "validation_failed"),
        ("dose_amount", json!("0.001"), 422, "unprocessable_content"),
        (
            "taken_at",
            json!("not-a-timestamp"),
            422,
            "unprocessable_content",
        ),
        (
            "client_uuid",
            json!("not-a-uuid"),
            422,
            "unprocessable_content",
        ),
        ("source_type", json!("unknown"), 404, "not_found"),
        ("source_id", json!("-1"), 422, "unprocessable_content"),
        (
            "taken_from_medication_id",
            json!(-1),
            422,
            "unprocessable_content",
        ),
        ("unsupported", json!(true), 422, "unprocessable_content"),
    ] {
        let mut request = take_body();
        attribute(&mut request, field, value)?;
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await?;
        assert_eq!(
            response.status().as_u16(),
            status,
            "Unexpected status for {field}"
        );
        let body: Value = response.json().await?;
        assert_eq!(
            code(&body),
            Some(expected_code),
            "Unexpected code for {field}"
        );
        if expected_code == "validation_failed" {
            assert_eq!(
                body.pointer("/error/errors/dose_amount"),
                Some(&json!(["must be a string"]))
            );
        }
        assert_eq!(app.fixture.effect().await, (0, "10.00".into(), 0, 0));
    }
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await?;
    assert_eq!(created.status().as_u16(), 201);
    let created: Value = created.json().await?;
    let mut retry = take_body();
    attribute(
        &mut retry,
        "client_uuid",
        json!("006B49D91DA242F1800B8C80867AEE1C"),
    )?;
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&retry)
        .send()
        .await?;
    assert_eq!(replay.status().as_u16(), 200);
    let replay: Value = replay.json().await?;
    assert_eq!(created.get("data"), replay.get("data"));
    for (field, value) in [
        ("taken_at", "not-a-timestamp"),
        ("dose_amount", "not-a-decimal"),
    ] {
        let mut retry = take_body();
        attribute(&mut retry, field, json!(value))?;
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&retry)
            .send()
            .await?;
        assert_eq!(
            response.status().as_u16(),
            409,
            "Duplicate invalid {field} must remain a conflict"
        );
        assert_eq!(code(&response.json::<Value>().await?), Some("conflict"));
    }
    assert_eq!(app.fixture.effect().await, (1, "8.00".into(), 1, 1));
    app.close().await;
    Ok(())
}
