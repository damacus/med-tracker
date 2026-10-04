use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use medtracker_contract_tests::{fixture, Target};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use postgres::{Client, NoTls};
use scraper::{Html, Selector};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    env,
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn database() -> Client {
    Client::connect(&env::var("CONTRACT_AUDIT_DATABASE_URL").unwrap(), NoTls).unwrap()
}

fn input(html: &str, name: &str) -> String {
    Html::parse_document(html)
        .select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next()
        .unwrap()
        .value()
        .attr("value")
        .unwrap()
        .to_owned()
}

struct Credential {
    id: String,
    handle: String,
    key: SigningKey,
    row_id: i64,
}

impl Credential {
    fn create() -> Self {
        let account = fixture().account_id;
        let id = URL_SAFE_NO_PAD.encode(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_be_bytes(),
        );
        let handle = URL_SAFE_NO_PAD.encode(format!("passkey-security-{id}"));
        let key = SigningKey::from_slice(&[0x42; 32]).unwrap();
        let point = key.verifying_key().to_sec1_point(false);
        let mut cose = vec![0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20];
        cose.extend_from_slice(point.x().unwrap());
        cose.extend_from_slice(&[0x22, 0x58, 0x20]);
        cose.extend_from_slice(point.y().unwrap());
        let mut db = database();
        let row_id = db.query_one("INSERT INTO account_webauthn_keys (account_id, webauthn_id, public_key, sign_count) VALUES ($1, $2, $3, 0) RETURNING id", &[&account, &id, &URL_SAFE_NO_PAD.encode(cose)]).unwrap().get(0);
        db.execute(
            "INSERT INTO account_webauthn_user_ids (account_id, webauthn_id) VALUES ($1, $2)",
            &[&account, &handle],
        )
        .unwrap();
        Self {
            id,
            handle,
            key,
            row_id,
        }
    }

    fn form(&self, target: &Target, counter: u32) -> Vec<(String, String)> {
        let html = target.get_html("/login").text().unwrap();
        let challenge = input(&html, "webauthn_auth_challenge");
        let origin = env::var("CONTRACT_BASE_URL")
            .unwrap()
            .trim_end_matches('/')
            .to_owned();
        let rp = url::Url::parse(&origin)
            .unwrap()
            .host_str()
            .unwrap()
            .to_owned();
        let client = serde_json::to_vec(&json!({"type": "webauthn.get", "challenge": challenge, "origin": origin, "crossOrigin": false})).unwrap();
        let mut auth = Sha256::digest(rp.as_bytes()).to_vec();
        auth.push(5);
        auth.extend_from_slice(&counter.to_be_bytes());
        let mut signed = auth.clone();
        signed.extend_from_slice(&Sha256::digest(&client));
        let signature: Signature = self.key.sign(&signed);
        let payload = json!({"id": self.id, "rawId": self.id, "type": "public-key", "response": {
            "clientDataJSON": URL_SAFE_NO_PAD.encode(client), "authenticatorData": URL_SAFE_NO_PAD.encode(auth),
            "signature": URL_SAFE_NO_PAD.encode(signature.to_der().as_bytes()), "userHandle": self.handle
        }});
        vec![
            (
                "authenticity_token".into(),
                input(&html, "authenticity_token"),
            ),
            ("webauthn_auth_challenge".into(), challenge),
            (
                "webauthn_auth_challenge_hmac".into(),
                input(&html, "webauthn_auth_challenge_hmac"),
            ),
            ("webauthn_auth".into(), payload.to_string()),
        ]
    }
}

impl Drop for Credential {
    fn drop(&mut self) {
        let mut db = database();
        db.execute(
            "DELETE FROM account_webauthn_keys WHERE id = $1",
            &[&self.row_id],
        )
        .unwrap();
        db.execute(
            "DELETE FROM account_webauthn_user_ids WHERE webauthn_id = $1",
            &[&self.handle],
        )
        .unwrap();
    }
}

#[test]
fn a_consumed_counterless_challenge_cannot_mint_another_session() {
    let credential = Credential::create();
    let target = Target::from_env();
    let login = target.get_html("/login");
    let original_cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let form = credential.form(&target, 0);
    let first = target.post_html_form_with_cookie("/webauthn-login", &form, &original_cookie);
    assert_eq!(first.status().as_u16(), 302);
    let replay = target.post_html_form_with_cookie("/webauthn-login", &form, &original_cookie);
    assert_eq!(replay.status().as_u16(), 200);
    assert!(!replay.status().is_redirection());
    assert!(!replay.headers().contains_key("set-cookie"));
}

#[test]
fn a_counter_that_cannot_be_persisted_is_rejected_without_creating_a_session() {
    let credential = Credential::create();
    let target = Target::from_env();
    let form = credential.form(&target, i32::MAX as u32 + 1);
    let response = target.post_html_form_from_client("/webauthn-login", "127.0.0.83", &form);
    assert_eq!(response.status().as_u16(), 200);
    assert!(!response.headers().contains_key("set-cookie"));
    assert_eq!(
        database()
            .query_one(
                "SELECT sign_count FROM account_webauthn_keys WHERE id = $1",
                &[&credential.row_id]
            )
            .unwrap()
            .get::<_, i32>(0),
        0
    );
}

#[test]
fn discoverable_login_requires_a_user_handle() {
    let credential = Credential::create();
    let target = Target::from_env();
    let mut form = credential.form(&target, 1);
    let payload = &mut form
        .iter_mut()
        .find(|(name, _)| name == "webauthn_auth")
        .unwrap()
        .1;
    let mut assertion: serde_json::Value = serde_json::from_str(payload).unwrap();
    assertion["response"]
        .as_object_mut()
        .unwrap()
        .remove("userHandle");
    *payload = assertion.to_string();
    let response = target.post_html_form_from_client("/webauthn-login", "127.0.0.84", &form);
    assert_eq!(response.status().as_u16(), 200);
    assert!(!response.headers().contains_key("set-cookie"));
}

#[test]
fn passkey_login_waits_for_account_before_a_concurrent_key_removal() {
    let credential = Credential::create();
    let target = Target::from_env();
    let form = credential.form(&target, 1);
    let mut db = database();
    let mut held = db.transaction().unwrap();
    held.query_one(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        &[&fixture().account_id],
    )
    .unwrap();
    let holder_pid: i32 = held
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let worker = thread::spawn(move || {
        target
            .post_html_form_from_client("/webauthn-login", "127.0.0.85", &form)
            .status()
            .as_u16()
    });
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut observer = database();
    loop {
        let waiting: bool = observer
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid)))",
                &[&holder_pid],
            )
            .unwrap()
            .get(0);
        if waiting {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "passkey login did not wait for account lock"
        );
        thread::sleep(Duration::from_millis(20));
    }
    held.execute(
        "DELETE FROM account_webauthn_keys WHERE id = $1",
        &[&credential.row_id],
    )
    .unwrap();
    held.commit().unwrap();
    assert_ne!(worker.join().unwrap(), 302);
}

#[test]
fn simultaneous_distinct_challenges_cannot_accept_the_same_positive_counter() {
    let credential = Credential::create();
    let targets = [Arc::new(Target::from_env()), Arc::new(Target::from_env())];
    let forms = targets
        .iter()
        .map(|target| credential.form(target, 1))
        .collect::<Vec<_>>();
    let mut db = database();
    let mut held = db.transaction().unwrap();
    held.query_one(
        "SELECT id FROM account_webauthn_keys WHERE id = $1 FOR UPDATE",
        &[&credential.row_id],
    )
    .unwrap();
    let workers = targets
        .into_iter()
        .zip(forms)
        .map(|(target, form)| {
            thread::spawn(move || {
                target
                    .post_html_form_from_client("/webauthn-login", "127.0.0.82", &form)
                    .status()
                    .as_u16()
            })
        })
        .collect::<Vec<_>>();
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut observer = database();
    loop {
        let waiting: i64 = observer.query_one("SELECT count(*) FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock' AND cardinality(pg_blocking_pids(pid)) > 0 AND (query LIKE '%accounts%' OR query LIKE '%account_webauthn_keys%')", &[]).unwrap().get(0);
        if waiting >= 2 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "both login transactions must reach the held credential lock"
        );
        thread::sleep(Duration::from_millis(20));
    }
    held.commit().unwrap();
    let statuses = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        statuses.iter().filter(|status| **status == 302).count(),
        1,
        "{statuses:?}"
    );
    assert_eq!(
        db.query_one(
            "SELECT sign_count FROM account_webauthn_keys WHERE id = $1",
            &[&credential.row_id]
        )
        .unwrap()
        .get::<_, i32>(0),
        1
    );
}
