use super::*;
use med_tracker::models::entities::location_membership;
use sea_orm::{ActiveModelTrait, Set};

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
