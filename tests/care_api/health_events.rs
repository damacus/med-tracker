use super::*;

#[tokio::test]
async fn health_events_documented_routes_create_read_update_and_revoke() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/health_events", app.origin);
    let response = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event": {
        "person_id":"73001", "event_kind":"illness", "title":"Synthetic cold", "started_on":"2026-10-01"
    }})).send().await.unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(-1);
    let read = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let etag = read
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("missing")
        .to_owned();
    let update = app
        .client
        .patch(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"health_event":{"title":"Synthetic recovered cold","ended_on":"2026-10-03"}}))
        .send()
        .await
        .unwrap();
    let update_status = update.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let denied = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    app.close().await;
    assert_eq!(
        (status, read_status, update_status, denied_status),
        (201, 200, 200, 404)
    );
    assert_eq!(body["data"]["ended_on"], Value::Null);
}

#[tokio::test]
async fn health_events_invalid_dates_hidden_links_and_audit_failure_are_atomic() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/health_events", app.origin);
    let base = json!({"health_event":{"person_id":"73001","event_kind":"suspected_side_effect","title":"Synthetic reaction","started_on":"2026-10-03","medication_ids":["80001"]}});
    for invalid in [
        json!({"ended_on":"2026-10-01"}),
        json!({"title":" "}),
        json!({"event_kind":"unknown"}),
        json!({"medication_ids":["99999"]}),
        json!({"medication_ids":["80001","80001"]}),
    ] {
        let mut body = base.clone();
        for (key, value) in invalid.as_object().unwrap() {
            body["health_event"][key] = value.clone();
        }
        let response = app
            .client
            .post(&endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!([404, 422].contains(&response.status().as_u16()));
    }
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_health_event_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='HealthEvent' THEN RAISE EXCEPTION 'Synthetic health event audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_health_event_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_health_event_audit()").await.unwrap();
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&base)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    app.fixture.admin.execute_unprepared("DROP TRIGGER reject_health_event_audit ON versions; DROP FUNCTION reject_health_event_audit(); CREATE FUNCTION reject_health_event_link() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Synthetic health event link rejection'; END $$; CREATE TRIGGER reject_health_event_link BEFORE INSERT ON health_event_medications FOR EACH ROW EXECUTE FUNCTION reject_health_event_link()").await.unwrap();
    let link_failure = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&base)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM health_events) AS events,(SELECT count(*) FROM health_event_medications) AS links,(SELECT count(*) FROM api_change_events WHERE record_type='HealthEvent') AS changes,(SELECT count(*) FROM versions WHERE item_type='HealthEvent') AS audits")).await.unwrap().unwrap();
    let counts: Vec<i64> = ["events", "links", "changes", "audits"]
        .iter()
        .map(|key| row.try_get("", key).unwrap())
        .collect();
    app.close().await;
    assert_eq!((status, link_failure), (500, 500));
    assert_eq!(counts, vec![0, 0, 0, 0]);
}

#[tokio::test]
async fn health_events_record_grant_creates_but_cannot_manage_or_public_delete() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("UPDATE person_access_grants SET access_level='record' WHERE id=78001; UPDATE household_memberships SET role='member' WHERE id=74001").await.unwrap();
    let endpoint = format!("{}/api/v1/households/72001/health_events", app.origin);
    let created = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event":{"person_id":"73001","event_kind":"illness","title":"Synthetic recorded illness","started_on":"2026-10-01"}})).send().await.unwrap();
    let status = created.status().as_u16();
    let body: Value = created.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(-1);
    let update = app
        .client
        .patch(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .json(&json!({"health_event":{"title":"Forbidden"}}))
        .send()
        .await
        .unwrap();
    let update_status = update.status().as_u16();
    let delete = app
        .client
        .delete(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let delete_status = delete.status().as_u16();
    app.close().await;
    assert_eq!((status, update_status, delete_status), (201, 403, 405));
}

#[tokio::test]
async fn health_events_collection_and_request_boundaries_follow_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/health_events", app.origin);
    let unauthenticated = app
        .client
        .get(&endpoint)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let collection = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let collection_status = collection.status().as_u16();
    let body: Value = collection.json().await.unwrap_or(Value::Null);
    let malformed = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"other":{}}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    let invalid_page = app
        .client
        .get(format!("{endpoint}?page=0"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Other synthetic household','health-event-foreign-household','UTC',now(),now())").await.unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/health_events",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(
        (
            unauthenticated,
            collection_status,
            malformed,
            invalid_page,
            foreign
        ),
        (401, 200, 400, 422, 403)
    );
    assert_eq!(body["data"], json!([]));
    assert_eq!(body["meta"], json!({"page":1,"per_page":1,"total_count":0}));
}

#[tokio::test]
async fn health_events_put_and_stale_preconditions_preserve_links_and_audits() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/health_events", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"health_event":{
            "person_id":"73001","event_kind":"suspected_side_effect","title":"Synthetic reaction",
            "started_on":"2026-10-01","medication_ids":["80001"]
        }}))
        .send()
        .await
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let original_etag = created
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let original: Value = created.json().await.unwrap();
    let id = original["data"]["id"].as_i64().unwrap();
    let resource = format!("{endpoint}/{id}");
    let missing = app
        .client
        .put(&resource)
        .bearer_auth(&token)
        .json(&json!({"health_event":{"title":"Missing precondition"}}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let updated = app.client.put(&resource).bearer_auth(&token).header("if-match",&original_etag)
        .json(&json!({"health_event":{"title":"Synthetic updated reaction","notes":"Retained draft"}})).send().await.unwrap();
    let updated_status = updated.status().as_u16();
    let current_etag = updated
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let stale = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .header("if-match", &original_etag)
        .json(&json!({"health_event":{"title":"Stale title","medication_ids":[]}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT h.title,(SELECT count(*) FROM health_event_medications WHERE health_event_id=h.id) AS links,(SELECT count(*) FROM versions WHERE item_type='HealthEvent' AND item_id=h.id) AS audits FROM health_events h WHERE h.id=$1",
        [id.into()])).await.unwrap().unwrap();
    let title: String = row.try_get("", "title").unwrap();
    let links: i64 = row.try_get("", "links").unwrap();
    let audits: i64 = row.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!(
        (missing_status, updated_status, stale_status),
        (409, 200, 409)
    );
    assert_ne!(original_etag, current_etag);
    assert_eq!(
        (title.as_str(), links, audits),
        ("Synthetic updated reaction", 1, 2)
    );
}

#[tokio::test]
async fn health_events_visibility_is_person_scoped_and_view_access_cannot_write() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO health_events(id,household_id,person_id,event_kind,title,started_on,created_at,updated_at) VALUES(87001,72001,73002,0,'Hidden synthetic event','2026-10-01',now(),now()); UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let endpoint = format!("{}/api/v1/households/72001/health_events", app.origin);
    let hidden = app
        .client
        .get(format!("{endpoint}/87001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let hidden_status = hidden.status().as_u16();
    let collection: Value = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let create = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"health_event":{
        "person_id":"73001","event_kind":"illness","title":"Forbidden view-only event","started_on":"2026-10-01"
    }})).send().await.unwrap();
    let create_status = create.status().as_u16();
    let rows = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM health_events) AS events,(SELECT count(*) FROM versions WHERE item_type='HealthEvent') AS audits")).await.unwrap().unwrap();
    let events: i64 = rows.try_get("", "events").unwrap();
    let audits: i64 = rows.try_get("", "audits").unwrap();
    app.close().await;
    assert_eq!((hidden_status, create_status), (404, 403));
    assert_eq!(collection["data"], json!([]));
    assert_eq!(collection["meta"]["total_count"], 0);
    assert_eq!((events, audits), (1, 0));
}

#[tokio::test]
async fn health_events_browser_role_matrix_preserves_view_record_manage_boundaries() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            health_events::browser,
        },
    };
    for (label, role, relationship, level, person_id, can_create, can_manage) in [
        (
            "administrator",
            "administrator",
            "family_member",
            "manage",
            73002,
            true,
            true,
        ),
        (
            "clinician",
            "member",
            "professional",
            "record",
            73002,
            true,
            false,
        ),
        ("self", "member", "self", "manage", 73001, true, true),
        (
            "carer",
            "member",
            "professional",
            "record",
            73002,
            true,
            false,
        ),
        ("parent", "member", "parent", "manage", 73002, true, true),
        (
            "unauthorised",
            "member",
            "parent",
            "view",
            73002,
            false,
            false,
        ),
    ] {
        let app = Application::new().await;
        app.fixture
            .admin
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE household_memberships SET role=$1 WHERE id=74001",
                [role.into()],
            ))
            .await
            .unwrap();
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE person_access_grants SET person_id=$1,relationship_type=$2,access_level=$3 WHERE id=78001",
            [person_id.into(),relationship.into(),level.into()]
        )).await.unwrap();
        let scope = HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: format!("synthetic-health-{label}"),
        };
        let provenance = CredentialProvenance {
            method: CredentialMethod::BrowserSession,
            reference: "synthetic-session".into(),
        };
        let mut draft = std::collections::HashMap::from([
            ("event_kind".into(), "illness".into()),
            ("title".into(), format!("Synthetic {label} event")),
            ("started_on".into(), "2026-10-01".into()),
            ("ongoing".into(), "1".into()),
        ]);
        let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
        let created = browser::save(&tenant, person_id, None, &draft, &provenance).await;
        assert_eq!(created.is_ok(), can_create, "{label}: create");
        if !can_create {
            tenant.rollback().await.unwrap();
            app.close().await;
            continue;
        }
        tenant.commit().await.unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
        let events = browser::collection(&tenant, person_id).await.unwrap();
        assert_eq!(events.len(), 1, "{label}: view");
        let id = events[0]["id"].as_i64().unwrap().to_string();
        draft.insert("etag".into(), events[0]["etag"].as_str().unwrap().into());
        draft.insert("title".into(), format!("Synthetic {label} updated"));
        assert_eq!(
            browser::save(&tenant, person_id, Some(&id), &draft, &provenance)
                .await
                .is_ok(),
            can_manage,
            "{label}: edit"
        );
        tenant.rollback().await.unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
        assert_eq!(
            browser::delete(
                &tenant,
                person_id,
                &id,
                events[0]["etag"].as_str().unwrap(),
                &provenance
            )
            .await
            .is_ok(),
            can_manage,
            "{label}: delete"
        );
        tenant.rollback().await.unwrap();
        app.fixture
            .admin
            .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
            .await
            .unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
        assert!(
            browser::collection(&tenant, person_id).await.is_err(),
            "{label}: revoked view"
        );
        assert!(
            browser::save(&tenant, person_id, None, &draft, &provenance)
                .await
                .is_err(),
            "{label}: revoked create"
        );
        tenant.rollback().await.unwrap();
        app.close().await;
    }
}

#[tokio::test]
async fn health_events_browser_preserves_unselectable_links_and_audits_link_changes() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            health_events::browser,
        },
    };
    let app = Application::new().await;
    let token = app.token().await;
    let response = app.client.post(format!("{}/api/v1/households/72001/health_events",app.origin))
        .bearer_auth(&token)
        .json(&json!({"health_event":{"person_id":"73001","event_kind":"suspected_side_effect","title":"Synthetic linked reaction","started_on":"2026-10-01","medication_ids":["80001"]}}))
        .send().await.unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    app.fixture.admin.execute_unprepared("DELETE FROM person_medications WHERE id=81001; UPDATE medications SET name='Renamed synthetic tablets' WHERE id=80001").await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-preserve-health-links".into(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    assert!(
        browser::medication_options(&tenant, 73001)
            .await
            .unwrap()
            .is_empty()
    );
    let (_, _, etag) = browser::event(&tenant, 73001, &id.to_string())
        .await
        .unwrap();
    let mut draft = std::collections::HashMap::from([
        ("event_kind".into(), "suspected_side_effect".into()),
        ("title".into(), "Synthetic edited reaction".into()),
        ("started_on".into(), "2026-10-01".into()),
        ("ongoing".into(), "1".into()),
        ("etag".into(), etag),
    ]);
    browser::save(&tenant, 73001, Some(&id.to_string()), &draft, &provenance)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let (_, links, _) = browser::event(&tenant, 73001, &id.to_string())
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].medication_name, "Synthetic tablets");
    tenant.rollback().await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81001,72001,73001,80001,2,'tablet',0,now(),now())").await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let (_, _, etag) = browser::event(&tenant, 73001, &id.to_string())
        .await
        .unwrap();
    draft.insert("etag".into(), etag);
    browser::save(&tenant, 73001, Some(&id.to_string()), &draft, &provenance)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT object_changes FROM versions WHERE item_type='HealthEvent' AND item_id=$1 AND event='update' ORDER BY id DESC LIMIT 1",[id.into()])).await.unwrap().unwrap();
    let changes: String = row.try_get("", "object_changes").unwrap();
    let changes: Value = serde_json::from_str(&changes).unwrap();
    assert_eq!(
        changes["medication_links"][0][0]["medication_name"],
        "Synthetic tablets"
    );
    assert_eq!(changes["medication_links"][1], json!([]));
    app.close().await;
}
