use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("JSON object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn assert_person(person: &Value) {
    assert_eq!(
        keys(person),
        [
            "age",
            "date_of_birth",
            "email",
            "has_capacity",
            "id",
            "location_ids",
            "location_portable_ids",
            "name",
            "notification_preference_id",
            "notification_preference_portable_id",
            "person_type",
            "portable_id",
            "updated_at",
        ]
    );
    assert!(person["id"].as_i64().is_some_and(|id| id > 0));
    assert!(person["portable_id"]
        .as_str()
        .is_some_and(|id| id.len() == 36));
    assert!(person["name"].as_str().is_some_and(|name| !name.is_empty()));
    assert!(person["has_capacity"].is_boolean());
    assert!(person["location_ids"].is_array());
    assert!(person["location_portable_ids"].is_array());
    OffsetDateTime::parse(person["updated_at"].as_str().expect("updated_at"), &Rfc3339)
        .expect("RFC3339 updated_at");
}

struct AdditionalHousehold {
    db: postgres::Client,
    household_id: i64,
    membership_id: i64,
}

impl AdditionalHousehold {
    fn new(account_id: i64) -> Self {
        let database_url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("database URL");
        let mut db =
            postgres::Client::connect(&database_url, postgres::NoTls).expect("contract database");
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let name = format!("Contract additional household {suffix}");
        let slug = format!("contract-additional-{suffix}");
        let household_id: i64 = db
            .query_one(
                "INSERT INTO households (created_by_account_id, name, slug, timezone, created_at, updated_at) VALUES ($1, $2, $3, 'UTC', now(), now()) RETURNING id",
                &[&account_id, &name, &slug],
            )
            .expect("additional household")
            .get(0);
        let membership_id: i64 = db
            .query_one(
                "INSERT INTO household_memberships (account_id, household_id, role, status, joined_at, created_at, updated_at) VALUES ($1, $2, 'owner', 'active', now(), now(), now()) RETURNING id",
                &[&account_id, &household_id],
            )
            .expect("additional household membership")
            .get(0);
        Self {
            db,
            household_id,
            membership_id,
        }
    }
}

impl Drop for AdditionalHousehold {
    fn drop(&mut self) {
        self.db
            .execute(
                "DELETE FROM household_memberships WHERE id = $1",
                &[&self.membership_id],
            )
            .expect("remove additional household membership");
        self.db
            .execute(
                "DELETE FROM households WHERE id = $1",
                &[&self.household_id],
            )
            .expect("remove additional household");
    }
}

#[test]
fn capabilities_and_household_reads_match_openapi_shapes() {
    let fixture = fixture();
    let target = Target::from_env();

    let capabilities = target.get("/api/v1/capabilities", None);
    assert_eq!(capabilities.status().as_u16(), 200);
    assert_eq!(capabilities.headers()["cache-control"], "no-store");
    let capabilities: Value = capabilities.json().expect("capabilities JSON");
    assert_eq!(keys(&capabilities), ["data"]);
    let data = &capabilities["data"];
    assert_eq!(
        keys(data),
        [
            "administration",
            "api_version",
            "authentication",
            "backups",
            "client_tools",
            "dose_outcomes",
            "fhir",
            "format",
            "invitations",
            "location_management",
            "medication_pause_periods",
            "medication_reviews",
            "portable_formats",
            "profile",
            "reports",
            "stock_removals",
            "sync",
        ]
    );
    assert_eq!(
        keys(&data["authentication"]),
        ["hosted_mobile", "methods", "mobile_oauth"]
    );
    assert_eq!(
        keys(&data["administration"]),
        [
            "app_tokens",
            "audit_logs",
            "fresh_mfa_required",
            "household",
            "invitations",
            "person_access_grants"
        ]
    );
    assert_eq!(
        keys(&data["dose_outcomes"]),
        [
            "actions",
            "max_read_days",
            "replacement_requires_version",
            "source_types"
        ]
    );
    assert_eq!(
        keys(&data["location_management"]),
        [
            "actions",
            "memberships_online_only",
            "person_memberships",
            "version_required"
        ]
    );
    assert_eq!(data["format"], "medtracker.api.capabilities.v1");
    assert_eq!(data["api_version"], "v1");

    let households = target.get("/api/v1/auth/households", Some(&fixture.access_token));
    assert_eq!(households.status().as_u16(), 200);
    let households: Value = households.json().expect("households JSON");
    assert_eq!(keys(&households), ["account_id", "data"]);
    assert_eq!(households["account_id"], fixture.account_id);
    let memberships = households["data"].as_array().expect("household array");
    assert_eq!(memberships.len(), 2);
    for membership in memberships {
        assert_eq!(
            keys(membership),
            ["id", "membership_id", "name", "role", "slug"]
        );
    }
    assert!(memberships.iter().any(|membership| {
        membership["id"] == fixture.household_id
            && membership["membership_id"] == fixture.owner_membership_id
            && membership["role"] == "owner"
    }));
    assert!(memberships.iter().any(|membership| {
        membership["id"] == fixture.medication_read_household_id
            && membership["role"] == "member"
    }));
    assert!(!households.to_string().contains(&fixture.foreign_email));
}

#[test]
fn household_list_scopes_app_tokens_and_keeps_mobile_account_scope() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut db = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let manager_account_id: i64 = db
        .query_one(
            "SELECT account_id FROM household_memberships WHERE id = $1",
            &[&fixture.manager_membership_id],
        )
        .expect("manager account")
        .get(0);
    let additional_manager = AdditionalHousehold::new(manager_account_id);
    let additional_owner = AdditionalHousehold::new(fixture.account_id);

    let app_list = target.get("/api/v1/auth/households", Some(&fixture.manager_app_token));
    assert_eq!(app_list.status().as_u16(), 200);
    assert_eq!(app_list.headers()["cache-control"], "no-store");
    let app_list: Value = app_list.json().expect("app household list JSON");
    assert_eq!(keys(&app_list), ["account_id", "data"]);
    assert_eq!(app_list["account_id"], manager_account_id);
    let app_rows = app_list["data"].as_array().expect("app household rows");
    assert_eq!(app_rows.len(), 1);
    assert_eq!(
        keys(&app_rows[0]),
        ["id", "membership_id", "name", "role", "slug"]
    );
    assert_eq!(app_rows[0]["id"], fixture.household_id);
    assert_eq!(app_rows[0]["membership_id"], fixture.manager_membership_id);
    assert_eq!(app_rows[0]["role"], "administrator");
    assert!(!app_list
        .to_string()
        .contains("Contract additional household"));
    assert_ne!(
        additional_manager.household_id,
        additional_owner.household_id
    );

    let integration = target.get(
        "/api/v1/auth/households",
        Some(&fixture.fhir_patient_scope_token),
    );
    assert_eq!(integration.status().as_u16(), 401);

    let mobile_list = target.get(
        "/api/v1/auth/households",
        Some(&fixture.medication_mobile_oauth_token),
    );
    assert_eq!(mobile_list.status().as_u16(), 200);
    let mobile_list: Value = mobile_list.json().expect("mobile household list JSON");
    assert_eq!(keys(&mobile_list), ["account_id", "data"]);
    assert_eq!(mobile_list["account_id"], fixture.account_id);
    let mobile_rows = mobile_list["data"]
        .as_array()
        .expect("mobile household rows");
    assert_eq!(mobile_rows.len(), 3);
    assert!(mobile_rows.iter().any(|row| row["id"] == fixture.household_id));
    assert!(mobile_rows
        .iter()
        .any(|row| row["id"] == fixture.medication_read_household_id));
    assert!(mobile_rows.iter().any(|row| {
        row["id"] == additional_owner.household_id
            && row["membership_id"] == additional_owner.membership_id
    }));
}

#[test]
fn people_collection_detail_and_query_validation_match_openapi() {
    let fixture = fixture();
    let target = Target::from_env();
    let base = format!("/api/v1/households/{}/people", fixture.household_id);

    let collection = target.get(&base, Some(&fixture.access_token));
    assert_eq!(collection.status().as_u16(), 200);
    let collection: Value = collection.json().expect("people collection JSON");
    assert_eq!(keys(&collection), ["data", "meta"]);
    assert_eq!(
        keys(&collection["meta"]),
        ["page", "per_page", "total_count"]
    );
    assert_eq!(collection["meta"]["page"], 1);
    assert_eq!(collection["meta"]["per_page"], 20);
    for person in collection["data"].as_array().expect("people") {
        assert_person(person);
    }
    assert!(collection["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|person| person["id"] == fixture.managed_person_id));

    for query in [
        "page=0",
        "per_page=101",
        "page=abc",
        "updated_since=not-a-date",
    ] {
        let response = target.get(&format!("{base}?{query}"), Some(&fixture.access_token));
        assert_eq!(response.status().as_u16(), 422, "{query}");
        let body: Value = response.json().expect("query validation JSON");
        assert_eq!(keys(&body), ["error"]);
        assert_eq!(body["error"]["code"], "unprocessable_content", "{query}");
    }

    let detail_path = format!("{base}/{}", fixture.managed_person_id);
    let detail = target.get(&detail_path, Some(&fixture.access_token));
    assert_eq!(detail.status().as_u16(), 200);
    assert!(detail.headers().get("etag").is_some());
    let detail: Value = detail.json().expect("person detail JSON");
    assert_eq!(keys(&detail), ["data"]);
    assert_person(&detail["data"]);

    let view_detail = target.get(&detail_path, Some(&fixture.view_access_token));
    assert_eq!(view_detail.status().as_u16(), 200);
    let hidden_detail = target.get(
        &format!("{base}/{}", fixture.hidden_person_id),
        Some(&fixture.access_token),
    );
    assert_eq!(hidden_detail.status().as_u16(), 404);
    let foreign_household = target.get(
        &format!("/api/v1/households/{}/people", fixture.foreign_household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(foreign_household.status().as_u16(), 403);
}
