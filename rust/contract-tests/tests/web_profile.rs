use medtracker_contract_tests::{fixture, Fixture, Target};
use scraper::{Html, Selector};
use serde_json::Value;
use std::io::{Cursor, Read};
use zip::ZipArchive;

fn assert_household_export_records(payload: &Value, fixture: &Fixture) {
    assert_eq!(payload["scope"], "single_person");
    let people = payload["records"]["people"].as_array().expect("people");
    assert!(people
        .iter()
        .any(|person| person["portable_id"] == fixture.managed_person_portable_id));
    let medications = payload["records"]["medications"]
        .as_array()
        .expect("medications");
    assert!(medications
        .iter()
        .any(|medication| medication["portable_id"] == fixture.managed_medication_portable_id));
    assert!(!payload.to_string().contains(&fixture.foreign_person_name));
}

fn token_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    for (selector, attribute) in [
        ("meta[name='csrf-token']", "content"),
        ("input[name='authenticity_token']", "value"),
    ] {
        let selector = Selector::parse(selector).expect("CSRF selector");
        if let Some(value) = document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr(attribute))
        {
            return value.to_owned();
        }
    }
    panic!("CSRF token in HTML");
}

fn profile_path(slug: &str) -> String {
    format!("/households/{slug}/profile")
}

fn sign_in(target: &Target, email: &str, slug: &str, client_ip: &str) -> String {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let login_token = token_from_html(&login.text().expect("login HTML"));
    let response = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".to_owned(), email.to_owned()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), login_token),
        ],
        client_ip,
    );
    assert_eq!(response.status().as_u16(), 302);
    let path = profile_path(slug);
    let profile = target.get_html(&path);
    assert_eq!(profile.status().as_u16(), 200);
    token_from_html(&profile.text().expect("profile HTML"))
}

fn issue_app_token(target: &Target, fixture: &Fixture, csrf: &str) -> (String, String) {
    let path = format!("{}/api_tokens", profile_path(&fixture.household_slug));
    let response = target.post_html_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf.to_owned()),
            (
                "api_app_token[name]".to_owned(),
                "Contract web token".to_owned(),
            ),
            (
                "api_app_token[household_membership_id]".to_owned(),
                fixture.owner_membership_id.to_string(),
            ),
        ],
    );
    assert_eq!(response.status().as_u16(), 201);
    let html = response.text().expect("new token HTML");
    let suffix = html.split_once("mt_app_").expect("one-time token").1;
    let raw_token = format!(
        "mt_app_{}",
        suffix
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric()
                || *character == '_'
                || *character == '-')
            .collect::<String>()
    );
    let document = Html::parse_document(&html);
    let selector = Selector::parse("form[action*='/profile/api_tokens/']").unwrap();
    let revoke_path = document
        .select(&selector)
        .next()
        .and_then(|form| form.value().attr("action"))
        .expect("revoke form")
        .to_owned();
    (raw_token, revoke_path)
}

fn assert_app_token_absent(target: &Target, fixture: &Fixture, name: &str) {
    let path = format!(
        "/api/v1/households/{}/admin/app_tokens",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("app token list JSON");
    let rows = body["data"].as_array().expect("app token list");
    assert!(!rows.iter().any(|row| row["name"] == name));
}

#[test]
fn web_token_requires_session_and_is_issued_once_then_revoked() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("{}/api_tokens", profile_path(&fixture.household_slug));
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let anonymous_token = token_from_html(&login.text().expect("login HTML"));
    let response = target.post_html_form(
        &path,
        &[
            ("api_app_token[name]".to_owned(), "denied".to_owned()),
            ("authenticity_token".to_owned(), anonymous_token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    assert!(response.headers()["location"]
        .to_str()
        .unwrap()
        .contains("/?household_slug="));

    let csrf = sign_in(
        &target,
        &fixture.primary_email,
        &fixture.household_slug,
        "198.51.100.21",
    );
    let (raw_token, revoke_path) = issue_app_token(&target, &fixture, &csrf);
    assert!(raw_token.starts_with("mt_app_"));
    let profile = target.get_html(&format!(
        "{}?section=advanced",
        profile_path(&fixture.household_slug)
    ));
    let html = profile.text().expect("profile read-back");
    assert!(html.contains("Contract web token"));
    assert!(!html.contains(&raw_token));

    let me_path = format!("/api/v1/households/{}/me", fixture.household_id);
    let me = target.get(&me_path, Some(&raw_token));
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().expect("me JSON");
    assert_eq!(me["data"]["email_address"], fixture.primary_email);

    let response = target.post_html_form(
        &revoke_path,
        &[
            ("_method".to_owned(), "delete".to_owned()),
            ("authenticity_token".to_owned(), csrf),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    assert_eq!(
        target.get(&me_path, Some(&raw_token)).status().as_u16(),
        401
    );

    let wrong = target.post_html_form(
        &path,
        &[
            (
                "api_app_token[name]".to_owned(),
                "Contract wrong CSRF".to_owned(),
            ),
            (
                "api_app_token[household_membership_id]".to_owned(),
                fixture.owner_membership_id.to_string(),
            ),
            ("authenticity_token".to_owned(), "wrong-token".to_owned()),
        ],
    );
    assert_eq!(wrong.status().as_u16(), 303);
    assert!(wrong.headers()["location"]
        .to_str()
        .expect("login redirect")
        .ends_with("/login"));
    assert_app_token_absent(&target, &fixture, "Contract wrong CSRF");
}

#[test]
fn web_token_rejects_foreign_membership_and_blank_name() {
    let target = Target::from_env();
    let fixture = fixture();
    let csrf = sign_in(
        &target,
        &fixture.primary_email,
        &fixture.household_slug,
        "198.51.100.22",
    );
    let path = format!("{}/api_tokens", profile_path(&fixture.household_slug));
    let blank = target.post_html_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf.clone()),
            ("api_app_token[name]".to_owned(), String::new()),
            (
                "api_app_token[household_membership_id]".to_owned(),
                fixture.owner_membership_id.to_string(),
            ),
        ],
    );
    assert_eq!(blank.status().as_u16(), 302);
    let foreign = target.post_html_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf.clone()),
            ("api_app_token[name]".to_owned(), "foreign".to_owned()),
            (
                "api_app_token[household_membership_id]".to_owned(),
                fixture.foreign_membership_id.to_string(),
            ),
        ],
    );
    assert_eq!(foreign.status().as_u16(), 404);
    let revoke_foreign = target.post_html_form(
        &format!("{path}/{}", fixture.foreign_app_token_id),
        &[
            ("_method".to_owned(), "delete".to_owned()),
            ("authenticity_token".to_owned(), csrf),
        ],
    );
    assert_eq!(revoke_foreign.status().as_u16(), 404);

    let missing = target.post_html_form(
        &path,
        &[
            (
                "api_app_token[name]".to_owned(),
                "Contract missing CSRF".to_owned(),
            ),
            (
                "api_app_token[household_membership_id]".to_owned(),
                fixture.owner_membership_id.to_string(),
            ),
        ],
    );
    assert_eq!(missing.status().as_u16(), 303);
    assert!(missing.headers()["location"]
        .to_str()
        .expect("login redirect")
        .ends_with("/login"));
    assert_app_token_absent(&target, &fixture, "Contract missing CSRF");
}

#[test]
fn web_profile_exports_require_session_and_return_download_contracts() {
    let target = Target::from_env();
    let fixture = fixture();
    let json_path = format!(
        "{}/data_exports/health_data_json",
        profile_path(&fixture.household_slug)
    );
    let anonymous = target.get_html(&json_path);
    assert_eq!(anonymous.status().as_u16(), 302);
    assert!(anonymous.headers()["location"]
        .to_str()
        .unwrap()
        .contains("/?household_slug="));

    sign_in(
        &target,
        &fixture.primary_email,
        &fixture.household_slug,
        "198.51.100.23",
    );
    let response = target.get_html(&json_path);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("application/json"));
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let payload: Value = response.json().expect("health data export");
    assert_eq!(payload["format"], "medtracker.health_data.v1");
    assert_household_export_records(&payload, &fixture);

    let zip_path = format!(
        "{}/data_exports/backup_zip",
        profile_path(&fixture.household_slug)
    );
    let response = target.get_html(&zip_path);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("application/zip"));
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    assert!(response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains("medtracker-backup-"));
    let bytes = response.bytes().expect("ZIP bytes");
    assert!(bytes.starts_with(b"PK"));
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("parse ZIP");
    assert_eq!(archive.len(), 1);
    let mut entry = archive
        .by_name("medtracker-backup.json")
        .expect("backup JSON member");
    let mut contents = String::new();
    entry
        .read_to_string(&mut contents)
        .expect("backup JSON bytes");
    let backup: Value = serde_json::from_str(&contents).expect("backup JSON");
    assert_eq!(backup["format"], "medtracker.backup.v1");
    assert_household_export_records(&backup, &fixture);
}

#[test]
fn web_profile_export_denies_foreign_household_and_invalid_mode() {
    let target = Target::from_env();
    let fixture = fixture();
    sign_in(
        &target,
        &fixture.foreign_email,
        &fixture.foreign_household_slug,
        "198.51.100.24",
    );
    let path = format!(
        "{}/data_exports/health_data_json",
        profile_path(&fixture.household_slug)
    );
    assert_eq!(target.get_html(&path).status().as_u16(), 302);

    let target = Target::from_env();
    sign_in(
        &target,
        &fixture.primary_email,
        &fixture.household_slug,
        "198.51.100.25",
    );
    let invalid = target.get_html(&format!(
        "{}/data_exports/invalid",
        profile_path(&fixture.household_slug)
    ));
    assert_eq!(invalid.status().as_u16(), 302);
}

#[test]
fn person_avatar_requires_visible_person_and_returns_inline_image() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/households/{}/people/{}/avatar",
        fixture.web_avatar_household_slug, fixture.web_avatar_person_id
    );
    let anonymous = target.get_html(&path);
    assert_eq!(anonymous.status().as_u16(), 302);
    sign_in(
        &target,
        &fixture.web_avatar_email,
        &fixture.web_avatar_household_slug,
        "198.51.100.26",
    );
    let image = target.get_html(&path);
    assert_eq!(image.status().as_u16(), 200);
    assert!(image.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("image/png"));
    assert!(image.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains("inline"));
    assert_eq!(
        image.bytes().expect("avatar bytes").as_ref(),
        b"contract-owner-avatar"
    );

    let hidden_path = format!(
        "/households/{}/people/{}/avatar",
        fixture.web_avatar_household_slug, fixture.web_avatar_hidden_person_id
    );
    assert_eq!(target.get_html(&hidden_path).status().as_u16(), 404);
    let foreign_path = format!(
        "/households/{}/people/{}/avatar",
        fixture.avatar_invalid_household_slug, fixture.web_avatar_person_id
    );
    assert_eq!(target.get_html(&foreign_path).status().as_u16(), 302);
}

#[test]
fn profile_avatar_delete_redirects_and_removes_public_read_back() {
    let target = Target::from_env();
    let fixture = fixture();
    let csrf = sign_in(
        &target,
        &fixture.web_avatar_email,
        &fixture.web_avatar_household_slug,
        "198.51.100.27",
    );
    let avatar_path = format!(
        "/households/{}/people/{}/avatar",
        fixture.web_avatar_household_slug, fixture.web_avatar_person_id
    );
    assert_eq!(target.get_html(&avatar_path).status().as_u16(), 200);
    let delete_path = format!(
        "{}/avatar",
        profile_path(&fixture.web_avatar_household_slug)
    );
    let response = target.post_html_form(
        &delete_path,
        &[
            ("_method".to_owned(), "delete".to_owned()),
            ("authenticity_token".to_owned(), csrf),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    assert_eq!(target.get_html(&avatar_path).status().as_u16(), 404);
}

#[test]
fn health_endpoint_is_public_and_returns_success() {
    let target = Target::from_env();
    let response = target.get_html("/up");
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/html"));
    let alternate_host = target.get_html_with_header("/up", "Host", "untrusted.example");
    assert_eq!(alternate_host.status().as_u16(), 200);
    let health = target.get_html_with_header("/health", "Host", "untrusted.example");
    assert_eq!(health.status().as_u16(), 404);
}
