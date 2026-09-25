use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use scraper::{Html, Selector};
use serde_json::{json, Value};

fn sign_in(target: &Target, email: &str) {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("login HTML"));
    let selector = Selector::parse("form[action='/login'] input[name='authenticity_token']")
        .expect("login selector");
    let token = document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF token");
    let fields = vec![
        ("email".to_string(), email.to_string()),
        ("password".to_string(), "password".to_string()),
        ("authenticity_token".to_string(), token.to_string()),
    ];
    assert_eq!(
        target.post_html_form("/login", &fields).status().as_u16(),
        302
    );
}

fn web_path(fixture: &Fixture, suffix: &str) -> String {
    format!("/households/{}{}", fixture.web_household_slug, suffix)
}

fn json_body(response: Response, status: u16) -> Value {
    let actual = response.status().as_u16();
    let body = response.text().expect("web JSON body");
    assert_eq!(actual, status, "{body}");
    serde_json::from_str(&body).expect("web JSON response")
}

fn assert_login_redirect(response: Response) {
    assert_eq!(response.status().as_u16(), 302);
    assert_eq!(response.headers()["location"], "/login");
}

#[test]
fn web_json_reads_require_login_and_search_is_scoped_and_literal() {
    let fixture = fixture();
    let target = Target::from_env();
    let search = web_path(&fixture, "/search.json");
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let scan = web_path(&fixture, "/medications/scan_restock_match.json");
    for path in [&search, &finder, &scan] {
        assert_login_redirect(target.get(path, None));
    }

    sign_in(&target, &fixture.primary_email);
    assert_eq!(
        json_body(target.get(&search, None), 200)["results"],
        json!([])
    );
    assert_eq!(
        json_body(target.get(&format!("{search}?q=%25"), None), 200)["results"],
        json!([])
    );
    let results = json_body(
        target.get(&format!("{search}?q=Contract%20managed%20medicine"), None),
        200,
    )["results"]
        .as_array()
        .expect("global search results")
        .clone();
    assert!(results.iter().any(|row| {
        row["type"] == "medication"
            && row["title"]
                .as_str()
                .is_some_and(|title| title.contains(&fixture.managed_medication_name))
    }));
    let results = json_body(
        target.get(&format!("{search}?q=Contract%20foreign%20medicine"), None),
        200,
    )["results"]
        .as_array()
        .expect("scoped global search results")
        .clone();
    assert!(!results.iter().any(|row| row["title"]
        .as_str()
        .is_some_and(|title| title.contains(&fixture.foreign_medication_name))));
}

#[test]
fn scan_restock_match_only_exposes_accessible_local_stock() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.primary_email);
    let scan = web_path(&fixture, "/medications/scan_restock_match.json");
    let matched = json_body(
        target.get(&format!("{scan}?q={}", fixture.lookup_barcode), None),
        200,
    );
    assert_eq!(matched["matched"], true);
    assert_eq!(matched["medication"]["id"], fixture.managed_medication_id);
    assert_eq!(matched["medication"]["current_supply"], "50");
    assert_eq!(
        json_body(
            target.get(&format!("{scan}?q={}", fixture.web_foreign_barcode), None),
            200
        ),
        json!({"matched": false})
    );
    assert_eq!(
        json_body(target.get(&format!("{scan}?q=0000000000000"), None), 200),
        json!({"matched": false})
    );
}

#[test]
fn finder_resolves_local_barcode_and_hides_foreign_stock_metadata() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.primary_email);
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let blank = json_body(target.get(&finder, None), 200);
    assert_eq!(blank["results"], json!([]));
    assert_eq!(
        blank["permissions"],
        json!({"can_create": true, "can_restock": true})
    );

    let local = json_body(
        target.get(&format!("{finder}?q={}", fixture.lookup_barcode), None),
        200,
    );
    assert_eq!(local["barcode"], fixture.lookup_barcode);
    assert_eq!(local["barcode_resolution"]["source"], "contract_catalog");
    assert_eq!(local["results"][0]["code"], fixture.lookup_code);
    assert_eq!(
        local["results"][0]["existing_medication"]["id"],
        fixture.managed_medication_id
    );

    let foreign = json_body(
        target.get(&format!("{finder}?q={}", fixture.web_foreign_barcode), None),
        200,
    );
    assert_eq!(
        foreign["results"][0]["display"],
        fixture.web_foreign_display
    );
    assert!(foreign["results"][0].get("existing_medication").is_none());
}

#[test]
fn finder_uses_deterministic_nhs_and_open_food_facts_fallbacks() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.primary_email);
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let nhs = json_body(
        target.get(&format!("{finder}?q=contractupstream"), None),
        200,
    );
    assert!(
        nhs["results"]
            .as_array()
            .expect("NHS results")
            .iter()
            .any(|row| row["code"] == "contract-upstream-500"
                && row["system"] == "https://dmd.nhs.uk")
    );

    let off_barcode = json_body(target.get(&format!("{finder}?q=5021265221301"), None), 200);
    assert_eq!(off_barcode["barcode"], "5021265221301");
    assert_eq!(off_barcode["results"][0]["category"], "Supplement");
    assert_eq!(off_barcode["results"][0]["package_quantity"], 30);
    let off_text = json_body(
        target.get(&format!("{finder}?q=vitamin%20contract"), None),
        200,
    );
    assert_eq!(off_text["results"][0]["source_label"], "Open Food Facts");
    assert_eq!(off_text["results"][0]["package_unit"], "tablet");

    let unavailable = json_body(
        target.get(&format!("{finder}?q=contractunavailable"), None),
        503,
    );
    assert_eq!(unavailable["results"], json!([]));
    assert_eq!(
        unavailable["error"],
        "Medication search is temporarily unavailable."
    );
}

#[test]
fn finder_throttles_after_sixty_requests_and_advertises_retry() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture.primary_email);
    let finder = web_path(&fixture, "/medication-finder/search.json");
    for _ in 0..60 {
        assert_eq!(
            target
                .get_from_local_client(&finder, "198.51.100.167")
                .status()
                .as_u16(),
            200
        );
    }
    let response = target.get_from_local_client(&finder, "198.51.100.167");
    assert_eq!(response.status().as_u16(), 429);
    assert!(
        response.headers()["retry-after"]
            .to_str()
            .expect("retry-after")
            .parse::<u64>()
            .expect("retry seconds")
            > 0
    );
    assert!(response
        .text()
        .expect("rate-limit body")
        .contains("Rate limit exceeded"));
}
