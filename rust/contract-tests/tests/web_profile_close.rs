use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn synthetic_account(
    db: &mut Client,
    source_account: i64,
    source_person: i64,
    household: i64,
) -> (i64, i64, i64, String, String) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let email = format!("advanced-close-{unique}@example.test");
    let token = format!("mt_app_advanced_close_{unique}");
    let account_id: i64 = db
        .query_one(
            "INSERT INTO accounts (email, status, password_hash, preferences, created_at, updated_at) SELECT $2, status, password_hash, preferences, now(), now() FROM accounts WHERE id = $1 RETURNING id",
            &[&source_account, &email],
        )
        .expect("synthetic account")
        .get(0);
    let person_id: i64 = db
        .query_one(
            "INSERT INTO people (account_id, household_id, name, email, person_type, date_of_birth, has_capacity, created_at, updated_at) SELECT $1, $2, 'Closure contract person', $3, person_type, date_of_birth, has_capacity, now(), now() FROM people WHERE id = $4 RETURNING id",
            &[&account_id, &household, &email, &source_person],
        )
        .expect("synthetic person")
        .get(0);
    db.execute(
        "INSERT INTO users (person_id, email_address, active, created_at, updated_at) VALUES ($1, $2, true, now(), now())",
        &[&person_id, &email],
    )
    .expect("synthetic user");
    let membership_id: i64 = db
        .query_one(
            "INSERT INTO household_memberships (account_id, household_id, person_id, permissions_version, role, status, joined_at, created_at, updated_at) SELECT $1, $2, $3, permissions_version, 'member', 'active', now(), now(), now() FROM household_memberships WHERE account_id = $4 AND household_id = $2 RETURNING id",
            &[&account_id, &household, &person_id, &source_account],
        )
        .expect("synthetic membership")
        .get(0);
    db.execute(
        "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'self', now(), now())",
        &[&household, &membership_id, &person_id],
    )
    .expect("synthetic self grant");
    db.execute(
        "INSERT INTO health_events (household_id, person_id, event_kind, title, started_on, created_at, updated_at) VALUES ($1, $2, 0, 'Synthetic retained history', CURRENT_DATE, now(), now())",
        &[&household, &person_id],
    )
    .expect("synthetic health history");
    db.execute(
        "INSERT INTO api_app_tokens (account_id, household_membership_id, token_digest, permissions_version, name, expires_at, last_used_at, created_at, updated_at) SELECT $1, $2, encode(digest($3, 'sha256'), 'hex'), permissions_version, 'Synthetic closure token', now() + interval '1 day', now(), now(), now() FROM household_memberships WHERE id = $2",
        &[&account_id, &membership_id, &token],
    )
    .expect("synthetic API token");
    (account_id, person_id, membership_id, email, token)
}

fn second_household_person(
    db: &mut Client,
    account_id: i64,
    household_id: i64,
) -> (i64, i64, String) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let email = format!("advanced-second-{unique}@example.test");
    let token = format!("mt_app_advanced_second_{unique}");
    let person_id: i64 = db.query_one(
        "INSERT INTO people (account_id, household_id, name, email, person_type, date_of_birth, has_capacity, created_at, updated_at) VALUES ($1, $2, 'Closure second household person', $3, 0, '1990-01-01', true, now(), now()) RETURNING id",
        &[&account_id, &household_id, &email],
    ).expect("second household person").get(0);
    db.execute(
        "INSERT INTO users (person_id, email_address, active, created_at, updated_at) VALUES ($1, $2, true, now(), now())",
        &[&person_id, &email],
    ).expect("second household user");
    let membership_id: i64 = db.query_one(
        "INSERT INTO household_memberships (account_id, household_id, person_id, permissions_version, role, status, joined_at, created_at, updated_at) VALUES ($1, $2, $3, 1, 'member', 'active', now(), now(), now()) RETURNING id",
        &[&account_id, &household_id, &person_id],
    ).expect("second household membership").get(0);
    db.execute(
        "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'self', now(), now())",
        &[&household_id, &membership_id, &person_id],
    ).expect("second household self grant");
    db.execute(
        "INSERT INTO health_events (household_id, person_id, event_kind, title, started_on, created_at, updated_at) VALUES ($1, $2, 0, 'Synthetic second retained history', CURRENT_DATE, now(), now())",
        &[&household_id, &person_id],
    ).expect("second household health history");
    db.execute(
        "INSERT INTO api_app_tokens (account_id, household_membership_id, token_digest, permissions_version, name, expires_at, last_used_at, created_at, updated_at) VALUES ($1, $2, encode(digest($3, 'sha256'), 'hex'), 1, 'Synthetic second closure token', now() + interval '1 day', now(), now(), now())",
        &[&account_id, &membership_id, &token],
    ).expect("second household API token");
    (person_id, membership_id, token)
}

#[test]
fn close_account_requires_password_and_revokes_access_without_erasing_health_history() {
    let fixture = fixture();
    let target = Target::from_env();
    let mut db = database();
    let slug: String = db
        .query_one(
            "SELECT slug FROM households WHERE id = $1",
            &[&fixture.profile_household_id],
        )
        .expect("profile household")
        .get(0);
    let (account_id, person_id, membership_id, email, app_token) = synthetic_account(
        &mut db,
        fixture.profile_account_id,
        fixture.profile_person_id,
        fixture.profile_household_id,
    );
    let second_household_id: i64 = db
        .query_one(
            "SELECT id FROM households WHERE id <> $1 ORDER BY id LIMIT 1",
            &[&fixture.profile_household_id],
        )
        .expect("second disposable household")
        .get(0);
    let (second_person_id, _, second_app_token) =
        second_household_person(&mut db, account_id, second_household_id);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    for index in 0..2 {
        db.execute(
            "INSERT INTO native_device_tokens (account_id, platform, device_token, created_at, updated_at) VALUES ($1, 'ios', $2, now(), now())",
            &[&account_id, &format!("closure-device-{unique}-{index}")],
        ).expect("account native device token");
        db.execute(
            "INSERT INTO push_subscriptions (account_id, endpoint, p256dh, auth, created_at, updated_at) VALUES ($1, $2, 'public-key', 'auth-key', now(), now())",
            &[&account_id, &format!("https://fcm.googleapis.com/fcm/send/closure-{unique}-{index}")],
        ).expect("account push subscription");
    }
    let login = target.get_html("/login");
    let login_csrf = csrf(&login.text().expect("login HTML"));
    let logged_in = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".to_owned(), email),
            ("password".to_owned(), "password".to_owned()),
            ("authenticity_token".to_owned(), login_csrf),
        ],
        "198.51.100.56",
    );
    assert_eq!(logged_in.status().as_u16(), 302);
    let profile = target.get_html(&format!("/households/{slug}/profile?section=advanced"));
    assert_eq!(profile.status().as_u16(), 200);
    let html = profile.text().expect("Advanced page");
    assert!(html.contains("name=\"api_app_token[name]\""));
    let csrf = csrf(&html);
    let issued = target.post_browser_form(
        &format!("/households/{slug}/profile/api_tokens"),
        &[
            ("authenticity_token".to_owned(), csrf.clone()),
            (
                "api_app_token[name]".to_owned(),
                "Self member token".to_owned(),
            ),
            (
                "api_app_token[household_membership_id]".to_owned(),
                membership_id.to_string(),
            ),
        ],
    );
    assert_eq!(issued.status().as_u16(), 201);
    let issued_html = issued.text().expect("full profile token page");
    assert!(issued_html.contains("profile-tabs"));
    assert!(issued_html.contains("Self member token"));
    assert!(issued_html.contains("mt_app_"));
    let path = format!("/households/{slug}/profile/close_account");
    let me = format!("/api/v1/households/{}/me", fixture.profile_household_id);
    let second_me = format!("/api/v1/households/{second_household_id}/me");
    assert_eq!(target.get(&me, Some(&app_token)).status().as_u16(), 200);
    assert_eq!(
        target
            .get(&second_me, Some(&second_app_token))
            .status()
            .as_u16(),
        200
    );

    let wrong = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf.clone()),
            ("password".to_owned(), "incorrect".to_owned()),
        ],
    );
    assert_eq!(wrong.status().as_u16(), 422);
    let status: i32 = db
        .query_one("SELECT status FROM accounts WHERE id = $1", &[&account_id])
        .expect("account remains active")
        .get(0);
    assert_eq!(status, 2);

    let closed = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".to_owned(), csrf),
            ("password".to_owned(), "password".to_owned()),
        ],
    );
    assert_eq!(closed.status().as_u16(), 303);
    assert_eq!(closed.headers()["location"], "/login");
    let row = db
        .query_one(
            "SELECT status, (SELECT account_id FROM people WHERE id = $2), (SELECT active FROM users WHERE person_id = $2), (SELECT count(*) FROM health_events WHERE person_id = $2), (SELECT count(*) FROM account_active_session_keys WHERE account_id = $1), (SELECT count(*) FROM api_app_tokens WHERE account_id = $1 AND revoked_at IS NULL) FROM accounts WHERE id = $1",
            &[&account_id, &person_id],
        )
        .expect("closed account state");
    assert_eq!(row.get::<_, i32>(0), 3);
    assert_eq!(row.get::<_, Option<i64>>(1), None);
    assert_eq!(row.get::<_, bool>(2), false);
    assert_eq!(row.get::<_, i64>(3), 1);
    assert_eq!(row.get::<_, i64>(4), 0);
    assert_eq!(row.get::<_, i64>(5), 0);
    assert_eq!(target.get(&me, Some(&app_token)).status().as_u16(), 401);
    assert_eq!(
        target
            .get(&second_me, Some(&second_app_token))
            .status()
            .as_u16(),
        401
    );
    let second = db.query_one(
        "SELECT account_id, (SELECT active FROM users WHERE person_id = $1), (SELECT count(*) FROM health_events WHERE person_id = $1) FROM people WHERE id = $1",
        &[&second_person_id],
    ).expect("second household retained person");
    assert_eq!(second.get::<_, Option<i64>>(0), None);
    assert_eq!(second.get::<_, bool>(1), false);
    assert_eq!(second.get::<_, i64>(2), 1);
    let credentials = db.query_one(
        "SELECT (SELECT count(*) FROM native_device_tokens WHERE account_id = $1), (SELECT count(*) FROM push_subscriptions WHERE account_id = $1)",
        &[&account_id],
    ).expect("revoked device credentials");
    assert_eq!(credentials.get::<_, i64>(0), 0);
    assert_eq!(credentials.get::<_, i64>(1), 0);
    let source_account_link: Option<i64> = db
        .query_one(
            "SELECT account_id FROM people WHERE id = $1",
            &[&fixture.profile_person_id],
        )
        .expect("other account remains linked")
        .get(0);
    assert_eq!(source_account_link, Some(fixture.profile_account_id));
    assert_eq!(
        target
            .get_html(&format!("/households/{slug}/profile"))
            .status()
            .as_u16(),
        302
    );
}
