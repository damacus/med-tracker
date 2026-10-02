use medtracker_contract_tests::fixture;
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::json;
use serde_json::Value;
use std::env;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn location_requests_from_a_non_loopback_peer_receive_documented_rate_limit_response() {
    let fixture = fixture();
    let origin = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let path = format!(
        "{origin}/api/v1/households/{}/locations",
        fixture.household_id
    );
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("HTTP client");
    let started = Instant::now();
    for _ in 0..601 {
        let response = client
            .get(&path)
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("location request");
        if response.status().as_u16() == 429 {
            assert!(started.elapsed() < Duration::from_secs(60));
            for header in [
                "retry-after",
                "ratelimit-limit",
                "ratelimit-remaining",
                "ratelimit-reset",
            ] {
                assert!(response.headers().get(header).is_some(), "missing {header}");
            }
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            let limit: u64 = response.headers()["ratelimit-limit"]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            let retry_after: u64 = response.headers()["retry-after"]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            let reset: u64 = response.headers()["ratelimit-reset"]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            let received = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            assert_eq!(limit, 300);
            assert!((1..=300).contains(&retry_after));
            assert_eq!(reset % 300, 0);
            assert!(reset >= received && reset <= received + 300);
            assert!(reset.saturating_sub(retry_after) <= received + 2);
            assert_eq!(response.headers()["content-type"], "application/json");
            let body: Value = response.json().expect("rate limit JSON");
            let object = body.as_object().expect("rate envelope");
            assert_eq!(object.len(), 1);
            let error = object["error"].as_object().expect("rate error");
            assert_eq!(error.len(), 2);
            assert_eq!(error["code"], "rate_limited");
            assert!(error["message"]
                .as_str()
                .is_some_and(|text| !text.is_empty()));
            let household = format!("/api/v1/households/{}", fixture.household_id);
            let location = format!(
                "{household}/locations/{}",
                fixture.primary_location_portable_id
            );
            let collection = format!("{household}/locations");
            let people = format!("{household}/people");
            let person = format!("{people}/{}", fixture.managed_person_id);
            let invitations = format!("{household}/admin/invitations");
            let membership_collection = format!("{location}/location_memberships");
            let routes = [
                (Method::GET, collection.clone(), None, false),
                (
                    Method::POST,
                    collection,
                    Some(json!({"location": {"name": "Rate limited location"}})),
                    false,
                ),
                (Method::GET, location.clone(), None, false),
                (
                    Method::PATCH,
                    location.clone(),
                    Some(json!({"location": {"name": "Rate limited patch"}})),
                    true,
                ),
                (Method::DELETE, location.clone(), None, true),
                (
                    Method::PUT,
                    location,
                    Some(json!({"location": {"name": "Rate limited put"}})),
                    true,
                ),
                (
                    Method::POST,
                    membership_collection.clone(),
                    Some(
                        json!({"location_membership": {"person_id": fixture.managed_person_id.to_string()}}),
                    ),
                    false,
                ),
                (
                    Method::DELETE,
                    format!("{membership_collection}/1"),
                    None,
                    false,
                ),
                (
                    Method::GET,
                    "/api/v1/auth/households".to_owned(),
                    None,
                    false,
                ),
                (Method::GET, people, None, false),
                (Method::GET, person, None, false),
                (Method::GET, invitations.clone(), None, false),
                (
                    Method::POST,
                    invitations.clone(),
                    Some(json!({"household_invitation": {
                        "email": format!("rate-limited-{}@example.test", fixture.invitation_accept_id),
                        "membership_role": "member"
                    }})),
                    false,
                ),
                (
                    Method::DELETE,
                    format!("{invitations}/{}", fixture.invitation_accept_id),
                    None,
                    false,
                ),
            ];
            for (method, route, body, versioned) in routes {
                let mut request = client
                    .request(method, format!("{origin}{route}"))
                    .bearer_auth(&fixture.access_token);
                if let Some(body) = body {
                    request = request.json(&body);
                }
                if versioned {
                    request = request.header("If-Match", "\"0\"");
                }
                let response = request.send().expect("rate-limited endpoint request");
                assert_eq!(response.status().as_u16(), 429, "{route}");
            }
            return;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    panic!("no 429 within 601 requests in under 60 seconds");
}
