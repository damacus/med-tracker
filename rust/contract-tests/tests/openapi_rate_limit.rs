use medtracker_contract_tests::fixture;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn location_requests_from_a_non_loopback_peer_receive_documented_rate_limit_response() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let path = format!(
        "{base}/api/v1/households/{}/locations",
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
            return;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    panic!("no 429 within 601 requests in under 60 seconds");
}
