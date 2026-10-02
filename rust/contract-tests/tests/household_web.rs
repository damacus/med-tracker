use medtracker_contract_tests::{Target, fixture};
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};

static LOGIN_CLIENT: AtomicUsize = AtomicUsize::new(1);

#[test]
fn dose_sources_expose_record_permissions_and_scoped_stock_for_the_browser() {
    let fixture = fixture();
    let owner = Target::from_env();
    sign_in(&owner);
    for (resource, source_id, stock_id) in [
        (
            "person_medications",
            fixture.dose_write_assignment_id,
            fixture.dose_write_medication_id,
        ),
        (
            "schedules",
            fixture.dose_write_schedule_id,
            fixture.dose_write_schedule_medication_id,
        ),
    ] {
        let path = format!(
            "/api/v1/households/{}/{resource}/{source_id}",
            fixture.household_id
        );
        let response = owner.get(&path, None);
        assert_eq!(response.status().as_u16(), 200);
        let body: Value = response.json().expect("owner dose source");
        assert_eq!(body["data"]["can_record"], true);
        let stock = body["data"]["eligible_stock_medication_ids"]
            .as_array()
            .expect("scoped stock IDs");
        assert!(stock.contains(&Value::from(stock_id)));
        assert!(!stock.contains(&Value::from(fixture.dose_write_foreign_medication_id)));
        let viewer = Target::from_env().get(&path, Some(&fixture.view_access_token));
        assert_eq!(viewer.status().as_u16(), 200);
        let body: Value = viewer.json().expect("viewer dose source");
        assert_eq!(body["data"]["can_record"], false);
    }
}

fn csrf(html: &str) -> String {
    let document = Html::parse_document(html);
    for (selector, attribute) in [
        ("input[name='authenticity_token']", "value"),
        ("meta[name='csrf-token']", "content"),
    ] {
        let selector = Selector::parse(selector).expect("CSRF selector");
        if let Some(value) = document
            .select(&selector)
            .next()
            .and_then(|input| input.value().attr(attribute))
        {
            return value.to_owned();
        }
    }
    panic!("native household form must render a CSRF token")
}

fn sign_in(target: &Target) -> String {
    let fixture = fixture();
    sign_in_as(target, &fixture.primary_email, &fixture.household_slug)
}

fn sign_in_as(target: &Target, email: &str, slug: &str) -> String {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let token = csrf(&login.text().expect("login HTML"));
    let client_ip = format!("198.18.24.{}", LOGIN_CLIENT.fetch_add(1, Ordering::Relaxed));
    let response = target.post_html_form_from_client(
        "/login",
        &client_ip,
        &[
            ("email".into(), email.into()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    let inventory = target.get_html(&format!("/households/{slug}/medications"));
    assert_eq!(inventory.status().as_u16(), 200);
    csrf(&inventory.text().expect("inventory HTML"))
}

#[test]
fn view_only_member_cannot_open_create_forms_or_edit_a_visible_person() {
    let fixture = fixture();
    let member = Target::from_env();
    let me = member.get(
        &format!("/api/v1/households/{}/me", fixture.household_id),
        Some(&fixture.view_access_token),
    );
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().expect("viewer account identity");
    let email = me["data"]["email_address"].as_str().expect("viewer email");
    sign_in_as(&member, email, &fixture.household_slug);
    for resource in ["people", "locations", "medications"] {
        let response = member.get_html(&format!(
            "/households/{}/{resource}/new",
            fixture.household_slug
        ));
        assert_eq!(
            response.status().as_u16(),
            403,
            "view-only member must not receive {resource} create form"
        );
    }
    let visible = member.get_html(&format!(
        "/households/{}/people/{}",
        fixture.household_slug, fixture.managed_person_id
    ));
    assert_eq!(visible.status().as_u16(), 200);
    let html = visible.text().expect("viewable person HTML");
    assert!(!html.contains(&format!(
        "href=\"/households/{}/people/{}/edit\"",
        fixture.household_slug, fixture.managed_person_id
    )));
    let edit = member.get_html(&format!(
        "/households/{}/people/{}/edit",
        fixture.household_slug, fixture.managed_person_id
    ));
    assert_eq!(edit.status().as_u16(), 403);
}

#[test]
fn authorised_people_locations_and_medication_forms_are_private_html() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target);
    for suffix in [
        "people",
        "people/new",
        "locations",
        "locations/new",
        "medications/new",
    ] {
        let response = target.get_html(&format!("/households/{}/{suffix}", fixture.household_slug));
        assert_eq!(
            response.status().as_u16(),
            200,
            "missing authorised route {suffix}"
        );
        assert!(
            response.headers()["cache-control"]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        let html = response.text().expect("household HTML");
        let document = Html::parse_document(&html);
        assert_eq!(document.select(&Selector::parse("h1").unwrap()).count(), 1);
        if suffix.ends_with("/new") {
            csrf(&html);
            assert_eq!(
                document
                    .select(&Selector::parse("form[method='post']").unwrap())
                    .count(),
                1
            );
        }
    }
}

#[test]
fn household_management_requires_a_session_and_hides_foreign_households() {
    let fixture = fixture();
    let anonymous = Target::from_env();
    let owner = Target::from_env();
    sign_in(&owner);
    for suffix in [
        "people",
        "people/new",
        "locations",
        "locations/new",
        "medications/new",
    ] {
        let anonymous_response =
            anonymous.get_html(&format!("/households/{}/{suffix}", fixture.household_slug));
        assert_eq!(
            anonymous_response.status().as_u16(),
            302,
            "anonymous route {suffix}"
        );
        let foreign = owner.get_html(&format!(
            "/households/{}/{suffix}",
            fixture.foreign_household_slug
        ));
        assert_eq!(
            foreign.status().as_u16(),
            404,
            "foreign household route {suffix}"
        );
        let body = foreign.text().expect("foreign denial HTML");
        assert!(!body.contains(&fixture.foreign_email));
    }
}

#[test]
fn household_form_mutations_reject_invalid_csrf_without_writing() {
    let fixture = fixture();
    for resource in ["people", "locations", "medications"] {
        let owner = Target::from_env();
        sign_in(&owner);
        let marker = format!("Contract rejected household {resource}");
        let response = owner.post_browser_form(
            &format!("/households/{}/{resource}", fixture.household_slug),
            &[
                ("authenticity_token".into(), "wrong-token".into()),
                ("name".into(), marker.clone()),
                ("date_of_birth".into(), "1980-01-01".into()),
                ("person_type".into(), "adult".into()),
                ("has_capacity".into(), "true".into()),
            ],
        );
        assert_eq!(
            response.status().as_u16(),
            403,
            "CSRF denial for {resource}"
        );
        let rows = owner.get(
            &format!("/api/v1/households/{}/{resource}", fixture.household_id),
            None,
        );
        assert_eq!(rows.status().as_u16(), 200);
        assert!(!rows.text().expect("API read back").contains(&marker));
    }
}

#[test]
fn household_ui_capabilities_follow_owner_and_viewer_permissions() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target);
    let path = format!(
        "/api/v1/households/{}/ui_capabilities",
        fixture.household_id
    );
    let owner = target.get(&path, None);
    assert_eq!(owner.status().as_u16(), 200, "missing UI capability route");
    let owner: Value = owner.json().expect("owner capabilities");
    assert_eq!(owner["data"]["people"]["create"], true);
    assert!(
        owner["data"]["people"]["manage_ids"]
            .as_array()
            .expect("manageable people")
            .contains(&Value::from(fixture.managed_person_id))
    );
    for resource in ["locations", "medications"] {
        assert_eq!(owner["data"][resource]["create"], true);
        assert_eq!(owner["data"][resource]["update"], true);
    }
    assert_eq!(owner["data"]["medications"]["manage_stock"], true);
    let viewer = target.get(&path, Some(&fixture.view_access_token));
    assert_eq!(viewer.status().as_u16(), 200);
    let viewer: Value = viewer.json().expect("viewer capabilities");
    assert_eq!(viewer["data"]["people"]["create"], false);
    assert!(
        viewer["data"]["people"]["manage_ids"]
            .as_array()
            .expect("viewer manageable people")
            .is_empty()
    );
    for resource in ["locations", "medications"] {
        assert_eq!(viewer["data"][resource]["create"], false);
        assert_eq!(viewer["data"][resource]["update"], false);
    }
    assert_eq!(viewer["data"]["medications"]["manage_stock"], false);
    let delegate = target.get(&path, Some(&fixture.delegated_access_token));
    assert_eq!(delegate.status().as_u16(), 200);
    let delegate: Value = delegate.json().expect("manage-granted member capabilities");
    assert_eq!(delegate["data"]["people"]["create"], true);
    let manageable = delegate["data"]["people"]["manage_ids"]
        .as_array()
        .expect("manage-granted people");
    assert!(manageable.contains(&Value::from(fixture.managed_person_id)));
    assert!(!manageable.contains(&Value::from(fixture.hidden_person_id)));
    assert!(!manageable.contains(&Value::from(fixture.foreign_person_id)));
    assert_eq!(
        target
            .get(&path, Some(&fixture.foreign_access_token))
            .status()
            .as_u16(),
        403
    );
    assert_eq!(Target::from_env().get(&path, None).status().as_u16(), 401);
}

#[test]
fn household_people_keep_dependent_capacity_false_after_a_forged_form_value() {
    let fixture = fixture();
    let target = Target::from_env();
    let token = sign_in(&target);
    for (person_type, birth) in [("minor", "2015-01-02"), ("dependent_adult", "1980-01-02")] {
        let marker = format!("Contract household capacity {person_type}");
        let response = target.post_browser_form(
            &format!("/households/{}/people", fixture.household_slug),
            &[
                ("authenticity_token".into(), token.clone()),
                ("name".into(), marker.clone()),
                ("date_of_birth".into(), birth.into()),
                ("person_type".into(), person_type.into()),
                ("has_capacity".into(), "true".into()),
            ],
        );
        assert!(
            [302, 303].contains(&response.status().as_u16()),
            "dependent form must use the existing API normalisation"
        );
        let rows = target.get(
            &format!(
                "/api/v1/households/{}/people?per_page=100",
                fixture.household_id
            ),
            None,
        );
        assert_eq!(rows.status().as_u16(), 200);
        let rows: Value = rows.json().expect("people read-back");
        let created = rows["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == marker)
            .expect("created dependent person");
        assert_eq!(created["has_capacity"], false);
    }
}

#[test]
fn household_people_hide_foreign_record_details_on_detail_and_edit_routes() {
    let fixture = fixture();
    let owner = Target::from_env();
    sign_in(&owner);
    for suffix in ["", "/edit"] {
        let response = owner.get_html(&format!(
            "/households/{}/people/{}{suffix}",
            fixture.household_slug, fixture.foreign_person_id
        ));
        assert_eq!(response.status().as_u16(), 404);
        assert!(
            !response
                .text()
                .expect("foreign person denial")
                .contains(&fixture.foreign_person_name)
        );
    }
}

#[test]
fn editing_medication_identity_preserves_existing_dosage_options_and_tracked_supply() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target);
    let api_path = format!(
        "/api/v1/households/{}/medications/{}",
        fixture.household_id, fixture.dose_write_tracked_medication_id
    );
    let option_path = format!(
        "/api/v1/households/{}/dosage_options/{}",
        fixture.household_id, fixture.dose_write_tracked_option_id
    );
    let before = target.get(&api_path, None);
    assert_eq!(before.status().as_u16(), 200);
    let before: Value = before.json().expect("tracked medication before edit");
    assert!(before["data"]["dose_amount"].is_null());
    let option_before = target.get(&option_path, None);
    assert_eq!(option_before.status().as_u16(), 200);
    let option_before: Value = option_before.json().expect("dosage option before edit");
    let edit_path = format!(
        "/households/{}/medications/{}/edit",
        fixture.household_slug, fixture.dose_write_tracked_medication_id
    );
    let edit = target.get_html(&edit_path);
    assert_eq!(edit.status().as_u16(), 200);
    let html = edit.text().expect("tracked medication edit HTML");
    let document = Html::parse_document(&html);
    for name in ["dose_amount", "dose_unit", "current_supply"] {
        assert_eq!(
            document
                .select(&Selector::parse(&format!("[name='{name}']")).unwrap())
                .count(),
            0,
            "tracked dosage {name} must not be submitted by identity editor"
        );
    }
    let etag = document
        .select(&Selector::parse("input[name='etag']").unwrap())
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("medication edit precondition")
        .to_owned();
    let text = |field: &str| {
        before["data"][field]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    let saved = target.post_browser_form(
        &format!(
            "/households/{}/medications/{}",
            fixture.household_slug, fixture.dose_write_tracked_medication_id
        ),
        &[
            ("authenticity_token".into(), csrf(&html)),
            ("etag".into(), etag),
            ("name".into(), text("name")),
            (
                "friendly_name".into(),
                "Contract options identity edit".into(),
            ),
            ("description".into(), text("description")),
            ("barcode".into(), text("barcode")),
            ("warnings".into(), text("warnings")),
            (
                "location_id".into(),
                before["data"]["location_id"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| before["data"]["location_id"].to_string()),
            ),
            (
                "reorder_threshold".into(),
                before["data"]["reorder_threshold"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| before["data"]["reorder_threshold"].to_string()),
            ),
        ],
    );
    assert!(
        [302, 303].contains(&saved.status().as_u16()),
        "tracked medication identity save must redirect"
    );
    let option_after = target.get(&option_path, None);
    assert_eq!(
        option_after.status().as_u16(),
        200,
        "identity edit must retain dosage option"
    );
    let option_after: Value = option_after.json().expect("dosage option after edit");
    assert_eq!(option_after["data"], option_before["data"]);
    let after = target.get(&api_path, None);
    assert_eq!(after.status().as_u16(), 200);
    let after: Value = after.json().expect("tracked medication after edit");
    assert_eq!(after["data"]["dose_amount"], before["data"]["dose_amount"]);
    assert_eq!(
        after["data"]["current_supply"],
        before["data"]["current_supply"]
    );
    assert_eq!(
        after["data"]["friendly_name"],
        "Contract options identity edit"
    );
}

#[test]
fn medication_read_exposes_editable_identity_and_warnings_without_losing_data() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target);
    let path = format!(
        "/api/v1/households/{}/medications/{}",
        fixture.household_id, fixture.managed_medication_id
    );
    let response = target.get(&path, None);
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("medication read");
    for field in ["friendly_name", "barcode", "warnings"] {
        assert!(
            body["data"].get(field).is_some(),
            "editable medication field {field} must survive an API read"
        );
    }
}

#[test]
fn medication_identity_update_requires_a_current_browser_precondition() {
    let fixture = fixture();
    let target = Target::from_env();
    let token = sign_in(&target);
    let api_path = format!(
        "/api/v1/households/{}/medications/{}",
        fixture.household_id, fixture.managed_medication_id
    );
    let web_path = format!(
        "/households/{}/medications/{}",
        fixture.household_slug, fixture.managed_medication_id
    );
    let before = target.get(&api_path, None);
    assert_eq!(before.status().as_u16(), 200);
    let before: Value = before.json().expect("medication before rejected edits");
    let mut fields = vec![
        ("authenticity_token".into(), token),
        ("name".into(), "Contract stale medication draft".into()),
        (
            "friendly_name".into(),
            "Retained stale display draft".into(),
        ),
        ("warnings".into(), "Retained stale warning draft".into()),
        (
            "location_id".into(),
            fixture.primary_location_id.to_string(),
        ),
        ("reorder_threshold".into(), "3.50".into()),
        ("dose_amount".into(), "2.50".into()),
        ("dose_unit".into(), "ml".into()),
        ("current_supply".into(), "20.75".into()),
    ];
    let missing = target.post_browser_form(&web_path, &fields);
    assert_eq!(
        missing.status().as_u16(),
        428,
        "missing browser ETag must not silently overwrite"
    );
    fields.push(("etag".into(), String::new()));
    let blank = target.post_browser_form(&web_path, &fields);
    assert_eq!(
        blank.status().as_u16(),
        428,
        "blank browser ETag must not silently overwrite"
    );
    fields.last_mut().unwrap().1 = "\"stale-version\"".into();
    let stale = target.post_browser_form(&web_path, &fields);
    assert_eq!(stale.status().as_u16(), 409);
    let html = stale.text().expect("stale medication edit HTML");
    assert!(html.contains("value=\"Contract stale medication draft\""));
    assert!(html.contains("value=\"Retained stale display draft\""));
    assert!(html.contains("value=\"2.50\""));
    assert!(html.contains("Retained stale warning draft"));
    let after = target.get(&api_path, None);
    assert_eq!(after.status().as_u16(), 200);
    let after: Value = after.json().expect("medication after rejected edits");
    assert_eq!(after["data"], before["data"]);
}
