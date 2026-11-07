use super::Application;
use med_tracker::models::_entities::household_memberships;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

#[tokio::test]
async fn consent_is_recipient_only_bound_and_withdrawable_over_api() {
    let app = Application::new().await;
    let recipient = app.fixture.foreign_household().await;
    let stranger = app.fixture.foreign_household().await;
    let household = app.fixture.household.id;
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(household),
        person_id: Set(recipient.account.person_id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let issue = |account: i64, target_household: i64| {
        let service = service.clone();
        async move {
            service
                .api_keys()
                .unwrap()
                .create(
                    &account.to_string(),
                    better_auth::server_api::CreateKeyOptions {
                        config_id: Some("medtracker-personal".into()),
                        name: Some("Consent transport test".into()),
                        expires_in: Some(chrono::Duration::days(1).num_seconds() as f64),
                        metadata: Some(json!({"households":[target_household]})),
                        permissions: Some(std::collections::HashMap::from([(
                            "medtracker".into(),
                            vec!["care:read".into(), "care:write".into()],
                        )])),
                        ..Default::default()
                    },
                )
                .await
                .unwrap()
                .key
        }
    };
    let recipient_key = issue(recipient.account.id, household).await;
    let stranger_key = issue(stranger.account.id, stranger.household.id).await;
    let read_key = service
        .api_keys()
        .unwrap()
        .create(
            &recipient.account.id.to_string(),
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Consent read-only".into()),
                expires_in: Some(chrono::Duration::days(1).num_seconds() as f64),
                metadata: Some(json!({"households":[household]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .key;
    let requester = app.token().await;
    let base = format!(
        "{}/api/v1/households/{household}/data_imports/consents",
        app.origin
    );
    let binding = json!({"source_sha256":"a".repeat(64),
        "source_person":{"source":"med_tracker05", "instance":"synthetic", "kind":"person", "id":"source-person"},
        "target_person":recipient.account.person_id});
    let oversized = app
        .client
        .post(&base)
        .bearer_auth(&requester)
        .header("content-type", "application/json")
        .body("x".repeat(64 * 1024 + 1))
        .send()
        .await
        .unwrap();
    assert_eq!(oversized.status().as_u16(), 422);
    let mut forged_actor = binding.clone();
    forged_actor["actor_account_id"] = json!(recipient.account.id);
    let rejected_actor = app
        .client
        .post(&base)
        .bearer_auth(&requester)
        .json(&forged_actor)
        .send()
        .await
        .unwrap();
    assert_eq!(rejected_actor.status().as_u16(), 400);
    let created = app
        .client
        .post(&base)
        .bearer_auth(&requester)
        .json(&binding)
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let created: serde_json::Value = created.json().await.unwrap();
    let id = created["data"]["id"].as_str().unwrap();
    assert_eq!(created["data"].as_object().unwrap().len(), 1);
    let own = format!("{base}/{id}");
    let member_request = app
        .client
        .post(&base)
        .bearer_auth(&recipient_key)
        .json(&binding)
        .send()
        .await
        .unwrap();
    assert_eq!(member_request.status().as_u16(), 403);
    let read_only = app
        .client
        .get(&own)
        .bearer_auth(&read_key)
        .send()
        .await
        .unwrap();
    assert_eq!(read_only.status().as_u16(), 200);
    let read_only_decide = app
        .client
        .post(format!("{own}/decision"))
        .bearer_auth(&read_key)
        .json(&json!({"binding":binding,"decision":"approved"}))
        .send()
        .await
        .unwrap();
    assert_eq!(read_only_decide.status().as_u16(), 403);
    let forbidden = app
        .client
        .get(&own)
        .bearer_auth(&requester)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status().as_u16(), 404);
    let forged_requester = app
        .client
        .post(format!("{own}/decision"))
        .bearer_auth(&requester)
        .json(&json!({"binding":binding,"decision":"approved"}))
        .send()
        .await
        .unwrap();
    assert_eq!(forged_requester.status().as_u16(), 404);
    let foreign = format!(
        "{}/api/v1/households/{}/data_imports/consents/{id}",
        app.origin, stranger.household.id
    );
    let denied = app
        .client
        .get(&foreign)
        .bearer_auth(&stranger_key)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 404);
    let foreign_decision = app
        .client
        .post(format!("{foreign}/decision"))
        .bearer_auth(&stranger_key)
        .json(&json!({"binding":binding,"decision":"approved"}))
        .send()
        .await
        .unwrap();
    assert_eq!(foreign_decision.status().as_u16(), 404);
    let read = app
        .client
        .get(&own)
        .bearer_auth(&recipient_key)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status().as_u16(), 200);
    let read: serde_json::Value = read.json().await.unwrap();
    assert_eq!(read["data"]["binding"], binding);
    assert_eq!(read["data"]["decision"], "pending");
    assert_eq!(read["data"].as_object().unwrap().len(), 5);
    for field in ["hash", "person", "target"] {
        let mut changed = binding.clone();
        match field {
            "hash" => changed["source_sha256"] = json!("b".repeat(64)),
            "person" => changed["source_person"]["id"] = json!("other-person"),
            _ => changed["target_person"] = json!(app.fixture.account.person_id),
        }
        let response = app
            .client
            .post(format!("{own}/decision"))
            .bearer_auth(&recipient_key)
            .json(&json!({"binding":changed,"decision":"approved"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 409);
    }
    for decision in ["approved", "rejected"] {
        let response = app
            .client
            .post(format!("{own}/decision"))
            .bearer_auth(&recipient_key)
            .json(&json!({"binding":binding,"decision":decision}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["data"]["decision"], decision);
        assert_eq!(body["data"]["binding"], binding);
    }
    let replay = app
        .client
        .post(format!("{own}/decision"))
        .bearer_auth(&recipient_key)
        .json(&json!({"binding":binding,"decision":"approved"}))
        .send()
        .await
        .unwrap();
    assert_eq!(replay.status().as_u16(), 409);
    app.close().await;
}
