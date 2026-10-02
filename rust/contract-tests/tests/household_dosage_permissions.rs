use medtracker_contract_tests::{Fixture, Target, fixture};
use scraper::{Html, Selector};
use serde_json::{Value, json};

fn input(html: &str, selector: &str, attribute: &str) -> String {
    Html::parse_document(html)
        .select(&Selector::parse(selector).unwrap())
        .next()
        .and_then(|field| field.value().attr(attribute))
        .expect("authentic native value")
        .to_owned()
}

fn login(target: &Target, email: &str, client: &str) {
    let response = target.get_html("/login");
    assert_eq!(response.status().as_u16(), 200);
    let csrf = input(
        &response.text().unwrap(),
        "input[name='authenticity_token']",
        "value",
    );
    let response = target.post_html_form_from_client(
        "/login",
        client,
        &[
            ("authenticity_token".into(), csrf),
            ("email".into(), email.into()),
            ("password".into(), "password".into()),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn csrf(target: &Target, fixture: &Fixture) -> String {
    let response = target.get_html(&format!(
        "/households/{}/medications",
        fixture.household_slug
    ));
    assert_eq!(response.status().as_u16(), 200);
    input(
        &response.text().unwrap(),
        "meta[name='csrf-token']",
        "content",
    )
}

#[test]
fn ordinary_member_with_person_manage_grant_cannot_manage_dosage_options() {
    let fixture = fixture();
    let owner = Target::from_env();
    login(&owner, &fixture.primary_email, "198.18.90.1");
    let base = format!("/api/v1/households/{}", fixture.household_id);
    let parent = format!("{base}/medications/{}", fixture.managed_medication_id);
    let options = format!("{base}/dosage_options");
    let before = read(&owner, &parent);
    let options_before = read(&owner, &options);
    let owner_csrf = csrf(&owner, &fixture);
    let grant = owner.post_browser_json(&format!("{base}/admin/person_access_grants"), &owner_csrf, &json!({"person_access_grant": {
        "household_membership_id": fixture.view_membership_id, "person_id": fixture.user_person_id,
        "access_level": "manage", "relationship_type": "family_member"
    }}));
    assert_eq!(grant.status().as_u16(), 201);
    let member = Target::from_env();
    login(&member, &fixture.web_view_email, "198.18.90.2");
    assert_eq!(
        read(&member, &format!("{base}/me"))["membership_role"],
        "member"
    );
    let capabilities = read(&member, &format!("{base}/ui_capabilities"));
    assert_eq!(capabilities["medications"]["create"], true);
    assert_eq!(capabilities["medications"]["update"], false);
    assert!(
        capabilities["people"]["manage_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == &json!(fixture.user_person_id))
    );
    let web = format!(
        "/households/{}/medications/{}/dosage_options",
        fixture.household_slug, fixture.managed_medication_id
    );
    let list = member.get_html(&web);
    assert_eq!(list.status().as_u16(), 200);
    let document = Html::parse_document(&list.text().unwrap());
    assert!(
        document
            .select(&Selector::parse("a[href$='/dosage_options/new'], a[href$='/edit']").unwrap())
            .next()
            .is_none()
    );
    assert_eq!(
        member.get_html(&format!("{web}/new")).status().as_u16(),
        403
    );
    let csrf = csrf(&member, &fixture);
    let fields = vec![
        ("authenticity_token".into(), csrf),
        ("amount".into(), "1.25".into()),
        ("unit".into(), "ml".into()),
        ("frequency".into(), "Daily".into()),
    ];
    assert_eq!(
        member.post_browser_form(&web, &fields).status().as_u16(),
        403
    );
    let id = options_before
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["medication_id"] == fixture.managed_medication_id)
        .expect("visible fixture dosage")["id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        member
            .get_html(&format!("{web}/{id}/edit"))
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        member
            .post_browser_form(&format!("{web}/{id}"), &fields)
            .status()
            .as_u16(),
        403
    );
    assert_eq!(read(&owner, &parent), before);
    assert_eq!(read(&owner, &options), options_before);
}
