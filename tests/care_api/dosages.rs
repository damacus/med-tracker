use super::*;

fn dosage_body() -> Value {
    json!({"dosage_option":{"medication_id":"80001","amount":"2","unit":"tablet","frequency":"daily","default_max_daily_doses":3,"default_min_hours_between_doses":"4","default_dose_cycle":"daily","current_supply":"12","reorder_threshold":"3"}})
}

#[tokio::test]
async fn dosage_conformance_tracked_description_update_retains_parent_inventory_callback() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/dosage_options", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&dosage_body())
        .send()
        .await
        .unwrap();
    let body: Value = created.json().await.unwrap();
    let id = body["data"]["id"].as_i64().unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE medications SET updated_at='2020-01-01' WHERE id=80001")
        .await
        .unwrap();
    let response = app
        .client
        .patch(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .json(&json!({"dosage_option":{"description":"Synthetic description only"}}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS supply,reorder_threshold::text AS threshold,updated_at>'2020-01-01' AS refreshed,(SELECT count(*) FROM versions WHERE item_type='Medication' AND event='api_update') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND action='update') AS changes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let threshold: String = row.try_get("", "threshold").unwrap();
    let refreshed: bool = row.try_get("", "refreshed").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!(status, 200);
    assert_eq!((supply.as_str(), threshold.as_str()), ("12.00", "3.00"));
    assert!(refreshed);
    assert_eq!((versions, changes), (2, 2));
}

#[tokio::test]
async fn dosage_conformance_untracked_creation_retains_raw_parent_mode_sync_without_version() {
    let app = Application::new().await;
    let token = app.token().await;
    let mut body = dosage_body();
    body["dosage_option"]["current_supply"] = Value::Null;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/dosage_options",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT dose_amount IS NULL AS mode,current_supply::text AS supply,(SELECT count(*) FROM versions WHERE item_type='Medication') AS parent_versions,(SELECT count(*) FROM versions WHERE item_type='MedicationDosageOption' AND event='api_create') AS option_versions,(SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND action='update') AS changes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let mode: bool = row.try_get("", "mode").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let counts: Vec<i64> = ["parent_versions", "option_versions", "changes"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert_eq!(status, 201);
    assert!(mode);
    assert_eq!(supply, "10.00");
    assert_eq!(counts, vec![0, 1, 1]);
}

async fn seed_removable_options(app: &Application) {
    app.fixture.admin.execute_unprepared("INSERT INTO dosages(id,household_id,medication_id,amount,unit,frequency,current_supply,reorder_threshold,default_dose_cycle,default_max_daily_doses,default_min_hours_between_doses,created_at,updated_at) VALUES(82001,72001,80001,2,'tablet','daily',6,3,0,4,0,now(),now()),(82002,72001,80001,1,'tablet','daily',4,2,0,4,0,now(),now()); UPDATE medications SET dose_amount=NULL,current_supply=10,reorder_threshold=5,supply_at_last_restock=10 WHERE id=80001").await.unwrap();
}

async fn removal_context(
    app: &Application,
) -> (
    med_tracker::models::identity::resource::ValidatedPrincipal,
    med_tracker::models::access::TenantTransaction,
) {
    let token = app.token().await;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    let principal =
        med_tracker::models::identity::resource::authenticate(&app.fixture.runtime, &headers)
            .await
            .unwrap();
    let tenant = principal
        .begin_household(
            &app.fixture.runtime,
            72001,
            "synthetic-remove-dosage".into(),
        )
        .await
        .unwrap();
    (principal, tenant)
}

#[tokio::test]
async fn dosage_removal_recalculates_parent_and_records_atomic_history() {
    let app = Application::new().await;
    seed_removable_options(&app).await;
    let (principal, tenant) = removal_context(&app).await;
    let result =
        med_tracker::models::care::dosages::destroy(&tenant, "82001", Some(principal.provenance()))
            .await;
    if result.is_ok() {
        tenant.commit().await.unwrap();
    } else {
        tenant.rollback().await.unwrap();
    }
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS supply,reorder_threshold::text AS threshold,supply_at_last_restock::text AS baseline,(SELECT count(*) FROM dosages WHERE id=82001) AS removed,(SELECT count(*) FROM versions WHERE item_type='MedicationDosageOption' AND event='destroy') AS versions,(SELECT count(*) FROM api_tombstones WHERE record_type='MedicationDosageOption') AS tombstones FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let values: Vec<String> = ["supply", "threshold", "baseline"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    let counts: Vec<i64> = ["removed", "versions", "tombstones"]
        .iter()
        .map(|field| row.try_get("", field).unwrap())
        .collect();
    app.close().await;
    assert!(result.is_ok());
    assert_eq!(values, vec!["4.00", "2.00", "10.00"]);
    assert_eq!(counts, vec![0, 1, 1]);
}

#[tokio::test]
async fn dosage_removal_protects_inactive_schedule_and_assignment_references() {
    for reference in ["assignment", "schedule"] {
        let app = Application::new().await;
        seed_removable_options(&app).await;
        if reference == "assignment" {
            app.fixture.admin.execute_unprepared("UPDATE person_medications SET source_dosage_option_id=82001,active=false WHERE id=81001").await.unwrap();
        } else {
            app.fixture.admin.execute_unprepared("INSERT INTO schedules(id,household_id,person_id,medication_id,source_dosage_option_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83001,72001,73001,80001,82001,false,'2026-10-05','2026-10-10',2,'tablet',6,'{}',now(),now())").await.unwrap();
        }
        let (principal, tenant) = removal_context(&app).await;
        let result = med_tracker::models::care::dosages::destroy(
            &tenant,
            "82001",
            Some(principal.provenance()),
        )
        .await;
        tenant.rollback().await.unwrap();
        let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM dosages WHERE id=82001) AS options,(SELECT count(*) FROM versions WHERE item_type='MedicationDosageOption') AS versions")).await.unwrap().unwrap();
        let counts: Vec<i64> = ["options", "versions"]
            .iter()
            .map(|field| row.try_get("", field).unwrap())
            .collect();
        app.close().await;
        assert!(
            matches!(
                result,
                Err(med_tracker::models::errors::OperationError::Validation { .. })
            ),
            "{reference}"
        );
        assert_eq!(counts, vec![1, 0]);
    }
}

#[tokio::test]
async fn dosage_removal_audit_rejection_rolls_back_deleted_option_and_parent_stock() {
    let app = Application::new().await;
    seed_removable_options(&app).await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE dosage_removal_audit_reached; GRANT USAGE,SELECT ON SEQUENCE dosage_removal_audit_reached TO med_tracker_app; CREATE FUNCTION reject_dosage_removal_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationDosageOption' AND NEW.event='destroy' THEN IF NOT EXISTS(SELECT 1 FROM dosages WHERE id=82001) AND (SELECT current_supply FROM medications WHERE id=80001)=4 THEN PERFORM nextval('dosage_removal_audit_reached'); END IF; RAISE EXCEPTION 'Synthetic removal audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_dosage_removal_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_dosage_removal_audit()").await.unwrap();
    let (principal, tenant) = removal_context(&app).await;
    let result =
        med_tracker::models::care::dosages::destroy(&tenant, "82001", Some(principal.provenance()))
            .await;
    tenant.rollback().await.unwrap();
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM dosage_removal_audit_reached) AS reached,(SELECT count(*) FROM dosages WHERE id=82001) AS options,(SELECT current_supply::text FROM medications WHERE id=80001) AS supply,(SELECT count(*) FROM api_tombstones WHERE record_type='MedicationDosageOption') AS tombstones")).await.unwrap().unwrap();
    let reached: bool = row.try_get("", "reached").unwrap();
    let options: i64 = row.try_get("", "options").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let tombstones: i64 = row.try_get("", "tombstones").unwrap();
    app.close().await;
    assert!(result.is_err());
    assert!(reached);
    assert_eq!((options, supply.as_str(), tombstones), (1, "10.00", 0));
}

#[tokio::test]
async fn dosage_removal_requires_current_manager_and_scoped_parent() {
    let app = Application::new().await;
    seed_removable_options(&app).await;
    let (principal, tenant) = removal_context(&app).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied =
        med_tracker::models::care::dosages::destroy(&tenant, "82001", Some(principal.provenance()))
            .await;
    tenant.rollback().await.unwrap();
    app.close().await;
    assert!(matches!(
        denied,
        Err(med_tracker::models::errors::OperationError::Forbidden)
    ));
}

#[tokio::test]
async fn dosage_api_create_is_immediately_readable_and_inventory_audit_are_atomic() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/dosage_options", app.origin);
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&dosage_body())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(0);
    let read = app
        .client
        .get(format!("{endpoint}/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let read_status = read.status().as_u16();
    let collection = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let collection_status = collection.status().as_u16();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT dose_amount IS NULL AS options_mode,current_supply::text AS supply,reorder_threshold::text AS threshold,(SELECT count(*) FROM versions WHERE item_type='MedicationDosageOption') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='MedicationDosageOption') AS changes FROM medications WHERE id=80001")).await.unwrap().unwrap();
    let mode: bool = row.try_get("", "options_mode").unwrap();
    let supply: String = row.try_get("", "supply").unwrap();
    let threshold: String = row.try_get("", "threshold").unwrap();
    let versions: i64 = row.try_get("", "versions").unwrap();
    let changes: i64 = row.try_get("", "changes").unwrap();
    app.close().await;
    assert_eq!((status, read_status, collection_status), (201, 200, 200));
    assert!(!etag.is_empty());
    assert_eq!(body["data"]["amount"], "2.0");
    assert!(mode);
    assert_eq!((supply.as_str(), threshold.as_str()), ("12.00", "3.00"));
    assert_eq!((versions, changes), (1, 1));
}

#[tokio::test]
async fn dosage_api_partial_update_and_optional_stale_etag_preserve_existing_values() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/dosage_options", app.origin);
    let created = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&dosage_body())
        .send()
        .await
        .unwrap();
    let etag = created
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body: Value = created.json().await.unwrap_or(Value::Null);
    let id = body["data"]["id"].as_i64().unwrap_or(0);
    let resource = format!("{endpoint}/{id}");
    let updated = app
        .client
        .patch(&resource)
        .bearer_auth(&token)
        .json(&json!({"dosage_option":{"description":"Synthetic revised option"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let updated: Value = updated.json().await.unwrap_or(Value::Null);
    let stale = app
        .client
        .put(&resource)
        .bearer_auth(&token)
        .header("if-match", etag)
        .json(&json!({"dosage_option":{"amount":"4"}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    app.close().await;
    assert_eq!((updated_status, stale_status), (200, 409));
    assert_eq!(updated["data"]["amount"], "2.0");
    assert_eq!(updated["data"]["description"], "Synthetic revised option");
}

#[tokio::test]
async fn dosage_api_invalid_and_duplicate_defaults_have_no_partial_effect() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/dosage_options", app.origin);
    let mut invalid = dosage_body();
    invalid["dosage_option"]["amount"] = json!("0");
    let rejected = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&invalid)
        .send()
        .await
        .unwrap();
    let mut valid = dosage_body();
    valid["dosage_option"]["default_for_adults"] = json!(true);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&valid)
        .send()
        .await
        .unwrap();
    let duplicate = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&valid)
        .send()
        .await
        .unwrap();
    let statuses = (
        rejected.status().as_u16(),
        first.status().as_u16(),
        duplicate.status().as_u16(),
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS options FROM dosages WHERE medication_id=80001",
        ))
        .await
        .unwrap()
        .unwrap();
    let options: i64 = row.try_get("", "options").unwrap();
    app.close().await;
    assert_eq!(statuses, (422, 201, 422));
    assert_eq!(options, 1);
}

#[tokio::test]
async fn dosage_api_member_write_denial_precedes_validation_and_foreign_reads_are_hidden() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/dosage_options",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let foreign = app
        .client
        .get(format!(
            "{}/api/v1/households/72002/dosage_options",
            app.origin
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let statuses = (denied.status().as_u16(), foreign.status().as_u16());
    app.close().await;
    assert_eq!(statuses, (403, 403));
}
