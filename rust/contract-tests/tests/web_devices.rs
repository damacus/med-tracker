use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use scraper::{Html, Selector};
use serde_json::Value;

fn web_path(fixture: &Fixture, suffix: &str) -> String {
    format!("/households/{}/{suffix}", fixture.web_device_household_slug)
}

fn csrf_from_html(html: &str, selector: &str, attribute: &str) -> String {
    let document = Html::parse_document(html);
    let parsed = Selector::parse(selector).expect("CSRF selector");
    document
        .select(&parsed)
        .next()
        .and_then(|element| element.value().attr(attribute))
        .unwrap_or_else(|| panic!("CSRF token missing for {selector}"))
        .to_owned()
}

fn login(target: &Target, fixture: &Fixture) -> String {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let token = csrf_from_html(
        &response.text().expect("login HTML"),
        "form[action='/login'] input[name='authenticity_token']",
        "value",
    );
    let response = target.post_html_form(
        "/login",
        &[
            ("email".to_owned(), fixture.web_device_email.clone()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    let response = target.get_html(&web_path(fixture, "profile"));
    assert_eq!(response.status().as_u16(), 200);
    csrf_from_html(
        &response.text().expect("profile HTML"),
        "meta[name='csrf-token']",
        "content",
    )
}

fn web_request(
    target: &Target,
    method: &str,
    path: &str,
    csrf: &str,
    accept: &str,
    fields: &[(&str, &str)],
) -> Response {
    let mut form = vec![("authenticity_token".to_owned(), csrf.to_owned())];
    form.extend(
        fields
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string())),
    );
    target.web_form_request(method, path, accept, &form)
}

fn json(response: Response) -> Value {
    response.json().expect("JSON response")
}

#[test]
fn web_native_device_tokens_require_session_and_csrf_and_keep_account_ownership() {
    let fixture = fixture();
    let path = web_path(&fixture, "native_device_tokens");
    let token = format!("contract-web-device-{}", fixture.web_device_household_id);
    let target = Target::from_env();
    let unauthenticated = web_request(
        &target,
        "POST",
        &path,
        "invalid",
        "application/json",
        &[("device_token", &token), ("platform", "ios")],
    );
    assert_eq!(unauthenticated.status().as_u16(), 401);
    assert!(json(unauthenticated)["error"].is_string());
    assert_eq!(
        web_request(
            &target,
            "POST",
            &web_path(&fixture, "push_subscription/test"),
            "invalid",
            "application/json",
            &[]
        )
        .status()
        .as_u16(),
        401
    );
    assert_eq!(
        web_request(
            &target,
            "PATCH",
            &web_path(&fixture, "notification_preference"),
            "invalid",
            "application/json",
            &[("notification_preference[enabled]", "0")]
        )
        .status()
        .as_u16(),
        401
    );
    let login_page = target.get_html("/login");
    assert_eq!(login_page.status().as_u16(), 200);
    let anonymous_csrf = csrf_from_html(
        &login_page.text().expect("anonymous login HTML"),
        "form[action='/login'] input[name='authenticity_token']",
        "value",
    );
    let denied = web_request(
        &target,
        "POST",
        &path,
        &anonymous_csrf,
        "application/json",
        &[("device_token", &token), ("platform", "ios")],
    );
    assert_eq!(denied.status().as_u16(), 302);
    assert_eq!(denied.headers()["location"], "/login");
    let csrf = login(&target, &fixture);

    let created = web_request(
        &target,
        "POST",
        &path,
        &csrf,
        "application/json",
        &[("device_token", &token), ("platform", "ios")],
    );
    assert_eq!(created.status().as_u16(), 201);
    assert!(created.bytes().expect("empty created body").is_empty());
    let repeated = web_request(
        &target,
        "POST",
        &path,
        &csrf,
        "application/json",
        &[("device_token", &token), ("platform", "android")],
    );
    assert_eq!(repeated.status().as_u16(), 201);

    let foreign = Target::from_env();
    let foreign_csrf = login_foreign(&foreign, &fixture);
    let foreign_path = format!(
        "/households/{}/native_device_tokens",
        fixture.foreign_household_slug
    );
    let hijack = web_request(
        &foreign,
        "POST",
        &foreign_path,
        &foreign_csrf,
        "application/json",
        &[("device_token", &token), ("platform", "ios")],
    );
    assert_eq!(hijack.status().as_u16(), 422);
    assert_eq!(json(hijack)["error"], "Unable to save device token.");
    let remove_path = format!("{path}/{token}");
    assert_eq!(
        web_request(
            &foreign,
            "DELETE",
            &format!("{foreign_path}/{token}"),
            &foreign_csrf,
            "application/json",
            &[]
        )
        .status()
        .as_u16(),
        204
    );
    assert_eq!(
        web_request(
            &target,
            "DELETE",
            &remove_path,
            &csrf,
            "application/json",
            &[]
        )
        .status()
        .as_u16(),
        204
    );
    assert_eq!(
        web_request(
            &foreign,
            "POST",
            &foreign_path,
            &foreign_csrf,
            "application/json",
            &[("device_token", &token), ("platform", "ios")]
        )
        .status()
        .as_u16(),
        201
    );
    assert_eq!(
        web_request(
            &target,
            "POST",
            &path,
            &csrf,
            "application/json",
            &[("device_token", &token), ("platform", "ios")]
        )
        .status()
        .as_u16(),
        422
    );
    push_lifecycle(&fixture, &target, &foreign, &csrf, &foreign_csrf);
    let wrong_csrf = web_request(
        &target,
        "POST",
        &path,
        "invalid",
        "application/json",
        &[("device_token", &token), ("platform", "ios")],
    );
    assert_eq!(wrong_csrf.status().as_u16(), 401);
}

fn login_foreign(target: &Target, fixture: &Fixture) -> String {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let token = csrf_from_html(
        &response.text().expect("login HTML"),
        "form[action='/login'] input[name='authenticity_token']",
        "value",
    );
    let response = target.post_html_form(
        "/login",
        &[
            ("email".to_owned(), fixture.foreign_email.clone()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    let profile = format!("/households/{}/profile", fixture.foreign_household_slug);
    let response = target.get_html(&profile);
    assert_eq!(response.status().as_u16(), 200);
    csrf_from_html(
        &response.text().expect("profile HTML"),
        "meta[name='csrf-token']",
        "content",
    )
}

fn push_lifecycle(
    fixture: &Fixture,
    target: &Target,
    foreign: &Target,
    csrf: &str,
    foreign_csrf: &str,
) {
    let path = web_path(fixture, "push_subscription");
    let test = web_request(
        target,
        "POST",
        &format!("{path}/test"),
        csrf,
        "application/json",
        &[],
    );
    assert_eq!(test.status().as_u16(), 204);

    let invalid = web_request(
        target,
        "POST",
        &path,
        csrf,
        "application/json",
        &[
            ("endpoint", "https://127.0.0.1/push/internal"),
            ("keys[p256dh]", "public-key"),
            ("keys[auth]", "auth-secret"),
        ],
    );
    assert_eq!(invalid.status().as_u16(), 422);
    assert!(json(invalid)["errors"]
        .as_array()
        .expect("validation errors")
        .iter()
        .any(|error| error == "Endpoint must be a supported HTTPS Web Push endpoint"));

    let endpoint = format!(
        "https://fcm.googleapis.com/fcm/send/contract-web-{}",
        fixture.web_device_household_id
    );
    let missing_keys = web_request(
        target,
        "POST",
        &path,
        csrf,
        "application/json",
        &[
            ("endpoint", &endpoint),
            ("keys[p256dh]", ""),
            ("keys[auth]", ""),
        ],
    );
    assert_eq!(missing_keys.status().as_u16(), 422);
    let errors = json(missing_keys)["errors"]
        .as_array()
        .expect("key errors")
        .clone();
    assert!(errors.iter().any(|error| error == "P256dh can't be blank"));
    assert!(errors.iter().any(|error| error == "Auth can't be blank"));

    let created = web_request(
        target,
        "POST",
        &path,
        csrf,
        "application/json",
        &[
            ("endpoint", &endpoint),
            ("keys[p256dh]", "public-key"),
            ("keys[auth]", "auth-secret"),
        ],
    );
    assert_eq!(created.status().as_u16(), 201);
    assert!(created.bytes().expect("empty created body").is_empty());
    let foreign_path = format!(
        "/households/{}/push_subscription",
        fixture.foreign_household_slug
    );
    let hijack = web_request(
        foreign,
        "POST",
        &foreign_path,
        foreign_csrf,
        "application/json",
        &[
            ("endpoint", &endpoint),
            ("keys[p256dh]", "other-key"),
            ("keys[auth]", "other-secret"),
        ],
    );
    assert_eq!(hijack.status().as_u16(), 422);
    assert!(json(hijack)["errors"]
        .as_array()
        .expect("duplicate error")
        .iter()
        .any(|error| error == "Endpoint has already been taken"));
    assert_eq!(
        web_request(
            foreign,
            "DELETE",
            &foreign_path,
            foreign_csrf,
            "application/json",
            &[("endpoint", &endpoint)]
        )
        .status()
        .as_u16(),
        204
    );
    assert_eq!(
        web_request(
            target,
            "DELETE",
            &path,
            csrf,
            "application/json",
            &[("endpoint", &endpoint)]
        )
        .status()
        .as_u16(),
        204
    );
    assert_eq!(
        web_request(
            foreign,
            "POST",
            &foreign_path,
            foreign_csrf,
            "application/json",
            &[
                ("endpoint", &endpoint),
                ("keys[p256dh]", "other-key"),
                ("keys[auth]", "other-secret")
            ]
        )
        .status()
        .as_u16(),
        201
    );
    assert_eq!(
        web_request(
            target,
            "POST",
            &path,
            csrf,
            "application/json",
            &[
                ("endpoint", &endpoint),
                ("keys[p256dh]", "public-key"),
                ("keys[auth]", "auth-secret")
            ]
        )
        .status()
        .as_u16(),
        422
    );
}

#[test]
fn web_notification_preference_turbo_and_html_updates_have_public_readback() {
    let fixture = fixture();
    let target = Target::from_env();
    let csrf = login(&target, &fixture);
    let preference_path = web_path(&fixture, "notification_preference");
    let stream = web_request(
        &target,
        "PATCH",
        &preference_path,
        &csrf,
        "text/vnd.turbo-stream.html",
        &[
            ("notification_preference[enabled]", "0"),
            ("notification_preference[morning_time]", "07:30"),
            ("notification_preference[afternoon_time]", "13:30"),
        ],
    );
    assert_eq!(stream.status().as_u16(), 200);
    assert_eq!(
        stream.headers()["content-type"],
        "text/vnd.turbo-stream.html; charset=utf-8"
    );
    let body = stream.text().expect("Turbo Stream response");
    assert!(body.contains("target=\"notifications-card\""));
    assert!(body.contains("target=\"flash\""));
    let api_path = format!(
        "/api/v1/households/{}/notification_preference",
        fixture.web_device_household_id
    );
    let read = target.get(&api_path, Some(&fixture.web_device_access_token));
    assert_eq!(read.status().as_u16(), 200);
    let preference = json(read)["data"].clone();
    assert_eq!(preference["enabled"], false);
    assert_eq!(preference["morning_time"], "07:30:00");

    let html = web_request(
        &target,
        "PATCH",
        &preference_path,
        &csrf,
        "text/html",
        &[
            ("notification_preference[enabled]", "1"),
            ("notification_preference[dose_due_enabled]", "1"),
        ],
    );
    assert_eq!(html.status().as_u16(), 302);
    assert!(html.headers()["location"]
        .to_str()
        .expect("profile redirect")
        .ends_with(&web_path(&fixture, "profile")));
    let updated =
        json(target.get(&api_path, Some(&fixture.web_device_access_token)))["data"].clone();
    assert_eq!(updated["enabled"], true);
    assert_eq!(updated["dose_due_enabled"], true);
    assert_eq!(updated["id"], preference["id"]);
    let section_path = format!("{preference_path}?section=notifications");
    let replaced = web_request(
        &target,
        "PUT",
        &section_path,
        &csrf,
        "text/html",
        &[("notification_preference[enabled]", "0")],
    );
    assert_eq!(replaced.status().as_u16(), 302);
    assert!(replaced.headers()["location"]
        .to_str()
        .expect("section redirect")
        .ends_with(&format!(
            "{}?section=notifications",
            web_path(&fixture, "profile")
        )));
    let replaced_preference =
        json(target.get(&api_path, Some(&fixture.web_device_access_token)))["data"].clone();
    assert_eq!(replaced_preference["enabled"], false);
    assert_eq!(replaced_preference["id"], preference["id"]);
}
