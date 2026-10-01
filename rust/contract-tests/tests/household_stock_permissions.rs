use medtracker_contract_tests::{Target, fixture};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::{Value, json};
use std::env;

fn input(html: &str, selector: &str, attribute: &str) -> String {
    Html::parse_document(html).select(&Selector::parse(selector).unwrap()).next()
        .and_then(|field| field.value().attr(attribute)).expect("native value").to_owned()
}

fn login(target: &Target, email: &str, client: &str) {
    let html = target.get_html("/login").text().unwrap();
    assert_eq!(target.post_html_form_from_client("/login", client, &[
        ("authenticity_token".into(), input(&html, "input[name='authenticity_token']", "value")),
        ("email".into(), email.into()), ("password".into(), "password".into()),
    ]).status().as_u16(), 302);
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

#[test]
fn ordinary_manage_grant_allows_order_only_and_revocation_rejects_captured_actions() {
    let fixture = fixture();
    let owner = Target::from_env();
    login(&owner, &fixture.primary_email, "198.18.35.1");
    let inventory = format!("/households/{}/medications", fixture.household_slug);
    let html = owner.get_html(&inventory).text().unwrap();
    let owner_csrf = input(&html, "meta[name='csrf-token']", "content");
    let base = format!("/api/v1/households/{}", fixture.household_id);
    let response = owner.post_browser_json(&format!("{base}/people"), &owner_csrf,
        &json!({"person": {"name": "Stock permission person", "person_type": "adult", "has_capacity": true, "date_of_birth": "1980-01-01"}}));
    assert_eq!(response.status().as_u16(), 201);
    let person = response.json::<Value>().unwrap()["data"].clone();
    let response = owner.post_browser_json(&format!("{base}/medications"), &owner_csrf,
        &json!({"medication": {"name": "Stock permission medicine", "location_id": fixture.primary_location_id,
            "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20", "reorder_threshold": "3"}}));
    assert_eq!(response.status().as_u16(), 201);
    let medication = response.json::<Value>().unwrap()["data"].clone();
    let id = medication["id"].as_i64().unwrap();
    let response = owner.post_browser_json(&format!("{base}/person_medications"), &owner_csrf,
        &json!({"person_medication": {"person_id": person["id"].as_i64().unwrap().to_string(), "medication_id": id.to_string(),
            "dose_amount": "1.25", "dose_unit": "ml", "administration_kind": "as_needed"}}));
    assert_eq!(response.status().as_u16(), 201);
    let response = owner.post_browser_json(&format!("{base}/admin/person_access_grants"), &owner_csrf,
        &json!({"person_access_grant": {"household_membership_id": fixture.view_membership_id, "person_id": person["id"], "access_level": "manage", "relationship_type": "family_member"}}));
    assert_eq!(response.status().as_u16(), 201);
    let grant = response.json::<Value>().unwrap()["data"]["id"].as_i64().unwrap();
    let member = Target::from_env();
    login(&member, &fixture.web_view_email, "198.18.35.2");
    assert_eq!(read(&member, &format!("{base}/me"))["membership_role"], "member");
    assert_eq!(read(&member, &format!("{base}/ui_capabilities"))["medications"]["create"], true);
    let web = format!("{inventory}/{id}/stock");
    let response = member.get_html(&web);
    assert_eq!(response.status().as_u16(), 200);
    let html = response.text().unwrap();
    let document = Html::parse_document(&html);
    assert!(document.select(&Selector::parse("a[href$='/stock/adjust'],a[href$='/stock/remove']").unwrap()).next().is_none());
    let member_csrf = input(&html, "input[name='authenticity_token']", "value");
    let parent_api = format!("{base}/medications/{id}");
    let before = read(&owner, &parent_api);
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let counts = |db: &mut Client| -> (i64, i64) {
        (db.query_one("SELECT count(*) FROM versions WHERE item_type = 'Medication' AND item_id = $1", &[&id]).unwrap().get(0),
         db.query_one("SELECT count(*) FROM api_change_events WHERE record_type = 'Medication' AND record_id = $1", &[&id]).unwrap().get(0))
    };
    let evidence = counts(&mut db);
    for action in ["adjust", "remove"] {
        assert_eq!(member.get_html(&format!("{web}/{action}")).status().as_u16(), 403);
        assert_eq!(member.post_browser_form(&format!("{web}/{action}"), &[
            ("authenticity_token".into(), member_csrf.clone()), ("new_quantity".into(), "90".into()), ("quantity".into(), "1.25".into()),
            ("reason".into(), "dropped".into()), ("submission_id".into(), "35000000-0000-4000-8000-000000000001".into()),
        ]).status().as_u16(), 403);
    }
    assert_eq!(read(&owner, &parent_api), before);
    assert_eq!(counts(&mut db), evidence);
    assert_eq!(member.post_browser_form(&format!("{web}/order"), &[("authenticity_token".into(), member_csrf.clone()), ("supplier".into(), "Community pharmacy".into()), ("quantity".into(), "8.25".into())]).status().as_u16(), 303);
    assert_eq!(member.post_browser_form(&format!("{web}/receive"), &[("authenticity_token".into(), member_csrf.clone())]).status().as_u16(), 303);
    let before_revoke = read(&owner, &parent_api);
    assert_eq!(before_revoke["reorder_status"], "received");
    assert_eq!(before_revoke["current_supply"], "20.0");
    let evidence = counts(&mut db);
    assert_eq!(owner.delete_browser_json(&format!("{base}/admin/person_access_grants/{grant}"), &owner_csrf).status().as_u16(), 204);
    assert_eq!(member.get_html(&web).status().as_u16(), 404);
    for action in ["adjust", "remove", "order", "receive"] {
        assert_eq!(member.post_browser_form(&format!("{web}/{action}"), &[("authenticity_token".into(), member_csrf.clone()), ("new_quantity".into(), "90".into())]).status().as_u16(), 404);
    }
    assert_eq!(read(&owner, &parent_api), before_revoke);
    assert_eq!(counts(&mut db), evidence);
}
