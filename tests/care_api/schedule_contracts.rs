use super::contract::{assert_value, contract, resolve};
use super::*;

pub(super) fn assert_unavailable(path: &str, method: &str, request_id: &str, body: &Value) {
    let response = resolve(
        contract(),
        &contract()["paths"][path][method]["responses"]["503"],
    );
    assert_value(
        contract(),
        resolve(
            contract(),
            &response["content"]["application/json"]["schema"],
        ),
        body,
        "schedule audit failure",
    );
    assert_eq!(
        body,
        &json!({"error": {
            "code": "service_unavailable",
            "message": "Service is temporarily unavailable",
            "request_id": request_id
        }})
    );
}

async fn create_schedule(app: &Application, token: &str) -> (String, Value, String) {
    let collection = format!(
        "{}/api/v1/households/{}/schedules",
        app.origin, app.fixture.household.id
    );
    let response = app
        .client
        .post(&collection)
        .bearer_auth(token)
        .json(&json!({"schedule": {
            "person_id": app.fixture.account.person_id.to_string(),
            "medication_id": app.fixture.medication.id.to_string(),
            "dose_amount": "2", "dose_unit": "tablet",
            "start_date": "2026-01-01", "end_date": "2027-01-01",
            "schedule_type": "prn", "schedule_config": {}
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    let body: Value = response.json().await.unwrap();
    let endpoint = format!("{collection}/{}", body["data"]["id"].as_i64().unwrap());
    (endpoint, body, etag)
}

#[tokio::test]
async fn schedule_conditional_get_matches_documented_empty_304_and_etag() {
    let app = Application::new().await;
    let token = app.token().await;
    let (endpoint, _, etag) = create_schedule(&app, &token).await;
    let response = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 304);
    assert_eq!(response.headers()["etag"].to_str().unwrap(), etag);
    assert!(!response.headers()["x-request-id"].is_empty());
    assert!(response.bytes().await.unwrap().is_empty());
    app.close().await;

    let operation = &contract()["paths"]["/households/{household_id}/schedules/{id}"]["get"];
    assert!(
        operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/if_none_match")
    );
    let response = &operation["responses"]["304"];
    assert_eq!(
        response["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    assert!(response.get("content").is_none());
}

#[test]
fn schedule_audited_writes_declare_shared_unavailable_response() {
    for (path, method) in [
        ("/households/{household_id}/schedules", "post"),
        ("/households/{household_id}/schedules/{id}", "patch"),
        ("/households/{household_id}/schedules/{id}", "put"),
    ] {
        assert_eq!(
            contract()["paths"][path][method]["responses"]["503"]["$ref"],
            "#/components/responses/ServiceUnavailable",
            "{method} {path} must declare its audited-write failure"
        );
    }
    assert!(
        contract()["paths"]["/households/{household_id}/schedules/{id}"]["put"]["responses"]["500"]
            .is_null(),
        "put /households/{{household_id}}/schedules/{{id}} must not declare an unreachable 500"
    );
    assert!(
        contract()["components"]["responses"]["InternalServerError"].is_null(),
        "No documented operation produces the internal-error envelope; keep the component removed"
    );
}

#[tokio::test]
async fn schedule_put_returns_documented_body_etag_and_merges_supplied_fields() {
    let app = Application::new().await;
    let token = app.token().await;
    let (endpoint, before, etag) = create_schedule(&app, &token).await;
    let response = app
        .client
        .put(&endpoint)
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"schedule": {"notes": "Synthetic PUT contract"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let updated_etag = response.headers()["etag"].to_str().unwrap().to_owned();
    assert_ne!(updated_etag, etag);
    let body: Value = response.json().await.unwrap();
    let operation = &contract()["paths"]["/households/{household_id}/schedules/{id}"]["put"];
    assert_eq!(
        operation["responses"]["200"]["headers"]["ETag"]["$ref"],
        "#/components/headers/etag"
    );
    assert_value(
        contract(),
        resolve(
            contract(),
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "PUT schedule response",
    );
    for field in [
        "id",
        "person_id",
        "medication_id",
        "dose_amount",
        "dose_unit",
        "start_date",
        "end_date",
        "schedule_type",
        "schedule_config",
    ] {
        assert_eq!(body["data"][field], before["data"][field], "{field}");
    }
    assert_eq!(body["data"]["notes"], "Synthetic PUT contract");
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    assert_eq!(read.headers()["etag"].to_str().unwrap(), updated_etag);
    assert_eq!(read.json::<Value>().await.unwrap(), body);
    app.close().await;
}
