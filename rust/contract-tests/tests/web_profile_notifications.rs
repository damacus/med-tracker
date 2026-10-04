use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn csrf(html: &str) -> String {
    let page = Html::parse_document(html);
    let selector = Selector::parse("input[name='authenticity_token']").unwrap();
    page.select(&selector)
        .next()
        .and_then(|node| node.value().attr("value"))
        .expect("CSRF input")
        .to_owned()
}

#[test]
fn notification_profile_saves_times_and_managed_adult_together() {
    let fixture = fixture();
    let mut db = Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable contract DB"),
        NoTls,
    )
    .expect("contract database");
    let slug: String = db
        .query_one(
            "SELECT slug FROM households WHERE id = $1",
            &[&fixture.profile_household_id],
        )
        .unwrap()
        .get(0);
    let membership_id: i64 = db
        .query_one(
            "SELECT id FROM household_memberships WHERE account_id = $1 AND household_id = $2",
            &[&fixture.profile_account_id, &fixture.profile_household_id],
        )
        .unwrap()
        .get(0);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let portable_id = format!("profile-managed-{unique}");
    let adult_id: i64 = db.query_one(
        "INSERT INTO people (household_id, portable_id, name, person_type, date_of_birth, has_capacity, created_at, updated_at) VALUES ($1, $2, 'Synthetic managed adult', 0, '1990-01-01', true, now(), now()) RETURNING id",
        &[&fixture.profile_household_id, &portable_id]).expect("adult fixture").get(0);
    let grant_id: i64 = db.query_one(
        "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, missed_dose_notifications_enabled, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'family_member', false, now(), now()) RETURNING id",
        &[&fixture.profile_household_id, &membership_id, &adult_id]).expect("managed grant fixture").get(0);
    let target = Target::from_env();
    let login = target.get_html("/login");
    let signed_in = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".into(), fixture.profile_email.clone()),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), csrf(&login.text().unwrap())),
        ],
        "198.51.100.58",
    );
    assert_eq!(signed_in.status().as_u16(), 302);
    let profile = target.get_html(&format!("/households/{slug}/profile?section=notifications"));
    assert_eq!(profile.status().as_u16(), 200);
    let html = profile.text().unwrap();
    assert!(html.contains("Synthetic managed adult"));
    assert!(html.contains("data-profile-push"));
    let csrf = csrf(&html);
    let action = format!("/households/{slug}/profile/notifications");
    let saved = target.post_browser_form(
        &action,
        &[
            ("authenticity_token".into(), csrf.clone()),
            ("enabled".into(), "true".into()),
            ("dose_due_enabled".into(), "true".into()),
            ("missed_dose_enabled".into(), "true".into()),
            ("low_stock_enabled".into(), "true".into()),
            ("morning_time".into(), "08:30".into()),
            ("afternoon_time".into(), "".into()),
            ("evening_time".into(), "18:15".into()),
            ("night_time".into(), "".into()),
            ("managed_person_ids[]".into(), "".into()),
            ("managed_person_ids[]".into(), adult_id.to_string()),
        ],
    );
    assert_eq!(saved.status().as_u16(), 303);
    let row = db.query_one("SELECT to_char(morning_time, 'HH24:MI'), to_char(evening_time, 'HH24:MI') FROM notification_preferences WHERE person_id = $1", &[&fixture.profile_person_id]).unwrap();
    assert_eq!(row.get::<_, Option<String>>(0).as_deref(), Some("08:30"));
    assert_eq!(row.get::<_, Option<String>>(1).as_deref(), Some("18:15"));
    let selected: bool = db
        .query_one(
            "SELECT missed_dose_notifications_enabled FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .unwrap()
        .get(0);
    assert!(selected);
    let cleared = target.post_browser_form(
        &action,
        &[
            ("authenticity_token".into(), csrf),
            ("enabled".into(), "true".into()),
            ("managed_person_ids[]".into(), "".into()),
            ("morning_time".into(), "".into()),
            ("afternoon_time".into(), "".into()),
            ("evening_time".into(), "".into()),
            ("night_time".into(), "".into()),
        ],
    );
    assert_eq!(cleared.status().as_u16(), 303);
    let selected: bool = db
        .query_one(
            "SELECT missed_dose_notifications_enabled FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .unwrap()
        .get(0);
    assert!(!selected);
    let morning: Option<String> = db.query_one("SELECT to_char(morning_time, 'HH24:MI') FROM notification_preferences WHERE person_id = $1", &[&fixture.profile_person_id]).unwrap().get(0);
    assert!(morning.is_none());
    db.execute(
        "DELETE FROM person_access_grants WHERE id = $1",
        &[&grant_id],
    )
    .unwrap();
    db.execute("DELETE FROM people WHERE id = $1", &[&adult_id])
        .unwrap();
}
