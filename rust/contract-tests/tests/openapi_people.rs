use medtracker_contract_tests::{fixture, Target};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

fn household_path(household_id: i64, suffix: &str) -> String {
    format!("/api/v1/households/{household_id}/{suffix}")
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos()
}

struct DisposableCredentials {
    db: postgres::Client,
    membership_ids: Vec<i64>,
    app_token_ids: Vec<i64>,
}

impl DisposableCredentials {
    fn new() -> Self {
        Self {
            db: postgres::Client::connect(
                &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("database URL"),
                postgres::NoTls,
            )
            .expect("contract database"),
            membership_ids: Vec::new(),
            app_token_ids: Vec::new(),
        }
    }

    fn membership(&mut self, account_id: i64, household_id: i64) -> i64 {
        let id: i64 = self.db.query_one(
            "INSERT INTO household_memberships (account_id, household_id, role, status, joined_at, created_at, updated_at) VALUES ($1, $2, 'member', 'active', now(), now(), now()) RETURNING id",
            &[&account_id, &household_id],
        ).expect("disposable membership").get(0);
        self.membership_ids.push(id);
        id
    }

    fn app_token(&mut self, account_id: i64, membership_id: i64) -> String {
        let raw = format!("mt_people_contract_{}", unique_suffix());
        let id: i64 = self.db.query_one(
            "INSERT INTO api_app_tokens (account_id, household_membership_id, token_digest, permissions_version, name, expires_at, last_used_at, created_at, updated_at) VALUES ($1, $2, encode(digest($3, 'sha256'), 'hex'), (SELECT permissions_version FROM household_memberships WHERE id = $2), 'People contract', now() + interval '1 day', now(), now(), now()) RETURNING id",
            &[&account_id, &membership_id, &raw],
        ).expect("disposable app token").get(0);
        self.app_token_ids.push(id);
        raw
    }

    fn member_app_token(&mut self, membership_id: i64) -> String {
        let account_id: i64 = self
            .db
            .query_one(
                "SELECT account_id FROM household_memberships WHERE id = $1",
                &[&membership_id],
            )
            .expect("membership account")
            .get(0);
        self.app_token(account_id, membership_id)
    }
}

impl Drop for DisposableCredentials {
    fn drop(&mut self) {
        for id in &self.app_token_ids {
            self.db
                .execute("DELETE FROM api_app_tokens WHERE id = $1", &[id])
                .expect("remove disposable app token");
        }
        for id in &self.membership_ids {
            self.db
                .execute(
                    "DELETE FROM api_change_events WHERE household_membership_id = $1",
                    &[id],
                )
                .expect("remove disposable membership sync events");
            self.db
                .execute(
                    "DELETE FROM person_access_grants WHERE household_membership_id = $1",
                    &[id],
                )
                .expect("remove disposable membership grants");
            self.db
                .execute("DELETE FROM household_memberships WHERE id = $1", &[id])
                .expect("remove disposable membership");
        }
    }
}

fn owner_app_token(
    credentials: &mut DisposableCredentials,
    fixture: &medtracker_contract_tests::Fixture,
) -> String {
    credentials.app_token(fixture.account_id, fixture.owner_membership_id)
}

fn api_error(response: reqwest::blocking::Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert!(body["error"]["code"].is_string());
    body
}

fn create_payload(name: String, email: Value, date_of_birth: &str) -> Value {
    json!({"person": {
        "name": name,
        "email": email,
        "date_of_birth": date_of_birth,
        "person_type": "adult",
        "has_capacity": true
    }})
}

fn create_side_effects(
    db: &mut postgres::Client,
    household_id: i64,
    membership_id: i64,
) -> (i64, i64, i64, i32, i64, i64) {
    let people: i64 = db
        .query_one(
            "SELECT count(*) FROM people WHERE household_id = $1",
            &[&household_id],
        )
        .expect("people count")
        .get(0);
    let locations: i64 = db
        .query_one(
            "SELECT count(*) FROM location_memberships WHERE household_id = $1",
            &[&household_id],
        )
        .expect("location membership count")
        .get(0);
    let grants: i64 = db
        .query_one(
            "SELECT count(*) FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2",
            &[&household_id, &membership_id],
        )
        .expect("grant count")
        .get(0);
    let permissions_version: i32 = db
        .query_one(
            "SELECT permissions_version FROM household_memberships WHERE id = $1",
            &[&membership_id],
        )
        .expect("membership permissions version")
        .get(0);
    let changes: i64 = db
        .query_one(
            "SELECT count(*) FROM api_change_events WHERE household_id = $1 AND record_type = 'Person'",
            &[&household_id],
        )
        .expect("person change event count")
        .get(0);
    let versions: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE household_id = $1 AND item_type = 'Person'",
            &[&household_id],
        )
        .expect("person version count")
        .get(0);
    (
        people,
        locations,
        grants,
        permissions_version,
        changes,
        versions,
    )
}

fn assert_person(person: &Value) {
    let object = person.as_object().expect("person object");
    for key in [
        "id",
        "portable_id",
        "updated_at",
        "name",
        "email",
        "date_of_birth",
        "person_type",
        "has_capacity",
        "age",
        "location_ids",
        "location_portable_ids",
        "notification_preference_id",
        "notification_preference_portable_id",
    ] {
        assert!(object.contains_key(key), "missing person field {key}");
    }
    assert_eq!(object.len(), 13, "unexpected person fields");
    assert!(person["id"].as_u64().is_some_and(|id| id > 0));
    let portable_id = person["portable_id"].as_str().expect("person portable ID");
    assert!(
        portable_id.len() == 36
            && [8, 13, 18, 23]
                .iter()
                .all(|index| portable_id.as_bytes()[*index] == b'-')
    );
    assert!(portable_id
        .bytes()
        .enumerate()
        .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit()));
    let updated_at = person["updated_at"].as_str().expect("person updated_at");
    assert!(
        updated_at.len() >= 20 && updated_at.as_bytes()[10] == b'T' && updated_at.contains(':')
    );
    assert!(person["name"]
        .as_str()
        .is_some_and(|name| !name.trim().is_empty()));
    assert!(person["email"].is_null() || person["email"].is_string());
    let birth = person["date_of_birth"]
        .as_str()
        .expect("person date of birth");
    assert!(birth.len() == 10 && birth.as_bytes()[4] == b'-' && birth.as_bytes()[7] == b'-');
    assert!(matches!(
        person["person_type"].as_str(),
        Some("adult" | "minor" | "dependent_adult")
    ));
    assert!(person["has_capacity"].is_boolean());
    assert!(person["age"].is_null() || person["age"].as_u64().is_some());
    assert!(person["location_ids"]
        .as_array()
        .is_some_and(|ids| ids.iter().all(|id| id.as_u64().is_some())));
    assert!(person["location_portable_ids"]
        .as_array()
        .is_some_and(|ids| ids.iter().all(Value::is_string)));
    assert_eq!(
        person["location_ids"].as_array().unwrap().len(),
        person["location_portable_ids"].as_array().unwrap().len()
    );
    assert!(
        person["notification_preference_id"].is_null()
            || person["notification_preference_id"].as_u64().is_some()
    );
    assert!(
        person["notification_preference_portable_id"].is_null()
            || person["notification_preference_portable_id"].is_string()
    );
    assert_eq!(
        person["notification_preference_id"].is_null(),
        person["notification_preference_portable_id"].is_null()
    );
}

fn person_effects(db: &mut postgres::Client, person_id: i64) -> (String, Option<String>, i64, i64) {
    let person = db
        .query_one(
            "SELECT name, email FROM people WHERE id = $1",
            &[&person_id],
        )
        .expect("person row");
    let changes: i64 = db.query_one(
        "SELECT count(*) FROM api_change_events WHERE record_type = 'Person' AND record_id = $1",
        &[&person_id],
    ).expect("person sync count").get(0);
    let versions: i64 = db
        .query_one(
            "SELECT count(*) FROM versions WHERE item_type = 'Person' AND item_id = $1",
            &[&person_id],
        )
        .expect("person version count")
        .get(0);
    (person.get(0), person.get(1), changes, versions)
}

fn assert_created_version(
    db: &mut postgres::Client,
    request_id: &str,
    item_type: &str,
    item_id: i64,
) {
    assert!(db.query_one(
        "SELECT EXISTS (SELECT 1 FROM versions WHERE request_id = $1 AND item_type = $2 AND item_id = $3 AND event = 'create')",
        &[&request_id, &item_type, &item_id],
    ).expect("created audit version").get::<_, bool>(0), "missing {item_type} create version");
}

fn assert_grant_audit(
    db: &mut postgres::Client,
    request_id: &str,
    grant_id: i64,
    membership_id: i64,
) {
    assert!(db.query_one(
        "SELECT EXISTS (SELECT 1 FROM security_audit_events WHERE request_id = $1 AND event_type = 'household_access.person_grant_changed' AND actor_membership_id = $2 AND metadata ->> 'target_grant_id' = $3 AND metadata ->> 'outcome' = 'success')",
        &[&request_id, &membership_id, &grant_id.to_string()],
    ).expect("grant security audit").get::<_, bool>(0));
}

fn assert_update_events(
    db: &mut postgres::Client,
    request_id: &str,
    person_id: i64,
    membership_id: i64,
) {
    assert!(db.query_one(
        "SELECT EXISTS (SELECT 1 FROM api_change_events WHERE request_id = $1 AND record_type = 'Person' AND record_id = $2 AND action = 'update' AND household_membership_id = $3)",
        &[&request_id, &person_id, &membership_id],
    ).expect("person update sync event").get::<_, bool>(0));
    assert!(db.query_one(
        "SELECT EXISTS (SELECT 1 FROM versions WHERE request_id = $1 AND item_type = 'Person' AND item_id = $2 AND event = 'update' AND actor_membership_id = $3)",
        &[&request_id, &person_id, &membership_id],
    ).expect("person update audit version").get::<_, bool>(0));
}

fn update_request(
    target: &Target,
    method: &str,
    path: &str,
    token: &str,
    body: &Value,
) -> reqwest::blocking::Response {
    match method {
        "PATCH" => target.patch_json(path, token, body),
        "PUT" => target.put_json(path, token, body),
        _ => panic!("unsupported update method"),
    }
}

#[test]
fn current_profile_is_the_authenticated_person_even_for_another_active_household() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    assert_ne!(fixture.household_id, fixture.profile_household_id);
    let membership_id = credentials.membership(fixture.profile_account_id, fixture.household_id);
    let app_token = credentials.app_token(fixture.profile_account_id, membership_id);
    let path = household_path(fixture.household_id, "me");
    let response = target.get(&path, Some(&app_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers().get("etag").is_none());
    let body: Value = response.json().expect("profile JSON");
    let me = &body["data"];
    assert!(me["id"].as_u64().is_some_and(|id| id > 0));
    assert_eq!(me["email_address"], fixture.profile_email);
    assert!(me["active"].is_boolean());
    assert_eq!(me["person"]["id"], fixture.profile_person_id);
    assert_eq!(me["account"]["id"], fixture.profile_account_id);
    assert_eq!(me["membership_role"], "member");
    let locations = credentials.db.query(
        "SELECT lm.location_id, location.portable_id FROM location_memberships lm JOIN locations location ON location.id = lm.location_id WHERE lm.person_id = $1 ORDER BY lm.id",
        &[&fixture.profile_person_id],
    ).expect("linked person's home locations");
    let location_ids: Vec<i64> = locations.iter().map(|row| row.get(0)).collect();
    let location_portable_ids: Vec<String> = locations.iter().map(|row| row.get(1)).collect();
    assert_eq!(me["person"]["location_ids"], json!(location_ids));
    assert_eq!(
        me["person"]["location_portable_ids"],
        json!(location_portable_ids)
    );
    assert_person(&me["person"]);
    assert_eq!(me.as_object().unwrap().len(), 6);
    assert_eq!(me["account"].as_object().unwrap().len(), 3);
    assert_eq!(me["account"]["email"], fixture.profile_email);
    assert!(matches!(
        me["account"]["status"].as_str(),
        Some("verified" | "unverified" | "closed")
    ));
    assert_eq!(body.as_object().unwrap().len(), 1);
}

#[test]
fn create_person_returns_a_strict_person_resource_and_etag() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let app_token = owner_app_token(&mut credentials, &fixture);
    let version_before: i32 = credentials
        .db
        .query_one(
            "SELECT permissions_version FROM household_memberships WHERE id = $1",
            &[&fixture.owner_membership_id],
        )
        .expect("owner permissions version")
        .get(0);
    let suffix = unique_suffix();
    let email = format!("  Contract-Person-{suffix}@Example.Test  ");
    let path = household_path(fixture.household_id, "people");
    let response = target.post_json_authorized(
        &path,
        &app_token,
        &create_payload(
            format!("Contract person {suffix}"),
            json!(email.clone()),
            "1980-01-01",
        ),
    );
    assert_eq!(response.status().as_u16(), 201);
    assert!(!response.headers()["etag"].to_str().unwrap().is_empty());
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let body: Value = response.json().expect("person JSON");
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert_person(&body["data"]);
    assert_eq!(body["data"]["email"], email.trim().to_ascii_lowercase());
    let person_id = body["data"]["id"].as_i64().expect("person ID");
    let home_id: i64 = credentials
        .db
        .query_one(
            "SELECT id FROM locations WHERE household_id = $1 AND name = 'Home'",
            &[&fixture.household_id],
        )
        .expect("Home location")
        .get(0);
    let assigned = credentials
        .db
        .query_one(
            "SELECT id, location_id FROM location_memberships WHERE household_id = $1 AND person_id = $2",
            &[&fixture.household_id, &person_id],
        )
        .expect("created person's location membership");
    let location_membership_id: i64 = assigned.get(0);
    assert_eq!(assigned.get::<_, i64>(1), home_id);
    assert_created_version(
        &mut credentials.db,
        &request_id,
        "LocationMembership",
        location_membership_id,
    );
    let access = credentials
        .db
        .query_one(
            "SELECT id, access_level FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND revoked_at IS NULL",
            &[&fixture.household_id, &fixture.owner_membership_id, &person_id],
        )
        .expect("created person's owner manage grant");
    assert_eq!(access.get::<_, String>(1), "manage");
    assert_grant_audit(
        &mut credentials.db,
        &request_id,
        access.get(0),
        fixture.owner_membership_id,
    );
    let version_after: i32 = credentials
        .db
        .query_one(
            "SELECT permissions_version FROM household_memberships WHERE id = $1",
            &[&fixture.owner_membership_id],
        )
        .expect("updated owner permissions version")
        .get(0);
    assert!(version_after > version_before);
    assert!(credentials
        .db
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM api_change_events WHERE record_type = 'Person' AND record_id = $1 AND request_id = $2 AND action = 'create')",
            &[&person_id, &request_id],
        )
        .expect("person sync event")
        .get::<_, bool>(0));
    assert_created_version(&mut credentials.db, &request_id, "Person", person_id);
    let after_create = create_side_effects(
        &mut credentials.db,
        fixture.household_id,
        fixture.owner_membership_id,
    );
    let current_token = credentials.member_app_token(fixture.owner_membership_id);
    api_error(
        target.post_json_authorized(
            &path,
            &current_token,
            &create_payload(
                format!("Duplicate person email {suffix}"),
                json!(email.trim()),
                "1980-01-01",
            ),
        ),
        422,
    );
    assert_eq!(
        create_side_effects(
            &mut credentials.db,
            fixture.household_id,
            fixture.owner_membership_id,
        ),
        after_create
    );
}

#[test]
fn first_person_in_a_household_creates_audited_home_and_location_membership() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let suffix = unique_suffix();
    let household_id: i64 = credentials.db.query_one(
        "INSERT INTO households (created_by_account_id, name, slug, timezone, created_at, updated_at) VALUES ($1, $2, $3, 'Europe/London', now(), now()) RETURNING id",
        &[&fixture.profile_account_id, &format!("People first home {suffix}"), &format!("people-first-home-{suffix}")],
    ).expect("disposable household").get(0);
    let membership_id = credentials.membership(fixture.profile_account_id, household_id);
    credentials
        .db
        .execute(
            "UPDATE household_memberships SET role = 'owner' WHERE id = $1",
            &[&membership_id],
        )
        .expect("disposable owner membership");
    let token = credentials.member_app_token(membership_id);
    let response = target.post_json_authorized(
        &household_path(household_id, "people"),
        &token,
        &create_payload(format!("First person {suffix}"), Value::Null, "1980-01-01"),
    );
    assert_eq!(response.status().as_u16(), 201);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let data: Value = response.json().expect("first person JSON");
    assert_person(&data["data"]);
    let person_id = data["data"]["id"].as_i64().expect("person ID");
    let location = credentials
        .db
        .query_one(
            "SELECT id, portable_id FROM locations WHERE household_id = $1 AND name = 'Home'",
            &[&household_id],
        )
        .expect("new Home location");
    let location_id: i64 = location.get(0);
    let portable_id: String = location.get(1);
    assert_eq!(data["data"]["location_ids"], json!([location_id]));
    assert_eq!(data["data"]["location_portable_ids"], json!([portable_id]));
    assert_created_version(&mut credentials.db, &request_id, "Location", location_id);
    assert!(credentials.db.query_one(
        "SELECT EXISTS (SELECT 1 FROM api_change_events WHERE request_id = $1 AND record_type = 'Location' AND record_id = $2 AND action = 'create')",
        &[&request_id, &location_id],
    ).expect("Home sync event").get::<_, bool>(0));
    let location_membership_id: i64 = credentials.db.query_one(
        "SELECT id FROM location_memberships WHERE household_id = $1 AND person_id = $2 AND location_id = $3",
        &[&household_id, &person_id, &location_id],
    ).expect("first location membership").get(0);
    assert_created_version(
        &mut credentials.db,
        &request_id,
        "LocationMembership",
        location_membership_id,
    );
}

#[test]
fn create_person_requires_an_active_manage_grant_and_reports_wrapper_errors() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let app_token = owner_app_token(&mut credentials, &fixture);
    let path = household_path(fixture.household_id, "people");
    let baseline = create_side_effects(
        &mut credentials.db,
        fixture.household_id,
        fixture.owner_membership_id,
    );
    api_error(
        target.post_json_authorized(&path, &app_token, &json!({})),
        400,
    );
    api_error(
        target.post_json_authorized(&path, &app_token, &json!({"person": 42})),
        400,
    );
    api_error(
        target.post_json_authorized(&path, &app_token, &json!({"person": {}})),
        422,
    );
    api_error(
        target.post_json_authorized(
            &path,
            &app_token,
            &json!({"person": {
                "name": "Adult without capacity",
                "date_of_birth": "1980-01-01",
                "person_type": "adult",
                "has_capacity": false
            }}),
        ),
        422,
    );
    api_error(
        target.post_json_authorized(
            &path,
            &fixture.view_access_token,
            &create_payload("View-only person".to_owned(), Value::Null, "1980-01-01"),
        ),
        403,
    );
    assert_eq!(
        create_side_effects(
            &mut credentials.db,
            fixture.household_id,
            fixture.owner_membership_id,
        ),
        baseline
    );
}

#[test]
fn a_member_with_a_manage_grant_can_create_a_person() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let app_token = credentials.member_app_token(fixture.manager_membership_id);
    let suffix = unique_suffix();
    let path = household_path(fixture.household_id, "people");
    let response = target.post_json_authorized(
        &path,
        &app_token,
        &create_payload(
            format!("Managed member person {suffix}"),
            Value::Null,
            "1980-01-01",
        ),
    );
    assert_eq!(response.status().as_u16(), 201);
    assert_person(&response.json::<Value>().expect("person JSON")["data"]);
}

#[test]
fn creating_a_minor_sets_no_capacity_and_delegates_carer_access() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let app_token = owner_app_token(&mut credentials, &fixture);
    let suffix = unique_suffix();
    let response = target.post_json_authorized(
        &household_path(fixture.household_id, "people"),
        &app_token,
        &json!({"person": {
            "name": format!("Contract minor {suffix}"),
            "date_of_birth": "2018-01-01",
            "person_type": "minor",
            "has_capacity": true
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let body: Value = response.json().expect("minor JSON");
    let minor = &body["data"];
    assert_person(minor);
    assert_eq!(minor["person_type"], "minor");
    assert_eq!(minor["has_capacity"], false);
    let minor_id = minor["id"].as_i64().expect("minor ID");
    assert!(credentials
        .db
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM carer_relationships WHERE household_id = $1 AND carer_id = $2 AND patient_id = $3 AND active)",
            &[&fixture.household_id, &fixture.user_person_id, &minor_id],
        )
        .expect("active carer relationship")
        .get::<_, bool>(0));
    let relationship_id: i64 = credentials.db.query_one(
        "SELECT id FROM carer_relationships WHERE household_id = $1 AND carer_id = $2 AND patient_id = $3 AND active",
        &[&fixture.household_id, &fixture.user_person_id, &minor_id],
    ).expect("active carer relationship").get(0);
    assert_created_version(
        &mut credentials.db,
        &request_id,
        "CarerRelationship",
        relationship_id,
    );
    let grant_id: i64 = credentials.db.query_one(
        "SELECT id FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND access_level = 'manage' AND relationship_type = 'family_member' AND carer_relationship_id = $4 AND revoked_at IS NULL",
        &[&fixture.household_id, &fixture.owner_membership_id, &minor_id, &relationship_id],
    ).expect("linked carer manage grant").get(0);
    assert_grant_audit(
        &mut credentials.db,
        &request_id,
        grant_id,
        fixture.owner_membership_id,
    );
    let current_token = credentials.member_app_token(fixture.owner_membership_id);
    for method in ["PATCH", "PUT"] {
        let path = household_path(fixture.household_id, &format!("people/{minor_id}"));
        let response = update_request(
            &target,
            method,
            &path,
            &current_token,
            &json!({"person": {"has_capacity": true}}),
        );
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            response.json::<Value>().expect("forced capacity JSON")["data"]["has_capacity"],
            false
        );
    }
}

#[test]
fn minor_creation_without_a_carer_is_rejected_without_partial_rows() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let membership_id = credentials.membership(fixture.profile_account_id, fixture.household_id);
    let no_grant_token = credentials.member_app_token(membership_id);
    api_error(
        target.post_json_authorized(
            &household_path(fixture.household_id, "people"),
            &no_grant_token,
            &create_payload("No manage grant".to_owned(), Value::Null, "1980-01-01"),
        ),
        403,
    );
    credentials
        .db
        .execute(
            "INSERT INTO person_access_grants (access_level, household_id, household_membership_id, person_id, relationship_type, granted_by_membership_id, created_at, updated_at) VALUES ('manage', $1, $2, $3, 'family_member', $4, now(), now())",
            &[&fixture.household_id, &membership_id, &fixture.managed_person_id, &fixture.owner_membership_id],
        )
        .expect("manage grant for no-carer member");
    let app_token = credentials.member_app_token(membership_id);
    let baseline = create_side_effects(&mut credentials.db, fixture.household_id, membership_id);
    api_error(
        target.post_json_authorized(
            &household_path(fixture.household_id, "people"),
            &app_token,
            &json!({"person": {
                "name": format!("No-carer minor {}", unique_suffix()),
                "date_of_birth": "2018-01-01",
                "person_type": "minor",
                "has_capacity": false
            }}),
        ),
        422,
    );
    assert_eq!(
        create_side_effects(&mut credentials.db, fixture.household_id, membership_id,),
        baseline
    );
}

#[test]
fn dependent_creation_restores_a_missing_self_grant_and_links_its_carer_grant() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let membership_id = credentials.membership(fixture.profile_account_id, fixture.household_id);
    let carer_id: i64 = credentials.db.query_one(
        "INSERT INTO people (household_id, portable_id, name, date_of_birth, person_type, has_capacity, created_at, updated_at) VALUES ($1, gen_random_uuid()::text, $2, DATE '1980-01-01', 0, true, now(), now()) RETURNING id",
        &[&fixture.household_id, &format!("Disposable carer {}", unique_suffix())],
    ).expect("disposable carer person").get(0);
    credentials
        .db
        .execute(
            "UPDATE household_memberships SET person_id = $1 WHERE id = $2",
            &[&carer_id, &membership_id],
        )
        .expect("link disposable member's carer person");
    credentials.db.execute(
        "INSERT INTO person_access_grants (access_level, household_id, household_membership_id, person_id, relationship_type, granted_by_membership_id, created_at, updated_at) VALUES ('manage', $1, $2, $3, 'family_member', $4, now(), now())",
        &[&fixture.household_id, &membership_id, &fixture.managed_person_id, &fixture.owner_membership_id],
    ).expect("independent manage grant");
    let token = credentials.member_app_token(membership_id);
    let version_before: i32 = credentials
        .db
        .query_one(
            "SELECT permissions_version FROM household_memberships WHERE id = $1",
            &[&membership_id],
        )
        .expect("membership version")
        .get(0);
    let response = target.post_json_authorized(
        &household_path(fixture.household_id, "people"),
        &token,
        &json!({"person": {
            "name": format!("Dependent with missing self grant {}", unique_suffix()),
            "date_of_birth": "1980-01-01",
            "person_type": "dependent_adult",
            "has_capacity": true
        }}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let data: Value = response.json().expect("dependent JSON");
    assert_person(&data["data"]);
    assert_eq!(data["data"]["has_capacity"], false);
    let dependent_id = data["data"]["id"].as_i64().expect("dependent ID");
    let row = credentials.db.query_one(
        "SELECT cr.id, pag.carer_relationship_id, pag.id FROM carer_relationships cr JOIN person_access_grants pag ON pag.person_id = cr.patient_id AND pag.household_membership_id = $1 WHERE cr.carer_id = $2 AND cr.patient_id = $3 AND cr.active AND pag.revoked_at IS NULL",
        &[&membership_id, &carer_id, &dependent_id],
    ).expect("linked carer relationship and grant");
    assert_eq!(row.get::<_, i64>(0), row.get::<_, i64>(1));
    assert_created_version(
        &mut credentials.db,
        &request_id,
        "CarerRelationship",
        row.get(0),
    );
    assert_grant_audit(&mut credentials.db, &request_id, row.get(2), membership_id);
    let self_grant = credentials.db.query_one(
        "SELECT id, access_level FROM person_access_grants WHERE household_membership_id = $1 AND person_id = $2 AND relationship_type = 'self' AND revoked_at IS NULL AND expires_at IS NULL",
            &[&membership_id, &carer_id],
    ).expect("restored self grant");
    assert_eq!(self_grant.get::<_, String>(1), "manage");
    assert_grant_audit(
        &mut credentials.db,
        &request_id,
        self_grant.get(0),
        membership_id,
    );
    let version_after: i32 = credentials
        .db
        .query_one(
            "SELECT permissions_version FROM household_memberships WHERE id = $1",
            &[&membership_id],
        )
        .expect("updated membership version")
        .get(0);
    assert_eq!(version_after, version_before + 2);
}

#[test]
fn patch_person_merges_attributes_without_if_match() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let app_token = owner_app_token(&mut credentials, &fixture);
    let path = household_path(
        fixture.household_id,
        &format!("people/{}", fixture.managed_person_id),
    );
    let before_response = target.get(&path, Some(&app_token));
    assert_eq!(before_response.status().as_u16(), 200);
    let before: Value = before_response.json().expect("existing person JSON");
    let before = before["data"].clone();
    let effects_before = person_effects(&mut credentials.db, fixture.managed_person_id);
    let response = target.patch_json(
        &path,
        &app_token,
        &json!({"person": {"name": "OpenAPI PATCH person"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(!response.headers()["etag"].to_str().unwrap().is_empty());
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let body: Value = response.json().expect("person JSON");
    assert_person(&body["data"]);
    assert_eq!(body["data"]["name"], "OpenAPI PATCH person");
    for field in ["email", "date_of_birth", "person_type", "has_capacity"] {
        assert_eq!(body["data"][field], before[field]);
    }
    let effects_after = person_effects(&mut credentials.db, fixture.managed_person_id);
    assert_eq!(effects_after.0, "OpenAPI PATCH person");
    assert_update_events(
        &mut credentials.db,
        &request_id,
        fixture.managed_person_id,
        fixture.owner_membership_id,
    );
    assert_eq!(
        (effects_after.2, effects_after.3),
        (effects_before.2 + 1, effects_before.3 + 1)
    );
    api_error(
        target.patch_json(&path, &app_token, &json!({"person": {"name": ""}})),
        422,
    );
    api_error(
        target.patch_json(
            &household_path(
                fixture.household_id,
                &format!("people/{}", fixture.hidden_person_id),
            ),
            &app_token,
            &json!({"person": {"name": "Hidden edit"}}),
        ),
        404,
    );
    api_error(
        target.patch_json(
            &path,
            &fixture.view_access_token,
            &json!({"person": {"name": "View edit"}}),
        ),
        403,
    );
}

#[test]
fn put_person_merges_attributes_by_portable_id_without_if_match() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let app_token = owner_app_token(&mut credentials, &fixture);
    let path = household_path(
        fixture.household_id,
        &format!("people/{}", fixture.managed_person_portable_id),
    );
    let before_response = target.get(&path, Some(&app_token));
    assert_eq!(before_response.status().as_u16(), 200);
    let before: Value = before_response.json().expect("existing person JSON");
    let before = before["data"].clone();
    let effects_before = person_effects(&mut credentials.db, fixture.managed_person_id);
    let response = target.put_json(
        &path,
        &app_token,
        &json!({"person": {"name": "OpenAPI PUT person"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert!(!response.headers()["etag"].to_str().unwrap().is_empty());
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let body: Value = response.json().expect("person JSON");
    assert_person(&body["data"]);
    assert_eq!(body["data"]["name"], "OpenAPI PUT person");
    for field in ["email", "date_of_birth", "person_type", "has_capacity"] {
        assert_eq!(body["data"][field], before[field]);
    }
    let effects_after = person_effects(&mut credentials.db, fixture.managed_person_id);
    assert_eq!(effects_after.0, "OpenAPI PUT person");
    assert_update_events(
        &mut credentials.db,
        &request_id,
        fixture.managed_person_id,
        fixture.owner_membership_id,
    );
    assert_eq!(
        (effects_after.2, effects_after.3),
        (effects_before.2 + 1, effects_before.3 + 1)
    );
}

#[test]
fn patch_and_put_normalize_email_and_merge_date_changes() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let token = owner_app_token(&mut credentials, &fixture);
    let path = household_path(
        fixture.household_id,
        &format!("people/{}", fixture.managed_person_id),
    );
    let email = format!("  People-Update-{}@Example.Test  ", unique_suffix());
    let response = target.patch_json(
        &path,
        &token,
        &json!({"person": {
            "email": email, "date_of_birth": "1970-01-01"
        }}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let data: Value = response.json().expect("normalized person JSON");
    assert_person(&data["data"]);
    assert_eq!(data["data"]["email"], email.trim().to_ascii_lowercase());
    assert_eq!(data["data"]["date_of_birth"], "1970-01-01");
    let response = target.put_json(&path, &token, &json!({"person": {"email": null}}));
    assert_eq!(response.status().as_u16(), 200);
    let data: Value = response.json().expect("cleared email JSON");
    assert_person(&data["data"]);
    assert!(data["data"]["email"].is_null());
    assert_eq!(data["data"]["date_of_birth"], "1970-01-01");
}

#[test]
fn unchanged_patch_and_put_do_not_emit_person_change_events() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let token = owner_app_token(&mut credentials, &fixture);
    let path = household_path(
        fixture.household_id,
        &format!("people/{}", fixture.managed_person_id),
    );
    let before = person_effects(&mut credentials.db, fixture.managed_person_id);
    for method in ["PATCH", "PUT"] {
        let response = update_request(
            &target,
            method,
            &path,
            &token,
            &json!({"person": {"name": before.0}}),
        );
        assert_eq!(response.status().as_u16(), 200);
        assert_person(&response.json::<Value>().expect("person JSON")["data"]);
        assert_eq!(
            person_effects(&mut credentials.db, fixture.managed_person_id),
            before
        );
    }
}

#[test]
fn all_four_people_operations_apply_shared_auth_and_household_boundaries() {
    let fixture = fixture();
    let target = Target::from_env();
    let person = json!({"person": {"name": "Denied write"}});
    let create_body = create_payload("Denied create".to_owned(), Value::Null, "1980-01-01");
    let me = household_path(fixture.household_id, "me");
    let create = household_path(fixture.household_id, "people");
    let update = household_path(
        fixture.household_id,
        &format!("people/{}", fixture.managed_person_id),
    );
    api_error(target.get(&me, None), 401);
    api_error(target.get(&me, Some("invalid-token")), 401);
    api_error(
        target.post_json_authorized(&create, "invalid-token", &create_body),
        401,
    );
    for method in ["PATCH", "PUT"] {
        api_error(
            update_request(&target, method, &update, "invalid-token", &person),
            401,
        );
    }
    let wrong_me = household_path(fixture.foreign_household_id, "me");
    let wrong_create = household_path(fixture.foreign_household_id, "people");
    let wrong_update = household_path(
        fixture.foreign_household_id,
        &format!("people/{}", fixture.managed_person_id),
    );
    api_error(target.get(&wrong_me, Some(&fixture.access_token)), 403);
    api_error(
        target.post_json_authorized(&wrong_create, &fixture.access_token, &create_body),
        403,
    );
    for method in ["PATCH", "PUT"] {
        api_error(
            update_request(
                &target,
                method,
                &wrong_update,
                &fixture.access_token,
                &person,
            ),
            403,
        );
    }
    let unknown_me = household_path(i64::MAX, "me");
    let unknown_create = household_path(i64::MAX, "people");
    let unknown_update = household_path(i64::MAX, &format!("people/{}", fixture.managed_person_id));
    api_error(target.get(&unknown_me, Some(&fixture.access_token)), 404);
    api_error(
        target.post_json_authorized(&unknown_create, &fixture.access_token, &create_body),
        404,
    );
    for method in ["PATCH", "PUT"] {
        api_error(
            update_request(
                &target,
                method,
                &unknown_update,
                &fixture.access_token,
                &person,
            ),
            404,
        );
    }
}

#[test]
fn create_rejects_invalid_attributes_without_person_side_effects() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let token = owner_app_token(&mut credentials, &fixture);
    let path = household_path(fixture.household_id, "people");
    let invalid = [
        json!({"person": {"name": " ", "date_of_birth": "1980-01-01"}}),
        json!({"person": {"name": "Invalid date", "date_of_birth": "1980-13-01"}}),
        json!({"person": {"name": "Invalid email", "date_of_birth": "1980-01-01", "email": "not an email"}}),
        json!({"person": {"name": "Wrong type", "date_of_birth": "1980-01-01", "person_type": "minor"}}),
        json!({"person": {"name": "Wrong age", "date_of_birth": "2018-01-01", "person_type": "dependent_adult"}}),
        json!({"person": {"name": "Unknown field", "date_of_birth": "1980-01-01", "unknown": true}}),
        json!({"person": {"name": "Bad capacity", "date_of_birth": "1980-01-01", "has_capacity": "true"}}),
        json!({"person": {"name": "Valid", "date_of_birth": "1980-01-01"}, "extra": true}),
    ];
    let before = create_side_effects(
        &mut credentials.db,
        fixture.household_id,
        fixture.owner_membership_id,
    );
    for body in invalid {
        api_error(target.post_json_authorized(&path, &token, &body), 422);
        assert_eq!(
            create_side_effects(
                &mut credentials.db,
                fixture.household_id,
                fixture.owner_membership_id
            ),
            before
        );
    }
}

#[test]
fn patch_and_put_reject_invalid_updates_without_changing_person_or_events() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut credentials = DisposableCredentials::new();
    let token = owner_app_token(&mut credentials, &fixture);
    let invalid = [
        json!({"person": {"name": ""}}),
        json!({"person": {"email": "not an email"}}),
        json!({"person": {"date_of_birth": "not-a-date"}}),
        json!({"person": {"person_type": "unknown"}}),
        json!({"person": {"person_type": "minor"}}),
        json!({"person": {"has_capacity": false}}),
        json!({"person": {"name": "Would partially update", "email": "invalid email"}}),
        json!({"person": {"unexpected": 1}}),
    ];
    for (method, id) in [
        ("PATCH", fixture.managed_person_id.to_string()),
        ("PUT", fixture.managed_person_portable_id.clone()),
    ] {
        let path = household_path(fixture.household_id, &format!("people/{id}"));
        let before = person_effects(&mut credentials.db, fixture.managed_person_id);
        for body in &invalid {
            api_error(update_request(&target, method, &path, &token, body), 422);
            assert_eq!(
                person_effects(&mut credentials.db, fixture.managed_person_id),
                before
            );
        }
        api_error(
            update_request(&target, method, &path, &token, &json!({})),
            400,
        );
        api_error(
            update_request(&target, method, &path, &token, &json!({"person": 1})),
            400,
        );
        assert_eq!(
            person_effects(&mut credentials.db, fixture.managed_person_id),
            before
        );
    }
}

#[test]
fn simultaneous_duplicate_email_creates_yield_one_person_and_one_validation_error() {
    let fixture = fixture();
    let mut credentials = DisposableCredentials::new();
    let tokens = [
        owner_app_token(&mut credentials, &fixture),
        credentials.member_app_token(fixture.manager_membership_id),
    ];
    let email = format!("people-race-{}@example.test", unique_suffix());
    let household_id = fixture.household_id;
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = tokens
        .into_iter()
        .enumerate()
        .map(|(index, token)| {
            let email = email.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let target = Target::from_env();
                barrier.wait();
                target
                    .post_json_authorized(
                        &household_path(household_id, "people"),
                        &token,
                        &create_payload(
                            format!("Racing person {index}"),
                            json!(email),
                            "1980-01-01",
                        ),
                    )
                    .status()
                    .as_u16()
            })
        })
        .collect();
    barrier.wait();
    let mut statuses: Vec<u16> = handles
        .into_iter()
        .map(|handle| handle.join().expect("create worker"))
        .collect();
    statuses.sort_unstable();
    assert_eq!(statuses, [201, 422]);
    let count: i64 = credentials
        .db
        .query_one(
            "SELECT count(*) FROM people WHERE household_id = $1 AND email = $2",
            &[&household_id, &email],
        )
        .expect("email rows")
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn z_rate_limit_wires_all_four_people_operations_without_writes() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("nonloopback API URL");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let started = Instant::now();
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{base}/api/v1/capabilities"))
            .send()
            .unwrap();
        if response.status().as_u16() == 429 {
            assert_eq!(response.headers()["ratelimit-limit"], "300");
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            assert!(response.headers().get("retry-after").is_some());
            let body: Value = response.json().unwrap();
            assert_eq!(body["error"]["code"], "rate_limited");
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < Duration::from_secs(60));
    let collection = format!("{base}{}", household_path(fixture.household_id, "people"));
    let detail = format!("{collection}/{}", fixture.managed_person_id);
    for request in [
        client.get(format!(
            "{base}{}",
            household_path(fixture.household_id, "me")
        )),
        client
            .post(&collection)
            .json(&json!({"person": {"name": "Rate denied"}})),
        client
            .patch(&detail)
            .json(&json!({"person": {"name": "Rate denied"}})),
        client
            .put(&detail)
            .json(&json!({"person": {"name": "Rate denied"}})),
    ] {
        let response = request.bearer_auth(&fixture.access_token).send().unwrap();
        assert_eq!(response.status().as_u16(), 429);
    }
}
