use super::*;

#[tokio::test]
async fn treatment_timestamp_filters_require_offsets_and_compare_the_same_instant() {
    let app = Application::new().await;
    let token = app.token().await;
    let created = app
        .client
        .post(format!("{}/api/v1/households/72001/schedules", app.origin))
        .bearer_auth(&token)
        .json(&json!({"schedule": {
            "person_id":"73001", "medication_id":"80001", "dose_amount":"2", "dose_unit":"tablet",
            "start_date":"2026-10-06", "end_date":"2026-10-06", "schedule_type":"prn", "schedule_config":{}
        }}))
        .send()
        .await
        .unwrap();
    let created_status = created.status().as_u16();
    let mut observed = Vec::new();
    for resource in ["schedules", "person_medications"] {
        for value in [
            "2026-10-06T10:00:00",
            "2026-10-06",
            "not-a-timestamp",
            "2000-01-01T00:00:00Z",
            "2000-01-01T01:00:00+01:00",
            "2100-01-01T00:00:00Z",
        ] {
            let query = serde_urlencoded::to_string([("updated_since", value)]).unwrap();
            let response = app
                .client
                .get(format!(
                    "{}/api/v1/households/72001/{resource}?{query}",
                    app.origin
                ))
                .bearer_auth(&token)
                .send()
                .await
                .unwrap();
            let status = response.status().as_u16();
            let body = response.json::<Value>().await.unwrap_or(Value::Null);
            observed.push((resource, value, status, body));
        }
    }
    app.close().await;
    assert_eq!(created_status, 201);
    for rows in observed.as_chunks::<6>().0 {
        let resource = rows[0].0;
        for row in &rows[..3] {
            assert_eq!(
                row.2, 422,
                "{resource} accepted invalid timestamp {}",
                row.1
            );
        }
        for row in &rows[3..] {
            assert_eq!(
                row.2, 200,
                "{resource} rejected RFC3339 timestamp {}",
                row.1
            );
        }
        assert_eq!(rows[3].3["data"], rows[4].3["data"]);
        assert_eq!(rows[3].3["meta"], rows[4].3["meta"]);
        assert!(!rows[3].3["data"].as_array().unwrap().is_empty());
        assert_eq!(rows[5].3["data"], json!([]));
        assert_eq!(rows[5].3["meta"]["total_count"], 0);
    }
}
