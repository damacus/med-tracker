use medtracker_contract_tests::{fixture, Fixture, Target};
use scraper::{Html, Selector};
use serde_json::json;

fn endpoint(route: &str, outcome: &str, household_id: i64) -> String {
    format!("https://fcm.googleapis.com/fcm/send/contract-push-{outcome}-{route}-{household_id}")
}

fn api_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/push_subscription")
}

fn register_api(target: &Target, path: &str, token: &str, endpoint: &str) -> u16 {
    target
        .post_json_authorized(
            path,
            token,
            &json!({"push_subscription": {
                "endpoint": endpoint,
                "keys": {"p256dh": "contract-public-key", "auth": "contract-auth-secret"}
            }}),
        )
        .status()
        .as_u16()
}

fn assert_provider_effects(target: &Target, fixture: &Fixture, route: &str, household_id: i64) {
    let observer_path = api_path(fixture.push_observer_household_id);
    let observer_token = &fixture.push_observer_access_token;
    assert_eq!(
        register_api(
            target,
            &observer_path,
            observer_token,
            &endpoint(route, "accepted", household_id),
        ),
        422
    );
    assert_eq!(
        register_api(
            target,
            &observer_path,
            observer_token,
            &endpoint(route, "transient", household_id),
        ),
        422
    );
    assert_eq!(
        register_api(
            target,
            &observer_path,
            observer_token,
            &endpoint(route, "expired", household_id),
        ),
        201
    );
}

fn csrf(html: &str, selector: &str, attribute: &str) -> String {
    let document = Html::parse_document(html);
    let selector = Selector::parse(selector).expect("CSRF selector");
    document
        .select(&selector)
        .next()
        .and_then(|element| element.value().attr(attribute))
        .expect("CSRF token")
        .to_owned()
}

fn web_form(target: &Target, path: &str, csrf: &str, fields: &[(&str, &str)]) -> u16 {
    let mut body = vec![("authenticity_token".to_owned(), csrf.to_owned())];
    body.extend(
        fields
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string())),
    );
    target
        .web_form_request("POST", path, "application/json", &body)
        .status()
        .as_u16()
}

fn login_web(target: &Target, email: &str, slug: &str, client_ip: &str) -> String {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let login_csrf = csrf(
        &login.text().expect("login HTML"),
        "form[action='/login'] input[name='authenticity_token']",
        "value",
    );
    let login = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".to_owned(), email.to_owned()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), login_csrf),
        ],
        client_ip,
    );
    assert_eq!(login.status().as_u16(), 302);
    let profile = target.get_html(&format!("/households/{slug}/profile"));
    assert_eq!(profile.status().as_u16(), 200);
    csrf(
        &profile.text().expect("profile HTML"),
        "meta[name='csrf-token']",
        "content",
    )
}

#[test]
fn api_test_push_reports_no_content_and_prunes_only_expired_subscriptions() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = api_path(fixture.push_api_household_id);
    for outcome in ["accepted", "transient", "expired"] {
        assert_eq!(
            register_api(
                &target,
                &path,
                &fixture.push_api_access_token,
                &endpoint("api", outcome, fixture.push_api_household_id),
            ),
            201
        );
    }
    let response = target.post_json_authorized(
        &format!("{path}/test"),
        &fixture.push_api_access_token,
        &json!({}),
    );
    assert_eq!(response.status().as_u16(), 204);
    assert!(response.bytes().expect("empty test response").is_empty());
    assert_provider_effects(&target, &fixture, "api", fixture.push_api_household_id);
}

#[test]
fn web_test_push_reports_no_content_and_prunes_only_expired_subscriptions() {
    let target = Target::from_env();
    let fixture = fixture();
    let profile_csrf = login_web(
        &target,
        &fixture.push_web_email,
        &fixture.push_web_household_slug,
        "198.51.100.86",
    );
    let path = format!(
        "/households/{}/push_subscription",
        fixture.push_web_household_slug
    );
    for outcome in ["accepted", "transient", "expired"] {
        let endpoint = endpoint("web", outcome, fixture.push_web_household_id);
        assert_eq!(
            web_form(
                &target,
                &path,
                &profile_csrf,
                &[
                    ("endpoint", &endpoint),
                    ("keys[p256dh]", "contract-public-key"),
                    ("keys[auth]", "contract-auth-secret"),
                ],
            ),
            201
        );
    }
    assert_eq!(
        web_form(&target, &format!("{path}/test"), &profile_csrf, &[]),
        204
    );
    assert_provider_effects(&target, &fixture, "web", fixture.push_web_household_id);
}

#[test]
fn sender_failure_returns_route_specific_service_unavailable_errors() {
    let fixture = fixture();
    let target = Target::from_env();
    let api_test = format!("{}/test", api_path(fixture.push_fatal_household_id));
    let failed =
        target.post_json_authorized(&api_test, &fixture.push_fatal_access_token, &json!({}));
    assert_eq!(failed.status().as_u16(), 503);
    let request_id = failed.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let body: serde_json::Value = failed.json().expect("API error JSON");
    assert_eq!(body["error"]["code"], "push_test_failed");
    assert_eq!(body["error"]["request_id"], request_id);

    let web = Target::from_env();
    let csrf = login_web(
        &web,
        &fixture.push_fatal_email,
        &fixture.push_fatal_household_slug,
        "198.51.100.87",
    );
    let web_test = format!(
        "/households/{}/push_subscription/test",
        fixture.push_fatal_household_slug
    );
    let response = web.web_form_request(
        "POST",
        &web_test,
        "application/json",
        &[("authenticity_token".to_owned(), csrf)],
    );
    assert_eq!(response.status().as_u16(), 503);
    let body: serde_json::Value = response.json().expect("web error JSON");
    assert_eq!(body["error"], "Unable to send test notification.");
}
