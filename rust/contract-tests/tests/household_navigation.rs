use medtracker_contract_tests::{Target, fixture};
use scraper::{Html, Selector};
use serde_json::Value;

fn sign_in(target: &Target, email: &str, client_ip: &str) {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let document = Html::parse_document(&response.text().expect("login HTML"));
    let selector =
        Selector::parse("form[action='/login'] input[name='authenticity_token']").unwrap();
    let token = document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .expect("login CSRF");
    let response = target.post_html_form_from_client(
        "/login",
        client_ip,
        &[
            ("email".into(), email.into()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), token.into()),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
}

fn page(target: &Target, path: &str) -> Html {
    let response = target.get_html(path);
    assert_eq!(
        response.status().as_u16(),
        200,
        "navigation starting page {path}"
    );
    Html::parse_document(&response.text().expect("starting page HTML"))
}

fn has_link(document: &Html, path: &str) -> bool {
    let selector = Selector::parse(&format!("a[href='{path}']")).expect("household link selector");
    document.select(&selector).any(|link| {
        !link.text().collect::<String>().trim().is_empty()
            || link
                .value()
                .attr("aria-label")
                .is_some_and(|label| !label.trim().is_empty())
    })
}

#[test]
fn owner_can_reach_people_locations_medication_creation_and_edit_from_inventory() {
    let fixture = fixture();
    let owner = Target::from_env();
    sign_in(
        &owner,
        &fixture.primary_email,
        "198.18.25.1",
    );
    let prefix = format!("/households/{}", fixture.household_slug);
    let inventory = page(&owner, &format!("{prefix}/medications"));
    for suffix in ["people", "locations", "medications/new"] {
        assert!(
            has_link(&inventory, &format!("{prefix}/{suffix}")),
            "owner inventory must provide a labelled {suffix} link"
        );
    }
    let medication = page(
        &owner,
        &format!("{prefix}/medications/{}", fixture.managed_medication_id),
    );
    assert!(
        has_link(
            &medication,
            &format!(
                "{prefix}/medications/{}/edit",
                fixture.managed_medication_id
            )
        ),
        "owner medication detail must provide a labelled edit link"
    );
}

#[test]
fn view_only_member_inventory_and_medication_detail_have_no_mutation_links() {
    let fixture = fixture();
    let viewer = Target::from_env();
    let me = viewer.get(
        &format!("/api/v1/households/{}/me", fixture.household_id),
        Some(&fixture.view_access_token),
    );
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().expect("viewer account identity");
    sign_in(
        &viewer,
        me["data"]["email_address"].as_str().unwrap(),
        "198.18.25.2",
    );
    let prefix = format!("/households/{}", fixture.household_slug);
    let inventory = page(&viewer, &format!("{prefix}/medications"));
    assert!(
        !has_link(&inventory, &format!("{prefix}/medications/new")),
        "view-only inventory must not advertise medication creation"
    );
    let medication = page(
        &viewer,
        &format!("{prefix}/medications/{}", fixture.managed_medication_id),
    );
    assert!(
        !has_link(
            &medication,
            &format!(
                "{prefix}/medications/{}/edit",
                fixture.managed_medication_id
            )
        ),
        "view-only medication detail must not advertise editing"
    );
}
