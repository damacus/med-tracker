use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;

const BASE: &str = "/api/fhir/R4";

fn read(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("application/fhir+json"));
    response.json().expect("FHIR JSON")
}

fn resource(target: &Target, fixture: &Fixture, kind: &str, id: &str) -> Value {
    let path = format!("{BASE}/{kind}/{id}");
    let body = read(target.get(&path, Some(&fixture.access_token)), 200);
    assert_eq!(body["resourceType"], kind);
    assert_eq!(body["id"], id);
    body
}

fn entries(body: &Value, kind: &str) -> Vec<Value> {
    assert_eq!(body["resourceType"], "Bundle");
    assert_eq!(body["type"], "searchset");
    let links = body["link"].as_array().expect("bundle links");
    assert!(links.iter().any(|link| link["relation"] == "self"));
    body["entry"]
        .as_array()
        .expect("bundle entries")
        .iter()
        .map(|entry| {
            assert_eq!(entry["search"]["mode"], "match");
            assert_eq!(entry["resource"]["resourceType"], kind);
            entry["resource"].clone()
        })
        .collect()
}

fn search(target: &Target, fixture: &Fixture, kind: &str, query: &str) -> Vec<Value> {
    entries(
        &read(
            target.get(
                &format!("{BASE}/{kind}?{query}"),
                Some(&fixture.access_token),
            ),
            200,
        ),
        kind,
    )
}

fn has_id(rows: &[Value], id: &str) -> bool {
    rows.iter().any(|row| row["id"] == id)
}

fn outcome(response: Response, status: u16, code: &str, private_text: &str) {
    let body = read(response, status);
    assert_eq!(body["resourceType"], "OperationOutcome");
    assert_eq!(body["issue"][0]["code"], code);
    assert!(!body.to_string().contains(private_text));
}

#[test]
fn smart_discovery_is_public_and_metadata_describes_all_five_readable_resources() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(&format!("{BASE}/.well-known/smart-configuration"), None);
    assert_eq!(response.status().as_u16(), 200);
    let config: Value = response.json().unwrap();
    for endpoint in [
        "authorization_endpoint",
        "token_endpoint",
        "revocation_endpoint",
    ] {
        assert!(config[endpoint].as_str().unwrap().starts_with("http"));
    }
    assert!(config["code_challenge_methods_supported"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "S256"));
    outcome(
        target.get(&format!("{BASE}/metadata"), None),
        401,
        "security",
        &fixture.foreign_person_name,
    );
    let metadata = read(
        target.get(&format!("{BASE}/metadata"), Some(&fixture.access_token)),
        200,
    );
    assert_eq!(metadata["resourceType"], "CapabilityStatement");
    assert_eq!(metadata["fhirVersion"], "4.0.1");
    assert_eq!(
        metadata["rest"][0]["security"]["service"][0]["coding"][0]["code"],
        "SMART-on-FHIR"
    );
    let resources = metadata["rest"][0]["resource"].as_array().unwrap();
    for kind in [
        "Patient",
        "Medication",
        "MedicationRequest",
        "MedicationStatement",
        "MedicationAdministration",
    ] {
        let entry = resources
            .iter()
            .find(|entry| entry["type"] == kind)
            .expect("resource capability");
        let interactions = entry["interaction"].as_array().unwrap();
        assert!(interactions.iter().any(|item| item["code"] == "read"));
        assert!(interactions
            .iter()
            .any(|item| item["code"] == "search-type"));
    }
}

#[test]
fn all_five_collections_and_reads_preserve_resource_identity_and_relationships() {
    let target = Target::from_env();
    let fixture = fixture();
    let resources = [
        ("Patient", fixture.managed_person_portable_id.as_str()),
        (
            "Medication",
            fixture.managed_medication_portable_id.as_str(),
        ),
        (
            "MedicationRequest",
            fixture.managed_schedule_portable_id.as_str(),
        ),
        (
            "MedicationStatement",
            fixture.managed_assignment_portable_id.as_str(),
        ),
        (
            "MedicationAdministration",
            fixture.managed_take_portable_id.as_str(),
        ),
    ];
    for (kind, id) in resources {
        let rows = search(&target, &fixture, kind, &format!("_id={id}"));
        assert_eq!(rows.len(), 1, "{kind} _id search");
        assert_eq!(rows[0]["id"], id);
        resource(&target, &fixture, kind, id);
    }
    let patient = format!("Patient/{}", fixture.managed_person_portable_id);
    let medication = format!("Medication/{}", fixture.managed_medication_portable_id);
    for (kind, id) in [
        ("MedicationRequest", &fixture.managed_schedule_portable_id),
        (
            "MedicationStatement",
            &fixture.managed_assignment_portable_id,
        ),
    ] {
        let body = resource(&target, &fixture, kind, id);
        assert_eq!(body["subject"]["reference"], patient);
        assert_eq!(body["medicationReference"]["reference"], medication);
    }
    let administration = resource(
        &target,
        &fixture,
        "MedicationAdministration",
        &fixture.managed_take_portable_id,
    );
    assert_eq!(administration["subject"]["reference"], patient);
    assert_eq!(
        administration["medicationReference"]["reference"],
        format!("Medication/{}", fixture.historical_medication_portable_id)
    );
    assert_eq!(administration["status"], "completed");
}

#[test]
fn search_filters_and_pagination_are_observable_without_order_assumptions() {
    let target = Target::from_env();
    let fixture = fixture();
    let person = &fixture.managed_person_portable_id;
    let medication = &fixture.managed_medication_portable_id;
    let cases = [
        (
            "Patient",
            "name=Contract%20managed".to_string(),
            person.as_str(),
        ),
        (
            "MedicationRequest",
            format!("patient=Patient/{person}&medication=Medication/{medication}&status=active"),
            fixture.managed_schedule_portable_id.as_str(),
        ),
        (
            "MedicationStatement",
            format!("subject=Patient/{person}&medication=Medication/{medication}&status=active"),
            fixture.managed_assignment_portable_id.as_str(),
        ),
        (
            "MedicationAdministration",
            format!("patient=Patient/{person}&status=completed"),
            fixture.managed_take_portable_id.as_str(),
        ),
    ];
    for (kind, query, id) in cases {
        let rows = search(&target, &fixture, kind, &query);
        if kind == "MedicationAdministration" {
            assert!(has_id(&rows, id));
            assert!(!has_id(&rows, &fixture.hidden_take_portable_id));
            assert!(rows
                .iter()
                .all(|row| row["subject"]["reference"] == format!("Patient/{person}")));
        } else {
            assert_eq!(
                rows.len(),
                1,
                "{kind} filter must select one fixture resource"
            );
            assert_eq!(rows[0]["id"], id);
        }
    }
    let medication_resource = resource(&target, &fixture, "Medication", medication);
    assert_eq!(medication_resource["form"]["text"], "Analgesic");
    let medication_by_form = search(&target, &fixture, "Medication", "form=Analgesic");
    assert_eq!(medication_by_form.len(), 1);
    assert_eq!(medication_by_form[0]["id"], medication.as_str());
    let medication_by_code = search(&target, &fixture, "Medication", "code=123456");
    assert_eq!(medication_by_code.len(), 1);
    assert_eq!(medication_by_code[0]["id"], medication.as_str());
    let administration = resource(
        &target,
        &fixture,
        "MedicationAdministration",
        &fixture.managed_take_portable_id,
    );
    let taken_on = &administration["effectiveDateTime"].as_str().unwrap()[..10];
    let dated = search(
        &target,
        &fixture,
        "MedicationAdministration",
        &format!("patient=Patient/{person}&date={taken_on}"),
    );
    assert_eq!(dated.len(), 1);
    assert_eq!(dated[0]["id"], fixture.managed_take_portable_id);
    let page = read(
        target.get(
            &format!("{BASE}/Patient?_count=1"),
            Some(&fixture.access_token),
        ),
        200,
    );
    assert_eq!(entries(&page, "Patient").len(), 1);
    assert!(page["total"].as_u64().unwrap() > 1);
    let next = page["link"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["relation"] == "next")
        .unwrap();
    let next_url = url::Url::parse(next["url"].as_str().unwrap()).unwrap();
    let target_origin = url::Url::parse(&std::env::var("CONTRACT_BASE_URL").unwrap()).unwrap();
    assert_eq!(next_url.origin(), target_origin.origin());
    assert_eq!(next_url.path(), format!("{BASE}/Patient"));
    let next_query: Vec<_> = next_url.query_pairs().collect();
    assert_eq!(next_query.len(), 2);
    assert!(next_query
        .iter()
        .any(|(name, value)| name == "_count" && value == "1"));
    assert!(next_query
        .iter()
        .any(|(name, value)| name == "page" && value == "2"));
    let next_path = format!("{}?{}", next_url.path(), next_url.query().unwrap());
    let second = read(target.get(&next_path, Some(&fixture.access_token)), 200);
    assert_eq!(entries(&second, "Patient").len(), 1);
    assert_ne!(
        page["entry"][0]["resource"]["id"],
        second["entry"][0]["resource"]["id"]
    );
}

#[test]
fn invalid_search_and_format_return_fhir_errors() {
    let target = Target::from_env();
    let fixture = fixture();
    for (kind, query) in [
        ("Patient", "family=Smith"),
        ("Patient", "birthdate=not-a-date"),
        ("MedicationRequest", "status=draft"),
        ("MedicationStatement", "status=entered-in-error"),
        ("MedicationAdministration", "status=in-progress"),
        ("MedicationAdministration", "date=not-a-date"),
    ] {
        outcome(
            target.get(
                &format!("{BASE}/{kind}?{query}"),
                Some(&fixture.access_token),
            ),
            422,
            "invalid",
            &fixture.foreign_person_name,
        );
    }
    outcome(
        target.get(
            &format!("{BASE}/Patient?_format=xml"),
            Some(&fixture.access_token),
        ),
        406,
        "not-supported",
        &fixture.foreign_person_name,
    );
}

#[test]
fn authentication_scope_and_household_boundaries_hide_private_resources() {
    let target = Target::from_env();
    let fixture = fixture();
    for (kind, id) in [
        ("Patient", &fixture.foreign_person_portable_id),
        ("Medication", &fixture.foreign_medication_portable_id),
        ("MedicationRequest", &fixture.foreign_schedule_portable_id),
        (
            "MedicationStatement",
            &fixture.foreign_assignment_portable_id,
        ),
        (
            "MedicationAdministration",
            &fixture.foreign_take_portable_id,
        ),
    ] {
        let path = format!("{BASE}/{kind}/{id}");
        outcome(
            target.get(&path, None),
            401,
            "security",
            &fixture.foreign_person_name,
        );
        outcome(
            target.get(&path, Some(&fixture.access_token)),
            404,
            "not-found",
            &fixture.foreign_person_name,
        );
        let rows = search(&target, &fixture, kind, &format!("_id={id}"));
        assert!(rows.is_empty(), "{kind} foreign resource leaked");
    }
    for kind in [
        "Patient",
        "Medication",
        "MedicationRequest",
        "MedicationStatement",
        "MedicationAdministration",
    ] {
        outcome(
            target.get(&format!("{BASE}/{kind}"), None),
            401,
            "security",
            &fixture.foreign_person_name,
        );
        let body = read(
            target.get(
                &format!("{BASE}/{kind}?_count=100"),
                Some(&fixture.access_token),
            ),
            200,
        );
        let serialized = body.to_string();
        assert!(!serialized.contains(&fixture.foreign_person_name));
        assert!(!serialized.contains(&fixture.foreign_medication_name));
        let rows = entries(&body, kind);
        assert!(!has_id(&rows, &fixture.foreign_person_portable_id));
        assert!(!has_id(&rows, &fixture.foreign_medication_portable_id));
        assert!(!has_id(&rows, &fixture.foreign_schedule_portable_id));
        assert!(!has_id(&rows, &fixture.foreign_assignment_portable_id));
        assert!(!has_id(&rows, &fixture.foreign_take_portable_id));
    }
    let patient_path = format!("{BASE}/Patient/{}", fixture.managed_person_portable_id);
    assert_eq!(
        read(
            target.get(&patient_path, Some(&fixture.fhir_patient_scope_token)),
            200
        )["id"],
        fixture.managed_person_portable_id
    );
    outcome(
        target.get(
            &format!("{BASE}/Patient/{}", fixture.hidden_person_portable_id),
            Some(&fixture.fhir_patient_scope_token),
        ),
        404,
        "not-found",
        &fixture.foreign_person_name,
    );
    let scoped = read(
        target.get(
            &format!("{BASE}/Patient"),
            Some(&fixture.fhir_patient_scope_token),
        ),
        200,
    );
    let scoped_rows = entries(&scoped, "Patient");
    assert!(has_id(&scoped_rows, &fixture.managed_person_portable_id));
    assert!(!has_id(&scoped_rows, &fixture.hidden_person_portable_id));
    for (kind, id) in [
        ("Medication", &fixture.managed_medication_portable_id),
        ("MedicationRequest", &fixture.managed_schedule_portable_id),
        (
            "MedicationStatement",
            &fixture.managed_assignment_portable_id,
        ),
        (
            "MedicationAdministration",
            &fixture.managed_take_portable_id,
        ),
    ] {
        for path in [format!("{BASE}/{kind}"), format!("{BASE}/{kind}/{id}")] {
            outcome(
                target.get(&path, Some(&fixture.fhir_patient_scope_token)),
                403,
                "security",
                &fixture.foreign_person_name,
            );
        }
    }
    outcome(
        target.get(&patient_path, Some(&fixture.fhir_revoked_scope_token)),
        401,
        "security",
        &fixture.foreign_person_name,
    );
}
