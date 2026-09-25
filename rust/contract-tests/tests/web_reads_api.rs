use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;
use std::env;

struct ReadSource<'a> {
    path: &'a str,
    visible_id: i64,
    visible_portable_id: &'a str,
    hidden_id: i64,
    foreign_id: i64,
}

struct TemporaryGrant {
    db: postgres::Client,
    id: i64,
}

impl TemporaryGrant {
    fn create(fixture: &Fixture) -> Self {
        let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL");
        let mut db = postgres::Client::connect(&url, postgres::NoTls).expect("contract database");
        let id = db
            .query_one(
                "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, granted_by_membership_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, $4, 'view', 'carer', NOW(), NOW()) RETURNING id",
                &[
                    &fixture.household_id,
                    &fixture.view_membership_id,
                    &fixture.hidden_person_id,
                    &fixture.owner_membership_id,
                ],
            )
            .expect("temporary viewer grant")
            .get(0);
        Self { db, id }
    }

    fn revoke(&mut self) {
        let updated = self
            .db
            .execute(
                "UPDATE person_access_grants SET revoked_at = NOW(), updated_at = NOW() WHERE id = $1",
                &[&self.id],
            )
            .expect("revoke temporary viewer grant");
        assert_eq!(updated, 1);
    }
}

impl Drop for TemporaryGrant {
    fn drop(&mut self) {
        let _ = self.db.execute(
            "DELETE FROM person_access_grants WHERE id = $1",
            &[&self.id],
        );
    }
}

fn read_sources(fixture: &Fixture) -> [ReadSource<'_>; 3] {
    [
        ReadSource {
            path: "people",
            visible_id: fixture.managed_person_id,
            visible_portable_id: &fixture.managed_person_portable_id,
            hidden_id: fixture.hidden_person_id,
            foreign_id: fixture.foreign_person_id,
        },
        ReadSource {
            path: "schedules",
            visible_id: fixture.managed_schedule_id,
            visible_portable_id: &fixture.managed_schedule_portable_id,
            hidden_id: fixture.hidden_schedule_id,
            foreign_id: fixture.foreign_schedule_id,
        },
        ReadSource {
            path: "person_medications",
            visible_id: fixture.managed_assignment_id,
            visible_portable_id: &fixture.managed_assignment_portable_id,
            hidden_id: fixture.hidden_assignment_id,
            foreign_id: fixture.foreign_assignment_id,
        },
    ]
}

fn path(household_id: i64, collection: &str) -> String {
    format!("/api/v1/households/{household_id}/{collection}")
}

fn body(response: Response, expected_status: u16) -> Value {
    let actual_status = response.status().as_u16();
    let text = response.text().expect("HTTP response body");
    assert_eq!(actual_status, expected_status, "{text}");
    serde_json::from_str(&text).expect("JSON response body")
}

fn rows(response: &Value) -> &[Value] {
    response["data"].as_array().expect("collection data")
}

#[test]
fn shared_collections_expose_visible_picker_rows_and_view_permissions() {
    let fixture = fixture();
    let target = Target::from_env();

    for source in read_sources(&fixture) {
        let collection = body(
            target.get(
                &path(fixture.household_id, source.path),
                Some(&fixture.view_access_token),
            ),
            200,
        );
        assert!(collection["meta"].is_object(), "{} meta", source.path);
        let visible = rows(&collection)
            .iter()
            .find(|row| row["id"] == source.visible_id)
            .expect("granted person or source must be readable");
        assert_eq!(visible["portable_id"], source.visible_portable_id);
        assert!(rows(&collection)
            .iter()
            .all(|row| { row["id"] != source.hidden_id && row["id"] != source.foreign_id }));

        if source.path == "people" {
            assert_eq!(visible["name"], fixture.web_managed_person_name);
            assert_eq!(visible["person_type"], "adult");
            assert_eq!(visible["has_capacity"], true);
            assert!(visible["date_of_birth"].is_string());
            assert!(visible.get("email").is_some());
            assert!(visible["location_ids"].is_array());
            assert!(visible["location_portable_ids"].is_array());
        } else {
            assert_eq!(visible["person_id"], fixture.managed_person_id);
            assert_eq!(
                visible["person_portable_id"],
                fixture.managed_person_portable_id
            );
            assert_eq!(visible["medication_id"], fixture.managed_medication_id);
            assert_eq!(
                visible["medication_portable_id"],
                fixture.managed_medication_portable_id
            );
            assert_eq!(visible["dose_amount"], "1.0");
            assert_eq!(visible["dose_unit"], "ml");
            assert_eq!(visible["active"], true);
            assert_eq!(visible["paused"], false);
            assert_eq!(visible["can_manage"], false);

            let paused_portable_id = if source.path == "schedules" {
                &fixture.fhir_stopped_schedule_portable_id
            } else {
                &fixture.fhir_stopped_assignment_portable_id
            };
            let paused = rows(&collection)
                .iter()
                .find(|row| row["portable_id"] == *paused_portable_id)
                .expect("visible paused source remains readable");
            assert_eq!(paused["active"], false);
            assert_eq!(paused["paused"], true);
            assert_eq!(paused["can_manage"], false);
        }
    }
}

#[test]
fn shared_collections_filter_before_stable_pagination() {
    let fixture = fixture();
    let target = Target::from_env();

    for source in read_sources(&fixture) {
        let base = path(fixture.household_id, source.path);
        let first = body(
            target.get(
                &format!("{base}?page=1&per_page=1"),
                Some(&fixture.access_token),
            ),
            200,
        );
        let second = body(
            target.get(
                &format!("{base}?page=2&per_page=1"),
                Some(&fixture.access_token),
            ),
            200,
        );
        assert_eq!(first["meta"]["page"], 1);
        assert_eq!(second["meta"]["page"], 2);
        assert_eq!(first["meta"]["per_page"], 1);
        assert_eq!(second["meta"]["per_page"], 1);
        assert_eq!(first["meta"]["total_count"], second["meta"]["total_count"]);
        assert!(first["meta"]["total_count"].as_u64().unwrap() >= 2);
        let first_id = rows(&first)[0]["id"].as_i64().unwrap();
        let second_id = rows(&second)[0]["id"].as_i64().unwrap();
        assert!(
            first_id < second_id,
            "{} must sort by ascending id",
            source.path
        );

        let future = body(
            target.get(
                &format!("{base}?updated_since=2100-01-01T00%3A00%3A00Z"),
                Some(&fixture.access_token),
            ),
            200,
        );
        assert!(rows(&future).is_empty());
        assert_eq!(future["meta"]["total_count"], 0);

        let invalid = body(
            target.get(
                &format!("{base}?updated_since=not-a-timestamp"),
                Some(&fixture.access_token),
            ),
            422,
        );
        assert_eq!(invalid["error"]["code"], "unprocessable_content");
    }

    let clamped = body(
        target.get(
            &format!("{}?per_page=999", path(fixture.household_id, "people")),
            Some(&fixture.access_token),
        ),
        200,
    );
    assert_eq!(clamped["meta"]["per_page"], 100);
}

#[test]
fn shared_details_match_collection_rows_and_hide_ungranted_records() {
    let fixture = fixture();
    let target = Target::from_env();

    for source in read_sources(&fixture) {
        let base = path(fixture.household_id, source.path);
        let collection = body(target.get(&base, Some(&fixture.view_access_token)), 200);
        let row = rows(&collection)
            .iter()
            .find(|row| row["id"] == source.visible_id)
            .expect("visible collection row");
        let detail = target.get(
            &format!("{base}/{}", source.visible_id),
            Some(&fixture.view_access_token),
        );
        assert_eq!(detail.status().as_u16(), 200);
        assert!(detail.headers().contains_key("etag"));
        assert_eq!(body(detail, 200)["data"], *row);

        if source.path != "people" {
            let portable = body(
                target.get(
                    &format!("{base}/{}", source.visible_portable_id),
                    Some(&fixture.view_access_token),
                ),
                200,
            );
            assert_eq!(portable["data"], *row);
        }

        for id in [source.hidden_id, source.foreign_id] {
            let denied = body(
                target.get(&format!("{base}/{id}"), Some(&fixture.view_access_token)),
                404,
            );
            assert_eq!(denied["error"]["code"], "not_found");
        }
        let foreign_household = body(
            target.get(
                &path(fixture.foreign_household_id, source.path),
                Some(&fixture.access_token),
            ),
            403,
        );
        assert_eq!(foreign_household["error"]["code"], "forbidden");
    }
}

#[test]
fn revoking_a_person_grant_removes_that_person_and_linked_sources_on_next_read() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut grant = TemporaryGrant::create(&fixture);

    for source in read_sources(&fixture) {
        let base = path(fixture.household_id, source.path);
        let with_grant = body(target.get(&base, Some(&fixture.view_access_token)), 200);
        assert!(rows(&with_grant)
            .iter()
            .any(|row| row["id"] == source.hidden_id));
        assert_eq!(
            target
                .get(
                    &format!("{base}/{}", source.hidden_id),
                    Some(&fixture.view_access_token)
                )
                .status()
                .as_u16(),
            200
        );
    }

    grant.revoke();

    for source in read_sources(&fixture) {
        let base = path(fixture.household_id, source.path);
        let revoked = body(target.get(&base, Some(&fixture.view_access_token)), 200);
        assert!(rows(&revoked)
            .iter()
            .all(|row| row["id"] != source.hidden_id));
        assert_eq!(
            target
                .get(
                    &format!("{base}/{}", source.hidden_id),
                    Some(&fixture.view_access_token)
                )
                .status()
                .as_u16(),
            404
        );
    }
}
