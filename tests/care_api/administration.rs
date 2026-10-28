use super::*;
use med_tracker::models::_entities::{
    api_idempotency_keys, carer_relationships, households, people, person_access_grants,
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};

async fn administration_app() -> Application {
    let app = Application::new().await;
    let mut household = app.fixture.household.clone().into_active_model();
    household.timezone = Set("Europe/London".into());
    household.update(&app.fixture.admin).await.unwrap();
    app
}

pub(super) async fn additional_member(app: &Application) -> household_memberships::Model {
    let actor = app.fixture.foreign_household().await;
    let now = chrono::Utc::now().naive_utc();
    let membership = household_memberships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(actor.account.person_id),
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
    actor.close().await;
    membership
}

async fn membership(app: &Application, id: i64) -> household_memberships::Model {
    household_memberships::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

async fn household(app: &Application) -> households::Model {
    households::Entity::find_by_id(app.fixture.household.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

async fn role(app: &Application, role: &str) {
    let mut membership = membership(app, app.fixture.membership.id)
        .await
        .into_active_model();
    membership.role = Set(role.into());
    membership.update(&app.fixture.admin).await.unwrap();
}

async fn relationship(app: &Application, patient: i64, kind: &str) -> carer_relationships::Model {
    let now = chrono::Utc::now().naive_utc();
    carer_relationships::ActiveModel {
        household_id: Set(app.fixture.household.id),
        carer_id: Set(app.fixture.adult.id),
        patient_id: Set(patient),
        relationship_type: Set(Some(kind.into())),
        active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}

async fn grant(
    app: &Application,
    membership: i64,
    relationship: Option<i64>,
    kind: &str,
) -> person_access_grants::Model {
    let now = chrono::Utc::now().naive_utc();
    person_access_grants::ActiveModel {
        household_id: Set(app.fixture.household.id),
        household_membership_id: Set(membership),
        person_id: Set(app.fixture.minor.id),
        access_level: Set("manage".into()),
        relationship_type: Set(kind.into()),
        carer_relationship_id: Set(relationship),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}

async fn version_count(app: &Application, kind: &str, event: Option<&str>) -> i64 {
    let mut query = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(versions::Column::ItemType.eq(kind));
    if let Some(event) = event {
        query = query.filter(versions::Column::Event.eq(event));
    }
    i64::try_from(query.count(&app.fixture.admin).await.unwrap()).unwrap()
}

async fn grant_counts(app: &Application, member: i64) -> (i64, i64) {
    let query = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdMembershipId.eq(member))
        .filter(person_access_grants::Column::PersonId.eq(app.fixture.minor.id));
    let total = query.clone().count(&app.fixture.admin).await.unwrap();
    let active = query
        .filter(person_access_grants::Column::RevokedAt.is_null())
        .count(&app.fixture.admin)
        .await
        .unwrap();
    (
        i64::try_from(total).unwrap(),
        i64::try_from(active).unwrap(),
    )
}

async fn rejected_count(app: &Application) -> i64 {
    let query = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(
            security_audit_events::Column::EventType.eq("household_access.person_grant_changed"),
        )
        .filter(
            Func::cust("jsonb_extract_path_text")
                .args([
                    Expr::col(security_audit_events::Column::Metadata),
                    Expr::val("outcome"),
                ])
                .eq("rejected"),
        );
    i64::try_from(query.count(&app.fixture.admin).await.unwrap()).unwrap()
}

async fn denied_attempts(app: &Application, request: &str) -> i64 {
    let query = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .filter(security_audit_events::Column::RequestId.eq(request))
        .filter(
            Func::cust("jsonb_extract_path_text")
                .args([
                    Expr::col(security_audit_events::Column::Metadata),
                    Expr::val("status"),
                ])
                .eq("403"),
        );
    i64::try_from(query.count(&app.fixture.admin).await.unwrap()).unwrap()
}

fn saved_digest(method: &str, path: &str, body: &str, household: i64) -> String {
    use sha2::{Digest, Sha256};
    #[derive(serde::Serialize)]
    struct SavedRequest<'a> {
        method: &'a str,
        path: &'a str,
        params: Value,
    }
    let mut params = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_str(body).unwrap()
    };
    params["household_id"] = json!(household.to_string());
    if let Some(id) = path
        .rsplit('/')
        .next()
        .filter(|id| id.parse::<i64>().is_ok())
    {
        params["id"] = json!(id);
    }
    hex::encode(Sha256::digest(
        serde_json::to_vec(&SavedRequest {
            method,
            path,
            params,
        })
        .unwrap(),
    ))
}

#[tokio::test]
async fn household_relationship_deactivation_repeat_preserves_timestamp_records_and_permissions() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::administration::delegation,
    };
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let relationship = relationship(&app, app.fixture.adult.id, "self").await;
    let relationship_id = relationship.id;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-relationship-repeat".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let first = delegation::deactivate(&tenant, relationship_id, None)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let repeated = delegation::deactivate(&tenant, relationship_id, None)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let relationships = carer_relationships::Entity::find()
        .filter(carer_relationships::Column::HouseholdId.eq(household_id))
        .filter(carer_relationships::Column::CarerId.eq(app.fixture.adult.id))
        .filter(carer_relationships::Column::PatientId.eq(app.fixture.adult.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let counts = vec![
        i64::try_from(relationships).unwrap(),
        version_count(&app, "CarerRelationship", None).await,
        i64::from(
            membership(&app, app.fixture.membership.id)
                .await
                .permissions_version,
        ),
    ];
    app.close().await;
    assert_eq!(first.id, repeated.id);
    assert_eq!(counts, vec![1, 1, 1]);
    assert_eq!(first.updated_at, repeated.updated_at);
}

#[tokio::test]
async fn household_settings_partial_update_replay_and_permission_order() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/settings",
        app.origin
    );
    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let body = json!({"household":{"name":"Synthetic renamed household"}});
    let update = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-settings-key", app.fixture.client_uuid),
        )
        .json(&body)
        .send()
        .await
        .unwrap();
    let update_status = update.status().as_u16();
    let update: Value = update.json().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-settings-key", app.fixture.client_uuid),
        )
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay.headers().get("idempotency-replayed").is_some();
    let replay: Value = replay.json().await.unwrap_or(Value::Null);
    role(&app, "member").await;
    let denied = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let saved = household(&app).await;
    let name = saved.name;
    let zone = saved.timezone;
    let audits = version_count(&app, "Household", Some("update")).await;
    app.close().await;
    assert_eq!(
        (read_status, update_status, replay_status, denied_status),
        (200, 200, 200, 403)
    );
    assert!(replay_marker);
    assert_eq!(replay, update);
    assert_eq!(
        (name, zone, audits),
        (
            "Synthetic renamed household".into(),
            "Europe/London".into(),
            1
        )
    );
}

#[tokio::test]
async fn household_membership_transitions_preserve_last_owner_and_increment_permissions() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    role(&app, "owner").await;
    let target = additional_member(&app).await;
    let target_id = target.id;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/memberships",
        app.origin
    );
    let listed = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = listed.status().as_u16();
    let changed = app
        .client
        .patch(format!("{endpoint}/{target_id}"))
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"role":"administrator"}}))
        .send()
        .await
        .unwrap();
    let change_status = changed.status().as_u16();
    let last_owner = app
        .client
        .delete(format!("{endpoint}/{}", app.fixture.membership.id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let last_owner_status = last_owner.status().as_u16();
    let promotion = app
        .client
        .patch(format!("{endpoint}/{target_id}"))
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"role":"owner"}}))
        .send()
        .await
        .unwrap();
    let promotion_status = promotion.status().as_u16();
    let owners = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(household_id))
        .filter(household_memberships::Column::Role.eq("owner"))
        .filter(household_memberships::Column::Status.eq("active"))
        .filter(household_memberships::Column::RevokedAt.is_null())
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let saved = membership(&app, target.id).await;
    let role = saved.role;
    let version = saved.permissions_version;
    app.close().await;
    assert_eq!(
        (
            list_status,
            change_status,
            last_owner_status,
            promotion_status
        ),
        (200, 200, 422, 422)
    );
    assert_eq!((owners, role, version), (1, "administrator".into(), 2));
}

#[tokio::test]
async fn household_grant_rejection_and_existing_revoke_replay_preserve_live_permission_version() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let target = additional_member(&app).await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/person_access_grants",
        app.origin
    );
    let extra_grant = grant(&app, target.id, None, "parent").await;
    let grant_id = extra_grant.id;
    let body = json!({"person_access_grant":{"household_membership_id":target.id,"person_id":app.fixture.minor.id,"access_level":"manage","relationship_type":"parent"}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-grant-key", app.fixture.client_uuid),
        )
        .json(&body)
        .send()
        .await
        .unwrap();
    let create_status = created.status().as_u16();
    let created: Value = created.json().await.unwrap_or(Value::Null);
    assert_eq!(created["error"]["code"], "forbidden");
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-grant-key", app.fixture.client_uuid),
        )
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let marker = replay.headers().get("idempotency-replayed").is_some();
    let revoked = app
        .client
        .delete(format!("{endpoint}/{grant_id}"))
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!(
                "{}-synthetic-existing-grant-revoke",
                app.fixture.client_uuid
            ),
        )
        .send()
        .await
        .unwrap();
    let revoke_status = revoked.status().as_u16();
    let revoke_replay = app
        .client
        .delete(format!("{endpoint}/{grant_id}"))
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!(
                "{}-synthetic-existing-grant-revoke",
                app.fixture.client_uuid
            ),
        )
        .send()
        .await
        .unwrap();
    let revoke_replay_status = revoke_replay.status().as_u16();
    let revoke_marker = revoke_replay
        .headers()
        .get("idempotency-replayed")
        .is_some();
    let version = membership(&app, target.id).await.permissions_version;
    let (total, active) = grant_counts(&app, target.id).await;
    app.close().await;
    assert_eq!(
        (
            create_status,
            replay_status,
            revoke_status,
            revoke_replay_status
        ),
        (403, 403, 204, 204)
    );
    assert!(!marker);
    assert!(revoke_marker);
    assert_eq!((version, active, total), (2, 0, 1));
}

#[tokio::test]
async fn household_membership_status_and_person_link_transitions_are_scoped() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let target = additional_member(&app).await;
    let target_id = target.id;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/memberships/{target_id}",
        app.origin
    );
    let suspend = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"status":"suspended"}}))
        .send()
        .await
        .unwrap();
    let reactivate = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"status":"active","person_id":target.person_id}}))
        .send()
        .await
        .unwrap();
    let invalid = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"person_id":-1}}))
        .send()
        .await
        .unwrap();
    let saved = membership(&app, target.id).await;
    let status = saved.status;
    let person = Some(saved.person_id);
    let version = saved.permissions_version;
    let target_person = target.person_id;
    let statuses = (
        suspend.status().as_u16(),
        reactivate.status().as_u16(),
        invalid.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (200, 200, 422));
    assert_eq!(
        (status, person, version),
        ("active".into(), Some(target_person), 3)
    );
}

#[tokio::test]
async fn household_grant_validation_permission_and_relationship_ownership_prevent_writes() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/person_access_grants",
        app.origin
    );
    let invalid=app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person_access_grant":{"household_membership_id":app.fixture.membership.id,"person_id":app.fixture.minor.id,"access_level":"owner","relationship_type":"parent"}})).send().await.unwrap();
    let foreign=app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person_access_grant":{"household_membership_id":app.fixture.membership.id,"person_id":-1,"access_level":"manage","relationship_type":"parent"}})).send().await.unwrap();
    let relationship = relationship(&app, app.fixture.minor.id, "parent").await;
    let relationship_id = relationship.id;
    let extra_grant = grant(
        &app,
        app.fixture.membership.id,
        Some(relationship_id),
        "parent",
    )
    .await;
    let grant_id = extra_grant.id;
    let owned = app
        .client
        .delete(format!("{endpoint}/{grant_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    role(&app, "member").await;
    let denied = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    let saved = person_access_grants::Entity::find_by_id(extra_grant.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let n = i64::from(saved.revoked_at.is_none());
    let rejected = rejected_count(&app).await;
    let statuses = (
        invalid.status().as_u16(),
        foreign.status().as_u16(),
        owned.status().as_u16(),
        denied.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (403, 403, 422, 403));
    assert_eq!(n, 1);
    assert_eq!(rejected, 3);
}

#[tokio::test]
async fn household_settings_and_grant_audit_failure_roll_back_effects_and_keys() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let fault = format!("administration_audit_{}", uuid::Uuid::new_v4().simple());
    let probe = format!("{fault}_probe");
    // SeaORM has no trigger, PL/pgSQL function or GRANT builder.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE {probe}; GRANT USAGE,SELECT ON SEQUENCE {probe} TO med_tracker_app; CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type IN ('api/admin/household_settings/updated','household_access.person_grant_changed') THEN PERFORM nextval('{probe}'); RAISE EXCEPTION 'Synthetic administration audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION {fault}()")).await.unwrap();
    let settings = app
        .client
        .patch(format!(
            "{}/api/v1/households/{household_id}/admin/settings",
            app.origin
        ))
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-rollback-settings", app.fixture.client_uuid),
        )
        .json(&json!({"household":{"name":"Must roll back settings"}}))
        .send()
        .await
        .unwrap();
    let grant=app.client.post(format!("{}/api/v1/households/{household_id}/admin/person_access_grants",app.origin)).bearer_auth(&token).header("idempotency-key",format!("{}-synthetic-rollback-grant",app.fixture.client_uuid)).json(&json!({"person_access_grant":{"household_membership_id":app.fixture.membership.id,"person_id":app.fixture.minor.id,"access_level":"manage","relationship_type":"parent"}})).send().await.unwrap();
    let query = Query::select()
        .column("is_called")
        .from(sea_orm::sea_query::Alias::new(probe.clone()))
        .to_owned();
    let reached: bool = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "is_called")
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {fault} ON security_audit_events; DROP FUNCTION {fault}(); DROP SEQUENCE {probe}")).await.unwrap();
    let settings_count = i64::from(household(&app).await.name == "Must roll back settings");
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdId.eq(household_id))
        .filter(person_access_grants::Column::PersonId.eq(app.fixture.minor.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let keys = api_idempotency_keys::Entity::find()
        .filter(api_idempotency_keys::Column::HouseholdId.eq(household_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let counts = vec![
        settings_count,
        i64::try_from(grants).unwrap(),
        i64::try_from(keys).unwrap(),
    ];
    let version = membership(&app, app.fixture.membership.id)
        .await
        .permissions_version;

    let statuses = (settings.status().as_u16(), grant.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (500, 500));
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0]);
    assert_eq!(version, 1);
}

#[tokio::test]
async fn household_settings_concurrent_key_retries_apply_once_and_conflict_on_changed_body() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/settings",
        app.origin
    );
    let body = json!({"household":{"name":"Synthetic concurrent household"}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-settings-concurrent", app.fixture.client_uuid),
        )
        .json(&body)
        .send();
    let second = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-settings-concurrent", app.fixture.client_uuid),
        )
        .json(&body)
        .send();
    let (first, second) = tokio::join!(first, second);
    let first = first.unwrap();
    let second = second.unwrap();
    let markers = usize::from(first.headers().get("idempotency-replayed").is_some())
        + usize::from(second.headers().get("idempotency-replayed").is_some());
    let changed = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header(
            "idempotency-key",
            format!("{}-synthetic-settings-concurrent", app.fixture.client_uuid),
        )
        .json(&json!({"household":{"name":"Must not replace"}}))
        .send()
        .await
        .unwrap();
    let n = version_count(&app, "Household", Some("update")).await;
    let statuses = (
        first.status().as_u16(),
        second.status().as_u16(),
        changed.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (200, 200, 409));
    assert_eq!((markers, n), (1, 1));
}

#[tokio::test]
async fn household_administration_saved_rails_retries_preserve_effects() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let target = additional_member(&app).await;
    let target_id = target.id;
    let extra_grant = grant(&app, target.id, None, "parent").await;
    let grant_id = extra_grant.id;
    let cases=[
        ("PATCH",format!("/api/v1/households/{household_id}/admin/settings"),json!({"household":{"name":"Synthetic Rails household"}}).to_string(),200),
        ("PATCH",format!("/api/v1/households/{household_id}/admin/memberships/{target_id}"),json!({"household_membership":{"role":"administrator"}}).to_string(),200),
        ("POST",format!("/api/v1/households/{household_id}/admin/person_access_grants"),json!({"person_access_grant":{"household_membership_id":target.id,"person_id":app.fixture.minor.id,"access_level":"manage","relationship_type":"parent"}}).to_string(),403),
        ("DELETE",format!("/api/v1/households/{household_id}/admin/person_access_grants/{grant_id}"),String::new(),204),
        ("DELETE",format!("/api/v1/households/{household_id}/admin/memberships/{target_id}"),String::new(),204),
    ];
    let mut statuses = Vec::new();
    let mut markers = Vec::new();
    let mut equal = Vec::new();
    let mut empty_delete_bodies = Vec::new();
    for (method, path, body, status) in cases {
        let path = path.as_str();
        let body = body.as_str();
        let digest = saved_digest(method, path, body, household_id);
        let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap();
        let original = app
            .client
            .request(method.clone(), format!("{}{path}", app.origin))
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .body(body.to_owned())
            .send()
            .await
            .unwrap();
        statuses.push(original.status().as_u16());
        let original_request_id = original
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut original: Value = if status == 204 {
            json!({})
        } else {
            original.json().await.unwrap()
        };
        let key = format!(
            "{}-synthetic-rails-admin-{status}-{path}",
            app.fixture.client_uuid
        );
        let now = chrono::Utc::now().naive_utc();
        api_idempotency_keys::ActiveModel {
            household_id: Set(household_id),
            account_id: Set(app.fixture.account.id),
            key: Set(key.clone()),
            request_method: Set(method.as_str().into()),
            request_path: Set(path.into()),
            request_digest: Set(digest),
            response_status: Set(status),
            response_body: Set(original.clone()),
            response_headers: Set(json!({})),
            expires_at: Set(now + chrono::Duration::hours(24)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
        if status == 403 {
            use med_tracker::models::{
                access::{self, Actor, HouseholdScope},
                care::locations::{self, KeyedResponse},
            };
            let tenant = access::begin(
                &app.fixture.runtime,
                &HouseholdScope {
                    actor: Actor {
                        account_id: app.fixture.account.id,
                    },
                    household_id: app.fixture.household.id,
                    request_id: "synthetic-cached-denial-diagnostic".into(),
                },
                None,
            )
            .await
            .unwrap();
            let input: Value = serde_json::from_str(body).unwrap();
            let cached = locations::lookup(&tenant, &key, method.as_str(), path, &input)
                .await
                .unwrap();
            assert!(
                matches!(cached, KeyedResponse::Replay(_)),
                "Exact denied-response fixture must reach maintained restricted cache lookup"
            );
            tenant.rollback().await.unwrap();
        }
        let replay = app
            .client
            .request(method.clone(), format!("{}{path}", app.origin))
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .header("idempotency-key", key.clone())
            .header("x-request-id", uuid::Uuid::new_v4().to_string())
            .body(body.to_owned())
            .send()
            .await
            .unwrap();
        statuses.push(replay.status().as_u16());
        let replay_request_id = replay
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        markers.push(replay.headers().get("idempotency-replayed").is_some());
        let mut replay: Value = if status == 204 {
            empty_delete_bodies.push(replay.bytes().await.unwrap().is_empty());
            json!({})
        } else {
            replay.json().await.unwrap()
        };
        if status == 403 {
            assert_eq!(rejected_count(&app).await, 1);
            assert_eq!(original["error"]["code"], "forbidden");
            assert_eq!(replay["error"]["code"], "forbidden");
            assert_eq!(
                original["error"]["request_id"].as_str(),
                original_request_id.as_deref()
            );
            assert_eq!(
                replay["error"]["request_id"].as_str(),
                replay_request_id.as_deref()
            );
            assert_eq!(
                denied_attempts(&app, replay_request_id.as_deref().unwrap()).await,
                1
            );
            original["error"]["request_id"] = Value::Null;
            replay["error"]["request_id"] = Value::Null;
            let default_id = app
                .client
                .request(method, format!("{}{path}", app.origin))
                .bearer_auth(&token)
                .header("content-type", "application/json")
                .header("idempotency-key", key)
                .body(body.to_owned())
                .send()
                .await
                .unwrap();
            assert_eq!(default_id.status().as_u16(), 403);
            assert!(
                default_id.headers().get("idempotency-replayed").is_some(),
                "Default server request IDs must preserve cached denial replay metadata"
            );
            let default_request_id = default_id
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            assert!(default_request_id.is_some());
            let mut default_body: Value = default_id.json().await.unwrap();
            assert_eq!(
                default_body["error"]["request_id"].as_str(),
                default_request_id.as_deref()
            );
            assert_eq!(default_body["error"]["code"], "forbidden");
            default_body["error"]["request_id"] = Value::Null;
            assert_eq!(default_body, original);
            assert_eq!(
                denied_attempts(&app, default_request_id.as_deref().unwrap()).await,
                1
            );
            assert_eq!(rejected_count(&app).await, 1);
        }
        equal.push(replay == original);
    }
    let settings = version_count(&app, "Household", Some("update")).await;
    let version = membership(&app, target.id).await.permissions_version;
    let (grants, _) = grant_counts(&app, target.id).await;
    app.close().await;
    assert_eq!(
        statuses,
        vec![200, 200, 200, 200, 403, 403, 204, 204, 204, 204]
    );
    assert_eq!(markers, vec![true; 5]);
    assert_eq!(empty_delete_bodies, vec![true; 2]);
    assert_eq!(equal, vec![true; 5]);
    assert_eq!((settings, version, grants), (1, 4, 1));
}

#[tokio::test]
async fn household_administration_strong_parameters_filter_protected_fields_and_reject_empty() {
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let target = additional_member(&app).await;
    let target_id = target.id;
    let settings = format!(
        "{}/api/v1/households/{household_id}/admin/settings",
        app.origin
    );
    let memberships = format!(
        "{}/api/v1/households/{household_id}/admin/memberships/{target_id}",
        app.origin
    );
    let grants = format!(
        "{}/api/v1/households/{household_id}/admin/person_access_grants",
        app.origin
    );
    let changed=app.client.patch(&settings).bearer_auth(&token).header("idempotency-key",format!("{}-synthetic-admin-filter",app.fixture.client_uuid)).json(&json!({"household":{"name":"Synthetic filtered household","status":"archived"},"ignored":1})).send().await.unwrap();
    let member=app.client.patch(&memberships).bearer_auth(&token).json(&json!({"household_membership":{"role":"administrator","permissions_version":900},"ignored":1})).send().await.unwrap();
    let grant=app.client.post(&grants).bearer_auth(&token).json(&json!({"person_access_grant":{"household_membership_id":target.id,"person_id":app.fixture.minor.id,"access_level":"manage","relationship_type":"parent","revoked_at":"2000-01-01"},"ignored":1})).send().await.unwrap();
    let empty_settings = app
        .client
        .patch(&settings)
        .bearer_auth(&token)
        .json(&json!({"household":{"status":"archived"}}))
        .send()
        .await
        .unwrap();
    let empty_member = app
        .client
        .patch(&memberships)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{}}))
        .send()
        .await
        .unwrap();
    let empty_grant = app
        .client
        .post(&grants)
        .bearer_auth(&token)
        .json(&json!({"person_access_grant":{}}))
        .send()
        .await
        .unwrap();
    let conflict=app.client.patch(&settings).bearer_auth(&token).header("idempotency-key",format!("{}-synthetic-admin-filter",app.fixture.client_uuid)).json(&json!({"household":{"name":"Synthetic filtered household","status":"archived"},"ignored":2})).send().await.unwrap();
    let statuses = vec![
        changed.status().as_u16(),
        member.status().as_u16(),
        grant.status().as_u16(),
        empty_settings.status().as_u16(),
        empty_member.status().as_u16(),
        empty_grant.status().as_u16(),
        conflict.status().as_u16(),
    ];
    let status = household(&app).await.status;
    let version = membership(&app, target.id).await.permissions_version;
    let (_, n) = grant_counts(&app, target.id).await;
    app.close().await;
    assert_eq!(statuses, vec![200, 200, 403, 400, 400, 400, 409]);
    assert_eq!((status, version, n), ("active".into(), 2, 0));
}

#[tokio::test]
async fn household_admin_settings_get_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/settings",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/admin/settings"]["get"];
    assert_eq!(operation["operationId"], "getHouseholdAdminSettings");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    assert!(
        operation["responses"]["200"].get("headers").is_none(),
        "getHouseholdAdminSettings must not promise cache headers"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let read = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let body: Value = read.json().await.unwrap();
    assert_eq!(read_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/HouseholdAdminSettingsResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "household settings response",
    );
    assert_eq!(body["data"]["id"], household_id);
    assert_eq!(body["data"]["name"], "Synthetic household");
    assert_eq!(body["data"]["slug"], app.fixture.household.slug);
    assert_eq!(body["data"]["timezone"], "Europe/London");
    assert_eq!(body["data"]["subscription_plan"], "free");

    role(&app, "member").await;
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_request_id = denied.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &denied_body,
        "member settings read",
    );
    assert_eq!(denied_body["error"]["code"], "forbidden");
    assert_eq!(denied_body["error"]["request_id"], denied_request_id);

    let other = app.fixture.foreign_household().await;
    let other_id = other.household.id;
    other.close().await;
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{other_id}/admin/settings",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_request_id = foreign.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household settings",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);
    app.close().await;
}

#[tokio::test]
async fn household_membership_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/memberships",
        app.origin
    );
    let contract = contract();
    let operation = &contract["paths"]["/households/{household_id}/admin/memberships"]["get"];
    assert_eq!(operation["operationId"], "listMemberships");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let list = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/HouseholdMembershipCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "membership collection",
    );
    let members = body["data"].as_array().unwrap();
    assert_eq!(members.len(), 3);
    assert_eq!(members[0]["id"], app.fixture.membership.id);
    assert_eq!(members[0]["account_id"], app.fixture.account.id);
    assert_eq!(members[0]["email"], app.fixture.account.email);
    assert_eq!(members[0]["person_id"], app.fixture.adult.id);
    assert_eq!(members[0]["person_name"], "Synthetic adult");
    assert_eq!(members[0]["role"], "administrator");
    assert_eq!(members[0]["person_id"], app.fixture.adult.id);
    assert!(members[0].get("user_id").is_none());

    let other = app.fixture.foreign_household().await;
    let other_id = other.household.id;
    other.close().await;
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{other_id}/admin/memberships",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_request_id = foreign.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household memberships",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    assert_eq!(foreign_body["error"]["request_id"], foreign_request_id);

    role(&app, "member").await;
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_request_id = denied.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &denied_body,
        "member membership list",
    );
    assert_eq!(denied_body["error"]["code"], "forbidden");
    assert_eq!(denied_body["error"]["request_id"], denied_request_id);

    app.close().await;
}
#[tokio::test]
async fn person_access_grant_list_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = administration_app().await;
    let household_id = app.fixture.household.id;
    let other = app.fixture.foreign_household().await;
    let other_id = other.household.id;
    other.close().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/admin/person_access_grants",
        app.origin
    );
    let contract = contract();
    let operation =
        &contract["paths"]["/households/{household_id}/admin/person_access_grants"]["get"];
    assert_eq!(operation["operationId"], "listPersonAccessGrants");
    assert_eq!(
        operation["responses"]["403"]["$ref"],
        "#/components/responses/Forbidden"
    );
    let forbidden = resolve(contract, &operation["responses"]["403"]);
    let forbidden_schema = resolve(
        contract,
        &forbidden["content"]["application/json"]["schema"],
    );

    let list = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let list_status = list.status().as_u16();
    let body: Value = list.json().await.unwrap();
    assert_eq!(list_status, 200);
    assert_eq!(
        operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/PersonAccessGrantCollectionResponse"
    );
    assert_value(
        contract,
        resolve(
            contract,
            &operation["responses"]["200"]["content"]["application/json"]["schema"],
        ),
        &body,
        "person access grant collection",
    );
    let grants = body["data"].as_array().unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0]["id"], app.fixture.grant.id);
    assert_eq!(grants[0]["person_id"], app.fixture.adult.id);
    assert_eq!(grants[0]["access_level"], "manage");
    assert_eq!(grants[0]["relationship_type"], "self");

    role(&app, "member").await;
    let denied = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    let denied_body: Value = denied.json().await.unwrap();
    assert_eq!(denied_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &denied_body,
        "member grant list",
    );
    assert_eq!(denied_body["error"]["code"], "forbidden");

    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/{other_id}/admin/person_access_grants",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let foreign_status = foreign.status().as_u16();
    let foreign_body: Value = foreign.json().await.unwrap();
    assert_eq!(foreign_status, 403);
    assert_value(
        contract,
        forbidden_schema,
        &foreign_body,
        "foreign household grants",
    );
    assert_eq!(foreign_body["error"]["code"], "forbidden");
    app.close().await;
}

#[tokio::test]
async fn household_relationship_removal_preserves_unrelated_grant_without_veto() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::administration::delegation,
    };
    let app = administration_app().await;
    let relationship = relationship(&app, app.fixture.minor.id, "parent").await;
    let relationship_id = relationship.id;
    let extra_grant = grant(&app, app.fixture.membership.id, None, "family_member").await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: app.fixture.account.id,
        },
        household_id: app.fixture.household.id,
        request_id: "synthetic-carer-removal-manual-grant".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let result = delegation::deactivate(&tenant, relationship_id, None).await;
    let success = result.is_ok();
    if success {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let active = carer_relationships::Entity::find_by_id(relationship.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .active;
    let manual = person_access_grants::Entity::find_by_id(extra_grant.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .revoked_at
        .is_none();
    let capacity = people::Entity::find_by_id(app.fixture.minor.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .has_capacity;
    app.close().await;
    assert!(success);
    assert!(!active);
    assert!(manual);
    assert!(!capacity);
}
