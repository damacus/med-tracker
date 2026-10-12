use super::*;

async fn seed_second_assignment(app: &Application) {
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES (80002,72001,79001,'Second synthetic',10,1,'tablet',now(),now())"
        )
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES (81002,72001,73001,80002,1,'tablet',1,now(),now())"
        )
        .await
        .unwrap();
}

fn reorder_url(app: &Application, id: &str) -> String {
    format!(
        "{}/api/v1/households/72001/person_medications/{id}/reorder",
        app.origin
    )
}

async fn position_of(app: &Application, id: i64) -> i32 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("SELECT position FROM person_medications WHERE id={id}"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "position")
        .unwrap()
}

#[tokio::test]
async fn person_medication_reorder_swaps_positions_and_records_events() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_second_assignment(&app).await;
    let moved = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let moved_status = moved.status().as_u16();
    let etag = moved
        .headers()
        .get("etag")
        .is_some_and(|value| value.to_str().is_ok_and(|etag| etag.starts_with('"')));
    let moved_body: Value = moved.json().await.unwrap();
    let first_position = position_of(&app, 81001).await;
    let second_position = position_of(&app, 81002).await;
    let edge = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let edge_status = edge.status().as_u16();
    let edge_body: Value = edge.json().await.unwrap();
    let back = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({"direction":"down"}))
        .send()
        .await
        .unwrap();
    let back_status = back.status().as_u16();
    let restored = position_of(&app, 81002).await;
    let counts = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type='PersonMedication' AND event='update') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='PersonMedication' AND action='update') AS changes",
        ))
        .await
        .unwrap()
        .unwrap();
    let versions: i64 = counts.try_get("", "versions").unwrap();
    let changes: i64 = counts.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(moved_status, 200);
    assert!(etag);
    assert_eq!(moved_body["data"]["id"], 81002);
    assert_eq!(moved_body["data"]["position"], 0);
    assert_eq!(second_position, 0);
    assert_eq!(first_position, 1);
    assert_eq!(edge_status, 200);
    assert_eq!(edge_body["data"]["position"], 0);
    assert_eq!(back_status, 200);
    assert_eq!(restored, 1);
    assert_eq!(versions, 4);
    assert_eq!(changes, 4);
}

#[tokio::test]
async fn person_medication_reorder_validates_direction_and_body() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_second_assignment(&app).await;
    let sideways = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({"direction":"sideways"}))
        .send()
        .await
        .unwrap();
    let sideways_status = sideways.status().as_u16();
    let sideways_body: Value = sideways.json().await.unwrap();
    let extra = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({"direction":"up","extra":true}))
        .send()
        .await
        .unwrap();
    let extra_status = extra.status().as_u16();
    let missing = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let scalar = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!("up"))
        .send()
        .await
        .unwrap();
    let scalar_status = scalar.status().as_u16();
    let malformed = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    let malformed_status = malformed.status().as_u16();
    let unchanged = position_of(&app, 81002).await;
    app.close().await;
    assert_eq!(sideways_status, 422);
    assert_eq!(sideways_body["error"]["code"], "validation_failed");
    assert_eq!(
        sideways_body["error"]["errors"]["direction"],
        json!(["must be up or down"])
    );
    assert_eq!(extra_status, 422);
    assert_eq!(missing_status, 422);
    assert_eq!(scalar_status, 400);
    assert_eq!(malformed_status, 400);
    assert_eq!(unchanged, 1);
}

#[tokio::test]
async fn person_medication_reorder_requires_manage_access() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_second_assignment(&app).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET access_level='view' WHERE id=78001")
        .await
        .unwrap();
    let denied = app
        .client
        .patch(reorder_url(&app, "81002"))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let foreign = app
        .client
        .patch(format!(
            "{}/api/v1/households/72002/person_medications/81002/reorder",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let missing_row = app
        .client
        .patch(reorder_url(&app, "999999"))
        .bearer_auth(&token)
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let missing_status = missing_row.status().as_u16();
    let anonymous = app
        .client
        .patch(reorder_url(&app, "81002"))
        .json(&json!({"direction":"up"}))
        .send()
        .await
        .unwrap();
    let anonymous_status = anonymous.status().as_u16();
    let unchanged = position_of(&app, 81002).await;
    app.close().await;
    assert_eq!(denied_status, 403);
    assert_eq!(foreign_status, 403);
    assert_eq!(missing_status, 404);
    assert_eq!(anonymous_status, 401);
    assert_eq!(unchanged, 1);
}
