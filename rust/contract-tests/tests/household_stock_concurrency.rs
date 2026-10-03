use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::env;

#[path = "household_completion_medication/authentication.rs"]
mod authentication;

fn input(html: &str, name: &str) -> String {
    Html::parse_document(html)
        .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next()
        .and_then(|field| field.value().attr("value"))
        .expect("native stock input")
        .to_owned()
}

fn login(target: &Target, fixture: &Fixture, client: &str) {
    let token = input(
        &target.get_html("/login").text().unwrap(),
        "authenticity_token",
    );
    let response = target.post_html_form_from_client(
        "/login",
        client,
        &[
            ("email".into(), fixture.primary_email.clone()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
}

fn setup(name: &str, client: &str) -> (Fixture, authentication::CurrentBearer, Target, i64) {
    let (fixture, guard) = authentication::fixture_with_current_bearer(client);
    let target = Target::from_env();
    login(&target, &fixture, client);
    let response = target.post_json_authorized(
        &format!("/api/v1/households/{}/medications", fixture.household_id),
        &fixture.access_token,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id,
            "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let id = response.json::<Value>().unwrap()["data"]["id"]
        .as_i64()
        .unwrap();
    (fixture, guard, target, id)
}

fn api(fixture: &Fixture, id: i64) -> String {
    format!(
        "/api/v1/households/{}/medications/{id}",
        fixture.household_id
    )
}

fn web(fixture: &Fixture, id: i64) -> String {
    format!(
        "/households/{}/medications/{id}/stock",
        fixture.household_slug
    )
}

fn read(target: &Target, fixture: &Fixture, id: i64) -> Value {
    let response = target.get(&api(fixture, id), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn counts(id: i64) -> (i64, i64) {
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    (
        db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1", &[&id]).unwrap().get(0),
        db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1", &[&id]).unwrap().get(0),
    )
}

fn form(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    assert_eq!(response.status().as_u16(), 200, "stock action form exists");
    response.text().unwrap()
}

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
struct ReadGate {
    db: Client,
    name: String,
    key: i32,
}

impl ReadGate {
    fn new(fixture: &Fixture, medication_id: i64) -> Self {
        let mut db = Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable database URL"),
            NoTls,
        )
        .unwrap();
        let session: String = db.query_one("SELECT audit_context->>'session_reference' FROM security_audit_events WHERE actor_account_id = $1 AND household_id = $2 AND metadata->>'controller' = 'api/v1/dosage_options' AND metadata->>'action' = 'index' AND audit_context->>'authentication_method' = 'browser_session' ORDER BY id DESC LIMIT 1", &[&fixture.account_id, &fixture.household_id]).expect("edit GET must establish unique browser audit session").get(0);
        let key = i32::try_from(medication_id).unwrap();
        let name = format!("completion_stock_gate_{key}");
        db.query_one("SELECT pg_advisory_lock(180034, $1)", &[&key])
            .unwrap();
        let mut gate = Self { db, name, key };
        let session = session.replace('\'', "''");
        gate.db.batch_execute(&format!("CREATE FUNCTION {}() RETURNS trigger LANGUAGE plpgsql AS $gate$ BEGIN IF NEW.actor_account_id = {} AND NEW.household_id = {} AND NEW.metadata->>'controller' = 'api/v1/dosage_options' AND NEW.metadata->>'action' = 'index' AND NEW.metadata->>'http_method' = 'GET' AND NEW.metadata->>'status' = '200' AND NEW.audit_context->>'authentication_method' = 'browser_session' AND NEW.audit_context->>'session_reference' = '{}' THEN PERFORM pg_advisory_xact_lock(180034, {}); END IF; RETURN NEW; END $gate$; CREATE TRIGGER {} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION {}();", gate.name, fixture.account_id, fixture.household_id, session, key, gate.name, gate.name)).expect("install disposable browser-read gate");
        gate
    }

    fn wait_for_read(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let blocked: bool = self.db.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype = 'advisory' AND classid = 180034::oid AND objid = $1::int::oid AND objsubid = 2 AND NOT granted)", &[&self.key]).unwrap().get(0);
            if blocked {
                return true;
            }
            thread::yield_now();
        }
        false
    }

    fn release(&mut self) {
        let released: bool = self
            .db
            .query_one("SELECT pg_advisory_unlock(180034, $1)", &[&self.key])
            .unwrap()
            .get(0);
        assert!(released, "test must own the gate lock");
    }
}

impl Drop for ReadGate {
    fn drop(&mut self) {
        let _ = self
            .db
            .query_one("SELECT pg_advisory_unlock(180034, $1)", &[&self.key]);
        if let Err(error) = self.db.batch_execute(&format!(
            "DROP TRIGGER IF EXISTS {} ON security_audit_events; DROP FUNCTION IF EXISTS {}();",
            self.name, self.name
        )) {
            eprintln!("failed to remove disposable medication read gate: {error}");
        }
    }
}

fn interleaved_option(synthetic_version_restore: bool) {
    let (fixture, _guard, target, id) = setup(
        if synthetic_version_restore {
            "Synthetic restored-version null-option stock race"
        } else {
            "Tracked first-option stock race"
        },
        if synthetic_version_restore {
            "198.18.34.2"
        } else {
            "198.18.34.1"
        },
    );
    let target = Arc::new(target);
    let parent_api = api(&fixture, id);
    if synthetic_version_restore {
        assert_eq!(
            target
                .patch_json(
                    &parent_api,
                    &fixture.access_token,
                    &json!({"medication": {"dose_amount": null}})
                )
                .status()
                .as_u16(),
            200
        );
    }
    let html = form(&target, &format!("{}/adjust", web(&fixture, id)));
    let original = target.get(&parent_api, Some(&fixture.access_token));
    let original_etag = original.headers()["etag"].to_str().unwrap().to_owned();
    let original_parent = original.json::<Value>().unwrap()["data"].clone();
    let fields = vec![
        (
            "authenticity_token".into(),
            input(&html, "authenticity_token"),
        ),
        ("etag".into(), original_etag.clone()),
        ("new_quantity".into(), "90.25".into()),
        ("reason".into(), "Original scalar draft".into()),
    ];
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let original_timestamp: String = db
        .query_one(
            "SELECT updated_at::text FROM medications WHERE id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(db.query_one("SELECT count(*) FROM dosages WHERE household_id = $1 AND medication_id = $2", &[&fixture.household_id, &id]).unwrap().get::<_, i64>(0), 0, "the gated scalar read precedes the first dosage option");
    let mut gate = ReadGate::new(&fixture, id);
    let submitted = fields.clone();
    let submitting = Arc::clone(&target);
    let endpoint = format!("{}/adjust", web(&fixture, id));
    let worker = thread::spawn(move || submitting.post_browser_form(&endpoint, &submitted));
    if !gate.wait_for_read() {
        gate.release();
        let response = worker.join().unwrap();
        panic!(
            "stock dosage-index read did not reach gate: {}",
            response.status()
        );
    }
    let owner = Target::from_env();
    let response = owner.post_json_authorized(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), &fixture.access_token,
        &json!({"dosage_option": {"medication_id": id.to_string(), "amount": "1.25", "unit": "tablet", "frequency": "daily",
            "current_supply": if synthetic_version_restore { Value::Null } else { json!("12.25") }, "reorder_threshold": null,
            "default_max_daily_doses": 4, "default_min_hours_between_doses": "0", "default_dose_cycle": "daily"}}));
    assert_eq!(response.status().as_u16(), 201);
    let option = response.json::<Value>().unwrap()["data"].clone();
    assert_eq!(db.query_one("SELECT count(*) FROM dosages WHERE household_id = $1 AND medication_id = $2", &[&fixture.household_id, &id]).unwrap().get::<_, i64>(0), 1, "the real concurrent create inserts the first dosage option");
    if synthetic_version_restore {
        assert_eq!(
            db.execute(
                "UPDATE medications SET updated_at = $1::text::timestamp WHERE id = $2",
                &[&original_timestamp, &id]
            )
            .unwrap(),
            1
        );
    }
    let parent_response = owner.get(&parent_api, Some(&fixture.access_token));
    let after_etag = parent_response.headers()["etag"]
        .to_str()
        .unwrap()
        .to_owned();
    let parent_before = parent_response.json::<Value>().unwrap()["data"].clone();
    if synthetic_version_restore {
        assert_eq!(
            after_etag, original_etag,
            "fixture-only restore isolates option-mode guard from version guard"
        );
        assert_eq!(parent_before, original_parent);
    } else {
        assert_ne!(after_etag, original_etag);
    }
    let evidence = counts(id);
    let option_id = option["id"].as_i64().unwrap();
    let option_api = format!(
        "/api/v1/households/{}/dosage_options/{option_id}",
        fixture.household_id
    );
    let option_evidence: (i64, i64) = (
        db.query_one("SELECT count(*) FROM versions WHERE item_type = 'MedicationDosageOption' AND item_id = $1", &[&option_id]).unwrap().get(0),
        db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'MedicationDosageOption' AND record_id = $1", &[&option_id]).unwrap().get(0),
    );
    gate.release();
    let response = worker.join().unwrap();
    assert_eq!(
        response.status().as_u16(),
        409,
        "interleaving must reject before quantity write"
    );
    let rejected = response.text().unwrap();
    assert_eq!(input(&rejected, "new_quantity"), "90.25");
    assert_eq!(input(&rejected, "reason"), "Original scalar draft");
    assert_eq!(input(&rejected, "etag"), original_etag);
    let parent_after = read(&owner, &fixture, id);
    assert_eq!(parent_after["current_supply"], parent_before["current_supply"], "rejected scalar draft must not change stock established by the option create");
    assert_eq!(parent_after, parent_before);
    assert_eq!(counts(id), evidence);
    let stored = owner.get(&option_api, Some(&fixture.access_token));
    assert_eq!(stored.status().as_u16(), 200);
    let option_after = stored.json::<Value>().unwrap()["data"].clone();
    assert_eq!(option_after["current_supply"], option["current_supply"], "rejected scalar draft must not change the new option stock");
    assert_eq!(option_after, option);
    assert_eq!(db.query_one("SELECT count(*) FROM versions WHERE item_type = 'MedicationDosageOption' AND item_id = $1", &[&option_id]).unwrap().get::<_, i64>(0), option_evidence.0);
    assert_eq!(db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'MedicationDosageOption' AND record_id = $1", &[&option_id]).unwrap().get::<_, i64>(0), option_evidence.1);
}

#[test]
fn tracked_option_created_after_browser_scalar_check_cannot_be_overwritten() {
    interleaved_option(false);
}

#[test]
fn synthetic_parent_version_restore_null_option_still_blocks_scalar_adjustment() {
    interleaved_option(true);
}

#[test]
fn missing_blank_and_whitespace_stock_tokens_keep_invalid_draft_and_write_nothing() {
    let (fixture, _guard, target, id) = setup("Missing stock adjustment token", "198.18.34.3");
    let html = form(&target, &format!("{}/adjust", web(&fixture, id)));
    let before = read(&target, &fixture, id);
    let evidence = counts(id);
    for token in [None, Some(""), Some(" \t ")] {
        let mut fields = vec![
            (
                "authenticity_token".into(),
                input(&html, "authenticity_token"),
            ),
            ("new_quantity".into(), "-0.25".into()),
            ("reason".into(), "Original invalid draft".into()),
        ];
        if let Some(token) = token {
            fields.push(("etag".into(), token.into()));
        }
        let response = target.post_browser_form(&format!("{}/adjust", web(&fixture, id)), &fields);
        assert_eq!(response.status().as_u16(), 428);
        let rejected = response.text().unwrap();
        assert_eq!(input(&rejected, "new_quantity"), "-0.25");
        assert_eq!(input(&rejected, "reason"), "Original invalid draft");
        assert_eq!(input(&rejected, "etag"), token.unwrap_or(""));
        assert_eq!(read(&target, &fixture, id), before);
        assert_eq!(counts(id), evidence);
    }
}

#[test]
fn stale_original_stock_token_rejects_without_changing_public_parent_api_behaviour() {
    let (fixture, _guard, target, id) = setup("Stale stock adjustment token", "198.18.34.4");
    let html = form(&target, &format!("{}/adjust", web(&fixture, id)));
    let reply = target.get(&api(&fixture, id), Some(&fixture.access_token));
    let original = reply.headers()["etag"].to_str().unwrap().to_owned();
    assert_eq!(target.patch_json(&format!("{}/adjust_inventory", api(&fixture, id)), &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "21.75", "reason": "Actual concurrent count"}})).status().as_u16(), 200);
    let before = read(&target, &fixture, id);
    let evidence = counts(id);
    let response = target.post_browser_form(
        &format!("{}/adjust", web(&fixture, id)),
        &[
            (
                "authenticity_token".into(),
                input(&html, "authenticity_token"),
            ),
            ("etag".into(), original.clone()),
            ("new_quantity".into(), "-0.25".into()),
            ("reason".into(), "Stale invalid draft".into()),
        ],
    );
    assert_eq!(response.status().as_u16(), 409);
    let rejected = response.text().unwrap();
    assert_eq!(input(&rejected, "etag"), original);
    assert_eq!(input(&rejected, "new_quantity"), "-0.25");
    assert_eq!(read(&target, &fixture, id), before);
    assert_eq!(counts(id), evidence);
    let response = target.post_json_authorized(&format!("/api/v1/households/{}/dosage_options", fixture.household_id), &fixture.access_token,
        &json!({"dosage_option": {"medication_id": id.to_string(), "amount": "1.25", "unit": "tablet", "frequency": "daily", "current_supply": "12.25", "reorder_threshold": null,
            "default_max_daily_doses": 4, "default_min_hours_between_doses": "0", "default_dose_cycle": "daily"}}));
    assert_eq!(response.status().as_u16(), 201);
    let option = response.json::<Value>().unwrap()["data"].clone();
    let response = target.patch_json_if_match(&format!("{}/adjust_inventory", api(&fixture, id)), &fixture.access_token,
        &json!({"adjustment": {"new_quantity": "30.25", "reason": "Public legacy parent-only count"}}), &original);
    assert_eq!(response.status().as_u16(), 200, "public header is not a trusted browser guard");
    assert_eq!(read(&target, &fixture, id)["current_supply"], "30.25");
    let response = target.get(&format!("/api/v1/households/{}/dosage_options/{}", fixture.household_id, option["id"].as_i64().unwrap()), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.json::<Value>().unwrap()["data"], option, "legacy parent API does not redistribute option stock");
}

fn revoke_membership_while_waiting_for_household_lock(create: bool) {
    let (fixture, _guard, target, id) = setup("Revocation race", "127.0.0.85");
    let before = read(&target, &fixture, id);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let mut held = db.transaction().unwrap();
    held.query_one(
        "SELECT id FROM households WHERE id = $1 FOR UPDATE",
        &[&fixture.household_id],
    )
    .unwrap();
    let token = fixture.access_token.clone();
    let path = if create {
        format!("/api/v1/households/{}/medications", fixture.household_id)
    } else {
        api(&fixture, id)
    };
    let location = fixture.primary_location_id;
    let worker = std::thread::spawn(move || {
        let target = Target::from_env();
        if create {
            target.post_json_authorized(&path, &token, &json!({"medication": {"name": "Must not be created after revocation", "location_id": location, "dose_amount": "1", "dose_unit": "tablet", "reorder_threshold": "3"}})).status().as_u16()
        } else {
            target
                .patch_json(
                    &path,
                    &token,
                    &json!({"medication": {"name": "Must not be changed after revocation"}}),
                )
                .status()
                .as_u16()
        }
    });
    let mut observer =
        Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        let waiting: bool = observer.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock' AND query LIKE '%households%')", &[]).unwrap().get(0);
        if waiting {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "request must reach household lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    held.execute(
        "UPDATE household_memberships SET status = 'revoked' WHERE id = $1",
        &[&fixture.owner_membership_id],
    )
    .unwrap();
    held.commit().unwrap();
    let status = worker.join();
    db.execute(
        "UPDATE household_memberships SET status = 'active' WHERE id = $1",
        &[&fixture.owner_membership_id],
    )
    .unwrap();
    let status = status.unwrap();
    assert!(
        [401, 403, 404].contains(&status),
        "revoked writer returned {status}"
    );
    assert_eq!(read(&target, &fixture, id), before);
    assert_eq!(db.query_one("SELECT count(*) FROM medications WHERE household_id = $1 AND name = 'Must not be created after revocation'", &[&fixture.household_id]).unwrap().get::<_, i64>(0), 0);
}

#[test]
fn medication_update_rechecks_membership_after_waiting_for_household_lock() {
    revoke_membership_while_waiting_for_household_lock(false);
}

#[test]
fn medication_create_rechecks_membership_after_waiting_for_household_lock() {
    revoke_membership_while_waiting_for_household_lock(true);
}
