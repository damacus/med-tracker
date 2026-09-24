use medtracker_contract_tests::Target;
use serde_json::Value;

#[test]
fn capabilities_are_public() {
    let response = Target::from_env().get("/api/v1/capabilities", None);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response
        .headers()
        .get("cache-control")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("no-store"));
    let body: Value = response.json().expect("JSON capabilities");
    assert_eq!(body["data"]["format"], "medtracker.api.capabilities.v1");
    assert_eq!(body["data"]["api_version"], "v1");
    assert!(body["data"]["authentication"]["methods"]
        .as_array()
        .unwrap()
        .contains(&Value::from("oauth_bearer")));
}
