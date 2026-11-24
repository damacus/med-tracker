use super::*;
use med_tracker::models::entities::version;
use sea_orm::{ActiveModelTrait, PaginatorTrait, Set};

#[tokio::test]
async fn stock_allocation_uses_inventory_units_for_tracked_dosages() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let medication_id = app.fixture.medication.id;
    let base = format!("{}/api/v1/households/{household_id}", app.origin);
    let token = app.token().await;
    let create_dosage = |amount| {
        app.client
            .post(format!("{base}/dosage_options"))
            .bearer_auth(&token)
            .json(&json!({"dosage_option": {
                "medication_id": medication_id.to_string(), "amount": amount, "unit": "mg",
                "frequency": "daily", "current_supply": "10", "reorder_threshold": "2",
                "default_max_daily_doses": 3, "default_min_hours_between_doses": "4",
                "default_dose_cycle": "daily"
            }}))
            .send()
    };
    let dosage = Application::assert_status(create_dosage("5").await.unwrap(), 201).await;
    let sibling = Application::assert_status(create_dosage("10").await.unwrap(), 201).await;
    let submit = |unit| {
        app.client
            .post(format!(
                "{base}/medications/{medication_id}/stock_allocations"
            ))
            .bearer_auth(&token)
            .json(&json!({"stock_allocation": {
                "submission_id": uuid::Uuid::new_v4(),
                "person_id": app.fixture.account.person_id,
                "dosage_id": dosage["data"]["id"],
                "quantity": "5",
                "stock_unit": unit
            }}))
            .send()
    };
    Application::assert_status(submit("mg").await.unwrap(), 422).await;
    let receipt = Application::assert_status(submit("units").await.unwrap(), 201).await;
    assert_eq!(receipt["data"]["stock_unit"], "units");
    assert_eq!(receipt["data"]["dosage_id"], dosage["data"]["id"]);
    let audit = version::Entity::find()
        .filter(version::Column::HouseholdId.eq(household_id))
        .filter(version::Column::ItemType.eq("StockAllocation"))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(audit.audit_context["policy_class"], "StockAllocationPolicy");
    assert_eq!(audit.audit_context["policy_query"], "create?");
    let dosage_endpoint = format!("{base}/dosage_options/{}", dosage["data"]["id"]);
    let rejected = app
        .client
        .patch(&dosage_endpoint)
        .bearer_auth(&token)
        .json(&json!({"dosage_option": {"current_supply": "4"}}))
        .send()
        .await
        .unwrap();
    let rejection = Application::assert_status(rejected, 422).await;
    assert_eq!(
        rejection["error"]["message"],
        "Stock remains allocated to a person"
    );
    let mut assignment: med_tracker::models::entities::person_medication::ActiveModel =
        app.fixture.assignment.clone().into();
    assignment.source_dosage_option_id = Set(dosage["data"]["id"].as_i64());
    assignment.dose_amount = Set(Some("5".parse().unwrap()));
    assignment.dose_unit = Set(Some("mg".into()));
    assignment.update(&app.fixture.admin).await.unwrap();
    let mut dose = shared_take_body(&app.fixture);
    dose["medication_take"]["dose_amount"] = json!("5");
    dose["medication_take"]["dose_unit"] = json!("mg");
    let recorded = app
        .client
        .post(format!("{base}/medication_takes"))
        .bearer_auth(&token)
        .json(&dose)
        .send()
        .await
        .unwrap();
    Application::assert_status(recorded, 201).await;
    for (id, expected) in [
        (&dosage["data"]["id"], "9.0"),
        (&sibling["data"]["id"], "10.0"),
    ] {
        let read = app
            .client
            .get(format!("{base}/dosage_options/{id}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(
            Application::assert_status(read, 200).await["data"]["current_supply"],
            expected
        );
    }
    app.close().await;
}

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
    let edit = app
        .client
        .patch(format!(
            "{}/api/v1/households/{household_id}/medications/{medication_id}",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"medication": {"current_supply": "6"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        Application::assert_status(edit, 422).await["error"]["message"],
        "Stock remains allocated to a person"
    );
    let export = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/portable_export",
            app.origin
        ))
        .bearer_auth(&token)
        .header(
            "x-medtracker-portable-passphrase",
            "synthetic test passphrase",
        )
        .send()
        .await
        .unwrap();
    assert!(
        Application::assert_status(export, 422).await["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("cannot preserve active stock reservations"))
    );
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
    for _ in 0..2 {
        let release = app
            .client
            .delete(format!("{endpoint}/{submission_id}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(release.status(), 204);
    }
    assert_eq!(app.fixture.effect().await.1, "8.00");
    assert_eq!(
        version::Entity::find()
            .filter(version::Column::HouseholdId.eq(household_id))
            .filter(version::Column::ItemType.eq("StockAllocation"))
            .filter(version::Column::Event.eq("release"))
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        1
    );
    Application::assert_status(
        submit(uuid::Uuid::new_v4(), "4.25").send().await.unwrap(),
        201,
    )
    .await;
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
    let assignment = app
        .client
        .get(format!(
            "{base}/person_medications/{}",
            app.fixture.assignment.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(
        Application::assert_status(assignment, 200).await["data"]["eligible_stock_medication_ids"],
        json!([])
    );

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
