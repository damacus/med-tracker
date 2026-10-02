use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use serde_json::{Value, json};
use std::env;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn state(fixture: &Fixture) -> Value {
    let mut db = Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap();
    let text: String = db.query_one("SELECT json_build_object('medications', (SELECT COALESCE(json_agg(m ORDER BY m.id), '[]'::json) FROM medications m WHERE household_id=$1), 'dosages', (SELECT COALESCE(json_agg(d ORDER BY d.id), '[]'::json) FROM dosages d WHERE household_id=$1), 'assignments', (SELECT COALESCE(json_agg(p ORDER BY p.id), '[]'::json) FROM person_medications p WHERE household_id=$1), 'schedules', (SELECT COALESCE(json_agg(s ORDER BY s.id), '[]'::json) FROM schedules s WHERE household_id=$1), 'takes', (SELECT COALESCE(json_agg(t ORDER BY t.id), '[]'::json) FROM medication_takes t WHERE household_id=$1), 'versions', (SELECT COALESCE(json_agg(v ORDER BY v.id), '[]'::json) FROM versions v WHERE household_id=$1), 'sync', (SELECT COALESCE(json_agg(e ORDER BY e.id), '[]'::json) FROM api_change_events e WHERE household_id=$1))::text", &[&fixture.household_id]).unwrap().get(0);
    serde_json::from_str(&text).unwrap()
}

fn attempts(source_id: &str) -> Vec<(Value, u16)> {
    let base = json!({"source_type": "person_medication", "source_id": source_id,
        "taken_at": OffsetDateTime::now_utc().replace_nanosecond(0).unwrap().format(&Rfc3339).unwrap()});
    let mut attempts = Vec::new();
    for field in ["source_type", "source_id"] {
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove(field);
        attempts.push((missing, 422));
        for value in [Value::Null, json!(""), json!(" "), json!(42), json!(false), json!([])] {
            let mut attributes = base.clone();
            attributes[field] = value;
            attempts.push((attributes, 422));
        }
    }
    for identifier in ["invalid-reference", "-1", "0", "1.5"] {
        let mut attributes = base.clone();
        attributes["source_type"] = json!("unsupported_source");
        attributes["source_id"] = json!(identifier);
        attempts.push((attributes, 422));
    }
    let mut unknown_field = base.clone();
    unknown_field["source_type"] = json!("unsupported_source");
    unknown_field["unexpected"] = json!("retained priority");
    attempts.push((unknown_field, 422));
    let mut unknown = base;
    unknown["source_type"] = json!("unsupported_source");
    attempts.push((unknown, 404));
    attempts
}

fn check(sync: bool) {
    let fixture = fixture();
    let target = Target::from_env();
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let response = target.get(&format!("{api}/person_medications/{}", fixture.dose_write_assignment_id), Some(&fixture.access_token));
    let status = response.status().as_u16();
    let source = response.json::<Value>().unwrap();
    assert_eq!(status, 200, "authorised seed source: {source}");
    let source_id = source["data"]["portable_id"].as_str().unwrap();
    let before = state(&fixture);
    for (index, (mut attributes, expected)) in attempts(source_id).into_iter().enumerate() {
        let unknown_field = attributes.get("unexpected").is_some();
        attributes["client_uuid"] = json!(format!("88888888-0000-4000-8000-{:012}", 100 + index + if sync { 100 } else { 0 }));
        attributes["taken_from_medication_id"] = json!(fixture.dose_write_medication_id);
        let (path, payload) = if sync {
            (format!("{api}/sync/batches"), json!({"batch": {"operations": [{"resource_type": "medication_take", "action": "create", "attributes": attributes}]}}))
        } else {
            (format!("{api}/medication_takes"), json!({"medication_take": attributes}))
        };
        let response = if sync {
            let key = format!("77777777-0000-4000-8000-{index:012}");
            target.post_json_with_key(&path, &fixture.access_token, &key, &payload)
        } else {
            target.post_json_authorized(&path, &fixture.access_token, &payload)
        };
        let status = response.status().as_u16();
        let body = response.json::<Value>().unwrap();
        assert_eq!(state(&fixture), before, "rejected source cannot write clinical stock/history, sync={sync}, attempt={index}");
        assert_eq!(status, expected, "source classification sync={sync}, attempt={index}: {body}");
        assert_eq!(body["error"]["code"], if expected == 404 { "not_found" } else { "unprocessable_content" });
        if unknown_field {
            assert_eq!(body["error"]["message"], "unknown medication_take field", "unknown fields retain priority over unsupported source classification");
        }
        assert!(body.get("data").is_none());
    }
}

#[test]
fn direct_unsupported_source_is_not_found_and_malformed_references_remain_invalid_without_writes() {
    check(false);
}

#[test]
fn sync_unsupported_source_is_not_found_and_malformed_references_remain_invalid_without_writes() {
    check(true);
}
