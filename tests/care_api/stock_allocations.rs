use super::*;
use med_tracker::models::entities::version;
use sea_orm::PaginatorTrait;

#[tokio::test]
async fn stock_allocation_reserves_exact_quantity_and_replays_once() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let medication_id = app.fixture.medication.id;
    let person_id = app.fixture.account.person_id;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medications/{medication_id}/stock_allocations",
        app.origin
    );
    let token = app.token().await;
    let submit = |submission_id: uuid::Uuid, quantity: &str| {
        app.client.post(&endpoint).bearer_auth(&token).json(&json!({
            "stock_allocation": {
                "submission_id": submission_id,
                "person_id": person_id,
                "quantity": quantity,
                "stock_unit": "tablet"
            }
        }))
    };

    let submission_id = uuid::Uuid::new_v4();
    let response = submit(submission_id, "6.25").send().await.unwrap();
    let original = Application::assert_status(response, 201).await;
    assert_eq!(original["data"]["submission_id"], submission_id.to_string());
    assert_eq!(original["data"]["quantity"], "6.25");
    assert_eq!(original["data"]["person_id"], person_id);
    assert_eq!(original["data"]["stock_unit"], "tablet");

    let replay = submit(submission_id, "6.25").send().await.unwrap();
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(Application::assert_status(replay, 201).await, original);
    assert_eq!(
        submit(submission_id, "6.26").send().await.unwrap().status(),
        409
    );
    assert_eq!(
        submit(uuid::Uuid::new_v4(), "3.76")
            .send()
            .await
            .unwrap()
            .status(),
        422
    );

    assert_eq!(app.fixture.effect().await.1, "10.00");
    assert_eq!(
        version::Entity::find()
            .filter(version::Column::HouseholdId.eq(household_id))
            .filter(version::Column::ItemType.eq("StockAllocation"))
            .filter(version::Column::Event.eq("create"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    let dose = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    Application::assert_status(dose, 201).await;
    assert_eq!(app.fixture.effect().await.1, "8.00");
    let remaining_communal = submit(uuid::Uuid::new_v4(), "3.75").send().await.unwrap();
    Application::assert_status(remaining_communal, 201).await;
    app.close().await;
}

#[tokio::test]
async fn stock_allocation_requires_personal_authority_and_serializes_competing_reservations() {
    let app = Application::new().await;
    let base = format!(
        "{}/api/v1/households/{}",
        app.origin, app.fixture.household.id
    );
    let medication = format!("{base}/medications/{}", app.fixture.medication.id);
    let token = app.token().await;
    let reserve = |person_id| {
        app.client
            .post(format!("{medication}/stock_allocations"))
            .bearer_auth(&token)
            .json(&json!({"stock_allocation": {
                "submission_id": uuid::Uuid::new_v4(),
                "person_id": person_id,
                "quantity": "9",
                "stock_unit": "tablet"
            }}))
            .send()
    };
    let forbidden = reserve(app.fixture.minor.id).await.unwrap();
    assert_eq!(forbidden.status(), 403);

    let child = app
        .client
        .post(format!("{base}/people"))
        .bearer_auth(&token)
        .json(&json!({"person": {
            "name": "Synthetic child",
            "date_of_birth": "2020-01-01",
            "person_type": "minor",
            "has_capacity": false,
            "initial_guardian_declaration": true
        }}))
        .send()
        .await
        .unwrap();
    let child = Application::assert_status(child, 201).await;
    let child_id = child["data"]["id"].as_i64().unwrap();
    let (one, two) = tokio::join!(reserve(child_id), reserve(child_id));
    let mut statuses = [
        one.unwrap().status().as_u16(),
        two.unwrap().status().as_u16(),
    ];
    statuses.sort_unstable();
    assert_eq!(statuses, [201, 422]);

    let take = shared_take_body(&app.fixture);
    let denied_dose = app
        .client
        .post(format!("{base}/medication_takes"))
        .bearer_auth(&token)
        .json(&take)
        .send()
        .await
        .unwrap();
    let denial = Application::assert_status(denied_dose, 422).await;
    assert!(denial.to_string().contains("out of stock"));
    assert_eq!(app.fixture.effect().await, (0, "10.00".into(), 0, 0));
    app.close().await;
}
