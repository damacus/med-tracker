use medtracker_contract_tests::{fixture, Target};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use std::env;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn database() -> Client {
    Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("disposable contract database"),
        NoTls,
    )
    .expect("connect to disposable contract database")
}

fn csrf(html: &str) -> String {
    Html::parse_document(html)
        .select(&Selector::parse("input[name='authenticity_token']").unwrap())
        .next()
        .and_then(|node| node.value().attr("value"))
        .expect("CSRF input")
        .to_owned()
}

fn synthetic_account(db: &mut Client) -> (i64, String, String) {
    let fixture = fixture();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let email = format!("security-race-{unique}@example.test");
    let account_id: i64 = db
        .query_one(
            "INSERT INTO accounts (email, status, password_hash, preferences, created_at, updated_at) SELECT $2, status, password_hash, preferences, now(), now() FROM accounts WHERE id = $1 RETURNING id",
            &[&fixture.profile_account_id, &email],
        )
        .expect("synthetic account")
        .get(0);
    let person_id: i64 = db
        .query_one(
            "INSERT INTO people (account_id, household_id, name, email, person_type, date_of_birth, has_capacity, created_at, updated_at) SELECT $1, $2, 'Security contract person', $3, person_type, date_of_birth, has_capacity, now(), now() FROM people WHERE id = $4 RETURNING id",
            &[&account_id, &fixture.profile_household_id, &email, &fixture.profile_person_id],
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
            &[&account_id, &fixture.profile_household_id, &person_id, &fixture.profile_account_id],
        )
        .expect("synthetic membership")
        .get(0);
    db.execute(
        "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'self', now(), now())",
        &[&fixture.profile_household_id, &membership_id, &person_id],
    )
    .expect("synthetic self grant");
    let slug: String = db
        .query_one(
            "SELECT slug FROM households WHERE id = $1",
            &[&fixture.profile_household_id],
        )
        .expect("profile household")
        .get(0);
    (account_id, email, slug)
}

#[test]
fn a_closed_account_cannot_finish_a_password_change_waiting_on_its_row_lock() {
    let mut db = database();
    let (account_id, email, slug) = synthetic_account(&mut db);
    let target = Target::from_env();
    let login = target.get_html("/login");
    let signed_in = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".into(), email),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), csrf(&login.text().unwrap())),
        ],
        "198.51.100.59",
    );
    assert_eq!(signed_in.status().as_u16(), 302);
    let profile = target.get_html(&format!("/households/{slug}/profile?section=security"));
    assert_eq!(profile.status().as_u16(), 200);
    let token = csrf(&profile.text().unwrap());
    let old_hash: String = db
        .query_one(
            "SELECT password_hash FROM accounts WHERE id = $1",
            &[&account_id],
        )
        .unwrap()
        .get(0);

    let mut held = db.transaction().expect("hold account lock");
    held.query_one(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        &[&account_id],
    )
    .expect("account lock");
    let holder_pid: i32 = held
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let worker = thread::spawn(move || {
        target
            .post_browser_form(
                &format!("/households/{slug}/settings/security/password"),
                &[
                    ("authenticity_token".into(), token),
                    ("current_password".into(), "password".into()),
                    ("new_password".into(), "new-password-123!".into()),
                    ("password_confirmation".into(), "new-password-123!".into()),
                ],
            )
            .status()
            .as_u16()
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut observer = database();
    loop {
        let blocked: bool = observer
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid)))",
                &[&holder_pid],
            )
            .expect("blocked request")
            .get(0);
        if blocked {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "password change did not wait on the account lock"
        );
        thread::sleep(Duration::from_millis(25));
    }
    held.execute(
        "UPDATE accounts SET status = 3 WHERE id = $1",
        &[&account_id],
    )
    .expect("close account while mutation waits");
    held.commit().expect("publish closed status");
    let status = worker.join().expect("password worker");
    assert_ne!(status, 500);
    let row = db
        .query_one(
            "SELECT status, password_hash FROM accounts WHERE id = $1",
            &[&account_id],
        )
        .unwrap();
    assert_eq!(row.get::<_, i32>(0), 3);
    assert_eq!(row.get::<_, String>(1), old_hash);
}

#[test]
fn email_confirmation_and_password_change_revoke_sessions_and_allow_new_credentials() {
    let mut db = database();
    let (account_id, email, slug) = synthetic_account(&mut db);
    let target = Target::from_env();
    let login = target.get_html("/login");
    assert_eq!(
        target
            .post_html_form_from_local_client(
                "/login",
                &[
                    ("email".into(), email),
                    ("password".into(), "password".into()),
                    ("authenticity_token".into(), csrf(&login.text().unwrap())),
                ],
                "198.51.100.60",
            )
            .status()
            .as_u16(),
        302
    );
    let profile_path = format!("/households/{slug}/profile?section=security");
    let security_base = format!("/households/{slug}/settings/security");
    let profile = target.get_html(&profile_path);
    assert_eq!(profile.status().as_u16(), 200);
    let profile_csrf = csrf(&profile.text().unwrap());
    let new_email = format!("changed-{account_id}@example.test");
    let denied = target.post_browser_form(
        &format!("{security_base}/email"),
        &[
            ("authenticity_token".into(), profile_csrf.clone()),
            ("email".into(), new_email.clone()),
            ("password".into(), "incorrect".into()),
        ],
    );
    assert_eq!(denied.status().as_u16(), 303);
    let pending: i64 = db
        .query_one(
            "SELECT count(*) FROM account_login_change_keys WHERE account_id = $1",
            &[&account_id],
        )
        .unwrap()
        .get(0);
    assert_eq!(pending, 0);
    let requested = target.post_browser_form(
        &format!("{security_base}/email"),
        &[
            ("authenticity_token".into(), profile_csrf),
            ("email".into(), new_email.clone()),
            ("password".into(), "password".into()),
        ],
    );
    assert_eq!(requested.status().as_u16(), 303);
    let token: String = db
        .query_one(
            "SELECT key FROM account_login_change_keys WHERE account_id = $1 AND login = $2",
            &[&account_id, &new_email],
        )
        .expect("pending email verification")
        .get(0);
    let verify = format!("{security_base}/email/verify");
    assert_eq!(
        target
            .get_html(&format!("{verify}?account_id={account_id}&token={token}"))
            .status()
            .as_u16(),
        200
    );
    let confirmed = target.post_browser_form(
        &verify,
        &[
            ("account_id".into(), account_id.to_string()),
            ("token".into(), token.clone()),
        ],
    );
    assert_eq!(confirmed.status().as_u16(), 303);
    assert_eq!(confirmed.headers()["location"], "/login");
    let replay = target.post_browser_form(
        &verify,
        &[
            ("account_id".into(), account_id.to_string()),
            ("token".into(), token),
        ],
    );
    assert_eq!(replay.status().as_u16(), 400);
    let saved_email: String = db
        .query_one("SELECT email FROM accounts WHERE id = $1", &[&account_id])
        .unwrap()
        .get(0);
    assert_eq!(saved_email, new_email);
    assert_eq!(target.get_html(&profile_path).status().as_u16(), 302);

    let target = Target::from_env();
    let login = target.get_html("/login");
    assert_eq!(
        target
            .post_html_form_from_local_client(
                "/login",
                &[
                    ("email".into(), new_email.clone()),
                    ("password".into(), "password".into()),
                    ("authenticity_token".into(), csrf(&login.text().unwrap())),
                ],
                "198.51.100.61",
            )
            .status()
            .as_u16(),
        302
    );
    let profile = target.get_html(&profile_path);
    assert_eq!(profile.status().as_u16(), 200);
    let original_hash: String = db
        .query_one(
            "SELECT password_hash FROM accounts WHERE id = $1",
            &[&account_id],
        )
        .unwrap()
        .get(0);
    let changed = target.post_browser_form(
        &format!("{security_base}/password"),
        &[
            ("authenticity_token".into(), csrf(&profile.text().unwrap())),
            ("current_password".into(), "password".into()),
            ("new_password".into(), "replacement-123!".into()),
            ("password_confirmation".into(), "replacement-123!".into()),
        ],
    );
    assert_eq!(changed.status().as_u16(), 303);
    assert_eq!(changed.headers()["location"], "/login");
    let saved_hash: String = db
        .query_one(
            "SELECT password_hash FROM accounts WHERE id = $1",
            &[&account_id],
        )
        .unwrap()
        .get(0);
    assert_ne!(saved_hash, original_hash);
    assert_eq!(target.get_html(&profile_path).status().as_u16(), 302);

    let target = Target::from_env();
    let login = target.get_html("/login");
    assert_eq!(
        target
            .post_html_form_from_local_client(
                "/login",
                &[
                    ("email".into(), new_email),
                    ("password".into(), "replacement-123!".into()),
                    ("authenticity_token".into(), csrf(&login.text().unwrap())),
                ],
                "198.51.100.62",
            )
            .status()
            .as_u16(),
        302
    );
    assert_eq!(target.get_html(&profile_path).status().as_u16(), 200);
}

#[test]
fn recovery_codes_require_current_password_even_after_multifactor_sign_in() {
    let mut db = database();
    let (account_id, email, slug) = synthetic_account(&mut db);
    let login_code = format!("login-recovery-{account_id}");
    let stored_code = format!("stored-recovery-{account_id}");
    db.execute(
        "INSERT INTO account_recovery_codes (id, code) VALUES ($1, $2), ($1, $3)",
        &[&account_id, &login_code, &stored_code],
    )
    .expect("synthetic recovery codes");
    let target = Target::from_env();
    let login = target.get_html("/login");
    let pending = target.post_html_form_from_local_client(
        "/login",
        &[
            ("email".into(), email),
            ("password".into(), "password".into()),
            ("authenticity_token".into(), csrf(&login.text().unwrap())),
        ],
        "198.51.100.63",
    );
    assert_eq!(pending.status().as_u16(), 302);
    assert_eq!(pending.headers()["location"], "/login-factor");
    let factor = target.get_html("/login-factor");
    assert_eq!(factor.status().as_u16(), 200);
    let completed = target.post_html_form(
        "/login-factor",
        &[
            ("authenticity_token".into(), csrf(&factor.text().unwrap())),
            ("factor".into(), "recovery".into()),
            ("code".into(), login_code),
        ],
    );
    assert_eq!(completed.status().as_u16(), 302);
    let path = format!("/households/{slug}/settings/security/recovery");
    let prompt = target.get_html(&path);
    assert_eq!(prompt.status().as_u16(), 200);
    let html = prompt.text().unwrap();
    assert!(
        !html.contains(&stored_code),
        "GET must not expose plaintext recovery codes"
    );
    let token = csrf(&html);
    assert_eq!(
        target
            .post_browser_form(
                &path,
                &[
                    ("authenticity_token".into(), "wrong".into()),
                    ("password".into(), "password".into()),
                ]
            )
            .status()
            .as_u16(),
        403
    );
    assert_eq!(
        target
            .post_browser_form(
                &path,
                &[
                    ("authenticity_token".into(), token.clone()),
                    ("password".into(), "incorrect".into()),
                ]
            )
            .status()
            .as_u16(),
        403
    );
    let viewed = target.post_browser_form(
        &path,
        &[
            ("authenticity_token".into(), token),
            ("password".into(), "password".into()),
        ],
    );
    assert_eq!(viewed.status().as_u16(), 200);
    assert!(viewed.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    assert!(viewed.text().unwrap().contains(&stored_code));
}
