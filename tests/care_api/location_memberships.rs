use super::*;
use med_tracker::models::entities::{location, location_membership, membership};
use sea_orm::{ActiveModelTrait, PaginatorTrait, Set};

#[tokio::test]
async fn dependent_creation_leaves_a_household_without_labels_empty() {
    let app = Application::new().await;
    let household = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    membership::ActiveModel {
        household_id: Set(household.household.id),
        person_id: Set(app.fixture.account.person_id),
        role: Set("administrator".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&household.admin)
    .await
    .unwrap();
    let labels =
        location::Entity::find().filter(location::Column::HouseholdId.eq(household.household.id));
    assert_eq!(labels.clone().count(&household.admin).await.unwrap(), 0);

    let response = app.client.post(format!("{}/api/v1/households/{}/people", app.origin, household.household.id))
        .bearer_auth(app.token().await)
        .json(&json!({"person": {"name": "Synthetic child", "date_of_birth": "2020-01-01", "person_type": "minor", "has_capacity": false, "initial_guardian_declaration": true}}))
        .send()
        .await
        .unwrap();
    let body = Application::assert_status(response, 201).await;
    assert_eq!(body["data"]["location_ids"], json!([]));
    assert_eq!(labels.count(&household.admin).await.unwrap(), 0);
    household.close().await;
    app.close().await;
}

#[tokio::test]
async fn obsolete_location_membership_routes_cannot_change_retained_links() {
    let app = Application::new().await;
    let household_id = app.fixture.household.id;
    let location_id = app.fixture.location.id;
    let person_id = app.fixture.account.person_id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/locations/{location_id}/location_memberships",
        app.origin
    );
    let create = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"location_membership": {"person_id": person_id.to_string()}}))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), 404);

    let now = chrono::Utc::now().naive_utc();
    let retained = location_membership::ActiveModel {
        household_id: Set(household_id),
        location_id: Set(location_id),
        person_id: Set(person_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let delete = app
        .client
        .delete(format!("{endpoint}/{}", retained.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(delete.status(), 404);
    assert_eq!(
        location_membership::Entity::find_by_id(retained.id)
            .one(&app.fixture.admin)
            .await
            .unwrap(),
        Some(retained)
    );
    app.close().await;
}
