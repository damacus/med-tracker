use super::*;
use med_tracker::models::_entities::{api_app_tokens, api_sessions, oauth_grants};
use sea_orm::{ActiveValue, QueryOrder};
use sha2::{Digest, Sha256};

pub(super) fn credential_token(app: &Application, label: &str) -> String {
    format!("{}-{label}", app.fixture.client_uuid)
}

pub(super) struct StoredCredentials {
    pub current: api_sessions::Model,
    pub expired: api_sessions::Model,
    pub revoked: api_sessions::Model,
    pub app_token: api_app_tokens::Model,
}

pub(super) async fn stored_credentials(app: &Application) -> StoredCredentials {
    let now = chrono::Utc::now().naive_utc();
    let mut rows = Vec::new();
    for (label, device, age, expires, revoked) in [
        ("current", "Current phone", 1, 15, false),
        ("expired", "Expired phone", 2, -60, false),
        ("revoked", "Revoked phone", 3, 15, true),
    ] {
        rows.push(
            api_sessions::ActiveModel {
                account_id: Set(app.fixture.account.id),
                household_membership_id: Set(Some(app.fixture.membership.id)),
                access_token_digest: Set(hex::encode(Sha256::digest(
                    credential_token(app, label).as_bytes(),
                ))),
                refresh_token_digest: Set(credential_token(app, &format!("{label}-refresh"))),
                permissions_version: Set(app.fixture.membership.permissions_version),
                access_expires_at: Set(now + chrono::Duration::minutes(expires)),
                refresh_expires_at: Set(if expires < 0 {
                    now - chrono::Duration::minutes(1)
                } else {
                    now + chrono::Duration::days(1)
                }),
                last_used_at: Set(if expires < 0 {
                    now - chrono::Duration::days(age)
                } else {
                    now
                }),
                device_name: Set(Some(device.into())),
                revoked_at: Set(revoked.then_some(now)),
                created_at: Set(now - chrono::Duration::days(age)),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap(),
        );
    }
    let [current, expired, revoked]: [api_sessions::Model; 3] = rows.try_into().unwrap();
    let app_token = api_app_tokens::ActiveModel {
        account_id: Set(app.fixture.account.id),
        household_membership_id: Set(app.fixture.membership.id),
        name: Set("Synthetic app".into()),
        token_digest: Set(hex::encode(Sha256::digest(
            credential_token(app, "app").as_bytes(),
        ))),
        permissions_version: Set(app.fixture.membership.permissions_version),
        expires_at: Set(now + chrono::Duration::days(30)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    StoredCredentials {
        current,
        expired,
        revoked,
        app_token,
    }
}

async fn account_versions(app: &Application, kind: &str, events: &[&str]) -> Vec<versions::Model> {
    versions::Entity::find()
        .filter(versions::Column::ItemType.eq(kind))
        .filter(versions::Column::ItemId.eq(app.fixture.account.id))
        .filter(versions::Column::Event.is_in(events.iter().copied()))
        .order_by_asc(versions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap()
}

async fn mobile_grant(app: &Application, age: i64, expired: bool) -> oauth_grants::Model {
    let now = chrono::Utc::now().naive_utc();
    let mut row = app.fixture.authorization.clone().into_active_model();
    row.id = ActiveValue::NotSet;
    row.code = Set(None);
    row.token_hash = Set(Some(uuid::Uuid::new_v4().to_string()));
    row.expires_in = Set(if expired {
        now - chrono::Duration::days(1)
    } else {
        now + chrono::Duration::minutes(15)
    });
    row.authenticated_at = Set(Some(now - chrono::Duration::days(age)));
    row.last_used_at = Set(Some(now - chrono::Duration::days(age)));
    row.device_name = Set(Some(
        if expired { "Old phone" } else { "Second phone" }.into(),
    ));
    row.created_at = Set(now - chrono::Duration::days(age));
    row.insert(&app.fixture.admin).await.unwrap()
}

async fn last_used(app: &Application) -> chrono::NaiveDateTime {
    oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .last_used_at
        .unwrap()
}

async fn assert_expanded_account_collections(app: &Application, credentials: &StoredCredentials) {
    let other = app.fixture.foreign_household().await;
    let held = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    let mut held_household = held.household.clone().into_active_model();
    held_household.lifecycle_state = Set("held".into());
    held_household.update(&app.fixture.admin).await.unwrap();
    let mut memberships = Vec::new();
    for household in [&other.household, &held.household] {
        memberships.push(
            household_memberships::ActiveModel {
                household_id: Set(household.id),
                person_id: Set(app.fixture.adult.id),
                role: Set("member".into()),
                joined_at: Set(now),
                status: Set("active".into()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await
            .unwrap(),
        );
    }
    for bearer in [
        credential_token(app, "current"),
        credential_token(app, "app"),
    ] {
        let households = app
            .client
            .get(format!("{}/api/v1/auth/households", app.origin))
            .bearer_auth(&bearer)
            .send()
            .await
            .unwrap();
        assert_eq!(households.status().as_u16(), 200);
        let body: Value = households.json().await.unwrap();
        assert_eq!(body["account_id"], app.fixture.account.id);
        let households = body["data"].as_array().unwrap();
        assert_eq!(households.len(), 2);
        assert_eq!(households[0]["id"], app.fixture.household.id);
        assert_eq!(households[0]["membership_id"], app.fixture.membership.id);
        assert_eq!(households[1]["id"], other.household.id);
        assert_eq!(households[1]["membership_id"], memberships[0].id);
        let sessions = app
            .client
            .get(format!("{}/api/v1/auth/sessions", app.origin))
            .bearer_auth(&bearer)
            .send()
            .await
            .unwrap();
        assert_eq!(sessions.status().as_u16(), 200);
        let text = sessions.text().await.unwrap();
        for secret in [
            credential_token(app, "current"),
            credentials.current.refresh_token_digest.clone(),
            credentials.expired.access_token_digest.clone(),
        ] {
            assert!(!text.contains(&secret));
        }
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["data"].as_array().unwrap().len(), 2);
        assert_eq!(body["data"][0]["id"], credentials.current.id);
        assert_eq!(body["data"][0]["household_id"], app.fixture.household.id);
        assert_eq!(body["data"][1]["id"], credentials.expired.id);
        assert_eq!(body["data"][1]["device_name"], "Expired phone");
    }
    held.close().await;
    other.close().await;
}

#[tokio::test]
async fn stored_session_revoke_is_account_scoped_and_invalidates_the_credential() {
    let app = Application::new().await;
    let credentials = stored_credentials(&app).await;
    let other = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    let foreign = api_sessions::ActiveModel {
        account_id: Set(other.account.id),
        household_membership_id: Set(Some(other.membership.id)),
        access_token_digest: Set(uuid::Uuid::new_v4().to_string()),
        refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
        permissions_version: Set(1),
        access_expires_at: Set(now + chrono::Duration::minutes(15)),
        refresh_expires_at: Set(now + chrono::Duration::days(1)),
        last_used_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let path = format!("{}/api/v1/auth/sessions", app.origin);
    let mut wrong_grant = mobile_grant(&app, 60, true).await;
    while [
        credentials.current.id,
        credentials.expired.id,
        credentials.revoked.id,
    ]
    .contains(&wrong_grant.id)
    {
        wrong_grant = mobile_grant(&app, 60, true).await;
    }
    for id in [-1, credentials.revoked.id, foreign.id, wrong_grant.id] {
        let response = app
            .client
            .delete(format!("{path}/{id}"))
            .bearer_auth(credential_token(&app, "current"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 404);
    }
    let revoked = app
        .client
        .delete(format!("{path}/{}", credentials.expired.id))
        .bearer_auth(credential_token(&app, "app"))
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status().as_u16(), 204);
    let repeat = app
        .client
        .delete(format!("{path}/{}", credentials.expired.id))
        .bearer_auth(credential_token(&app, "app"))
        .send()
        .await
        .unwrap();
    assert_eq!(repeat.status().as_u16(), 404);
    let own = app
        .client
        .delete(format!("{path}/{}", credentials.current.id))
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    assert_eq!(own.status().as_u16(), 204);
    let after = app
        .client
        .get(&path)
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    assert_eq!(after.status().as_u16(), 401);
    let audit = account_versions(
        &app,
        "AuthenticationToken",
        &["auth_token/api_session/revoked"],
    )
    .await;
    assert_eq!(audit.len(), 2);
    other.close().await;
    app.close().await;
}

#[tokio::test]
async fn mobile_oauth_sessions_use_grant_namespace_and_lifetime() {
    let app = Application::new().await;
    let credentials = stored_credentials(&app).await;
    let token = app.token().await;
    let old_mobile = mobile_grant(&app, 60, true).await;
    let path = format!("{}/api/v1/auth/sessions", app.origin);
    let response = app
        .client
        .get(&path)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["id"], app.fixture.authorization.id);
    let wrong_namespace = app
        .client
        .delete(format!(
            "{path}/{}",
            [
                credentials.current.id,
                credentials.expired.id,
                credentials.revoked.id
            ]
            .into_iter()
            .find(|id| *id != app.fixture.authorization.id && *id != old_mobile.id)
            .unwrap()
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_namespace.status().as_u16(), 404);
    let own = app
        .client
        .delete(format!("{path}/{}", app.fixture.authorization.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(own.status().as_u16(), 204);
    let after = app
        .client
        .get(&path)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(after.status().as_u16(), 401);
    let audit = account_versions(&app, "Account", &["mobile_oauth.revoked"]).await;
    assert_eq!(audit.len(), 1);
    app.close().await;
}

#[tokio::test]
async fn logout_is_idempotent_and_revokes_each_presented_credential_kind() {
    let app = Application::new().await;
    stored_credentials(&app).await;
    let mobile = app.token().await;
    let path = format!("{}/api/v1/auth/logout", app.origin);
    assert_eq!(
        app.client
            .delete(&path)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        app.client
            .delete(&path)
            .bearer_auth("unknown")
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        204
    );
    for bearer in [
        credential_token(&app, "current"),
        credential_token(&app, "app"),
        mobile.clone(),
    ] {
        assert_eq!(
            app.client
                .delete(&path)
                .bearer_auth(&bearer)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            204
        );
        assert_eq!(
            app.client
                .delete(&path)
                .bearer_auth(&bearer)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            204
        );
        assert_eq!(
            app.client
                .get(format!("{}/api/v1/auth/sessions", app.origin))
                .bearer_auth(&bearer)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            401
        );
    }
    app.close().await;
}

#[tokio::test]
async fn audit_failure_rolls_back_revocation_and_logout_instead_of_returning_no_content() {
    let app = Application::new().await;
    let credentials = stored_credentials(&app).await;
    let fault = format!(
        "reject_account_session_audit_{}",
        uuid::Uuid::new_v4().simple()
    );
    // SeaORM has no trigger or PL/pgSQL function builder. This fault injection
    // requires whole-run exclusivity and removes both objects below.
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='AuthenticationToken' AND NEW.event='auth_token/api_session/revoked' THEN RAISE EXCEPTION 'Synthetic session audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION {fault}()")).await.unwrap();
    let revoke = app
        .client
        .delete(format!(
            "{}/api/v1/auth/sessions/{}",
            app.origin, credentials.current.id
        ))
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    assert_eq!(revoke.status().as_u16(), 500);
    let logout = app
        .client
        .delete(format!("{}/api/v1/auth/logout", app.origin))
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status().as_u16(), 500);
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON versions; DROP FUNCTION {fault}()"
        ))
        .await
        .unwrap();
    let row = api_sessions::Entity::find_by_id(credentials.current.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(row.revoked_at.is_none());
    let still_valid = app
        .client
        .get(format!("{}/api/v1/auth/sessions", app.origin))
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    assert_eq!(still_valid.status().as_u16(), 200);
    app.close().await;
}

#[tokio::test]
async fn revocation_audit_identifies_the_target_credential_and_actor_without_secrets() {
    let app = Application::new().await;
    let credentials = stored_credentials(&app).await;
    let selected = app
        .client
        .delete(format!(
            "{}/api/v1/auth/sessions/{}",
            app.origin, credentials.expired.id
        ))
        .bearer_auth(credential_token(&app, "app"))
        .send()
        .await
        .unwrap();
    assert_eq!(selected.status().as_u16(), 204);
    let logout = app
        .client
        .delete(format!("{}/api/v1/auth/logout", app.origin))
        .bearer_auth(credential_token(&app, "app"))
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status().as_u16(), 204);
    let rows = account_versions(
        &app,
        "AuthenticationToken",
        &[
            "auth_token/api_session/revoked",
            "auth_token/api_app_token/revoked",
        ],
    )
    .await;
    assert_eq!(rows.len(), 2);
    for row in rows {
        let event = row.event;
        let object = row.object.unwrap();
        let metadata: Value = serde_json::from_str(&object).unwrap();
        let expected_id = if event == "auth_token/api_app_token/revoked" {
            credentials.app_token.id
        } else {
            credentials.expired.id
        };
        assert_eq!(metadata["credential_id"], expected_id);
        assert_eq!(
            row.whodunnit.as_deref(),
            Some(app.fixture.account.id.to_string().as_str())
        );
        let context = row.audit_context;
        assert_eq!(context["actor_account_id"], app.fixture.account.id);
        assert_eq!(context["actor_person_id"], app.fixture.adult.id);
        assert!(!object.contains(&credential_token(&app, "app")));
        assert!(!object.contains(&credentials.expired.refresh_token_digest));
    }
    let security = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::ActorAccountId.eq(app.fixture.account.id))
        .filter(security_audit_events::Column::EventType.is_in([
            "auth_token/api_session/revoked",
            "auth_token/api_app_token/revoked",
        ]))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(security.len(), 2);
    for row in security {
        let event = row.event_type;
        let metadata = row.metadata;
        assert_eq!(
            metadata["credential_id"],
            if event == "auth_token/api_app_token/revoked" {
                credentials.app_token.id
            } else {
                credentials.expired.id
            }
        );
    }
    app.close().await;
}

#[tokio::test]
async fn insufficient_mobile_scope_preserves_the_bearer_challenge() {
    let app = Application::new().await;
    let token = app.token().await;
    let current = oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut grant = current.clone().into_active_model();
    grant.access_token_scopes = Set(Some("patient/*.read".into()));
    grant.access_token_scope_hash = Set(current.token_hash);
    grant.update(&app.fixture.admin).await.unwrap();
    let response = app
        .client
        .get(format!("{}/api/v1/auth/sessions", app.origin))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 403);
    let challenge = response
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(challenge.contains("insufficient_scope"));
    app.close().await;
}

#[tokio::test]
async fn successful_mobile_bearer_use_extends_the_inactivity_deadline() {
    let app = Application::new().await;
    let token = app.token().await;
    let before = last_used(&app).await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/people",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
    let after = last_used(&app).await;
    assert!(after > before + chrono::Duration::days(1));
    app.close().await;
}

#[tokio::test]
async fn rejected_mobile_bearers_do_not_restore_activity() {
    let app = Application::new().await;
    let token = app.token().await;
    for kind in 0..3 {
        let now = chrono::Utc::now().naive_utc();
        let current = oauth_grants::Entity::find_by_id(app.fixture.authorization.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let mut grant = current.into_active_model();
        match kind {
            0 => {
                grant.last_used_at = Set(Some(now - chrono::Duration::days(31)));
            }
            1 => {
                grant.last_used_at = Set(Some(now - chrono::Duration::days(1)));
                grant.expires_in = Set(now - chrono::Duration::minutes(1));
            }
            _ => {
                grant.expires_in = Set(now + chrono::Duration::minutes(15));
                grant.revoked_at = Set(Some(now));
            }
        }
        grant.update(&app.fixture.admin).await.unwrap();
        let before = last_used(&app).await;
        let response = app
            .client
            .get(format!(
                "{}/api/v1/households/{}/people",
                app.origin, app.fixture.household.id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        let after = last_used(&app).await;
        assert_eq!(after, before);
    }
    app.close().await;
}

#[tokio::test]
async fn same_application_mobile_revocations_identify_each_grant_in_audit() {
    let app = Application::new().await;
    let token = app.token().await;
    let second = mobile_grant(&app, 1, false).await;
    let path = format!("{}/api/v1/auth/sessions", app.origin);
    for id in [second.id, app.fixture.authorization.id] {
        let response = app
            .client
            .delete(format!("{path}/{id}"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 204);
    }
    let rows = account_versions(&app, "Account", &["mobile_oauth.revoked"]).await;
    assert_eq!(rows.len(), 2);
    let ids = rows
        .into_iter()
        .map(|row| {
            let object = row.object.unwrap();
            let metadata: Value = serde_json::from_str(&object).unwrap();
            metadata["oauth_grant_id"].as_i64().unwrap()
        })
        .collect::<Vec<_>>();
    assert!(ids.contains(&app.fixture.authorization.id));
    assert!(ids.contains(&second.id));
    app.close().await;
}

#[tokio::test]
async fn account_discovery_and_sessions_follow_contract_and_household_visibility() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let credentials = stored_credentials(&app).await;
    let contract = contract();
    let operation = &contract["paths"]["/auth/households"]["get"];
    assert_eq!(operation["operationId"], "listHouseholds");
    assert_eq!(
        operation["responses"]["401"]["$ref"],
        "#/components/responses/Unauthorized"
    );

    let list = app
        .client
        .get(format!("{}/api/v1/auth/households", app.origin))
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/AuthHouseholdCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "auth household collection",
    );
    assert_eq!(body["account_id"], app.fixture.account.id);
    let households = body["data"].as_array().unwrap();
    assert_eq!(households.len(), 1);
    assert_eq!(households[0]["id"], app.fixture.household.id);
    assert_eq!(households[0]["membership_id"], app.fixture.membership.id);

    let anonymous = app
        .client
        .get(format!("{}/api/v1/auth/households", app.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status().as_u16(), 401);
    let operation = &contract["paths"]["/auth/sessions"]["get"];
    assert_eq!(operation["operationId"], "listSessions");
    assert_eq!(
        operation["responses"]["401"]["$ref"],
        "#/components/responses/Unauthorized"
    );

    let list = app
        .client
        .get(format!("{}/api/v1/auth/sessions", app.origin))
        .bearer_auth(credential_token(&app, "current"))
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let text = list.text().await.unwrap();
    assert_eq!(list_status, 200);
    for secret in [
        credential_token(&app, "current"),
        credentials.current.refresh_token_digest.clone(),
        credentials.expired.access_token_digest.clone(),
        credentials.revoked.access_token_digest.clone(),
    ] {
        assert!(!text.contains(&secret), "leaked {secret}");
    }
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/AuthSessionCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "auth session collection",
    );
    let sessions = body["data"].as_array().unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0]["id"], credentials.current.id);
    assert_eq!(sessions[0]["device_name"], "Current phone");
    assert_eq!(sessions[1]["id"], credentials.expired.id);
    assert!(
        sessions
            .iter()
            .all(|session| session["id"] != credentials.revoked.id)
    );

    let anonymous = app
        .client
        .get(format!("{}/api/v1/auth/sessions", app.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status().as_u16(), 401);
    assert_expanded_account_collections(&app, &credentials).await;
    app.close().await;
}
