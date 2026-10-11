use super::*;

#[tokio::test]
async fn household_relationship_repeat_preserves_timestamp_records_and_permissions() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::administration::delegation,
    };
    let app = Application::new().await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-relationship-repeat".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let first = delegation::assign(&tenant, 73001, 73001, "self", None)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let repeated = delegation::assign(&tenant, 73001, 73001, "self", None)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM carer_relationships WHERE carer_id=73001 AND patient_id=73001) AS relationships,(SELECT count(*) FROM versions WHERE item_type='CarerRelationship') AS versions,(SELECT permissions_version::bigint FROM household_memberships WHERE id=74001) AS permissions")).await.unwrap().unwrap();
    let counts: Vec<i64> = ["relationships", "versions", "permissions"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(first.id, repeated.id);
    assert_eq!(counts, vec![1, 1, 1]);
    assert_eq!(first.updated_at, repeated.updated_at);
}

#[tokio::test]
async fn household_settings_partial_update_replay_and_permission_order() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/admin/settings", app.origin);
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
        .header("idempotency-key", "synthetic-settings-key")
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
        .header("idempotency-key", "synthetic-settings-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay.headers().get("idempotency-replayed").is_some();
    let replay: Value = replay.json().await.unwrap_or(Value::Null);
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
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
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT name,timezone,(SELECT count(*) FROM versions WHERE item_type='Household' AND event='update') AS audits FROM households WHERE id=72001")).await.unwrap().unwrap();
    let name: String = row.try_get("", "name").unwrap();
    let zone: String = row.try_get("", "timezone").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
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
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001; INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71002,'synthetic-target@example.test',2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71002,72001,73002,'member','active',now(),now(),now())").await.unwrap();
    let endpoint = format!("{}/api/v1/households/72001/admin/memberships", app.origin);
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
        .patch(format!("{endpoint}/74002"))
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"role":"administrator"}}))
        .send()
        .await
        .unwrap();
    let change_status = changed.status().as_u16();
    let last_owner = app
        .client
        .delete(format!("{endpoint}/74001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let last_owner_status = last_owner.status().as_u16();
    let promotion = app
        .client
        .patch(format!("{endpoint}/74002"))
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"role":"owner"}}))
        .send()
        .await
        .unwrap();
    let promotion_status = promotion.status().as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM household_memberships WHERE household_id=72001 AND role='owner' AND status='active' AND revoked_at IS NULL) AS owners,role,permissions_version FROM household_memberships WHERE id=74002")).await.unwrap().unwrap();
    let owners: i64 = row.try_get("", "owners").unwrap();
    let role: String = row.try_get("", "role").unwrap();
    let version: i32 = row.try_get("", "permissions_version").unwrap();
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
async fn household_grant_create_revoke_replay_preserves_live_permission_version() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71002,'synthetic-grant-target@example.test',2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71002,72001,73002,'member','active',now(),now(),now())").await.unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/admin/person_access_grants",
        app.origin
    );
    let body = json!({"person_access_grant":{"household_membership_id":74002,"person_id":73002,"access_level":"manage","relationship_type":"parent"}});
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-grant-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let create_status = created.status().as_u16();
    let created: Value = created.json().await.unwrap_or(Value::Null);
    let id = created["data"]["id"].as_i64().unwrap_or(-1);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-grant-key")
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let marker = replay.headers().get("idempotency-replayed").is_some();
    let revoked = app
        .client
        .delete(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let revoke_status = revoked.status().as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT permissions_version,(SELECT count(*) FROM person_access_grants WHERE household_membership_id=74002 AND person_id=73002 AND revoked_at IS NULL) AS active,(SELECT count(*) FROM person_access_grants WHERE household_membership_id=74002 AND person_id=73002) AS total FROM household_memberships WHERE id=74002")).await.unwrap().unwrap();
    let version: i32 = row.try_get("", "permissions_version").unwrap();
    let active: i64 = row.try_get("", "active").unwrap();
    let total: i64 = row.try_get("", "total").unwrap();
    app.close().await;
    assert_eq!(
        (create_status, replay_status, revoke_status),
        (201, 201, 204)
    );
    assert!(marker);
    assert_eq!((version, active, total), (3, 0, 1));
}

#[tokio::test]
async fn household_membership_status_and_person_link_transitions_are_scoped() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71002,'synthetic-transition@example.test',2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71002,72001,73002,'member','active',now(),now(),now())").await.unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/admin/memberships/74002",
        app.origin
    );
    let suspend = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"status":"suspended","person_id":null}}))
        .send()
        .await
        .unwrap();
    let reactivate = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"status":"active","person_id":73002}}))
        .send()
        .await
        .unwrap();
    let invalid = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household_membership":{"person_id":999999}}))
        .send()
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,person_id,permissions_version FROM household_memberships WHERE id=74002",
        ))
        .await
        .unwrap()
        .unwrap();
    let status: String = row.try_get("", "status").unwrap();
    let person: Option<i64> = row.try_get("", "person_id").unwrap();
    let version: i32 = row.try_get("", "permissions_version").unwrap();
    let statuses = (
        suspend.status().as_u16(),
        reactivate.status().as_u16(),
        invalid.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (200, 200, 422));
    assert_eq!((status, person, version), ("active".into(), Some(73002), 3));
}

#[tokio::test]
async fn household_grant_validation_permission_and_relationship_ownership_prevent_writes() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/admin/person_access_grants",
        app.origin
    );
    let invalid=app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person_access_grant":{"household_membership_id":74001,"person_id":73002,"access_level":"owner","relationship_type":"parent"}})).send().await.unwrap();
    let foreign=app.client.post(&endpoint).bearer_auth(&token).json(&json!({"person_access_grant":{"household_membership_id":74001,"person_id":999999,"access_level":"manage","relationship_type":"parent"}})).send().await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(86001,72001,73001,73002,'parent',true,now(),now()); INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,carer_relationship_id,created_at,updated_at) VALUES(78002,72001,74001,73002,'manage','parent',86001,now(),now())").await.unwrap();
    let owned = app
        .client
        .delete(format!("{endpoint}/78002"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM person_access_grants WHERE id=78002 AND revoked_at IS NULL) AS n,(SELECT count(*) FROM security_audit_events WHERE event_type='household_access.person_grant_changed' AND metadata->>'outcome'='rejected') AS rejected",
        ))
        .await
        .unwrap()
        .unwrap();
    let n: i64 = row.try_get("", "n").unwrap();
    let rejected: i64 = row.try_get("", "rejected").unwrap();
    let statuses = (
        invalid.status().as_u16(),
        foreign.status().as_u16(),
        owned.status().as_u16(),
        denied.status().as_u16(),
    );
    app.close().await;
    assert_eq!(statuses, (422, 422, 422, 403));
    assert_eq!(n, 1);
    assert_eq!(rejected, 3);
}

#[tokio::test]
async fn household_settings_and_grant_audit_failure_roll_back_effects_and_keys() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE administration_audit_reached; GRANT USAGE,SELECT ON SEQUENCE administration_audit_reached TO med_tracker_app; CREATE FUNCTION reject_administration_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type IN ('api/admin/household_settings/updated','household_access.person_grant_changed') THEN PERFORM nextval('administration_audit_reached'); RAISE EXCEPTION 'Synthetic administration audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_administration_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_administration_audit()").await.unwrap();
    let settings = app
        .client
        .patch(format!(
            "{}/api/v1/households/72001/admin/settings",
            app.origin
        ))
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-rollback-settings")
        .json(&json!({"household":{"name":"Must roll back settings"}}))
        .send()
        .await
        .unwrap();
    let grant=app.client.post(format!("{}/api/v1/households/72001/admin/person_access_grants",app.origin)).bearer_auth(&token).header("idempotency-key","synthetic-rollback-grant").json(&json!({"person_access_grant":{"household_membership_id":74001,"person_id":73002,"access_level":"manage","relationship_type":"parent"}})).send().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM administration_audit_reached) AS reached,(SELECT count(*) FROM households WHERE name='Must roll back settings') AS settings,(SELECT count(*) FROM person_access_grants WHERE person_id=73002) AS grants,(SELECT count(*) FROM api_idempotency_keys WHERE key LIKE 'synthetic-rollback-%') AS keys,(SELECT permissions_version FROM household_memberships WHERE id=74001) AS version")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let counts: Vec<i64> = ["settings", "grants", "keys"]
        .iter()
        .map(|name| row.try_get("", name).unwrap())
        .collect();
    let version: i32 = row.try_get("", "version").unwrap();
    let statuses = (settings.status().as_u16(), grant.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (500, 500));
    assert!(reached);
    assert_eq!(counts, vec![0, 0, 0]);
    assert_eq!(version, 1);
}

#[tokio::test]
async fn household_settings_concurrent_key_retries_apply_once_and_conflict_on_changed_body() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/admin/settings", app.origin);
    let body = json!({"household":{"name":"Synthetic concurrent household"}});
    let first = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-settings-concurrent")
        .json(&body)
        .send();
    let second = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .header("idempotency-key", "synthetic-settings-concurrent")
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
        .header("idempotency-key", "synthetic-settings-concurrent")
        .json(&json!({"household":{"name":"Must not replace"}}))
        .send()
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM versions WHERE item_type='Household' AND event='update'",
        ))
        .await
        .unwrap()
        .unwrap();
    let n: i64 = row.try_get("", "n").unwrap();
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
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71002,'synthetic-rails-admin@example.test',2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71002,72001,73002,'member','active',now(),now(),now()); SELECT setval(pg_get_serial_sequence('person_access_grants','id'),78001,true)").await.unwrap();
    let cases = [
        (
            "PATCH",
            "/api/v1/households/72001/admin/settings",
            r#"{"household":{"name":"Synthetic Rails household"}}"#,
            "30f120d56073d9689845cc8c00429da677d7e7b8c26b85c8121f86abaa2dc49a",
            200,
        ),
        (
            "PATCH",
            "/api/v1/households/72001/admin/memberships/74002",
            r#"{"household_membership":{"role":"administrator"}}"#,
            "b62e11f1f155f08bc2e73c47ff52e7c31413c79bbd96ba18aa69783609d58966",
            200,
        ),
        (
            "POST",
            "/api/v1/households/72001/admin/person_access_grants",
            r#"{"person_access_grant":{"household_membership_id":74002,"person_id":73002,"access_level":"manage","relationship_type":"parent"}}"#,
            "7428980add9962a5c2bef49f2fddfc6c5bbed72a378781a7126d7f86692a2f3d",
            201,
        ),
        (
            "DELETE",
            "/api/v1/households/72001/admin/person_access_grants/78002",
            "",
            "a6085e6fa3f0d1de63aacf15d93826c60dc956657d1f0d0cb185745447b1d526",
            204,
        ),
        (
            "DELETE",
            "/api/v1/households/72001/admin/memberships/74002",
            "",
            "dd8644e4705108ded366a64372d7d8c18f1a7c86999677ce53b3be6f560e3788",
            204,
        ),
    ];
    let mut statuses = Vec::new();
    let mut markers = Vec::new();
    let mut equal = Vec::new();
    let mut empty_delete_bodies = Vec::new();
    for (method, path, body, digest, status) in cases {
        let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap();
        let original = app
            .client
            .request(method.clone(), format!("{}{path}", app.origin))
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        statuses.push(original.status().as_u16());
        let original: Value = if status == 204 {
            json!({})
        } else {
            original.json().await.unwrap()
        };
        let key = format!("synthetic-rails-admin-{status}-{path}");
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_idempotency_keys(household_id,account_id,key,request_method,request_path,request_digest,response_status,response_body,response_headers,expires_at,created_at,updated_at) VALUES(72001,71001,$1,$2,$3,$4,$5,$6,'{}',now()+interval '24 hours',now(),now())",[key.clone().into(),method.as_str().into(),path.into(),digest.into(),status.into(),original.clone().into()])).await.unwrap();
        let replay = app
            .client
            .request(method, format!("{}{path}", app.origin))
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .header("idempotency-key", key)
            .body(body)
            .send()
            .await
            .unwrap();
        statuses.push(replay.status().as_u16());
        markers.push(replay.headers().get("idempotency-replayed").is_some());
        let replay: Value = if status == 204 {
            empty_delete_bodies.push(replay.bytes().await.unwrap().is_empty());
            json!({})
        } else {
            replay.json().await.unwrap()
        };
        equal.push(replay == original);
    }
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM versions WHERE item_type='Household' AND event='update') AS settings,(SELECT permissions_version FROM household_memberships WHERE id=74002) AS version,(SELECT count(*) FROM person_access_grants WHERE household_membership_id=74002 AND person_id=73002) AS grants")).await.unwrap().unwrap();
    let settings: i64 = row.try_get("", "settings").unwrap();
    let version: i32 = row.try_get("", "version").unwrap();
    let grants: i64 = row.try_get("", "grants").unwrap();
    app.close().await;
    assert_eq!(
        statuses,
        vec![200, 200, 200, 200, 201, 201, 204, 204, 204, 204]
    );
    assert_eq!(markers, vec![true; 5]);
    assert_eq!(empty_delete_bodies, vec![true; 2]);
    assert_eq!(equal, vec![true; 5]);
    assert_eq!((settings, version, grants), (1, 5, 1));
}

#[tokio::test]
async fn household_administration_strong_parameters_filter_protected_fields_and_reject_empty() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71002,'synthetic-filter-admin@example.test',2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71002,72001,73002,'member','active',now(),now(),now())").await.unwrap();
    let settings = format!("{}/api/v1/households/72001/admin/settings", app.origin);
    let memberships = format!(
        "{}/api/v1/households/72001/admin/memberships/74002",
        app.origin
    );
    let grants = format!(
        "{}/api/v1/households/72001/admin/person_access_grants",
        app.origin
    );
    let changed=app.client.patch(&settings).bearer_auth(&token).header("idempotency-key","synthetic-admin-filter").json(&json!({"household":{"name":"Synthetic filtered household","status":"archived"},"ignored":1})).send().await.unwrap();
    let member=app.client.patch(&memberships).bearer_auth(&token).json(&json!({"household_membership":{"role":"administrator","permissions_version":900},"ignored":1})).send().await.unwrap();
    let grant=app.client.post(&grants).bearer_auth(&token).json(&json!({"person_access_grant":{"household_membership_id":74002,"person_id":73002,"access_level":"manage","relationship_type":"parent","revoked_at":"2000-01-01"},"ignored":1})).send().await.unwrap();
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
    let conflict=app.client.patch(&settings).bearer_auth(&token).header("idempotency-key","synthetic-admin-filter").json(&json!({"household":{"name":"Synthetic filtered household","status":"archived"},"ignored":2})).send().await.unwrap();
    let statuses = vec![
        changed.status().as_u16(),
        member.status().as_u16(),
        grant.status().as_u16(),
        empty_settings.status().as_u16(),
        empty_member.status().as_u16(),
        empty_grant.status().as_u16(),
        conflict.status().as_u16(),
    ];
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT status FROM households WHERE id=72001) AS status,(SELECT permissions_version FROM household_memberships WHERE id=74002) AS version,(SELECT count(*) FROM person_access_grants WHERE household_membership_id=74002 AND person_id=73002 AND revoked_at IS NULL) AS grants")).await.unwrap().unwrap();
    let status: String = row.try_get("", "status").unwrap();
    let version: i32 = row.try_get("", "version").unwrap();
    let n: i64 = row.try_get("", "grants").unwrap();
    app.close().await;
    assert_eq!(statuses, vec![200, 200, 201, 400, 400, 400, 409]);
    assert_eq!((status, version, n), ("active".into(), 3, 1));
}

#[tokio::test]
async fn household_admin_settings_get_matches_documented_contract() {
    use super::contract::{assert_value, contract, resolve};
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/admin/settings", app.origin);
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
    assert_eq!(body["data"]["id"], 72001);
    assert_eq!(body["data"]["name"], "Synthetic household");
    assert_eq!(body["data"]["slug"], "persistence-fixture");
    assert_eq!(body["data"]["timezone"], "Europe/London");
    assert_eq!(body["data"]["subscription_plan"], "free");

    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
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

    app.fixture
        .admin
        .execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Foreign synthetic household','api-settings-foreign','UTC',now(),now())")
        .await
        .unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/92001/admin/settings",
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
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/admin/memberships", app.origin);
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
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["id"], 74001);
    assert_eq!(members[0]["account_id"], 71001);
    assert_eq!(members[0]["email"], "persistence@example.test");
    assert_eq!(members[0]["person_id"], 73001);
    assert_eq!(members[0]["person_name"], "Synthetic adult");
    assert_eq!(members[0]["role"], "administrator");
    assert_eq!(members[0]["user_id"], 77001);

    app.fixture
        .admin
        .execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Foreign synthetic household','api-memberships-foreign','UTC',now(),now())")
        .await
        .unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/92001/admin/memberships",
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

    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
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
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/admin/person_access_grants",
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
    assert_eq!(grants[0]["id"], 78001);
    assert_eq!(grants[0]["person_id"], 73001);
    assert_eq!(grants[0]["access_level"], "manage");
    assert_eq!(grants[0]["relationship_type"], "self");

    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
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
            "{}/api/v1/households/72002/admin/person_access_grants",
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
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(86001,72001,73001,73002,'parent',true,now(),now()); INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78002,72001,74001,73002,'manage','family_member',now(),now())").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-carer-removal-manual-grant".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let result = delegation::deactivate(&tenant, 86001, None).await;
    let success = result.is_ok();
    if success {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT active FROM carer_relationships WHERE id=86001) AS active,(SELECT revoked_at IS NULL FROM person_access_grants WHERE id=78002) AS manual,(SELECT has_capacity FROM people WHERE id=73002) AS capacity")).await.unwrap().unwrap();
    let active: bool = row.try_get("", "active").unwrap();
    let manual: bool = row.try_get("", "manual").unwrap();
    let capacity: bool = row.try_get("", "capacity").unwrap();
    app.close().await;
    assert!(success);
    assert!(!active);
    assert!(manual);
    assert!(!capacity);
}
