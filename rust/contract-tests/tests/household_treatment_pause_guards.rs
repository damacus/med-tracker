use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::{env, sync::Arc, thread, time::{Duration, Instant}};

#[path = "household_completion_medication/authentication.rs"]
mod authentication;

fn input(html: &str, name: &str) -> String {
    let document = Html::parse_document(html);
    let node = document.select(&Selector::parse(&format!("[name='{name}']")).unwrap()).next().expect("original native pause field");
    if node.value().name() == "textarea" { node.text().collect() }
    else if node.value().name() == "select" { node.select(&Selector::parse("option[selected]").unwrap()).next().unwrap().value().attr("value").unwrap().into() }
    else { node.value().attr("value").unwrap().into() }
}

fn login(target: &Target, fixture: &Fixture, client: &str) {
    let html = target.get_html("/login").text().unwrap();
    assert_eq!(target.post_html_form_from_client("/login", client, &[("authenticity_token".into(), input(&html, "authenticity_token")), ("email".into(), fixture.primary_email.clone()), ("password".into(), "password".into())]).status().as_u16(), 302);
}

#[derive(Clone, Copy)]
enum Kind { Assignment, Schedule }

impl Kind {
    fn resource(self) -> &'static str { match self { Self::Assignment => "person_medications", Self::Schedule => "schedules" } }
    fn body(self) -> &'static str { match self { Self::Assignment => "person_medication", Self::Schedule => "schedule" } }
    fn browser(self) -> &'static str { match self { Self::Assignment => "assignments", Self::Schedule => "schedules" } }
    fn version(self) -> &'static str { match self { Self::Assignment => "PersonMedication", Self::Schedule => "Schedule" } }
}

fn setup(kind: Kind, client: &str) -> (Fixture, authentication::CurrentBearer, Target, Value) {
    let (fixture, guard) = authentication::fixture_with_current_bearer(client);
    let target = Target::from_env();
    login(&target, &fixture, client);
    let base = format!("/api/v1/households/{}", fixture.household_id);
    let response = target.post_json_authorized(&format!("{base}/medications"), &fixture.access_token, &json!({"medication": {"name": format!("Pause boundary {client}"), "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}));
    assert_eq!(response.status().as_u16(), 201);
    let medication = response.json::<Value>().unwrap()["data"].clone();
    let mut attributes = json!({"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml", "notes": "Original source"});
    match kind {
        Kind::Assignment => { attributes["administration_kind"] = "as_needed".into(); }
        Kind::Schedule => { attributes["schedule_type"] = "daily".into(); attributes["schedule_config"] = json!({"times": ["08:00"]}); attributes["start_date"] = "2030-03-30".into(); attributes["end_date"] = "2030-04-10".into(); }
    }
    let response = target.post_json_authorized(&format!("{base}/{}", kind.resource()), &fixture.access_token, &json!({(kind.body()): attributes}));
    assert_eq!(response.status().as_u16(), 201);
    let source = response.json::<Value>().unwrap()["data"].clone();
    (fixture, guard, target, source)
}

fn api(fixture: &Fixture, kind: Kind, source: &Value) -> String { format!("/api/v1/households/{}/{}/{}", fixture.household_id, kind.resource(), source["id"].as_i64().unwrap()) }
fn web(fixture: &Fixture, kind: Kind, source: &Value, action: &str) -> String { format!("/households/{}/people/{}/{}/{}/{action}", fixture.household_slug, fixture.journey_browser_person_id, kind.browser(), source["id"].as_i64().unwrap()) }
fn form(target: &Target, path: &str) -> String { let response = target.get_html(path); assert_eq!(response.status().as_u16(), 200); response.text().unwrap() }
fn read(target: &Target, fixture: &Fixture, path: &str) -> Value { let response = target.get(path, Some(&fixture.access_token)); assert_eq!(response.status().as_u16(), 200); response.json::<Value>().unwrap()["data"].clone() }
fn history(target: &Target, fixture: &Fixture, kind: Kind, source: &Value) -> Value {
    read(target, fixture, &format!("/api/v1/households/{}/medication_pause_periods?source_type={}&source_id={}&per_page=100", fixture.household_id, kind.body(), source["portable_id"].as_str().unwrap()))
}

fn fields(html: &str, resuming: bool, kind: Kind, source: &Value) -> Vec<(String, String)> {
    let mut fields = ["authenticity_token", "etag", "submission_id"].into_iter().map(|name| (name.into(), input(html, name))).collect::<Vec<_>>();
    fields.push(("source_type".into(), kind.body().into()));
    fields.push(("source_id".into(), source["portable_id"].as_str().unwrap().into()));
    if resuming { for name in ["pause_period_id", "period_etag"] { fields.push((name.into(), input(html, name))); } }
    else { fields.push(("reason".into(), "clinician_advice".into())); fields.push(("note".into(), "Retained </textarea><script>pause</script> & note".into())); }
    fields
}

fn retained(html: &str, fields: &[(String, String)]) {
    for (name, value) in fields { assert_eq!(input(html, name), *value, "rejected pause draft {name}"); }
    assert!(!html.contains("<script>pause</script>"));
}

fn counts(fixture: &Fixture, kind: Kind, source: &Value) -> (i64, i64, i64, i64) {
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let id = source["id"].as_i64().unwrap();
    let version = kind.version();
    let source_column = match kind { Kind::Assignment => "person_medication_id", Kind::Schedule => "schedule_id" };
    (
        db.query_one("SELECT count(*) FROM versions WHERE item_type = $1 AND item_id = $2", &[&version, &id]).unwrap().get(0),
        db.query_one("SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = $2 AND record_id = $3", &[&fixture.household_id, &version, &id]).unwrap().get(0),
        db.query_one(&format!("SELECT count(*) FROM versions WHERE item_type = 'MedicationPausePeriod' AND item_id IN (SELECT id FROM medication_pause_periods WHERE {source_column} = $1)"), &[&id]).unwrap().get(0),
        db.query_one(&format!("SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'MedicationPausePeriod' AND record_id IN (SELECT id FROM medication_pause_periods WHERE {source_column} = $2)"), &[&fixture.household_id, &id]).unwrap().get(0),
    )
}

fn paused(target: &Target, fixture: &Fixture, kind: Kind, source: &Value) {
    let endpoint = web(fixture, kind, source, "pause");
    let html = form(target, &endpoint);
    assert_eq!(target.post_browser_form(&endpoint, &fields(&html, false, kind, source)).status().as_u16(), 303);
    assert!(read(target, fixture, &api(fixture, kind, source))["paused"].as_bool().unwrap());
}

fn change_source(target: &Target, fixture: &Fixture, kind: Kind, source: &Value) {
    let response = target.patch_json(&api(fixture, kind, source), &fixture.access_token, &json!({(kind.body()): {"notes": "Concurrent source change"}}));
    assert_eq!(response.status().as_u16(), 200);
}

fn no_write(target: &Target, fixture: &Fixture, kind: Kind, source: &Value, original: &Value, periods: &Value, count: (i64, i64, i64, i64)) {
    assert_eq!(read(target, fixture, &api(fixture, kind, source)), *original);
    assert_eq!(history(target, fixture, kind, source), *periods);
    assert_eq!(counts(fixture, kind, source), count);
}

fn stale(kind: Kind, resuming: bool, client: &str) {
    let (fixture, _guard, target, source) = setup(kind, client);
    if resuming { paused(&target, &fixture, kind, &source); }
    let endpoint = web(&fixture, kind, &source, if resuming { "resume" } else { "pause" });
    let html = form(&target, &endpoint);
    let fields = fields(&html, resuming, kind, &source);
    change_source(&Target::from_env(), &fixture, kind, &source);
    let original = read(&target, &fixture, &api(&fixture, kind, &source));
    let periods = history(&target, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    let response = target.post_browser_form(&endpoint, &fields);
    assert_eq!(response.status().as_u16(), 409, "original source token protects the pause action");
    retained(&response.text().unwrap(), &fields);
    no_write(&target, &fixture, kind, &source, &original, &periods, count);
}

fn missing(kind: Kind, resuming: bool, client: &str) {
    let (fixture, _guard, target, source) = setup(kind, client);
    if resuming { paused(&target, &fixture, kind, &source); }
    let endpoint = web(&fixture, kind, &source, if resuming { "resume" } else { "pause" });
    let html = form(&target, &endpoint);
    let original = read(&target, &fixture, &api(&fixture, kind, &source));
    let periods = history(&target, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    for token in [None, Some(""), Some("   ")] {
        let mut draft = fields(&html, resuming, kind, &source);
        draft.retain(|(name, _)| name != "etag");
        if let Some(token) = token { draft.push(("etag".into(), token.into())); }
        let response = target.post_browser_form(&endpoint, &draft);
        assert_eq!(response.status().as_u16(), 428, "missing original source token must not mutate treatment");
        let rejected = response.text().unwrap();
        retained(&rejected, &draft);
        assert_eq!(input(&rejected, "etag"), token.unwrap_or_default());
        no_write(&target, &fixture, kind, &source, &original, &periods, count);
    }
}

#[test] fn assignment_pause_rejects_stale_source() { stale(Kind::Assignment, false, "198.18.42.1"); }
#[test] fn assignment_resume_rejects_stale_source_with_unchanged_period() { stale(Kind::Assignment, true, "198.18.42.2"); }
#[test] fn schedule_pause_rejects_stale_source() { stale(Kind::Schedule, false, "198.18.42.3"); }
#[test] fn schedule_resume_rejects_stale_source_with_unchanged_period() { stale(Kind::Schedule, true, "198.18.42.4"); }
#[test] fn assignment_pause_requires_original_source_token() { missing(Kind::Assignment, false, "198.18.42.5"); }
#[test] fn assignment_resume_requires_original_source_token() { missing(Kind::Assignment, true, "198.18.42.6"); }

struct ReadGate { db: Client, name: String, key: i32 }

impl ReadGate {
    fn new(fixture: &Fixture, kind: Kind, source: &Value) -> Self {
        let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
        let controller = format!("api/v1/{}", kind.resource());
        let session: String = db.query_one("SELECT audit_context->>'session_reference' FROM security_audit_events WHERE actor_account_id = $1 AND household_id = $2 AND metadata->>'controller' = $3 AND metadata->>'action' = 'show' AND audit_context->>'authentication_method' = 'browser_session' ORDER BY id DESC LIMIT 1", &[&fixture.account_id, &fixture.household_id, &controller]).expect("pause form establishes the browser source-read session").get(0);
        let key = i32::try_from(source["id"].as_i64().unwrap()).unwrap();
        let name = format!("completion_pause_gate_{key}");
        db.query_one("SELECT pg_advisory_lock(180042, $1)", &[&key]).unwrap();
        let mut gate = Self { db, name, key };
        let session = session.replace('\'', "''");
        gate.db.batch_execute(&format!("CREATE FUNCTION {}() RETURNS trigger LANGUAGE plpgsql AS $gate$ BEGIN IF NEW.actor_account_id = {} AND NEW.household_id = {} AND NEW.metadata->>'controller' = '{}' AND NEW.metadata->>'action' = 'show' AND NEW.metadata->>'http_method' = 'GET' AND NEW.metadata->>'status' = '200' AND NEW.audit_context->>'authentication_method' = 'browser_session' AND NEW.audit_context->>'session_reference' = '{}' THEN PERFORM pg_advisory_xact_lock(180042, {}); END IF; RETURN NEW; END $gate$; CREATE TRIGGER {} BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION {}();", gate.name, fixture.account_id, fixture.household_id, controller, session, key, gate.name, gate.name)).expect("install disposable source-show gate");
        gate
    }

    fn wait_for_read(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let blocked: bool = self.db.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype = 'advisory' AND classid = 180042::oid AND objid = $1::int::oid AND objsubid = 2 AND NOT granted)", &[&self.key]).unwrap().get(0);
            if blocked { return true; }
            thread::yield_now();
        }
        false
    }

    fn release(&mut self) {
        let released: bool = self.db.query_one("SELECT pg_advisory_unlock(180042, $1)", &[&self.key]).unwrap().get(0);
        assert!(released, "fixture must own the source-read gate");
    }
}

impl Drop for ReadGate {
    fn drop(&mut self) {
        let _ = self.db.query_one("SELECT pg_advisory_unlock(180042, $1)", &[&self.key]);
        if let Err(error) = self.db.batch_execute(&format!("DROP TRIGGER IF EXISTS {} ON security_audit_events; DROP FUNCTION IF EXISTS {}();", self.name, self.name)) {
            eprintln!("failed to remove disposable treatment read gate: {error}");
        }
    }
}

fn interleaved(kind: Kind, resuming: bool, client: &str) {
    let (fixture, _guard, target, source) = setup(kind, client);
    if resuming { paused(&target, &fixture, kind, &source); }
    let target = Arc::new(target);
    let endpoint = web(&fixture, kind, &source, if resuming { "resume" } else { "pause" });
    let html = form(&target, &endpoint);
    let fields = fields(&html, resuming, kind, &source);
    let mut gate = ReadGate::new(&fixture, kind, &source);
    let submitted = fields.clone();
    let worker_target = Arc::clone(&target);
    let worker_endpoint = endpoint.clone();
    let worker = thread::spawn(move || worker_target.post_browser_form(&worker_endpoint, &submitted));
    if !gate.wait_for_read() {
        gate.release();
        let response = worker.join().unwrap();
        panic!("browser source read never reached deterministic gate: {}", response.status());
    }
    let owner = Target::from_env();
    change_source(&owner, &fixture, kind, &source);
    let original = read(&owner, &fixture, &api(&fixture, kind, &source));
    let periods = history(&owner, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    gate.release();
    let response = worker.join().unwrap();
    assert_eq!(response.status().as_u16(), 409, "source changed after browser read but before canonical pause write");
    retained(&response.text().unwrap(), &fields);
    no_write(&owner, &fixture, kind, &source, &original, &periods, count);
}

#[test] fn assignment_pause_rejects_interleaved_source_change() { interleaved(Kind::Assignment, false, "198.18.42.7"); }
#[test] fn assignment_resume_rejects_interleaved_source_change() { interleaved(Kind::Assignment, true, "198.18.42.8"); }
#[test] fn schedule_pause_rejects_interleaved_source_change() { interleaved(Kind::Schedule, false, "198.18.42.9"); }
#[test] fn schedule_resume_rejects_interleaved_source_change() { interleaved(Kind::Schedule, true, "198.18.42.10"); }

fn replay(kind: Kind, client: &str) {
    let (fixture, _guard, target, source) = setup(kind, client);
    let pause = web(&fixture, kind, &source, "pause");
    let html = form(&target, &pause);
    let original = fields(&html, false, kind, &source);
    assert_eq!(target.post_browser_form(&pause, &original).status().as_u16(), 303);
    let after = read(&target, &fixture, &api(&fixture, kind, &source));
    let periods = history(&target, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    assert_eq!(target.post_browser_form(&pause, &original).status().as_u16(), 303, "authorised exact pause replay precedes its own source-version change");
    no_write(&target, &fixture, kind, &source, &after, &periods, count);
    let mut changed = original.clone();
    changed.iter_mut().find(|(name, _)| name == "note").unwrap().1 = "Changed payload under original key".into();
    let response = target.post_browser_form(&pause, &changed);
    assert_eq!(response.status().as_u16(), 409);
    retained(&response.text().unwrap(), &changed);
    no_write(&target, &fixture, kind, &source, &after, &periods, count);
    let resume = web(&fixture, kind, &source, "resume");
    let html = form(&target, &resume);
    let original = fields(&html, true, kind, &source);
    assert_eq!(target.post_browser_form(&resume, &original).status().as_u16(), 303);
    let after = read(&target, &fixture, &api(&fixture, kind, &source));
    let periods = history(&target, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    assert_eq!(target.post_browser_form(&resume, &original).status().as_u16(), 303, "exact resume replay addresses the original closed period");
    no_write(&target, &fixture, kind, &source, &after, &periods, count);
    assert_eq!(periods.as_array().unwrap().len(), 1);
    assert!(!periods[0]["ended_at"].is_null());
}

#[test] fn assignment_pause_resume_exact_replay_and_payload_conflict() { replay(Kind::Assignment, "198.18.42.11"); }
#[test] fn schedule_pause_resume_exact_replay_and_payload_conflict() { replay(Kind::Schedule, "198.18.42.12"); }

fn public_compatibility(kind: Kind, client: &str) {
    let (fixture, _guard, target, source) = setup(kind, client);
    change_source(&target, &fixture, kind, &source);
    let base = format!("/api/v1/households/{}/medication_pause_periods", fixture.household_id);
    let response = target.post_json_authorized(&base, &fixture.access_token, &json!({"medication_pause_period": {"source_type": kind.body(), "source_id": source["portable_id"], "reason": "clinician_advice", "note": "Public API retains its original preconditions"}}));
    assert_eq!(response.status().as_u16(), 201, "public pause has no browser source-token requirement");
    let period = response.json::<Value>().unwrap()["data"].clone();
    let response = target.patch_json(&api(&fixture, kind, &source), &fixture.access_token, &json!({(kind.body()): {"notes": "Another change while paused"}}));
    assert_eq!(response.status().as_u16(), 200);
    let response = target.post_json_authorized(&format!("{base}/{}/resume", period["id"].as_str().unwrap()), &fixture.access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200, "public resume retains optional period If-Match and no source token");
    let updated = read(&target, &fixture, &api(&fixture, kind, &source));
    assert_eq!(updated["paused"], false);
    assert_eq!(updated["notes"], "Another change while paused");
    let periods = history(&target, &fixture, kind, &source);
    assert_eq!(periods.as_array().unwrap().len(), 1);
    assert_eq!(periods[0]["id"], period["id"]);
    assert!(!periods[0]["ended_at"].is_null());
}

#[test] fn assignment_public_pause_resume_contract_remains_unchanged() { public_compatibility(Kind::Assignment, "198.18.42.13"); }
#[test] fn schedule_public_pause_resume_contract_remains_unchanged() { public_compatibility(Kind::Schedule, "198.18.42.14"); }

#[test]
fn resume_requires_original_period_token_separately_from_source_token() {
    let (fixture, _guard, target, source) = setup(Kind::Assignment, "198.18.42.15");
    paused(&target, &fixture, Kind::Assignment, &source);
    let endpoint = web(&fixture, Kind::Assignment, &source, "resume");
    let html = form(&target, &endpoint);
    let original = read(&target, &fixture, &api(&fixture, Kind::Assignment, &source));
    let periods = history(&target, &fixture, Kind::Assignment, &source);
    let count = counts(&fixture, Kind::Assignment, &source);
    for token in [None, Some(""), Some("   ")] {
        let mut draft = fields(&html, true, Kind::Assignment, &source);
        draft.retain(|(name, _)| name != "period_etag");
        if let Some(token) = token { draft.push(("period_etag".into(), token.into())); }
        let response = target.post_browser_form(&endpoint, &draft);
        assert_eq!(response.status().as_u16(), 428, "the source token cannot replace the original period token");
        let html = response.text().unwrap();
        retained(&html, &draft);
        assert_eq!(input(&html, "period_etag"), token.unwrap_or_default());
        no_write(&target, &fixture, Kind::Assignment, &source, &original, &periods, count);
    }
}

#[test]
fn stale_period_token_rejects_resume_with_current_source_token() {
    let (fixture, _guard, target, source) = setup(Kind::Assignment, "198.18.42.16");
    paused(&target, &fixture, Kind::Assignment, &source);
    let endpoint = web(&fixture, Kind::Assignment, &source, "resume");
    let html = form(&target, &endpoint);
    let mut draft = fields(&html, true, Kind::Assignment, &source);
    draft.iter_mut().find(|(name, _)| name == "period_etag").unwrap().1 = "\"stale-original-period\"".into();
    let original = read(&target, &fixture, &api(&fixture, Kind::Assignment, &source));
    let periods = history(&target, &fixture, Kind::Assignment, &source);
    let count = counts(&fixture, Kind::Assignment, &source);
    let response = target.post_browser_form(&endpoint, &draft);
    assert_eq!(response.status().as_u16(), 409);
    retained(&response.text().unwrap(), &draft);
    no_write(&target, &fixture, Kind::Assignment, &source, &original, &periods, count);
}

fn mismatched_identity(resuming: bool, client: &str) {
    let kind = Kind::Assignment;
    let (fixture, _guard, target, source) = setup(kind, client);
    if resuming { paused(&target, &fixture, kind, &source); }
    let endpoint = web(&fixture, kind, &source, if resuming { "resume" } else { "pause" });
    let html = form(&target, &endpoint);
    let original = read(&target, &fixture, &api(&fixture, kind, &source));
    let periods = history(&target, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    let other_source_id = "00000000-0000-4000-8000-000000000042";
    assert_ne!(source["portable_id"].as_str().unwrap(), other_source_id);
    for (name, value) in [("source_type", "schedule".to_owned()), ("source_id", other_source_id.to_owned())] {
        let mut draft = fields(&html, resuming, kind, &source);
        draft.iter_mut().find(|(field, _)| field == name).unwrap().1 = value;
        let response = target.post_browser_form(&endpoint, &draft);
        assert_eq!(response.status().as_u16(), 409, "original source identity must match the addressed treatment");
        retained(&response.text().unwrap(), &draft);
        no_write(&target, &fixture, kind, &source, &original, &periods, count);
    }
}

#[test] fn pause_rejects_mismatched_original_source_kind_and_identity() { mismatched_identity(false, "198.18.42.17"); }
#[test] fn resume_rejects_mismatched_original_source_kind_and_identity() { mismatched_identity(true, "198.18.42.18"); }
fn invalid_submission_key(resuming: bool, client: &str) {
    let kind = Kind::Assignment;
    let (fixture, _guard, target, source) = setup(kind, client);
    if resuming { paused(&target, &fixture, kind, &source); }
    let endpoint = web(&fixture, kind, &source, if resuming { "resume" } else { "pause" });
    let html = form(&target, &endpoint);
    let original = read(&target, &fixture, &api(&fixture, kind, &source));
    let periods = history(&target, &fixture, kind, &source);
    let count = counts(&fixture, kind, &source);
    for key in [None, Some(""), Some("   "), Some("not-a-uuid")] {
        let mut draft = fields(&html, resuming, kind, &source);
        draft.retain(|(name, _)| name != "submission_id");
        if let Some(key) = key { draft.push(("submission_id".into(), key.into())); }
        let response = target.post_browser_form(&endpoint, &draft);
        assert_eq!(response.status().as_u16(), 428, "pause action requires its original replay key before dispatch");
        let rejected = response.text().unwrap();
        retained(&rejected, &draft);
        assert_eq!(input(&rejected, "submission_id"), key.unwrap_or_default());
        no_write(&target, &fixture, kind, &source, &original, &periods, count);
    }
}

#[test]
fn pause_requires_original_submission_key() {
    invalid_submission_key(false, "198.18.42.19");
}

#[test]
fn resume_requires_original_submission_key() {
    invalid_submission_key(true, "198.18.42.20");
}
