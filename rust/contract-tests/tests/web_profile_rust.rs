use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::multipart;
use scraper::{Html, Selector};
use serde_json::Value;
use std::io::{Cursor, Read};
use zip::ZipArchive;

fn profile_path(slug: &str) -> String {
    format!("/households/{slug}/profile")
}

fn token_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    for (selector, attribute) in [
        ("meta[name='csrf-token']", "content"),
        ("input[name='authenticity_token']", "value"),
    ] {
        let selector = Selector::parse(selector).unwrap();
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

fn sign_in(target: &Target, email: &str, slug: &str) -> String {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let csrf = token_from_html(&login.text().unwrap());
    let response = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".to_owned(), email.to_owned()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), csrf),
        ],
        "198.51.100.21",
    );
    assert_eq!(response.status().as_u16(), 302);
    let page = target.get_html(&profile_path(slug));
    assert_eq!(page.status().as_u16(), 200);
    token_from_html(&page.text().unwrap())
}

fn assert_login_redirect(response: reqwest::blocking::Response) {
    assert!(response.status().is_redirection());
    assert!(response.headers()["location"]
        .to_str()
        .unwrap()
        .ends_with("/login"));
}

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

fn assert_no_store(response: &reqwest::blocking::Response) {
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
}

fn issue_app_token(target: &Target, fixture: &Fixture, csrf: &str) -> (String, String) {
    let path = format!("{}/api_tokens", profile_path(&fixture.household_slug));
    let response = target.post_browser_form(
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
    assert!(html.contains("profile-tabs"));
    assert!(html.contains("profile-advanced-panel"));
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
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/app_tokens",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("app token list JSON");
    assert!(!body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["name"] == name));
}

#[test]
fn token_requires_session_and_is_issued_once_then_revoked() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("{}/api_tokens", profile_path(&fixture.household_slug));
    assert_login_redirect(target.post_browser_form(
        &path,
        &[("api_app_token[name]".to_owned(), "denied".to_owned())],
    ));

    let csrf = sign_in(&target, &fixture.primary_email, &fixture.household_slug);
    let (raw_token, revoke_path) = issue_app_token(&target, &fixture, &csrf);
    assert!(raw_token.starts_with("mt_app_"));
    let read_back = target.get_html(&format!(
        "{}?section=advanced",
        profile_path(&fixture.household_slug)
    ));
    let html = read_back.text().expect("profile read-back");
    assert!(html.contains("Contract web token"));
    assert!(!html.contains(&raw_token));

    let me_path = format!("/api/v1/households/{}/me", fixture.household_id);
    let me = target.get(&me_path, Some(&raw_token));
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().expect("me JSON");
    assert_eq!(me["data"]["email_address"], fixture.primary_email);

    let response = target.post_browser_form(
        &revoke_path,
        &[("authenticity_token".to_owned(), csrf.clone())],
    );
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        target.get(&me_path, Some(&raw_token)).status().as_u16(),
        401
    );

    let wrong = target.post_browser_form(
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
    assert_eq!(wrong.status().as_u16(), 403);
    assert_app_token_absent(&target, &fixture, "Contract wrong CSRF");

    let missing = target.post_browser_form(
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
    assert_eq!(missing.status().as_u16(), 403);
    assert_app_token_absent(&target, &fixture, "Contract missing CSRF");
}

#[test]
fn token_rejects_foreign_membership_and_blank_name() {
    let target = Target::from_env();
    let fixture = fixture();
    let csrf = sign_in(&target, &fixture.primary_email, &fixture.household_slug);
    let path = format!("{}/api_tokens", profile_path(&fixture.household_slug));
    let blank = target.post_browser_form(
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
    assert_eq!(blank.status().as_u16(), 422);

    let foreign = target.post_browser_form(
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
    let revoke_foreign = target.post_browser_form(
        &format!("{path}/{}", fixture.foreign_app_token_id),
        &[("authenticity_token".to_owned(), csrf)],
    );
    assert_eq!(revoke_foreign.status().as_u16(), 404);
    assert_app_token_absent(&target, &fixture, "foreign");
}

#[test]
fn exports_require_session_and_return_scoped_json_and_zip() {
    let target = Target::from_env();
    let fixture = fixture();
    let json_path = format!(
        "{}/data_exports/health_data_json",
        profile_path(&fixture.household_slug)
    );
    assert_login_redirect(target.get_html(&json_path));
    sign_in(&target, &fixture.primary_email, &fixture.household_slug);

    let response = target.get_html(&json_path);
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("application/json"));
    let payload: Value = response.json().expect("health data export");
    assert_eq!(payload["format"], "medtracker.health_data.v1");
    assert_household_export_records(&payload, &fixture);

    let zip_path = format!(
        "{}/data_exports/backup_zip",
        profile_path(&fixture.household_slug)
    );
    let response = target.get_html(&zip_path);
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("application/zip"));
    assert!(response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains("medtracker-backup-"));
    let bytes = response.bytes().expect("ZIP bytes");
    assert!(bytes.starts_with(b"PK"));
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("parse ZIP");
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
fn exports_deny_foreign_household_and_unknown_mode() {
    let fixture = fixture();
    let foreign_target = Target::from_env();
    sign_in(
        &foreign_target,
        &fixture.foreign_email,
        &fixture.foreign_household_slug,
    );
    let json_path = format!(
        "{}/data_exports/health_data_json",
        profile_path(&fixture.household_slug)
    );
    assert_eq!(foreign_target.get_html(&json_path).status().as_u16(), 404);

    let target = Target::from_env();
    sign_in(&target, &fixture.primary_email, &fixture.household_slug);
    let invalid = target.get_html(&format!(
        "{}/data_exports/invalid",
        profile_path(&fixture.household_slug)
    ));
    assert_eq!(invalid.status().as_u16(), 404);
}

#[test]
fn avatar_route_requires_access_and_returns_inline_bytes_until_removed() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/profile/avatar",
        fixture.avatar_household_id
    );
    assert_eq!(target.get(&path, None).status().as_u16(), 401);
    let bytes = b"contract-owner-avatar".to_vec();
    let response = target.put_multipart(
        &path,
        &fixture.avatar_access_token,
        multipart::Form::new().part(
            "avatar",
            multipart::Part::bytes(bytes.clone())
                .file_name("profile.png")
                .mime_str("image/png")
                .unwrap(),
        ),
    );
    assert_eq!(response.status().as_u16(), 200);
    let image = target.get(&path, Some(&fixture.avatar_access_token));
    assert_eq!(image.status().as_u16(), 200);
    assert_no_store(&image);
    assert!(image.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("image/png"));
    assert!(image.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains("inline"));
    assert_eq!(image.bytes().unwrap().as_ref(), bytes.as_slice());
    assert_eq!(
        target
            .get(&path, Some(&fixture.foreign_access_token))
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        target
            .delete(&path, Some(&fixture.avatar_access_token))
            .status()
            .as_u16(),
        204
    );
    assert_eq!(
        target
            .get(&path, Some(&fixture.avatar_access_token))
            .status()
            .as_u16(),
        404
    );
}

#[test]
fn timezone_form_persists_a_rails_label_and_denies_untrusted_writes() {
    let target = Target::from_env();
    let fixture = fixture();
    let csrf = sign_in(&target, &fixture.primary_email, &fixture.household_slug);
    let path = profile_path(&fixture.household_slug);
    let original = target.get(
        &format!("/api/v1/households/{}/profile", fixture.household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(original.status().as_u16(), 200);
    let original: Value = original.json().unwrap();
    let original = original["data"]["time_zone"].as_str().unwrap().to_owned();

    let response = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf.clone()),
            ("time_zone".to_owned(), "London".to_owned()),
        ],
    );
    assert_eq!(response.status().as_u16(), 303);
    let profile = target.get_html(&path).text().unwrap();
    assert!(profile.contains("<dd>London</dd>"));
    let saved = target.get(
        &format!("/api/v1/households/{}/profile", fixture.household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(
        saved.json::<Value>().unwrap()["data"]["time_zone"],
        "London"
    );

    let wrong_csrf = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".to_owned(), "wrong-token".to_owned()),
            ("time_zone".to_owned(), "UTC".to_owned()),
        ],
    );
    assert_eq!(wrong_csrf.status().as_u16(), 403);
    let invalid = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf.clone()),
            ("time_zone".to_owned(), "Mars/Olympus".to_owned()),
        ],
    );
    assert_eq!(invalid.status().as_u16(), 422);

    let restored = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf),
            ("time_zone".to_owned(), original),
        ],
    );
    assert_eq!(restored.status().as_u16(), 303);
}

#[test]
fn health_probe_succeeds_without_a_content_type_assumption() {
    let target = Target::from_env();
    assert_eq!(target.get_html("/up").status().as_u16(), 200);
    assert_eq!(target.get_html("/health").status().as_u16(), 404);
}
