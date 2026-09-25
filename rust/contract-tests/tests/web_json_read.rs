use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::sync::OnceLock;

fn sign_in(target: &Target, email: &str) {
    let client_ip = format!(
        "203.0.113.{}",
        20 + email
            .bytes()
            .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(byte)))
            % 220
    );
    let response = target.get_html_from_local_client("/login", &client_ip);
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
        target
            .post_html_form_from_local_client("/login", &fields, &client_ip)
            .status()
            .as_u16(),
        302
    );
}

fn owner_target(fixture: &Fixture) -> &'static Target {
    static OWNER: OnceLock<Target> = OnceLock::new();
    OWNER.get_or_init(|| {
        let target = Target::from_env();
        sign_in(&target, &fixture.primary_email);
        target
    })
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
    let unauthenticated = Target::from_env();
    let search = web_path(&fixture, "/search.json");
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let scan = web_path(&fixture, "/medications/scan_restock_match.json");
    for path in [&search, &finder, &scan] {
        assert_login_redirect(unauthenticated.get(path, None));
    }

    let target = owner_target(&fixture);
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
    let target = owner_target(&fixture);
    let scan = web_path(&fixture, "/medications/scan_restock_match.json");
    let matched = json_body(
        target.get(&format!("{scan}?q={}", fixture.lookup_barcode), None),
        200,
    );
    assert_eq!(matched["matched"], true);
    assert_eq!(matched["medication"]["id"], fixture.managed_medication_id);
    let medication = json_body(
        Target::from_env().get(
            &format!(
                "/api/v1/households/{}/medications/{}",
                fixture.household_id, fixture.managed_medication_id
            ),
            Some(&fixture.access_token),
        ),
        200,
    );
    let api_supply = medication["data"]["current_supply"]
        .as_str()
        .expect("API current supply");
    let expected_supply = if api_supply.contains('.') {
        api_supply.trim_end_matches('0').trim_end_matches('.')
    } else {
        api_supply
    };
    assert_eq!(matched["medication"]["current_supply"], expected_supply);
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
    let target = owner_target(&fixture);
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
    let target = owner_target(&fixture);
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let nhs = json_body(
        target.get(&format!("{finder}?q=contractupstream"), None),
        200,
    );
    let nhs_results = nhs["results"].as_array().expect("NHS results");
    let nhs_tablets = nhs_results
        .iter()
        .find(|row| row["code"] == "contract-upstream-500")
        .expect("deterministic NHS tablets");
    assert_eq!(nhs_tablets["display"], "Contractupstream 500mg tablets");
    assert_eq!(nhs_tablets["system"], "https://dmd.nhs.uk");
    assert_eq!(nhs_tablets["concept_class"], "AMPP");
    assert_eq!(nhs["review_guidance"], json!({ "status": "available" }));
    for result in nhs_results {
        assert_eq!(result["review_prompts"], json!([]));
        assert_eq!(result["review_prompt_filter"], json!({ "hidden_count": 0 }));
    }

    let off_barcode = json_body(target.get(&format!("{finder}?q=5021265221301"), None), 200);
    assert_eq!(off_barcode["barcode"], "5021265221301");
    let off_text = json_body(
        target.get(&format!("{finder}?q=vitamin%20contract"), None),
        200,
    );
    for response in [&off_barcode, &off_text] {
        let results = response["results"].as_array().expect("supplement results");
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result["code"], json!(null));
        assert_eq!(result["barcode"], "5021265221301");
        assert_eq!(result["name"], "Contract Vitamin");
        assert_eq!(result["description"], "Daily multivitamin food supplement");
        assert_eq!(
            result["display"],
            "Contract Vitamin (Contract Brand) 30 tablets"
        );
        assert_eq!(result["system"], "https://world.openfoodfacts.org");
        assert_eq!(result["concept_class"], "Supplement");
        assert_eq!(result["category"], "Supplement");
        assert_eq!(result["package_size"], "30 tablets");
        assert_eq!(result["package_quantity"], 30);
        assert_eq!(result["package_unit"], "tablet");
        assert_eq!(result["source_label"], "Open Food Facts");
    }

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
fn carer_search_respects_person_grants_and_finder_is_restock_only() {
    let fixture = fixture();
    let search = web_path(&fixture, "/search.json");
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let hidden_authorized = Target::from_env();
    sign_in(&hidden_authorized, &fixture.web_feed_email);
    let hidden_query = fixture.web_hidden_person_name.replace(' ', "%20");
    let authorized_results = json_body(
        hidden_authorized.get(&format!("{search}?q={hidden_query}"), None),
        200,
    );
    assert!(authorized_results["results"]
        .as_array()
        .expect("granted search results")
        .iter()
        .any(|row| row["type"] == "person" && row["title"] == fixture.web_hidden_person_name));

    let carer = Target::from_env();
    sign_in(&carer, &fixture.web_view_email);
    let managed_query = fixture.web_managed_person_name.replace(' ', "%20");
    let managed_results = json_body(carer.get(&format!("{search}?q={managed_query}"), None), 200);
    assert!(managed_results["results"]
        .as_array()
        .expect("carer granted search results")
        .iter()
        .any(|row| row["type"] == "person" && row["title"] == fixture.web_managed_person_name));
    let hidden_results = json_body(carer.get(&format!("{search}?q={hidden_query}"), None), 200);
    assert!(!hidden_results["results"]
        .as_array()
        .expect("carer ungranted search results")
        .iter()
        .any(|row| row["title"] == fixture.web_hidden_person_name));

    let matched = json_body(
        carer.get(&format!("{finder}?q={}", fixture.lookup_barcode), None),
        200,
    );
    assert_eq!(
        matched["permissions"],
        json!({"can_create": false, "can_restock": true})
    );
    assert_eq!(
        matched["results"][0]["existing_medication"]["id"],
        fixture.managed_medication_id
    );
}

#[test]
fn finder_filters_deterministic_nhs_results_by_form_and_strength() {
    let fixture = fixture();
    let target = owner_target(&fixture);
    let finder = web_path(&fixture, "/medication-finder/search.json");

    let liquid = json_body(
        target.get(&format!("{finder}?q=contractupstream&form=liquid"), None),
        200,
    );
    assert_eq!(liquid["form"], "liquid");
    assert_eq!(liquid["strength"], Value::Null);
    let mut liquid_codes: Vec<&str> = liquid["results"]
        .as_array()
        .expect("liquid results")
        .iter()
        .map(|row| row["code"].as_str().expect("liquid code"))
        .collect();
    liquid_codes.sort_unstable();
    assert_eq!(
        liquid_codes,
        ["contract-upstream-125", "contract-upstream-250"]
    );

    let strength = json_body(
        target.get(
            &format!("{finder}?q=contractupstream&strength=0.5%20g"),
            None,
        ),
        200,
    );
    assert_eq!(strength["form"], Value::Null);
    assert_eq!(strength["strength"], "500mg");
    let strength_codes: Vec<&str> = strength["results"]
        .as_array()
        .expect("strength results")
        .iter()
        .map(|row| row["code"].as_str().expect("strength code"))
        .collect();
    assert_eq!(strength_codes, ["contract-upstream-500"]);
}

#[test]
fn finder_throttles_after_sixty_requests_and_advertises_retry() {
    let fixture = fixture();
    let target = owner_target(&fixture);
    let finder = web_path(&fixture, "/medication-finder/search.json");
    let client_ip = format!("198.51.100.{}", 20 + fixture.household_id % 220);
    for _ in 0..60 {
        assert_eq!(
            target
                .get_from_local_client(&finder, &client_ip)
                .status()
                .as_u16(),
            200
        );
    }
    let response = target.get_from_local_client(&finder, &client_ip);
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
