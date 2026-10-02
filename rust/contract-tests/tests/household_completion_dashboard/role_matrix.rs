use super::{Fixture, Html, Selector, Target, Value, database, fixture, people, sign_in};
use serde_json::json;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn data(target: &Target, path: &str, token: Option<&str>) -> Value {
    let response = target.get(path, token);
    assert_eq!(response.status().as_u16(), 200, "authorised read {path}");
    response.json::<Value>().unwrap()["data"].clone()
}

fn identity(fixture: &Fixture, token: &str, role: &str) -> Value {
    let me = data(
        &Target::from_env(),
        &format!("/api/v1/households/{}/me", fixture.household_id),
        Some(token),
    );
    assert_eq!(me["membership_role"], role);
    assert_eq!(me["active"], true);
    me
}

fn browser(me: &Value) -> Target {
    let target = Target::from_env();
    sign_in(&target, me["email_address"].as_str().unwrap());
    target
}

fn csrf(target: &Target, fixture: &Fixture) -> String {
    let response = target.get_html(&format!(
        "/households/{}/medications",
        fixture.household_slug
    ));
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().unwrap());
    document
        .select(&Selector::parse("meta[name='csrf-token']").unwrap())
        .next()
        .and_then(|token| token.value().attr("content"))
        .expect("real owner API CSRF")
        .to_owned()
}

fn dashboard(target: &Target, fixture: &Fixture, selection: Option<i64>, manager: bool) -> String {
    let expected_people = people(target, fixture);
    let mut path = format!("/households/{}/dashboard", fixture.household_slug);
    if let Some(selection) = selection {
        path.push_str(&format!("?dashboard_person_id={selection}"));
    }
    let response = target.get_html(&path);
    assert_eq!(
        response.status().as_u16(),
        200,
        "permitted dashboard {path}"
    );
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let html = response.text().unwrap();
    let document = Html::parse_document(&html);
    let mut choices: Vec<String> = document
        .select(&Selector::parse("[data-testid='dashboard-person-option']").unwrap())
        .map(|choice| {
            choice
                .value()
                .attr("href")
                .unwrap()
                .rsplit('=')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect();
    let mut expected: Vec<_> = expected_people.iter().map(i64::to_string).collect();
    expected.push("all".into());
    choices.sort();
    expected.sort();
    assert_eq!(choices, expected, "exact permitted person choices");
    let hidden = data(
        &Target::from_env(),
        &format!(
            "/api/v1/households/{}/people/{}",
            fixture.household_id, fixture.hidden_person_id
        ),
        Some(&fixture.feed_access_token),
    );
    let text = document.root_element().text().collect::<String>();
    assert!(!text.contains(hidden["name"].as_str().unwrap()));
    assert!(!text.contains(&fixture.foreign_person_name));
    let administration = document
        .select(&Selector::parse("nav button").unwrap())
        .any(|button| button.text().collect::<String>().trim() == "Administration");
    assert_eq!(
        administration, manager,
        "household administration follows role, independently of person grant"
    );
    html
}

#[test]
fn owner_with_authorised_self_profile_can_use_dashboard() {
    let fixture = fixture();
    let owner = Target::from_env();
    sign_in(&owner, &fixture.primary_email);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let me = data(&owner, &format!("{api}/me"), None);
    assert_eq!(me["membership_role"], "owner");
    let profile = data(&owner, &format!("{api}/profile"), None);
    assert_eq!(
        profile["person_id"].as_str().unwrap(),
        fixture.user_person_id.to_string()
    );
    let permitted = people(&owner, &fixture);
    assert!(permitted.contains(&fixture.user_person_id));
    assert!(permitted.contains(&fixture.managed_person_id));
    let html = dashboard(&owner, &fixture, Some(fixture.user_person_id), true);
    assert!(html.contains(me["person"]["name"].as_str().unwrap()));
    assert_eq!(people(&owner, &fixture), permitted);
}

#[test]
fn administrator_without_person_grants_sees_no_unpermitted_clinical_data() {
    let fixture = fixture();
    let me = identity(&fixture, &fixture.manager_access_token, "administrator");
    let administrator = browser(&me);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    assert!(people(&administrator, &fixture).is_empty());
    assert_eq!(
        administrator
            .get(&format!("{api}/profile"), None)
            .status()
            .as_u16(),
        403
    );
    dashboard(&administrator, &fixture, None, true);
    assert!(people(&administrator, &fixture).is_empty());
    assert_eq!(
        administrator
            .get(&format!("{api}/people/{}", fixture.managed_person_id), None)
            .status()
            .as_u16(),
        404
    );
    assert_eq!(
        administrator
            .get(&format!("{api}/profile"), None)
            .status()
            .as_u16(),
        403
    );
}

#[test]
fn member_with_manage_carer_grant_can_use_only_permitted_dashboard_people() {
    let fixture = fixture();
    let me = identity(&fixture, &fixture.delegated_access_token, "member");
    let carer = browser(&me);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let mut db = database();
    let row = db.query_one("SELECT access_level, relationship_type FROM person_access_grants WHERE household_id = $1 AND person_id = $2 AND household_membership_id IN (SELECT id FROM household_memberships WHERE account_id = $3) AND revoked_at IS NULL", &[&fixture.household_id, &fixture.managed_person_id, &me["account"]["id"].as_i64().unwrap()]).unwrap();
    assert_eq!(row.get::<_, String>(0), "manage");
    assert_eq!(row.get::<_, String>(1), "carer");
    assert_eq!(people(&carer, &fixture), vec![fixture.managed_person_id]);
    assert_eq!(
        carer.get(&format!("{api}/profile"), None).status().as_u16(),
        403
    );
    dashboard(&carer, &fixture, Some(fixture.managed_person_id), false);
    assert_eq!(people(&carer, &fixture), vec![fixture.managed_person_id]);
}

struct ParentGrant<'a> {
    owner: &'a Target,
    csrf: String,
    path: String,
    active: bool,
}

impl ParentGrant<'_> {
    fn revoke(&mut self) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.owner.delete_browser_json(&self.path, &self.csrf)
        }));
        match result {
            Ok(response) if response.status().as_u16() == 204 => {
                self.active = false;
                true
            }
            _ => false,
        }
    }
}

impl Drop for ParentGrant<'_> {
    fn drop(&mut self) {
        if self.active && !self.revoke() {
            eprintln!("failed to revoke disposable parent grant");
        }
    }
}

fn create(owner: &Target, csrf: &str, path: &str, body: &Value) -> Value {
    let response = owner.post_browser_json(path, csrf, body);
    assert_eq!(
        response.status().as_u16(),
        201,
        "real fixture creation {path}"
    );
    response.json::<Value>().unwrap()["data"].clone()
}

fn listed_grant(owner: &Target, api: &str, id: &Value) -> Value {
    let grants = data(owner, &format!("{api}/admin/person_access_grants"), None);
    let matching: Vec<_> = grants
        .as_array()
        .unwrap()
        .iter()
        .filter(|grant| grant["id"] == *id)
        .collect();
    assert_eq!(matching.len(), 1, "exact created grant in real collection");
    matching[0].clone()
}

#[test]
fn member_with_public_parent_grant_can_use_minor_source_without_household_manager_access() {
    let fixture = fixture();
    let me = identity(&fixture, &fixture.delegated_access_token, "member");
    let care_me = identity(&fixture, &fixture.care_access_token, "owner");
    let creator = browser(&care_me);
    let creator_csrf = csrf(&creator, &fixture);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let mut db = database();
    let membership_id: i64 = db
        .query_one(
            "SELECT id FROM household_memberships WHERE household_id = $1 AND account_id = $2",
            &[
                &fixture.household_id,
                &me["account"]["id"].as_i64().unwrap(),
            ],
        )
        .unwrap()
        .get(0);
    let child = create(
        &creator,
        &creator_csrf,
        &format!("{api}/people"),
        &json!({"person": {"name": format!("Contract parent minor {}", membership_id), "date_of_birth": "2018-01-01", "person_type": "minor", "has_capacity": false}}),
    );
    assert_eq!(child["person_type"], "minor");
    assert_eq!(child["has_capacity"], false);
    let child_id = child["id"].as_i64().unwrap();
    let owner = browser(&care_me);
    let owner_csrf = csrf(&owner, &fixture);
    assert_eq!(
        data(&owner, &format!("{api}/people/{child_id}"), None),
        child
    );
    let response = owner.post_browser_json(&format!("{api}/admin/person_access_grants"), &owner_csrf, &json!({"person_access_grant": {"household_membership_id": membership_id, "person_id": child_id, "access_level": "manage", "relationship_type": "parent"}}));
    assert_eq!(response.status().as_u16(), 201);
    let grant = response.json::<Value>().unwrap()["data"].clone();
    let mut cleanup = ParentGrant {
        owner: &owner,
        csrf: owner_csrf.clone(),
        path: format!(
            "{api}/admin/person_access_grants/{}",
            grant["id"].as_i64().unwrap()
        ),
        active: true,
    };
    assert_eq!(grant["relationship_type"], "parent");
    assert_eq!(grant["access_level"], "manage");
    assert_eq!(listed_grant(&owner, &api, &grant["id"]), grant);
    let medication = create(
        &owner,
        &owner_csrf,
        &format!("{api}/medications"),
        &json!({"medication": {"name": format!("Contract minor source {}", membership_id), "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "20.00", "reorder_threshold": "5"}}),
    );
    let source = create(
        &owner,
        &owner_csrf,
        &format!("{api}/person_medications"),
        &json!({"person_medication": {"person_id": child["portable_id"], "medication_id": medication["portable_id"], "dose_amount": "1.25", "dose_unit": "ml", "administration_kind": "as_needed", "max_daily_doses": 3, "min_hours_between_doses": "4", "dose_cycle": "daily"}}),
    );
    let parent = browser(&me);
    let current_me = data(&parent, &format!("{api}/me"), None);
    assert_eq!(current_me["membership_role"], "member");
    assert_eq!(
        parent
            .get(&format!("{api}/profile"), None)
            .status()
            .as_u16(),
        403
    );
    let mut expected_people = vec![fixture.managed_person_id, child_id];
    expected_people.sort();
    let mut permitted = people(&parent, &fixture);
    permitted.sort();
    assert_eq!(permitted, expected_people);
    let source_path = format!(
        "{api}/person_medications/{}",
        source["id"].as_i64().unwrap()
    );
    let source_before = data(&parent, &source_path, None);
    assert_eq!(source_before["person_id"], child_id);
    assert_eq!(source_before["can_record"], true);
    assert!(
        source_before["eligible_stock_medication_ids"]
            .as_array()
            .unwrap()
            .contains(&medication["id"])
    );
    let medication_path = format!("{api}/medications/{}", medication["id"].as_i64().unwrap());
    let medication_before = data(&owner, &medication_path, None);
    let stock_path = format!("{medication_path}/stock_removals");
    let stock_before = data(&owner, &stock_path, None);
    let html = dashboard(&parent, &fixture, Some(child_id), false);
    let document = Html::parse_document(&html);
    let text = document.root_element().text().collect::<String>();
    assert!(text.contains(child["name"].as_str().unwrap()));
    let tasks: Vec<_> = document
        .select(&Selector::parse("[data-testid='dashboard-as-needed-task']").unwrap())
        .collect();
    assert_eq!(tasks.len(), 1);
    assert!(
        tasks[0]
            .text()
            .collect::<String>()
            .contains(medication["name"].as_str().unwrap())
    );
    assert_eq!(data(&parent, &source_path, None), source_before);
    assert_eq!(data(&owner, &medication_path, None), medication_before);
    assert_eq!(data(&owner, &stock_path, None), stock_before);
    let viewer = super::viewer(&fixture);
    assert_eq!(people(&viewer, &fixture), vec![fixture.managed_person_id]);
    assert_eq!(
        viewer
            .get(&format!("{api}/profile"), None)
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        viewer
            .get(&format!("{api}/people/{child_id}"), None)
            .status()
            .as_u16(),
        404
    );
    assert!(cleanup.revoke(), "normal parent-grant cleanup must succeed");
    let revoked = listed_grant(&owner, &api, &grant["id"]);
    assert!(!revoked["revoked_at"].is_null());
    let after = browser(&me);
    assert_eq!(
        data(&after, &format!("{api}/me"), None)["membership_role"],
        "member"
    );
    assert_eq!(people(&after, &fixture), vec![fixture.managed_person_id]);
    assert_eq!(
        after
            .get(&format!("{api}/people/{child_id}"), None)
            .status()
            .as_u16(),
        404
    );
    assert_eq!(
        after
            .get_html(&format!(
                "/households/{}/dashboard?dashboard_person_id={child_id}",
                fixture.household_slug
            ))
            .status()
            .as_u16(),
        404
    );
}
