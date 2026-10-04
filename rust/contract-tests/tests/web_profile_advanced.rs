use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use std::env;

fn database() -> Client {
    Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable contract database"),
        NoTls,
    )
    .expect("connect to disposable contract database")
}

fn csrf(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector = Selector::parse("input[name='authenticity_token']").unwrap();
    document
        .select(&selector)
        .next()
        .and_then(|element| element.value().attr("value"))
        .expect("CSRF input")
        .to_owned()
}

fn sign_in(target: &Target, email: &str, slug: &str) -> String {
    let login = target.get_html("/login");
    let token = csrf(&login.text().expect("login page"));
    let response = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".to_owned(), email.to_owned()),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), token),
        ],
        "198.51.100.55",
    );
    assert_eq!(response.status().as_u16(), 302);
    let page = target.get_html(&format!("/households/{slug}/profile?section=advanced"));
    assert_eq!(page.status().as_u16(), 200);
    csrf(&page.text().expect("Advanced page"))
}

#[test]
fn advanced_experiments_save_each_choice_without_resetting_other_preferences() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut db = database();
    let slug: String = db
        .query_one(
            "SELECT slug FROM households WHERE id = $1",
            &[&fixture.profile_household_id],
        )
        .expect("profile household")
        .get(0);
    let token = sign_in(&target, &fixture.profile_email, &slug);
    let action = format!("/households/{slug}/profile/experiments");

    let response = target.post_browser_form(
        &action,
        &[
            ("authenticity_token".to_owned(), token.clone()),
            ("wizard_variant".to_owned(), "modal".to_owned()),
        ],
    );
    assert_eq!(response.status().as_u16(), 303);
    let row = db
        .query_one(
            "SELECT preferences->>'wizard_variant', preferences->>'dashboard_variant' FROM accounts WHERE id = $1",
            &[&fixture.profile_account_id],
        )
        .expect("saved account preferences");
    assert_eq!(row.get::<_, Option<String>>(0).as_deref(), Some("modal"));
    assert_ne!(
        row.get::<_, Option<String>>(1).as_deref(),
        Some("family_lanes")
    );

    let response = target.post_browser_form(
        &action,
        &[
            ("authenticity_token".to_owned(), token.clone()),
            ("dashboard_variant".to_owned(), "family_lanes".to_owned()),
        ],
    );
    assert_eq!(response.status().as_u16(), 303);
    let response = target.post_browser_form(
        &action,
        &[
            ("authenticity_token".to_owned(), token.clone()),
            (
                "medication_launcher_variant".to_owned(),
                "context_aware".to_owned(),
            ),
        ],
    );
    assert_eq!(response.status().as_u16(), 303);
    let page = target.get_html(&format!("/households/{slug}/profile?section=advanced"));
    let html = page.text().expect("updated Advanced page");
    for choice in ["modal", "family_lanes", "context_aware"] {
        assert!(html.contains(&format!("value=\"{choice}\" checked")));
    }

    let denied = target.post_browser_form(
        &action,
        &[
            ("authenticity_token".to_owned(), "wrong".to_owned()),
            ("wizard_variant".to_owned(), "slideover".to_owned()),
        ],
    );
    assert_eq!(denied.status().as_u16(), 403);
    let saved: Option<String> = db
        .query_one(
            "SELECT preferences->>'wizard_variant' FROM accounts WHERE id = $1",
            &[&fixture.profile_account_id],
        )
        .expect("unchanged account")
        .get(0);
    assert_eq!(saved.as_deref(), Some("modal"));
}
