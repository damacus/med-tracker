use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use medtracker_contract_tests::{fixture, Target};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use url::form_urlencoded;

const NHS_PORT: u16 = 20090;
const AI_PORT: u16 = 20091;
const PUSH_PORT: u16 = 20092;
const SOURCE_PORT: u16 = 20089;
const FCM_PORT: u16 = 20097;
const APNS_PORT: u16 = 20088;
const OPF_PORT: u16 = 20086;
const OFF_PORT: u16 = 20085;
const AI_SUCCESS: u8 = 0;
const AI_INVENTED_SOURCE: u8 = 1;
const AI_UNAVAILABLE: u8 = 2;
const AI_INVALID_RESPONSE: u8 = 3;
const SOURCE_SUCCESS: u8 = 0;
const SOURCE_REDIRECT: u8 = 1;
const SOURCE_MISSING: u8 = 2;
const CALPOL_SOURCE: &str =
    "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol";

#[derive(Clone, Debug)]
struct RecordedRequest {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct FakeResponse {
    status: u16,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
}

impl FakeResponse {
    fn json(status: u16, value: Value) -> Self {
        Self {
            status,
            body: value.to_string().into_bytes(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
        }
    }

    fn html(status: u16, value: &str) -> Self {
        Self {
            status,
            body: value.as_bytes().to_vec(),
            headers: vec![(
                "Content-Type".to_owned(),
                "text/html; charset=utf-8".to_owned(),
            )],
        }
    }
}

struct FakeHttpServer {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl FakeHttpServer {
    fn start(
        port: u16,
        handler: impl Fn(&RecordedRequest) -> FakeResponse + Send + Sync + 'static,
    ) -> Self {
        let listener =
            TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)))
                .expect("bind deterministic provider fake");
        listener
            .set_nonblocking(true)
            .expect("configure nonblocking provider listener");
        let address = listener.local_addr().expect("provider fake address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let handler = Arc::new(handler);
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if let Some(request) = read_request(&mut stream) {
                            recorded
                                .lock()
                                .expect("provider request lock")
                                .push(request.clone());
                            write_response(&mut stream, handler(&request));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("provider request lock").clone()
    }
}

impl Drop for FakeHttpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("provider fake worker");
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        let mut chunk = [0_u8; 8192];
        let count = stream.read(&mut chunk).ok()?;
        if count == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() > 1_048_576 {
            return None;
        }
    };
    let header_text = String::from_utf8_lossy(&bytes[..header_end - 4]);
    let mut lines = header_text.split("\r\n");
    let mut request_line = lines.next()?.split_whitespace();
    let method = request_line.next()?.to_owned();
    let target = request_line.next()?.to_owned();
    let headers: HashMap<String, String> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    while bytes.len() < header_end + content_length {
        let mut chunk = [0_u8; 8192];
        let count = stream.read(&mut chunk).ok()?;
        if count == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    Some(RecordedRequest {
        method,
        target,
        headers,
        body: bytes[header_end..header_end + content_length].to_vec(),
    })
}

fn write_response(stream: &mut TcpStream, response: FakeResponse) {
    let reason = match response.status {
        201 => "Created",
        302 => "Found",
        404 => "Not Found",
        410 => "Gone",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let mut headers = response.headers;
    headers.push(("Content-Length".to_owned(), response.body.len().to_string()));
    headers.push(("Connection".to_owned(), "close".to_owned()));
    let mut bytes = format!("HTTP/1.1 {} {}\r\n", response.status, reason).into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&response.body);
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
}

fn source_payload(mode: u8) -> Value {
    if mode == AI_INVENTED_SOURCE {
        return json!({
            "medication": {"name": "Calpol SixPlus"},
            "doses": [{
                "amount": "5", "unit": "ml", "default_max_daily_doses": 4,
                "default_min_hours_between_doses": "4", "default_dose_cycle": "daily",
                "evidence": {"url": "https://www.calpol.co.uk/our-products/invented-advice",
                    "title": "Invented guidance", "text": "Unsupported source text"}
            }],
            "sources": [{"url": "https://www.calpol.co.uk/our-products/invented-advice",
                "title": "Invented guidance"}],
            "errors": []
        });
    }
    json!({
        "medication": {"name": "Calpol SixPlus"},
        "doses": [{
            "amount": "5", "unit": "ml", "default_max_daily_doses": 4,
            "default_min_hours_between_doses": "4", "default_dose_cycle": "daily",
            "evidence": {"url": CALPOL_SOURCE, "title": "Model supplied title",
                "text": "Children 6-8 years: 5 ml, up to 4 doses in 24 hours. Leave at least 4 hours between doses."}
        }],
        "sources": [{"url": CALPOL_SOURCE, "title": "Model supplied title"}],
        "errors": []
    })
}

fn ai_response_content(mode: u8) -> String {
    source_payload(mode).to_string()
}

fn json_from_ai_response(request: &RecordedRequest, mode: u8) -> FakeResponse {
    if mode == AI_UNAVAILABLE {
        return FakeResponse::json(503, json!({"error": "contract provider unavailable"}));
    }
    let content = if mode == AI_INVALID_RESPONSE {
        "not JSON".to_owned()
    } else {
        ai_response_content(mode)
    };
    let body = if request.target.starts_with("/v1/messages") {
        json!({"content": [{"type": "text", "text": content}]})
    } else if request.target.starts_with("/v1beta/models/") {
        json!({"candidates": [{"content": {"parts": [{"text": content}]}}]})
    } else {
        json!({"choices": [{"message": {"content": content}}]})
    };
    FakeResponse::json(200, body)
}

fn source_response(request: &RecordedRequest, mode: u8) -> FakeResponse {
    if mode == SOURCE_REDIRECT {
        return FakeResponse {
            status: 302,
            body: Vec::new(),
            headers: vec![(
                "Location".to_owned(),
                "http://127.0.0.1:20087/sink".to_owned(),
            )],
        };
    }
    let query = request
        .target
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default();
    let requested_url = form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "url")
        .map(|(_, value)| value.into_owned());
    if mode == SOURCE_MISSING || requested_url.as_deref() != Some(CALPOL_SOURCE) {
        return FakeResponse::json(404, json!({"error": "trusted source missing"}));
    }
    FakeResponse::html(
        200,
        "<html><head><title>Verified Calpol guidance</title></head><body><main>Children 6-8 years: 5 ml, up to 4 doses in 24 hours. Leave at least 4 hours between doses.</main></body></html>",
    )
}

fn nhs_response(request: &RecordedRequest) -> FakeResponse {
    if request.method == "POST" && request.target.starts_with("/authorisation/") {
        return FakeResponse::json(
            200,
            json!({"access_token": "contract-nhs-token", "expires_in": 3600}),
        );
    }
    if request.method == "GET"
        && request
            .target
            .starts_with("/production1/fhir/ValueSet/$expand")
    {
        let query = request
            .target
            .split_once('?')
            .map(|(_, query)| query)
            .unwrap_or_default();
        let parameters: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
        let contains = if parameters.get("filter").map(String::as_str) != Some("contractupstream") {
            Vec::new()
        } else if parameters.get("url").map(String::as_str)
            == Some("https://dmd.nhs.uk/ValueSet/VMP")
        {
            vec![
                json!({"code": "contract-upstream-250", "display": "Contractupstream 250mg/5ml oral suspension", "system": "https://dmd.nhs.uk", "extension": [{"url": "http://hl7.org/fhir/StructureDefinition/valueset-concept-comments", "valueString": "AMPP"}]}),
                json!({"code": "contract-upstream-500", "display": "Contractupstream 500mg tablets", "system": "https://dmd.nhs.uk", "extension": [{"url": "http://hl7.org/fhir/StructureDefinition/valueset-concept-comments", "valueString": "VMPP"}]}),
            ]
        } else {
            vec![
                json!({"code": "contract-upstream-125", "display": "Contractupstream 125mg/5ml oral suspension", "system": "https://dmd.nhs.uk", "extension": [{"url": "http://hl7.org/fhir/StructureDefinition/valueset-concept-comments", "valueString": "AMPP"}]}),
            ]
        };
        return FakeResponse::json(200, json!({"expansion": {"contains": contains}}));
    }
    FakeResponse::json(404, json!({"error": "unexpected NHS request"}))
}

fn opf_response(_request: &RecordedRequest) -> FakeResponse {
    FakeResponse::json(
        200,
        json!({"status": 1, "product": {
            "product_name": "Contract ibuprofen 200 mg tablets",
            "brands": "Wire brand",
            "quantity": "16 tablets",
            "categories_tags_en": ["en:medicines"]
        }}),
    )
}

fn off_product() -> Value {
    json!({
        "product_name": "Contract vitamin C",
        "generic_name": "Vitamin C dietary supplement",
        "brands": "Wire supplement brand",
        "quantity": "60 tablets",
        "categories_tags_en": ["en:dietary-supplements", "en:vitamins"]
    })
}

fn off_response(request: &RecordedRequest) -> FakeResponse {
    if request.target.starts_with("/cgi/search.pl?") {
        FakeResponse::json(
            200,
            json!({"products": [{"code": "5000000000001", "product": off_product()}]}),
        )
    } else if request.target.starts_with("/api/v2/product/") {
        let barcode = request
            .target
            .split("/api/v2/product/")
            .nth(1)
            .and_then(|value| value.split('.').next())
            .unwrap_or_default();
        FakeResponse::json(
            200,
            json!({"status": 1, "code": barcode, "product": off_product()}),
        )
    } else {
        FakeResponse::json(404, json!({"error": "unexpected OFF request"}))
    }
}

fn push_response(request: &RecordedRequest) -> FakeResponse {
    if request.target.contains("/push/expired-") {
        FakeResponse::json(410, json!({"error": "subscription expired"}))
    } else if request.target.contains("/push/transient-") {
        FakeResponse::json(500, json!({"error": "temporary provider failure"}))
    } else {
        FakeResponse {
            status: 201,
            body: Vec::new(),
            headers: Vec::new(),
        }
    }
}

fn fcm_response(request: &RecordedRequest) -> FakeResponse {
    let payload = json_body(request);
    let token = payload
        .pointer("/message/token")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if token.contains("unregistered") {
        FakeResponse::json(404, json!({"error": {"status": "UNREGISTERED"}}))
    } else if token.contains("transient") {
        FakeResponse::json(500, json!({"error": {"status": "INTERNAL"}}))
    } else {
        FakeResponse::json(
            200,
            json!({"name": "projects/contract-project/messages/contract-message"}),
        )
    }
}

fn apns_response(request: &RecordedRequest) -> FakeResponse {
    if request.target.contains("ios-gone-") {
        FakeResponse::json(410, json!({"reason": "Unregistered"}))
    } else if request.target.contains("ios-transient-") {
        FakeResponse::json(500, json!({"reason": "InternalServerError"}))
    } else {
        FakeResponse::json(200, json!({}))
    }
}

fn json_body(request: &RecordedRequest) -> Value {
    serde_json::from_slice(&request.body).expect("provider request JSON")
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("HTTP client")
}

fn request_id(response: &reqwest::blocking::Response) -> String {
    response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .expect("response request ID")
        .to_owned()
}

fn assert_external_audit(
    response: &reqwest::blocking::Response,
    household_id: i64,
    item_type: &str,
    event: &str,
    expected_status: &str,
    expected_count_key: &str,
    expected_count: u64,
    private_value: &str,
) {
    let request_id = request_id(response);
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let rows = database
        .query(
            "SELECT item_type, item_id, household_id, event, object, request_id, (SELECT household_id FROM household_memberships WHERE id = versions.actor_membership_id) FROM versions WHERE request_id = $1 AND item_type = $2 AND event = $3",
            &[&request_id, &item_type, &event],
        )
        .expect("external provider audit versions");
    assert_eq!(rows.len(), 1, "one correlated {event} version");
    let row = &rows[0];
    assert_eq!(row.get::<_, String>(0), item_type);
    assert_eq!(row.get::<_, i64>(1), 0);
    assert_eq!(row.get::<_, i64>(2), household_id);
    assert_eq!(row.get::<_, String>(3), event);
    assert_eq!(row.get::<_, String>(5), request_id);
    assert_eq!(
        row.get::<_, i64>(6),
        household_id,
        "actor belongs to audited household"
    );
    let object_text: String = row.get(4);
    assert!(
        !object_text.contains(private_value),
        "audit excludes raw input"
    );
    let object: Value = serde_json::from_str(&object_text).expect("audit object JSON");
    let hash_key = if item_type == "ExternalMedicineLookup" {
        "query_hash"
    } else {
        "identity_hash"
    };
    let hash = object[hash_key].as_str().expect("one-way input hash");
    assert_eq!(hash.len(), 64);
    assert!(hash
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    assert_eq!(object["result_status"], expected_status);
    assert_eq!(object[expected_count_key], expected_count);
}

fn api_post_json(
    base_url: &str,
    path: &str,
    token: &str,
    body: &Value,
) -> reqwest::blocking::Response {
    client()
        .post(format!("{}{}", base_url.trim_end_matches('/'), path))
        .bearer_auth(token)
        .json(body)
        .send()
        .expect("API must respond")
}

fn hex_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("hex text"), 16).expect("hex byte")
        })
        .collect()
}

fn base64url(hex: &str) -> String {
    URL_SAFE_NO_PAD.encode(hex_bytes(hex))
}

fn post_without_body(url: &str, token: &str) -> reqwest::blocking::Response {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("HTTP client")
        .post(url)
        .bearer_auth(token)
        .send()
        .expect("target must respond")
}

#[test]
fn medication_lookup_route_returns_an_empty_response_for_an_empty_query() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/medication_lookup",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let payload: serde_json::Value = response.json().expect("lookup JSON");
    assert_eq!(payload["results"], json!([]));
    assert_eq!(payload["permissions"]["can_create"], true);
}

#[test]
fn paid_ai_suggestions_route_returns_a_draft_response() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/ai_medication_suggestions",
        fixture.lookup_paid_household_id
    );
    let response = target.post_json_authorized(
        &path,
        &fixture.lookup_paid_access_token,
        &json!({"medication": {"name": "Contract medicine"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let payload: serde_json::Value = response.json().expect("suggestion JSON");
    assert!(payload["data"]["medication"].is_object());
    assert!(payload["data"]["doses"].is_array());
    assert!(payload["data"]["sources"].is_array());
    assert!(payload["data"]["errors"].is_array());
}

#[test]
fn push_test_route_accepts_a_bodyless_request_and_returns_no_content() {
    let fixture = fixture();
    let url = format!(
        "{}/api/v1/households/{}/push_subscription/test",
        env::var("CONTRACT_BASE_URL").expect("contract API URL"),
        fixture.push_api_household_id
    );
    let response = post_without_body(&url, &fixture.push_api_access_token);
    assert_eq!(response.status().as_u16(), 204);
    assert!(response
        .bytes()
        .expect("empty push test response")
        .is_empty());
}

#[test]
fn medication_lookup_uses_oauth_and_both_nhs_value_sets() {
    let server = FakeHttpServer::start(NHS_PORT, nhs_response);
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/medication_lookup?q=contractupstream",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_external_audit(
        &response,
        fixture.household_id,
        "ExternalMedicineLookup",
        "nhs_dmd/search",
        "success",
        "result_count",
        3,
        "contractupstream",
    );
    let payload: Value = response.json().expect("lookup JSON");
    let results = payload["results"].as_array().expect("lookup results");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["code"], "contract-upstream-250");
    assert_eq!(results[2]["code"], "contract-upstream-125");

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].method, "POST");
    assert!(requests[0].target.starts_with(
        "/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token"
    ));
    let form: HashMap<String, String> = form_urlencoded::parse(&requests[0].body)
        .into_owned()
        .collect();
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("client_credentials")
    );
    assert_eq!(
        form.get("client_id").map(String::as_str),
        Some("contract-nhs-id")
    );
    assert_eq!(
        form.get("client_secret").map(String::as_str),
        Some("contract-nhs-secret")
    );
    let mut value_sets = Vec::new();
    for request in &requests[1..] {
        assert_eq!(request.method, "GET");
        assert!(request
            .target
            .starts_with("/production1/fhir/ValueSet/$expand?"));
        assert_eq!(
            request.headers.get("authorization").map(String::as_str),
            Some("Bearer contract-nhs-token")
        );
        assert_eq!(
            request.headers.get("accept").map(String::as_str),
            Some("application/json")
        );
        let query: HashMap<String, String> = form_urlencoded::parse(
            request
                .target
                .split_once('?')
                .expect("FHIR query")
                .1
                .as_bytes(),
        )
        .into_owned()
        .collect();
        assert_eq!(
            query.get("filter").map(String::as_str),
            Some("contractupstream")
        );
        assert_eq!(query.get("count").map(String::as_str), Some("20"));
        value_sets.push(query.get("url").cloned().expect("value set URL"));
    }
    value_sets.sort();
    assert_eq!(
        value_sets,
        [
            "https://dmd.nhs.uk/ValueSet/AMP",
            "https://dmd.nhs.uk/ValueSet/VMP"
        ]
    );
}

#[test]
fn ai_providers_use_their_protocols_and_only_return_fetched_sources() {
    let source_mode = Arc::new(AtomicU8::new(SOURCE_SUCCESS));
    let source_state = source_mode.clone();
    let source_server = FakeHttpServer::start(SOURCE_PORT, move |request| {
        source_response(request, source_state.load(Ordering::Relaxed))
    });
    let ai_mode = Arc::new(AtomicU8::new(AI_SUCCESS));
    let ai_state = ai_mode.clone();
    let ai_server = FakeHttpServer::start(AI_PORT, move |request| {
        json_from_ai_response(request, ai_state.load(Ordering::Relaxed))
    });
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/ai_medication_suggestions",
        fixture.lookup_paid_household_id
    );
    let identity = json!({"medication": {"name": "Calpol Six Plus"}});
    let providers = [
        (
            "CONTRACT_AI_OPENAI_BASE_URL",
            "gpt-4o-mini",
            "/v1/chat/completions",
            "authorization",
        ),
        (
            "CONTRACT_AI_OPENROUTER_BASE_URL",
            "openai/gpt-4o-mini",
            "/v1/chat/completions",
            "authorization",
        ),
        (
            "CONTRACT_AI_ANTHROPIC_BASE_URL",
            "claude-sonnet-4-20250514",
            "/v1/messages",
            "x-api-key",
        ),
        (
            "CONTRACT_AI_GEMINI_BASE_URL",
            "gemini-2.5-flash",
            "/v1beta/models/gemini-2.5-flash:generateContent",
            "gemini-key",
        ),
    ];
    let http = client();
    for (base_env, model, expected_path, auth_header) in providers {
        let base = env::var(base_env).expect("provider-specific API base URL");
        let response = api_post_json(&base, &path, &fixture.lookup_paid_access_token, &identity);
        assert_eq!(response.status().as_u16(), 200, "{base_env}");
        assert_external_audit(
            &response,
            fixture.lookup_paid_household_id,
            "AiMedicationSuggestion",
            "ai_medication/suggestion",
            "found",
            "source_count",
            1,
            "Calpol Six Plus",
        );
        assert_external_audit(
            &response,
            fixture.lookup_paid_household_id,
            "AiMedicationSuggestion",
            "ai_medication/suggestion",
            "found",
            "dose_count",
            1,
            "Calpol Six Plus",
        );
        let payload: Value = response.json().expect("AI suggestion JSON");
        assert_eq!(payload["data"]["medication"]["name"], "Calpol SixPlus");
        assert_eq!(payload["data"]["doses"][0]["amount"], "5");
        assert_eq!(
            payload["data"]["doses"][0]["evidence"]["url"],
            CALPOL_SOURCE
        );
        assert_eq!(
            payload["data"]["doses"][0]["evidence"]["title"],
            "Verified Calpol guidance"
        );
        assert!(payload["data"]["doses"][0]["evidence"]["text"]
            .as_str()
            .is_some_and(|text| text.contains("Children 6-8 years")));
        assert_eq!(payload["data"]["sources"][0]["url"], CALPOL_SOURCE);
        assert_eq!(
            payload["data"]["sources"][0]["title"],
            "Verified Calpol guidance"
        );
        assert_eq!(payload["data"]["errors"], json!([]));
        let requests = ai_server.requests();
        let request = requests.last().expect("provider request");
        assert!(
            request.target.starts_with(expected_path),
            "{}: {}",
            base_env,
            request.target
        );
        let body = json_body(request);
        if auth_header == "gemini-key" {
            assert!(request.target.starts_with(expected_path));
            assert_eq!(
                request.headers.get("x-goog-api-key").map(String::as_str),
                Some("contract-provider-key")
            );
            assert!(
                !request.target.contains("key="),
                "Gemini key must not be placed in the URL"
            );
            assert!(body.get("contents").is_some());
        } else {
            assert_eq!(body["model"], model);
        }
        assert!(body.to_string().contains("Children 6-8 years"));
        match auth_header {
            "authorization" => assert_eq!(
                request.headers.get(auth_header).map(String::as_str),
                Some("Bearer contract-provider-key")
            ),
            "x-api-key" => assert_eq!(
                request.headers.get(auth_header).map(String::as_str),
                Some("contract-provider-key")
            ),
            "gemini-key" => {}
            _ => unreachable!(),
        }
    }
    assert_eq!(ai_server.requests().len(), 4);
    assert_eq!(source_server.requests().len(), 4);

    let malformed = http
        .post(format!(
            "{}{}",
            env::var("CONTRACT_AI_OPENAI_BASE_URL")
                .expect("OpenAI API base URL")
                .trim_end_matches('/'),
            path
        ))
        .bearer_auth(&fixture.lookup_paid_access_token)
        .json(&json!({"medication": {"name": "Calpol Six Plus", "unlisted": "reject"}}))
        .send()
        .expect("invalid AI identity response");
    assert_eq!(malformed.status().as_u16(), 422);
    assert_eq!(
        ai_server.requests().len(),
        4,
        "invalid identity must not reach provider"
    );
}

#[test]
fn ai_drops_invented_sources_and_reports_missing_or_redirected_evidence() {
    let source_mode = Arc::new(AtomicU8::new(SOURCE_SUCCESS));
    let source_state = source_mode.clone();
    let source_server = FakeHttpServer::start(SOURCE_PORT, move |request| {
        source_response(request, source_state.load(Ordering::Relaxed))
    });
    let ai_mode = Arc::new(AtomicU8::new(AI_INVENTED_SOURCE));
    let ai_state = ai_mode.clone();
    let _ai_server = FakeHttpServer::start(AI_PORT, move |request| {
        json_from_ai_response(request, ai_state.load(Ordering::Relaxed))
    });
    let fixture = fixture();
    let base = env::var("CONTRACT_AI_OPENAI_BASE_URL").expect("OpenAI API base URL");
    let path = format!(
        "/api/v1/households/{}/ai_medication_suggestions",
        fixture.lookup_paid_household_id
    );
    let identity = json!({"medication": {"name": "Calpol Six Plus"}});

    let response = api_post_json(&base, &path, &fixture.lookup_paid_access_token, &identity);
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("AI suggestion JSON");
    assert_eq!(payload["data"]["doses"], json!([]));
    assert_eq!(payload["data"]["sources"], json!([]));
    assert!(!payload.to_string().contains("invented-advice"));
    assert_eq!(source_server.requests().len(), 1);

    ai_mode.store(AI_UNAVAILABLE, Ordering::Relaxed);
    let response = api_post_json(&base, &path, &fixture.lookup_paid_access_token, &identity);
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("AI provider failure draft");
    assert_eq!(payload["data"]["medication"], json!({}));
    assert_eq!(payload["data"]["errors"], json!(["suggestion_unavailable"]));

    ai_mode.store(AI_INVALID_RESPONSE, Ordering::Relaxed);
    let response = api_post_json(&base, &path, &fixture.lookup_paid_access_token, &identity);
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("invalid AI response draft");
    assert_eq!(payload["data"]["errors"], json!(["invalid_model_response"]));

    ai_mode.store(AI_SUCCESS, Ordering::Relaxed);
    source_mode.store(SOURCE_MISSING, Ordering::Relaxed);
    let response = api_post_json(&base, &path, &fixture.lookup_paid_access_token, &identity);
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("missing source draft");
    assert_eq!(
        payload["data"],
        json!({"medication": {}, "doses": [], "sources": [], "errors": ["trusted_source_unavailable"]})
    );

    let redirect_sink = FakeHttpServer::start(20087, |_| {
        FakeResponse::json(200, json!({"unexpected": true}))
    });
    source_mode.store(SOURCE_REDIRECT, Ordering::Relaxed);
    let response = api_post_json(&base, &path, &fixture.lookup_paid_access_token, &identity);
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("redirected source draft");
    assert_eq!(
        payload["data"]["errors"],
        json!(["trusted_source_unavailable"])
    );
    assert!(
        redirect_sink.requests().is_empty(),
        "untrusted redirect target must not be fetched"
    );
}

#[test]
fn push_test_sends_encrypted_web_push_and_prunes_only_expired_subscriptions() {
    let push_server = FakeHttpServer::start(PUSH_PORT, push_response);
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/push_subscription",
        fixture.push_api_household_id
    );
    let public_key = base64url(
        "047cf27b188d034f7e8a52380304b51ac3c08969e277f21b35a60b48fc4766997807775510db8ed040293d9ac69f7430dbba7dade63ce982299e04b79d227873d1",
    );
    let auth_secret = URL_SAFE_NO_PAD.encode([0x42_u8; 16]);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let endpoints = ["accepted", "transient", "expired"]
        .map(|outcome| format!("http://127.0.0.1:{PUSH_PORT}/push/{outcome}-{unique}"));
    for endpoint in &endpoints {
        let response = target.post_json_authorized(
            &path,
            &fixture.push_api_access_token,
            &json!({"push_subscription": {"endpoint": endpoint, "keys": {"p256dh": public_key, "auth": auth_secret}}}),
        );
        assert_eq!(response.status().as_u16(), 201);
    }
    let url = format!(
        "{}/api/v1/households/{}/push_subscription/test",
        env::var("CONTRACT_BASE_URL").expect("contract API URL"),
        fixture.push_api_household_id
    );
    let response = post_without_body(&url, &fixture.push_api_access_token);
    assert_eq!(response.status().as_u16(), 204);

    let requests = push_server.requests();
    assert_eq!(requests.len(), 3);
    for request in &requests {
        assert_eq!(request.method, "POST");
        assert_eq!(
            request.headers.get("content-encoding").map(String::as_str),
            Some("aes128gcm")
        );
        assert!(request
            .headers
            .get("authorization")
            .is_some_and(|value| value.starts_with("vapid t=")));
        assert!(!request.body.is_empty());
        assert!(!String::from_utf8_lossy(&request.body).contains("MedTracker"));
    }
    let received_paths = requests
        .iter()
        .map(|request| request.target.clone())
        .collect::<Vec<_>>();
    for endpoint in &endpoints {
        let path = endpoint.trim_start_matches(&format!("http://127.0.0.1:{PUSH_PORT}"));
        assert!(received_paths
            .iter()
            .any(|received| received.starts_with(path)));
    }
    for endpoint in &endpoints[..2] {
        assert!(
            push_subscription_exists(endpoint),
            "successful and transient subscriptions remain stored"
        );
    }
    assert!(
        !push_subscription_exists(&endpoints[2]),
        "410 removes only the expired subscription"
    );
}

#[test]
fn push_test_sends_fcm_and_apns_requests_and_prunes_only_permanent_failures() {
    let fcm_server = FakeHttpServer::start(FCM_PORT, fcm_response);
    let apns_server = FakeHttpServer::start(APNS_PORT, apns_response);
    let target = Target::from_env();
    let fixture = fixture();
    let native_path = format!(
        "/api/v1/households/{}/native_device_tokens",
        fixture.push_api_household_id
    );
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let tokens = [
        format!("android-accepted-{unique}"),
        format!("android-unregistered-{unique}"),
        format!("android-transient-{unique}"),
        format!("ios-accepted-{unique}"),
        format!("ios-gone-{unique}"),
        format!("ios-transient-{unique}"),
    ];
    for (index, token) in tokens.iter().enumerate() {
        let platform = if index < 3 { "android" } else { "ios" };
        let mut attributes = json!({"device_token": token, "platform": platform});
        if platform == "ios" {
            attributes["apns_environment"] = json!("sandbox");
        }
        let response = target.post_json_authorized(
            &native_path,
            &fixture.push_api_access_token,
            &json!({"native_device_token": attributes}),
        );
        assert_eq!(response.status().as_u16(), 201);
        assert!(response
            .text()
            .expect("empty registration response")
            .is_empty());
    }
    let fixture_account_id = api_session_account_id(&fixture.push_api_access_token);
    for token in &tokens {
        assert_eq!(native_device_token_account(token), Some(fixture_account_id));
    }
    assert_push_household_active(fixture.push_api_household_id);
    let test_path = format!(
        "{}/api/v1/households/{}/push_subscription/test",
        env::var("CONTRACT_BASE_URL").expect("contract API URL"),
        fixture.push_api_household_id
    );
    let response = post_without_body(&test_path, &fixture.push_api_access_token);
    assert_eq!(response.status().as_u16(), 204);

    let fcm_requests = fcm_server.requests();
    assert_eq!(fcm_requests.len(), 3);
    for request in &fcm_requests {
        assert_eq!(request.method, "POST");
        assert_eq!(
            request.target,
            "/v1/projects/contract-project/messages:send"
        );
        assert_eq!(
            request.headers.get("authorization").map(String::as_str),
            Some("Bearer contract-fcm-token")
        );
        let body = json_body(request);
        assert!(body.pointer("/message/notification/title").is_some());
        assert!(body.pointer("/message/data/path").is_some());
    }
    let fcm_tokens = fcm_requests
        .iter()
        .map(|request| {
            json_body(request)
                .pointer("/message/token")
                .and_then(Value::as_str)
                .expect("FCM token")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fcm_tokens
            .iter()
            .filter(|token| token.starts_with("android-"))
            .count(),
        3
    );

    let apns_requests = apns_server.requests();
    assert_eq!(apns_requests.len(), 3);
    for request in &apns_requests {
        assert_eq!(request.method, "POST");
        assert!(request.target.starts_with("/3/device/ios-"));
        assert_eq!(
            request.headers.get("apns-topic").map(String::as_str),
            Some("io.contract.app")
        );
        assert_eq!(
            request.headers.get("apns-push-type").map(String::as_str),
            Some("alert")
        );
        assert_eq!(
            request.headers.get("apns-priority").map(String::as_str),
            Some("10")
        );
        let authorization = request.headers.get("authorization").expect("APNs JWT");
        let jwt = authorization
            .strip_prefix("bearer ")
            .expect("lowercase bearer scheme");
        let claims = jwt.split('.').collect::<Vec<_>>();
        assert_eq!(claims.len(), 3);
        let header: Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(claims[0])
                .expect("JWT header base64"),
        )
        .expect("JWT header JSON");
        let body: Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(claims[1])
                .expect("JWT claims base64"),
        )
        .expect("JWT claims JSON");
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["kid"], "CONTRACTKEY");
        assert_eq!(body["iss"], "CONTRACTTEAM");
        let push_body: Value = json_body(request);
        assert_eq!(push_body["aps"]["alert"]["title"], "MedTracker");
    }

    for token in [&tokens[0], &tokens[2], &tokens[3], &tokens[5]] {
        assert!(
            native_device_token_exists(token),
            "accepted or transient native token remains"
        );
    }
    for token in [&tokens[1], &tokens[4]] {
        assert!(
            !native_device_token_exists(token),
            "permanent provider failure removes token"
        );
    }
}

#[test]
fn push_test_succeeds_without_vapid_when_only_native_devices_are_registered() {
    let fcm_server = FakeHttpServer::start(FCM_PORT, fcm_response);
    let apns_server = FakeHttpServer::start(APNS_PORT, apns_response);
    let fixture = fixture();
    let base = env::var("CONTRACT_PUSH_FAILURE_BASE_URL")
        .expect("isolated API origin without VAPID configuration");
    let path = format!(
        "/api/v1/households/{}/native_device_tokens",
        fixture.push_fatal_household_id
    );
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let android = format!("android-no-vapid-{unique}");
    let ios = format!("ios-no-vapid-{unique}");
    for (token, platform) in [(&android, "android"), (&ios, "ios")] {
        let mut attributes = json!({"device_token": token, "platform": platform});
        if platform == "ios" {
            attributes["apns_environment"] = json!("sandbox");
        }
        let response = api_post_json(
            &base,
            &path,
            &fixture.push_fatal_access_token,
            &json!({"native_device_token": attributes}),
        );
        assert_eq!(response.status().as_u16(), 201);
    }
    let fixture_account_id = api_session_account_id(&fixture.push_fatal_access_token);
    for token in [&android, &ios] {
        assert_eq!(native_device_token_account(token), Some(fixture_account_id));
    }
    let test_url = format!(
        "{base}/api/v1/households/{}/push_subscription/test",
        fixture.push_fatal_household_id
    );
    assert_eq!(
        household_push_subscription_count(fixture.push_fatal_household_id),
        0
    );
    assert_push_household_active(fixture.push_fatal_household_id);
    let response = post_without_body(&test_url, &fixture.push_fatal_access_token);
    assert_eq!(response.status().as_u16(), 204);
    assert_eq!(fcm_server.requests().len(), 1);
    assert_eq!(apns_server.requests().len(), 1);
    assert!(native_device_token_exists(&android));
    assert!(native_device_token_exists(&ios));
}

#[test]
fn push_test_returns_a_structured_503_when_sender_configuration_is_missing() {
    let fixture = fixture();
    let target = Target::from_env();
    let subscription_path = format!(
        "/api/v1/households/{}/push_subscription",
        fixture.push_api_household_id
    );
    let public_key = base64url(
        "047cf27b188d034f7e8a52380304b51ac3c08969e277f21b35a60b48fc4766997807775510db8ed040293d9ac69f7430dbba7dade63ce982299e04b79d227873d1",
    );
    let auth_secret = URL_SAFE_NO_PAD.encode([0x42_u8; 16]);
    let endpoint = format!(
        "http://127.0.0.1:{PUSH_PORT}/push/missing-vapid-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    );
    let registration = target.post_json_authorized(
        &subscription_path,
        &fixture.push_api_access_token,
        &json!({"push_subscription": {"endpoint": endpoint, "keys": {"p256dh": public_key, "auth": auth_secret}}}),
    );
    assert_eq!(registration.status().as_u16(), 201);
    assert!(push_subscription_exists(&endpoint));
    let base = env::var("CONTRACT_PUSH_FAILURE_BASE_URL")
        .expect("isolated API origin without VAPID configuration");
    let path = format!(
        "{base}/api/v1/households/{}/push_subscription/test",
        fixture.push_api_household_id
    );
    let response = post_without_body(&path, &fixture.push_api_access_token);
    assert_eq!(response.status().as_u16(), 503);
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = response.json().expect("push error JSON");
    assert_eq!(body["error"]["code"], "push_test_failed");
    assert_eq!(
        body["error"]["message"],
        "Unable to send test notification."
    );
    let body_request_id = body["error"]["request_id"]
        .as_str()
        .expect("error request ID");
    assert!(!body_request_id.is_empty());
    if let Some(request_id) = request_id {
        assert_eq!(body["error"]["request_id"], request_id);
    }
    assert!(
        push_subscription_exists(&endpoint),
        "failed send retains web subscription"
    );
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("endpoint", &endpoint)
        .finish();
    let cleanup = target.delete(
        &format!("{subscription_path}?{query}"),
        Some(&fixture.push_api_access_token),
    );
    assert_eq!(cleanup.status().as_u16(), 204);
}

#[test]
fn push_test_returns_a_structured_503_when_the_vapid_private_key_is_malformed() {
    let push_server = FakeHttpServer::start(PUSH_PORT, push_response);
    let fixture = fixture();
    let target = Target::from_env();
    let subscription_path = format!(
        "/api/v1/households/{}/push_subscription",
        fixture.push_observer_household_id
    );
    let public_key = base64url(
        "047cf27b188d034f7e8a52380304b51ac3c08969e277f21b35a60b48fc4766997807775510db8ed040293d9ac69f7430dbba7dade63ce982299e04b79d227873d1",
    );
    let auth_secret = URL_SAFE_NO_PAD.encode([0x42_u8; 16]);
    let endpoint = format!(
        "http://127.0.0.1:{PUSH_PORT}/push/invalid-vapid-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    );
    let registration = target.post_json_authorized(
        &subscription_path,
        &fixture.push_observer_access_token,
        &json!({"push_subscription": {"endpoint": endpoint, "keys": {"p256dh": public_key, "auth": auth_secret}}}),
    );
    assert_eq!(registration.status().as_u16(), 201);
    assert!(push_subscription_exists(&endpoint));
    let base = env::var("CONTRACT_PUSH_INVALID_VAPID_BASE_URL")
        .expect("isolated API origin with malformed VAPID configuration");
    let path = format!(
        "{base}/api/v1/households/{}/push_subscription/test",
        fixture.push_observer_household_id
    );
    let response = post_without_body(&path, &fixture.push_observer_access_token);
    assert_eq!(response.status().as_u16(), 503);
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = response.json().expect("push error JSON");
    assert_eq!(body["error"]["code"], "push_test_failed");
    assert_eq!(
        body["error"]["message"],
        "Unable to send test notification."
    );
    let body_request_id = body["error"]["request_id"]
        .as_str()
        .expect("error request ID");
    assert!(!body_request_id.is_empty());
    if let Some(request_id) = request_id {
        assert_eq!(body["error"]["request_id"], request_id);
    }
    assert!(
        push_subscription_exists(&endpoint),
        "malformed VAPID retains web subscription"
    );
    assert!(
        push_server.requests().is_empty(),
        "malformed VAPID must not emit browser-push requests"
    );
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("endpoint", &endpoint)
        .finish();
    let cleanup = target.delete(
        &format!("{subscription_path}?{query}"),
        Some(&fixture.push_observer_access_token),
    );
    assert_eq!(cleanup.status().as_u16(), 204);
}

#[test]
fn push_test_succeeds_with_malformed_vapid_when_only_native_devices_are_registered() {
    let fcm_server = FakeHttpServer::start(FCM_PORT, fcm_response);
    let apns_server = FakeHttpServer::start(APNS_PORT, apns_response);
    let fixture = fixture();
    let base = env::var("CONTRACT_PUSH_INVALID_VAPID_BASE_URL")
        .expect("isolated API origin with malformed VAPID configuration");
    let path = format!(
        "/api/v1/households/{}/native_device_tokens",
        fixture.push_observer_household_id
    );
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let android = format!("android-invalid-vapid-{unique}");
    let ios = format!("ios-invalid-vapid-{unique}");
    for (token, platform) in [(&android, "android"), (&ios, "ios")] {
        let mut attributes = json!({"device_token": token, "platform": platform});
        if platform == "ios" {
            attributes["apns_environment"] = json!("sandbox");
        }
        let response = api_post_json(
            &base,
            &path,
            &fixture.push_observer_access_token,
            &json!({"native_device_token": attributes}),
        );
        assert_eq!(response.status().as_u16(), 201);
    }
    let fixture_account_id = api_session_account_id(&fixture.push_observer_access_token);
    for token in [&android, &ios] {
        assert_eq!(native_device_token_account(token), Some(fixture_account_id));
    }
    let test_url = format!(
        "{base}/api/v1/households/{}/push_subscription/test",
        fixture.push_observer_household_id
    );
    assert_eq!(
        household_push_subscription_count(fixture.push_observer_household_id),
        0
    );
    assert_push_household_active(fixture.push_observer_household_id);
    let response = post_without_body(&test_url, &fixture.push_observer_access_token);
    assert_eq!(response.status().as_u16(), 204);
    assert_eq!(fcm_server.requests().len(), 1);
    assert_eq!(apns_server.requests().len(), 1);
    assert!(native_device_token_exists(&android));
    assert!(native_device_token_exists(&ios));
}

fn push_subscription_exists(endpoint: &str) -> bool {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    database
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM push_subscriptions WHERE endpoint = $1)",
            &[&endpoint],
        )
        .expect("push subscription lookup")
        .get(0)
}

fn household_push_subscription_count(household_id: i64) -> i64 {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    database
        .query_one(
            "SELECT count(*) FROM push_subscriptions ps WHERE EXISTS (SELECT 1 FROM household_memberships hm WHERE hm.account_id = ps.account_id AND hm.household_id = $1)",
            &[&household_id],
        )
        .expect("household web push subscription count")
        .get(0)
}

fn native_device_token_exists(device_token: &str) -> bool {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    database
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM native_device_tokens WHERE device_token = $1)",
            &[&device_token],
        )
        .expect("native device token lookup")
        .get(0)
}

fn api_session_account_id(access_token: &str) -> i64 {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    database
        .query_one(
            "SELECT account_id FROM api_sessions WHERE access_token_digest = encode(digest($1, 'sha256'), 'hex') AND revoked_at IS NULL",
            &[&access_token],
        )
        .expect("fixture API session account")
        .get(0)
}

fn native_device_token_account(device_token: &str) -> Option<i64> {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    database
        .query_opt(
            "SELECT account_id FROM native_device_tokens WHERE device_token = $1",
            &[&device_token],
        )
        .expect("registered native token account")
        .map(|row| row.get(0))
}

fn assert_push_household_active(household_id: i64) {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let row = database
        .query_one(
            "SELECT status, lifecycle_state FROM households WHERE id = $1",
            &[&household_id],
        )
        .expect("push test household");
    assert_eq!(row.get::<_, String>(0), "active");
    assert_eq!(row.get::<_, String>(1), "active");
}

struct LookupSourcePriorityGuard {
    row_id: i64,
    original: Option<String>,
}

impl LookupSourcePriorityGuard {
    fn set(priority: &[&str]) -> Self {
        let mut database = postgres::Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
            postgres::NoTls,
        )
        .expect("contract database");
        let configured = serde_json::to_string(priority).expect("priority JSON");
        let existing = database
            .query_opt(
                "SELECT id, medicine_lookup_source_priority::text FROM app_settings ORDER BY id LIMIT 1",
                &[],
            )
            .expect("current source priority");
        if let Some(row) = existing {
            let row_id: i64 = row.get(0);
            let original: String = row.get(1);
            database
                .execute(
                    "UPDATE app_settings SET medicine_lookup_source_priority = $1::text::jsonb WHERE id = $2",
                    &[&configured, &row_id],
                )
                .expect("set source priority");
            Self {
                row_id,
                original: Some(original),
            }
        } else {
            let row_id: i64 = database
                .query_one(
                    "INSERT INTO app_settings (created_at, updated_at, medicine_lookup_source_priority) VALUES (CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $1::text::jsonb) RETURNING id",
                    &[&configured],
                )
                .expect("create disposable app settings row")
                .get(0);
            Self {
                row_id,
                original: None,
            }
        }
    }
}

impl Drop for LookupSourcePriorityGuard {
    fn drop(&mut self) {
        if let Ok(mut database) = postgres::Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap_or_default(),
            postgres::NoTls,
        ) {
            if let Some(original) = &self.original {
                let _ = database.execute(
                    "UPDATE app_settings SET medicine_lookup_source_priority = $1::text::jsonb WHERE id = $2",
                    &[original, &self.row_id],
                );
            } else {
                let _ = database.execute("DELETE FROM app_settings WHERE id = $1", &[&self.row_id]);
            }
        }
    }
}

struct LookupEnrichmentRows {
    query_gtin: String,
    stored_gtin: String,
    related_gtin: String,
    amp_code: String,
    family_code: String,
    group_code: String,
    evidence_id: String,
}

impl Drop for LookupEnrichmentRows {
    fn drop(&mut self) {
        let Ok(mut database) = postgres::Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap_or_default(),
            postgres::NoTls,
        ) else {
            return;
        };
        let _ = database.execute(
            "DELETE FROM medication_review_evidence_records WHERE source_record_id = $1",
            &[&self.evidence_id],
        );
        let _ = database.execute(
            "DELETE FROM nhs_dmd_barcodes WHERE gtin = ANY($1)",
            &[&vec![self.stored_gtin.clone(), self.related_gtin.clone()]],
        );
        let _ = database.execute(
            "DELETE FROM nhs_dmd_amp_trade_families WHERE amp_code = $1",
            &[&self.amp_code],
        );
        let _ = database.execute(
            "DELETE FROM nhs_dmd_trade_families WHERE code = $1",
            &[&self.family_code],
        );
        let _ = database.execute(
            "DELETE FROM nhs_dmd_trade_family_groups WHERE code = $1",
            &[&self.group_code],
        );
        let _ = database.execute(
            "DELETE FROM barcode_catalog_entries WHERE source = 'contract_catalog' AND gtin = $1",
            &[&self.stored_gtin],
        );
        let _ = database.execute(
            "DELETE FROM medications WHERE barcode = ANY($1)",
            &[&vec![self.stored_gtin.clone(), self.related_gtin.clone()]],
        );
    }
}

fn insert_lookup_enrichment_rows(household_id: i64) -> LookupEnrichmentRows {
    let mut database = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let digits = format!("{:012}", suffix % 1_000_000_000_000);
    let query_gtin = format!("00{digits}");
    let stored_gtin = format!("0{digits}");
    let related_gtin = format!("9{digits}");
    let lookup_code = format!("contract-amp-{suffix}");
    let related_code = format!("contract-related-amp-{suffix}");
    let amp_code = format!("contract-amp-family-{suffix}");
    let family_code = format!("contract-family-{suffix}");
    let group_code = format!("contract-group-{suffix}");
    let evidence_id = format!("contract-lookup-evidence-{suffix}");
    let location_id: i64 = database
        .query_one(
            "SELECT id FROM locations WHERE household_id = $1 ORDER BY id LIMIT 1",
            &[&household_id],
        )
        .expect("household location")
        .get(0);

    database
        .execute(
            "INSERT INTO barcode_catalog_entries (gtin, display, source, code, system, concept_class, created_at, updated_at) VALUES ($1, 'Warfarin', 'contract_catalog', $2, 'https://dmd.nhs.uk', 'AMP', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            &[&stored_gtin, &lookup_code],
        )
        .expect("catalogue barcode");
    for (barcode, medication_name, code) in [
        (&stored_gtin, "Warfarin", &lookup_code),
        (&related_gtin, "Paracetamol", &related_code),
    ] {
        database
            .execute(
                "INSERT INTO medications (household_id, location_id, name, barcode, dmd_code, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                &[&household_id, &location_id, &medication_name, &barcode, &code],
            )
            .expect("visible family medication");
        database
            .execute(
                "INSERT INTO nhs_dmd_barcodes (gtin, code, display, system, concept_class, amp_code, created_at, updated_at) VALUES ($1, $2, $3, 'https://dmd.nhs.uk', 'AMP', $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
                &[&barcode, &code, &medication_name, &amp_code],
            )
            .expect("NHS barcode mapping");
    }
    database
        .execute(
            "INSERT INTO nhs_dmd_trade_family_groups (code, name, created_at, updated_at) VALUES ($1, 'Contract family group', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            &[&group_code],
        )
        .expect("trade family group");
    let group_id: i64 = database
        .query_one(
            "SELECT id FROM nhs_dmd_trade_family_groups WHERE code = $1",
            &[&group_code],
        )
        .expect("trade family group id")
        .get(0);
    let family_id: i64 = database
        .query_one(
            "INSERT INTO nhs_dmd_trade_families (code, name, trade_family_group_id, created_at, updated_at) VALUES ($1, 'Contract warfarin family', $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING id",
            &[&family_code, &group_id],
        )
        .expect("trade family")
        .get(0);
    database
        .execute(
            "INSERT INTO nhs_dmd_amp_trade_families (amp_code, trade_family_id, created_at, updated_at) VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            &[&amp_code, &family_id],
        )
        .expect("AMP trade family mapping");
    database
        .execute(
            "INSERT INTO medication_review_evidence_records (source_name, source_record_id, source_url, retrieved_on, product_name, label_section, evidence_text, risk_level, match_confidence, match_status, candidate_terms, interacting_terms, created_at, updated_at) VALUES ('Contract lookup source', $1, 'https://example.test/lookup-evidence', CURRENT_DATE, 'Warfarin interaction', 'warnings', 'Avoid warfarin with paracetamol because bleeding risk may increase.', 'high', 'high', 'unreviewed', ARRAY['warfarin', 'paracetamol'], ARRAY['paracetamol'], CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            &[&evidence_id],
        )
        .expect("review evidence");

    LookupEnrichmentRows {
        query_gtin,
        stored_gtin,
        related_gtin,
        amp_code,
        family_code,
        group_code,
        evidence_id,
    }
}

#[test]
fn lookup_matches_one_leading_zero_and_enriches_family_related_medicine_and_review_evidence() {
    let target = Target::from_env();
    let fixture = fixture();
    let rows = insert_lookup_enrichment_rows(fixture.household_id);
    assert_eq!(rows.query_gtin.len(), 14);
    assert!(rows.query_gtin.starts_with("00"));
    let response = target.get(
        &format!(
            "/api/v1/households/{}/medication_lookup?q={}",
            fixture.household_id, rows.query_gtin
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("enriched lookup JSON");
    let results = payload["results"].as_array().expect("lookup results");
    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(payload["barcode"], rows.query_gtin);
    assert_eq!(result["display"], "Warfarin");
    assert_eq!(result["trade_family"]["code"], rows.family_code);
    assert_eq!(result["trade_family"]["name"], "Contract warfarin family");
    assert_eq!(result["trade_family_group"]["code"], rows.group_code);
    assert_eq!(result["existing_medication"]["name"], "Warfarin");
    let related = result["related_medications"]
        .as_array()
        .expect("related medications");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0]["name"], "Paracetamol");
    let prompts = result["review_prompts"].as_array().expect("review prompts");
    let prompt = prompts
        .iter()
        .find(|prompt| prompt["evidence_record_id"].as_u64().is_some())
        .expect("matched review evidence");
    assert_eq!(prompt["risk_level"], "high");
    assert_eq!(prompt["match_type"], "ingredient");
    assert_eq!(prompt["source_instruction"], "avoid");
    assert!(prompt["evidence_text"]
        .as_str()
        .is_some_and(|text| text.contains("Avoid warfarin with paracetamol")));
}

#[test]
fn lookup_source_priority_selects_open_products_facts_before_or_after_curated_catalog() {
    let opf_server = FakeHttpServer::start(OPF_PORT, opf_response);
    let fixture = fixture();
    let target = Target::from_env();
    let _restore = LookupSourcePriorityGuard::set(&["open_products_facts", "curated_catalog"]);
    let path = format!(
        "/api/v1/households/{}/medication_lookup?q=5021265232062",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_external_audit(
        &response,
        fixture.household_id,
        "ExternalMedicineLookup",
        "open_products_facts/barcode_lookup",
        "success",
        "result_count",
        1,
        "5021265232062",
    );
    let payload: Value = response.json().expect("Open Products Facts lookup");
    assert_eq!(
        payload["barcode_resolution"]["source"],
        "open_products_facts"
    );
    assert_eq!(
        payload["results"][0]["display"],
        "Contract ibuprofen 200 mg tablets (Wire brand) 16 tablets"
    );
    assert_eq!(opf_server.requests().len(), 1);
    let provider_request = &opf_server.requests()[0];
    assert_eq!(provider_request.method, "GET");
    assert!(provider_request
        .target
        .starts_with("/api/v2/product/5021265232062.json?"));
    assert_eq!(
        provider_request.headers.get("accept").map(String::as_str),
        Some("application/json")
    );

    let _curated_first =
        LookupSourcePriorityGuard::set(&["curated_catalog", "open_products_facts"]);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("curated lookup");
    assert_eq!(payload["barcode_resolution"]["source"], "curated");
    assert_eq!(
        payload["results"][0]["display"],
        "Pregnacare Plus tablets and capsules (Vitabiotics Ltd)"
    );
    assert_eq!(
        opf_server.requests().len(),
        1,
        "the higher-priority curated result avoids provider lookup"
    );
}

#[test]
fn lookup_returns_open_food_facts_supplement_barcode_and_search_results() {
    let off_server = FakeHttpServer::start(OFF_PORT, off_response);
    let nhs_server = FakeHttpServer::start(NHS_PORT, nhs_response);
    let fixture = fixture();
    let target = Target::from_env();
    let barcode = format!(
        "7{:012}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
            % 1_000_000_000_000
    );
    let barcode_path = format!(
        "/api/v1/households/{}/medication_lookup?q={barcode}",
        fixture.household_id
    );
    let response = target.get(&barcode_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_external_audit(
        &response,
        fixture.household_id,
        "ExternalMedicineLookup",
        "open_food_facts/barcode_lookup",
        "success",
        "result_count",
        1,
        &barcode,
    );
    let payload: Value = response.json().expect("Open Food Facts barcode lookup");
    assert_eq!(payload["barcode_resolution"]["source"], "open_food_facts");
    assert_eq!(payload["results"][0]["category"], "Supplement");
    assert_eq!(payload["results"][0]["package_quantity"], "60.0");
    assert_eq!(payload["results"][0]["package_unit"], "tablet");
    assert!(off_server.requests().iter().any(|request| {
        request
            .target
            .starts_with(&format!("/api/v2/product/{barcode}.json?"))
    }));

    let response = target.get(
        &format!(
            "/api/v1/households/{}/medication_lookup?q=contract%20vitamin",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_external_audit(
        &response,
        fixture.household_id,
        "ExternalMedicineLookup",
        "open_food_facts/search",
        "success",
        "result_count",
        1,
        "contract vitamin",
    );
    let payload: Value = response.json().expect("Open Food Facts supplement search");
    let results = payload["results"].as_array().expect("supplement results");
    assert!(
        results
            .iter()
            .any(|result| result["category"] == "Supplement"
                && result["name"] == "Contract vitamin C")
    );
    let requests = off_server.requests();
    let search = requests
        .iter()
        .find(|request| request.target.starts_with("/cgi/search.pl?"))
        .expect("OFF search request");
    assert!(search.target.contains("search_terms=contract+vitamin"));
    assert!(nhs_server.requests().len() >= 3);
}
