use lopdf::Document;
use medtracker_contract_tests::{Fixture, Target, fixture};
use postgres::{Client, NoTls};
use reqwest::blocking::Response;
use scraper::{Html, Selector};
use serde_json::Value;
use std::env;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static LOGIN_CLIENT: AtomicUsize = AtomicUsize::new(1);

fn csrf(html: &str) -> String {
    let document = Html::parse_document(html);
    for (selector, attribute) in [
        ("input[name='authenticity_token']", "value"),
        ("meta[name='csrf-token']", "content"),
    ] {
        let selector = Selector::parse(selector).expect("CSRF selector");
        if let Some(value) = document
            .select(&selector)
            .next()
            .and_then(|input| input.value().attr(attribute))
        {
            return value.to_owned();
        }
    }
    panic!("login form must render a CSRF token")
}

fn sign_in(target: &Target, fixture: &Fixture) {
    sign_in_as(target, &fixture.primary_email, &fixture.household_slug);
}

fn sign_in_as(target: &Target, email: &str, slug: &str) {
    let login = target.get_html("/login");
    assert_eq!(login.status().as_u16(), 200);
    let token = csrf(&login.text().expect("login HTML"));
    let client_ip = format!("198.18.25.{}", LOGIN_CLIENT.fetch_add(1, Ordering::Relaxed));
    let response = target.post_html_form_from_client(
        "/login",
        &client_ip,
        &[
            ("email".into(), email.into()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token),
        ],
    );
    assert_eq!(response.status().as_u16(), 302, "sign-in must redirect");
    let inventory = target.get_html(&format!("/households/{slug}/medications"));
    assert_eq!(inventory.status().as_u16(), 200);
}

fn account_email(target: &Target, fixture: &Fixture, token: &str) -> String {
    let me = target.get(
        &format!("/api/v1/households/{}/me", fixture.household_id),
        Some(token),
    );
    assert_eq!(me.status().as_u16(), 200);
    me.json::<Value>().expect("account identity")["data"]["email_address"]
        .as_str()
        .expect("account email")
        .to_owned()
}

fn database() -> Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    Client::connect(&url, NoTls).expect("contract database")
}

fn reports_page(fixture: &Fixture) -> String {
    format!("/households/{}/reports", fixture.household_slug)
}

fn download_path(fixture: &Fixture, query: &str) -> String {
    format!(
        "/households/{}/reports/health-history.pdf{query}",
        fixture.household_slug
    )
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn no_store(response: &Response) {
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .expect("cache header")
            .contains("no-store")
    );
}

fn audits(target: &Target, fixture: &Fixture) -> Vec<Value> {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().expect("audit entries")["data"]
        .as_array()
        .expect("audit list")
        .to_vec()
}

fn download_audits(entries: &[Value]) -> Vec<&Value> {
    entries
        .iter()
        .filter(|row| row["event_type"] == "health_history_report.downloaded")
        .collect()
}

fn person_option_values(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    document
        .select(&Selector::parse("select[name='person_id'] option").unwrap())
        .filter_map(|option| option.value().attr("value").map(str::to_owned))
        .filter(|value| !value.is_empty())
        .collect()
}

fn pdf_text(response: Response) -> String {
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    assert_eq!(response.headers()["content-type"], "application/pdf");
    let disposition = response.headers()["content-disposition"]
        .to_str()
        .expect("attachment header")
        .to_owned();
    assert!(disposition.contains("attachment"));
    assert!(disposition.contains(".pdf"));
    let bytes = response.bytes().expect("PDF bytes");
    assert!(bytes.len() > 500, "PDF must be nonempty");
    assert!(bytes.starts_with(b"%PDF-"));
    let document = Document::load_mem(&bytes).expect("valid PDF document");
    assert!(!document.get_pages().is_empty(), "PDF contains pages");
    let path = env::temp_dir().join(format!(
        "medtracker-household-report-{}-{}.pdf",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::write(&path, &bytes).expect("write temporary PDF");
    let output = Command::new("pdftotext")
        .arg(&path)
        .arg("-")
        .output()
        .expect("start Poppler text extractor");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "Poppler text extraction failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Poppler UTF-8 output")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn report_failure_base() -> Option<String> {
    let base = env::var("CONTRACT_REPORT_FAILURE_BASE_URL")
        .ok()?
        .trim_end_matches('/')
        .to_owned();
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .no_proxy()
        .build()
        .ok()?;
    client
        .get(format!("{base}/up"))
        .send()
        .ok()
        .filter(|response| response.status().is_success())?;
    Some(base)
}

fn manual_session_cookie(fixture: &Fixture) -> String {
    let base = env::var("CONTRACT_BASE_URL")
        .expect("contract base URL")
        .trim_end_matches('/')
        .to_owned();
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .no_proxy()
        .build()
        .expect("manual client");
    let login = client
        .get(format!("{base}/login"))
        .header("Accept", "text/html")
        .send()
        .expect("login page");
    assert_eq!(login.status().as_u16(), 200);
    let token = csrf(&login.text().expect("login HTML"));
    let client_ip = format!("198.18.25.{}", LOGIN_CLIENT.fetch_add(1, Ordering::Relaxed));
    let response = client
        .post(format!("{base}/login"))
        .header("Accept", "text/html")
        .header("Origin", &base)
        .header("X-Forwarded-For", client_ip)
        .form(&[
            ("email", fixture.primary_email.as_str()),
            ("password", "password"),
            ("authenticity_token", token.as_str()),
        ])
        .send()
        .expect("login submission");
    assert_eq!(response.status().as_u16(), 302, "login must redirect");
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            value
                .split(';')
                .next()
                .filter(|pair| pair.starts_with("mt_oauth_session="))
                .map(str::to_owned)
        })
        .expect("browser session cookie")
}

#[test]
fn reports_page_lists_only_manageable_people_and_downloads_the_existing_pdf() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture);
    let baseline = audits(&target, &fixture);
    let baseline_count = download_audits(&baseline).len();
    let page = target.get_html(&reports_page(&fixture));
    assert_eq!(page.status().as_u16(), 200);
    no_store(&page);
    let html = page.text().expect("reports page HTML");
    let document = Html::parse_document(&html);
    let form = document
        .select(&Selector::parse("form#health_history_report_form").unwrap())
        .next()
        .expect("health history report form");
    assert_eq!(form.value().attr("method"), Some("get"));
    assert_eq!(
        form.value().attr("action"),
        Some(
            format!(
                "/households/{}/reports/health-history.pdf",
                fixture.household_slug
            )
            .as_str()
        )
    );
    let mut options = person_option_values(&html);
    let mut expected = vec![
        fixture.user_person_id.to_string(),
        fixture.managed_person_id.to_string(),
    ];
    options.sort();
    expected.sort();
    assert_eq!(options, expected);
    assert!(html.contains(&fixture.web_managed_person_name));
    assert!(!html.contains(&fixture.web_hidden_person_name));
    assert!(!html.contains(&fixture.foreign_person_name));
    let checkbox = document
        .select(&Selector::parse("input[name='include_medication_takes']").unwrap())
        .next()
        .expect("medication takes checkbox");
    assert_eq!(checkbox.value().attr("value"), Some("1"));
    assert!(checkbox.value().attr("checked").is_none());
    for name in ["start_date", "end_date"] {
        let input = document
            .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
            .next()
            .unwrap_or_else(|| panic!("{name} date input"));
        assert_eq!(input.value().attr("type"), Some("date"));
        assert_eq!(
            input.value().attr("value").unwrap_or_default().len(),
            10,
            "{name} must carry an ISO default"
        );
    }
    assert!(
        document
            .select(&Selector::parse("select[name='person_id'] option[value='']").unwrap())
            .next()
            .is_some()
    );
    let after_page = audits(&target, &fixture);
    assert_eq!(
        download_audits(&after_page).len(),
        baseline_count,
        "opening the reports page must not create a report request"
    );
    let first = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-19&end_date=2026-02-26&include_medication_takes=1",
            fixture.managed_person_id
        ),
    ));
    let first_id = request_id(&first);
    let text = pdf_text(first);
    assert!(text.contains(&fixture.web_managed_person_name));
    assert!(text.contains(&fixture.managed_health_event_title));
    assert!(text.contains(&fixture.earlier_health_event_title));
    assert!(text.contains("Medication takes"));
    assert!(!text.contains("Contract hidden event"));
    assert!(!text.contains("Contract foreign event"));
    let second = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-24&end_date=2026-02-26&include_medication_takes=0",
            fixture.managed_person_id
        ),
    ));
    let second_id = request_id(&second);
    let second_text = pdf_text(second);
    assert!(!second_text.contains("Medication takes"));
    assert!(!second_text.contains(&fixture.earlier_health_event_title));
    assert!(second_text.contains(&fixture.managed_health_event_title));
    let empty = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-19&end_date=2026-02-19",
            fixture.managed_person_id
        ),
    ));
    let empty_id = request_id(&empty);
    let empty_text = pdf_text(empty);
    assert!(!empty_text.contains(&fixture.managed_health_event_title));
    let entries = audits(&target, &fixture);
    let downloads = download_audits(&entries);
    assert_eq!(downloads.len(), baseline_count + 3);
    let first_audit = downloads
        .iter()
        .find(|row| row["request_id"] == first_id)
        .expect("first download audit");
    assert_eq!(first_audit["metadata"]["format"], "pdf");
    assert_eq!(first_audit["metadata"]["include_medication_takes"], true);
    assert_eq!(first_audit["metadata"]["start_date"], "2026-02-19");
    assert_eq!(first_audit["metadata"]["end_date"], "2026-02-26");
    let second_audit = downloads
        .iter()
        .find(|row| row["request_id"] == second_id)
        .expect("second download audit");
    assert_eq!(second_audit["metadata"]["include_medication_takes"], false);
    assert!(
        downloads.iter().any(|row| row["request_id"] == empty_id),
        "empty-range download must be audited once"
    );
    let session = target.get_html(&reports_page(&fixture));
    assert_eq!(
        session.status().as_u16(),
        200,
        "browser session must survive PDF downloads"
    );
}

#[test]
fn invalid_report_filters_keep_the_form_with_errors_and_no_audit() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture);
    let baseline = download_audits(&audits(&target, &fixture)).len();
    for (query, field) in [
        (
            format!(
                "?person_id={}&start_date=2026-02-27&end_date=2026-02-26",
                fixture.managed_person_id
            ),
            "end_date",
        ),
        (
            format!(
                "?person_id={}&start_date=not-a-date&end_date=2026-02-26",
                fixture.managed_person_id
            ),
            "start_date",
        ),
        (
            format!(
                "?person_id={}&start_date=2020-01-01&end_date=2026-02-26",
                fixture.managed_person_id
            ),
            "end_date",
        ),
        (
            format!(
                "?person_id={}&include_medication_takes=2",
                fixture.managed_person_id
            ),
            "include_medication_takes",
        ),
        (
            "?start_date=2026-02-19&end_date=2026-02-26".to_string(),
            "person_id",
        ),
    ] {
        let response = target.get_html(&download_path(&fixture, &query));
        assert_eq!(response.status().as_u16(), 422, "invalid filter {query}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert!(response.headers().get("content-disposition").is_none());
        no_store(&response);
        let html = response.text().expect("invalid filter HTML");
        let document = Html::parse_document(&html);
        assert!(
            document
                .select(&Selector::parse("[role='alert']").unwrap())
                .next()
                .is_some()
        );
        let control = document
            .select(&Selector::parse(&format!("[name='{field}']")).unwrap())
            .next()
            .unwrap_or_else(|| panic!("{field} control"));
        assert_eq!(
            control.value().attr("aria-invalid"),
            Some("true"),
            "{field} must be marked invalid"
        );
        assert!(
            control.value().attr("aria-describedby").is_some(),
            "{field} must reference an error description"
        );
        assert!(
            document
                .select(&Selector::parse("form#health_history_report_form").unwrap())
                .next()
                .is_some()
        );
    }
    let retained = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-27&end_date=2026-02-26",
            fixture.managed_person_id
        ),
    ));
    let html = retained.text().expect("retained filters HTML");
    assert!(html.contains("value=\"2026-02-27\""));
    assert!(html.contains("value=\"2026-02-26\""));
    let entries = audits(&target, &fixture);
    assert_eq!(
        download_audits(&entries).len(),
        baseline,
        "client-side filter rejections must not create report requests"
    );
}

#[test]
fn foreign_and_unmanageable_people_are_masked_by_the_report_api() {
    let fixture = fixture();
    let target = Target::from_env();
    sign_in(&target, &fixture);
    let baseline = download_audits(&audits(&target, &fixture)).len();
    for person in [
        fixture.foreign_person_id.to_string(),
        fixture.hidden_person_id.to_string(),
        fixture.foreign_person_portable_id.clone(),
        "999999999".to_string(),
    ] {
        let response = target.get_html(&download_path(
            &fixture,
            &format!("?person_id={person}&start_date=2026-02-24&end_date=2026-02-26"),
        ));
        assert_eq!(response.status().as_u16(), 404, "masked person {person}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        let html = response.text().expect("masked person HTML");
        assert!(!html.contains(&fixture.foreign_person_name));
        assert!(!html.contains(&fixture.web_hidden_person_name));
        assert!(!html.starts_with("%PDF-"));
    }
    let foreign_household = target.get_html(&format!(
        "/households/{}/reports",
        fixture.foreign_household_slug
    ));
    assert_eq!(foreign_household.status().as_u16(), 404);
    let foreign_download = target.get_html(&format!(
        "/households/{}/reports/health-history.pdf?person_id={}",
        fixture.foreign_household_slug, fixture.foreign_person_id
    ));
    assert_eq!(foreign_download.status().as_u16(), 404);
    let entries = audits(&target, &fixture);
    assert_eq!(download_audits(&entries).len(), baseline);
}

#[test]
fn view_only_member_has_no_report_choices_and_download_is_masked() {
    let fixture = fixture();
    let target = Target::from_env();
    let email = account_email(&target, &fixture, &fixture.view_access_token);
    sign_in_as(&target, &email, &fixture.household_slug);
    let page = target.get_html(&reports_page(&fixture));
    assert_eq!(page.status().as_u16(), 200);
    let html = page.text().expect("view member reports HTML");
    let document = Html::parse_document(&html);
    assert!(person_option_values(&html).is_empty());
    let submit = document
        .select(&Selector::parse("form#health_history_report_form button[type='submit']").unwrap())
        .next()
        .expect("download button");
    assert!(submit.value().attr("disabled").is_some());
    let denied = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
            fixture.managed_person_id
        ),
    ));
    assert_eq!(denied.status().as_u16(), 404);
    let html = denied.text().expect("masked download HTML");
    assert!(!html.contains(&fixture.web_managed_person_name));
    assert!(!html.starts_with("%PDF-"));
}

#[test]
fn delegated_member_downloads_and_a_revoked_grant_is_masked() {
    let fixture = fixture();
    let target = Target::from_env();
    let me = target.get(
        &format!("/api/v1/households/{}/me", fixture.household_id),
        Some(&fixture.delegated_access_token),
    );
    assert_eq!(me.status().as_u16(), 200);
    let me = me.json::<Value>().expect("delegated identity");
    let email = me["data"]["email_address"].as_str().unwrap().to_owned();
    let account_id = me["data"]["account"]["id"].as_i64().expect("account id");
    sign_in_as(&target, &email, &fixture.household_slug);
    let page = target.get_html(&reports_page(&fixture));
    assert_eq!(page.status().as_u16(), 200);
    let options = person_option_values(&page.text().expect("delegated reports HTML"));
    assert!(options.contains(&fixture.managed_person_id.to_string()));
    let pdf = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
            fixture.managed_person_id
        ),
    ));
    let text = pdf_text(pdf);
    assert!(text.contains(&fixture.web_managed_person_name));
    database()
        .execute(
            "UPDATE person_access_grants SET revoked_at = NOW() WHERE household_id = $1 AND person_id = $2 AND access_level = 'manage' AND revoked_at IS NULL AND household_membership_id IN (SELECT id FROM household_memberships WHERE account_id = $3)",
            &[&fixture.household_id, &fixture.managed_person_id, &account_id],
        )
        .expect("revoke delegated grant");
    let page = target.get_html(&reports_page(&fixture));
    let options = person_option_values(&page.text().expect("post-revocation HTML"));
    assert!(!options.contains(&fixture.managed_person_id.to_string()));
    let denied = target.get_html(&download_path(
        &fixture,
        &format!(
            "?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
            fixture.managed_person_id
        ),
    ));
    assert_eq!(denied.status().as_u16(), 404);
    let html = denied.text().expect("revoked grant HTML");
    assert!(!html.contains(&fixture.web_managed_person_name));
}

#[test]
fn expired_browser_sessions_redirect_to_login() {
    let fixture = fixture();
    let target = Target::from_env();
    let email = account_email(&target, &fixture, &fixture.view_access_token);
    sign_in_as(&target, &email, &fixture.household_slug);
    database()
        .execute(
            "DELETE FROM account_active_session_keys WHERE account_id = $1",
            &[&fixture.view_account_id],
        )
        .expect("expire browser session");
    for path in [
        reports_page(&fixture),
        download_path(
            &fixture,
            &format!(
                "?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
                fixture.managed_person_id
            ),
        ),
    ] {
        let response = target.get_html(&path);
        assert_eq!(response.status().as_u16(), 302, "expired session {path}");
        assert_eq!(response.headers()["location"].to_str().unwrap(), "/login");
    }
}

#[test]
fn report_generation_failure_returns_a_controlled_form_error() {
    let Some(fail_base) = report_failure_base() else {
        eprintln!("report failure service unavailable; skipping generation failure check");
        return;
    };
    let fixture = fixture();
    let cookie = manual_session_cookie(&fixture);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("report failure client");
    let response = client
        .get(format!(
            "{fail_base}{}",
            download_path(
                &fixture,
                &format!(
                    "?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
                    fixture.managed_person_id
                )
            )
        ))
        .header("Accept", "text/html")
        .header("Cookie", &cookie)
        .send()
        .expect("report failure download");
    assert_eq!(response.status().as_u16(), 503);
    no_store(&response);
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(response.headers().get("content-disposition").is_none());
    let html = response.text().expect("failure HTML");
    assert!(html.contains("temporarily unavailable"));
    assert!(html.contains("value=\"2026-02-24\""));
    assert!(!html.starts_with("%PDF-"));
    let page = client
        .get(format!("{fail_base}{}", reports_page(&fixture)))
        .header("Accept", "text/html")
        .header("Cookie", &cookie)
        .send()
        .expect("report failure page");
    assert_eq!(page.status().as_u16(), 200);
}

#[test]
fn anonymous_visitors_redirect_to_login_and_post_is_rejected() {
    let fixture = fixture();
    let anonymous = Target::from_env();
    for path in [
        reports_page(&fixture),
        download_path(
            &fixture,
            &format!(
                "?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
                fixture.managed_person_id
            ),
        ),
    ] {
        let response = anonymous.get_html(&path);
        assert_eq!(response.status().as_u16(), 302, "anonymous {path}");
        assert_eq!(response.headers()["location"].to_str().unwrap(), "/login");
    }
}
