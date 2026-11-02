use super::*;
use crate::person_owned_schedules::{another_owner, personal_change, plans};

#[tokio::test]
async fn person_owned_schedule_restore_uses_terminal_local_snapshot_after_personal_edit() {
    let app = Application::new().await;
    another_owner(&app).await;
    let token = app.token().await;
    let id = crate::schedule_lifecycle::create_schedule(&app, &token).await;
    let first = plans(&app, &token).await["data"][0].clone();
    let response = app
        .client
        .delete(format!(
            "{}/api/v1/households/{}/admin/memberships/{}",
            app.origin, app.fixture.household.id, app.fixture.membership.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let edited = personal_change(
        &app,
        &token,
        &first,
        json!({"schedule":{"dose_amount":"7"}}),
    )
    .await;
    assert_eq!(edited.status().as_u16(), 200);
    let mut restored = app.fixture.membership.clone().into_active_model();
    restored.status = Set("active".into());
    restored.revoked_at = Set(None);
    restored.permissions_version = Set(100);
    restored.update(&app.fixture.admin).await.unwrap();
    let source = rows::schedules::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(source.execution_ended_at.is_some());
    let payload = super::source_bundle(&app, &token, &source).await;
    let before = super::effects(&app, id).await;
    assert_eq!(
        before["source"]["dose_amount"]
            .as_str()
            .unwrap()
            .parse::<sea_orm::prelude::Decimal>()
            .unwrap(),
        sea_orm::prelude::Decimal::TWO
    );
    assert_eq!(
        before["canonical"]["dose_amount"]
            .as_str()
            .unwrap()
            .parse::<sea_orm::prelude::Decimal>()
            .unwrap(),
        sea_orm::prelude::Decimal::from(7)
    );
    for dry in [true, false, false] {
        let response = super::import(&app, &token, &payload, dry).await;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(status, if dry { 200 } else { 201 }, "{body}");
        assert_eq!(super::effects(&app, id).await, before);
    }
    let mut reopened = payload.clone();
    reopened["records"]["schedules"][0]["active"] = json!(true);
    reopened["records"]["schedules"][0]["retired_at"] = Value::Null;
    let response = super::import(&app, &token, &reopened, false).await;
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(super::effects(&app, id).await, before);
    app.close().await;
}
