use medtracker_contract_tests::{Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::env;

fn input(html: &str, name: &str) -> String {
    Html::parse_document(html).select(&Selector::parse(&format!("[name='{name}']")).unwrap()).next().unwrap().value().attr("value").unwrap().into()
}

fn login(target: &Target, email: &str, client: &str) {
    let html = target.get_html("/login").text().unwrap();
    assert_eq!(target.post_html_form_from_client("/login", client, &[("authenticity_token".into(), input(&html, "authenticity_token")), ("email".into(), email.into()), ("password".into(), "password".into())]).status().as_u16(), 302);
}

fn csrf(target: &Target, path: &str) -> String {
    let html = target.get_html(path).text().unwrap();
    Html::parse_document(&html).select(&Selector::parse("meta[name='csrf-token']").unwrap()).next().unwrap().value().attr("content").unwrap().into()
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

#[test]
fn ordinary_current_manage_grant_allows_treatments_but_revocation_denies_captured_actions_and_replay() {
    let fixture = fixture();
    let owner = Target::from_env();
    login(&owner, &fixture.primary_email, "198.18.44.1");
    let inventory = format!("/households/{}/medications", fixture.household_slug);
    let owner_csrf = csrf(&owner, &inventory);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let response = owner.post_browser_json(&format!("{api}/people"), &owner_csrf, &json!({"person": {"name": "Treatment permission person", "person_type": "adult", "has_capacity": true, "date_of_birth": "1980-01-01"}}));
    assert_eq!(response.status().as_u16(), 201);
    let person = response.json::<Value>().unwrap()["data"].clone();
    let medication = read(&owner, &format!("{api}/medications/{}", fixture.dose_write_medication_id));
    let response = owner.post_browser_json(&format!("{api}/admin/person_access_grants"), &owner_csrf, &json!({"person_access_grant": {"household_membership_id": fixture.view_membership_id, "person_id": person["id"], "access_level": "manage", "relationship_type": "family_member"}}));
    assert_eq!(response.status().as_u16(), 201);
    let grant = response.json::<Value>().unwrap()["data"]["id"].as_i64().unwrap();
    let member = Target::from_env();
    login(&member, &fixture.web_view_email, "198.18.44.2");
    assert_eq!(read(&member, &format!("{api}/me"))["membership_role"], "member");
    let base = format!("/households/{}/people/{}", fixture.household_slug, person["id"].as_i64().unwrap());
    let response = member.get_html(&format!("{base}/assignments/new"));
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let stock_choice = Selector::parse(&format!("select[name='medication_id'] option[value='{}']", fixture.dose_write_medication_id)).unwrap();
    assert_eq!(Html::parse_document(&html).select(&stock_choice).count(), 1, "submitted medication is an actual authorised choice");
    assert_eq!(read(&member, &format!("{api}/medications/{}", fixture.dose_write_medication_id))["id"], medication["id"]);
    let member_csrf = input(&html, "authenticity_token");
    let assignment = vec![("authenticity_token".into(), member_csrf.clone()), ("submission_id".into(), input(&html, "submission_id")), ("medication_id".into(), medication["id"].as_i64().unwrap().to_string()), ("dose_amount".into(), "1.25".into()), ("dose_unit".into(), "ml".into()), ("administration_kind".into(), "as_needed".into()), ("notes".into(), "Ordinary member entries".into())];
    assert_eq!(member.post_browser_form(&format!("{base}/assignments"), &assignment).status().as_u16(), 303);
    let sources = read(&owner, &format!("{api}/person_medications?per_page=100"));
    let source = sources.as_array().unwrap().iter().find(|row| row["person_id"] == person["id"] && row["medication_id"] == medication["id"]).unwrap().clone();
    let source_id = source["id"].as_i64().unwrap();
    let member_path = format!("{base}/assignments/{source_id}");
    let response = member.get_html(&format!("{base}/schedules/new?type=daily"));
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let schedule_draft = vec![("authenticity_token".into(), member_csrf.clone()), ("submission_id".into(), input(&html, "submission_id")), ("medication_id".into(), medication["id"].as_i64().unwrap().to_string()), ("dose_amount".into(), "1.25".into()), ("dose_unit".into(), "ml".into()), ("schedule_type".into(), "daily".into()), ("start_date".into(), "2030-03-30".into()), ("end_date".into(), "2030-04-10".into()), ("time_0".into(), "08:00".into()), ("notes".into(), "Ordinary member schedule".into())];
    assert_eq!(member.post_browser_form(&format!("{base}/schedules"), &schedule_draft).status().as_u16(), 303);
    let schedules = read(&owner, &format!("{api}/schedules?per_page=100"));
    let schedule = schedules.as_array().unwrap().iter().find(|row| row["person_id"] == person["id"] && row["medication_id"] == medication["id"]).unwrap().clone();
    let schedule_id = schedule["id"].as_i64().unwrap();
    let schedule_path = format!("{base}/schedules/{schedule_id}");
    let response = member.get_html(&format!("{schedule_path}/pause"));
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let mut schedule_pause = ["authenticity_token", "submission_id", "etag", "source_type", "source_id"].into_iter().map(|name| (name.into(), input(&html, name))).collect::<Vec<_>>();
    schedule_pause.push(("reason".into(), "clinician_advice".into()));
    schedule_pause.push(("note".into(), "Captured ordinary member schedule pause".into()));
    let response = member.get_html(&format!("{member_path}/pause"));
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let mut pause = ["authenticity_token", "submission_id", "etag", "source_type", "source_id"].into_iter().map(|name| (name.into(), input(&html, name))).collect::<Vec<_>>();
    pause.push(("reason".into(), "clinician_advice".into()));
    pause.push(("note".into(), "Member can pause".into()));
    let mut forged = pause.clone();
    forged.iter_mut().find(|(name, _)| name == "authenticity_token").unwrap().1 = "wrong-csrf".into();
    let before = read(&owner, &format!("{api}/person_medications/{source_id}"));
    let schedule_before = read(&owner, &format!("{api}/schedules/{schedule_id}"));
    assert_eq!(member.post_browser_form(&format!("{member_path}/pause"), &forged).status().as_u16(), 403);
    assert_eq!(read(&owner, &format!("{api}/person_medications/{source_id}")), before);
    assert_eq!(member.post_browser_form(&format!("{member_path}/pause"), &pause).status().as_u16(), 303);
    assert_eq!(member.post_browser_form(&format!("{member_path}/pause"), &pause).status().as_u16(), 303);
    let html = member.get_html(&format!("{member_path}/resume")).text().unwrap();
    let resume = ["authenticity_token", "submission_id", "etag", "source_type", "source_id", "pause_period_id", "period_etag"].into_iter().map(|name| (name.into(), input(&html, name))).collect::<Vec<_>>();
    let before = read(&owner, &format!("{api}/person_medications/{source_id}"));
    let history_path = format!("{api}/medication_pause_periods?source_type=person_medication&source_id={}&per_page=100", source["portable_id"].as_str().unwrap());
    let history = read(&owner, &history_path);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let counts = |db: &mut Client| -> Vec<i64> {
        let row = db.query_one("SELECT (SELECT count(*) FROM versions WHERE item_type='PersonMedication' AND item_id=$1), (SELECT count(*) FROM api_change_events WHERE record_type='PersonMedication' AND record_id=$1), (SELECT count(*) FROM versions WHERE item_type='MedicationPausePeriod' AND item_id IN (SELECT id FROM medication_pause_periods WHERE person_medication_id=$1)), (SELECT count(*) FROM api_change_events WHERE record_type='MedicationPausePeriod' AND record_id IN (SELECT id FROM medication_pause_periods WHERE person_medication_id=$1))", &[&source_id]).unwrap();
        let mut counts = (0..4).map(|index| row.get(index)).collect::<Vec<i64>>();
        let schedule = db.query_one("SELECT (SELECT count(*) FROM versions WHERE item_type='Schedule' AND item_id=$1), (SELECT count(*) FROM api_change_events WHERE record_type='Schedule' AND record_id=$1), (SELECT count(*) FROM versions WHERE item_type='MedicationPausePeriod' AND item_id IN (SELECT id FROM medication_pause_periods WHERE schedule_id=$1)), (SELECT count(*) FROM api_change_events WHERE record_type='MedicationPausePeriod' AND record_id IN (SELECT id FROM medication_pause_periods WHERE schedule_id=$1))", &[&schedule_id]).unwrap();
        counts.extend((0..4).map(|index| schedule.get::<usize, i64>(index)));
        counts
    };
    let evidence = counts(&mut db);
    assert_eq!(owner.delete_browser_json(&format!("{api}/admin/person_access_grants/{grant}"), &owner_csrf).status().as_u16(), 204);
    for path in [format!("{base}/assignments/new"), format!("{base}/schedules/new?type=daily"), format!("{member_path}/edit"), format!("{member_path}/pause"), format!("{member_path}/resume"), format!("{member_path}/history"), format!("{schedule_path}/edit"), format!("{schedule_path}/pause"), format!("{schedule_path}/history")] {
        assert_eq!(member.get_html(&path).status().as_u16(), 404);
    }
    for (path, draft) in [(format!("{base}/assignments"), &assignment), (format!("{base}/schedules"), &schedule_draft), (format!("{schedule_path}/pause"), &schedule_pause), (format!("{member_path}/pause"), &pause), (format!("{member_path}/resume"), &resume)] {
        assert_eq!(member.post_browser_form(&path, draft).status().as_u16(), 404, "current permission defeats captured form and exact replay");
    }
    assert_eq!(read(&owner, &format!("{api}/person_medications/{source_id}")), before);
    assert_eq!(read(&owner, &format!("{api}/schedules/{schedule_id}")), schedule_before);
    assert_eq!(read(&owner, &history_path), history);
    assert_eq!(counts(&mut db), evidence);
}
