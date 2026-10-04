#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_policy_matches_the_rails_account_requirement() {
        assert!(!password_meets_requirements("short1!"));
        assert!(!password_meets_requirements("abcdefghijkl"));
        assert!(!password_meets_requirements("abcdefghijk1"));
        assert!(!password_meets_requirements("abcdefghijk!"));
        assert!(password_meets_requirements("abcdefghij1!"));
        assert!(!password_meets_requirements("😀😀😀1!"));
        assert!(password_meets_requirements("éééééééééé1!"));
        assert!(!password_meets_requirements(&format!(
            "{}1!",
            "a".repeat(71)
        )));
    }

    #[test]
    fn email_change_rejects_invalid_or_oversized_addresses() {
        assert!(valid_email("someone@example.test"));
        assert!(!valid_email("not-an-email"));
        assert!(!valid_email("person@example.test\r\nBcc:evil@example.test"));
        assert!(!valid_email(&format!("{}@example.test", "a".repeat(320))));
    }

    #[test]
    fn new_otp_seed_matches_rodauth_storage_format() {
        let seed = new_otp_seed().expect("system randomness");
        assert_eq!(seed.len(), 32);
        assert!(seed
            .bytes()
            .all(|byte| b"abcdefghijklmnopqrstuvwxyz234567".contains(&byte)));
    }

    #[test]
    fn new_recovery_code_matches_rodauth_random_key_format() {
        let code = new_recovery_code().expect("system randomness");
        assert_eq!(code.len(), 43);
        assert!(code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
    }

    #[test]
    fn registered_es256_key_round_trips_through_rails_cose_storage() {
        use webauthn_rs_core::proto::{
            COSEAlgorithm, COSEEC2Key, COSEKey, COSEKeyType, ECDSACurve,
        };
        let point = p256::ecdsa::SigningKey::from_slice(&[42; 32])
            .unwrap()
            .verifying_key()
            .to_sec1_point(false);
        let key = COSEKey {
            type_: COSEAlgorithm::ES256,
            key: COSEKeyType::EC_EC2(COSEEC2Key {
                curve: ECDSACurve::SECP256R1,
                x: point.x().unwrap().to_vec().into(),
                y: point.y().unwrap().to_vec().into(),
            }),
        };
        let stored = rails_cose_public_key(&key).expect("Rails-compatible COSE encoding");
        let decoded = URL_SAFE_NO_PAD.decode(stored).unwrap();
        let value: serde_cbor_2::Value = serde_cbor_2::from_slice(&decoded).unwrap();
        assert_eq!(COSEKey::try_from(&value).unwrap(), key);
    }

    #[test]
    fn registered_rsa_and_eddsa_keys_round_trip_through_rails_cose_storage() {
        use webauthn_rs_core::proto::{COSEAlgorithm, COSEOKPKey, COSERSAKey, EDDSACurve};
        let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
        let rsa_key = COSEKey {
            type_: COSEAlgorithm::RS256,
            key: COSEKeyType::RSA(COSERSAKey {
                n: rsa.n().to_vec().into(),
                e: [1, 0, 1],
            }),
        };
        let ed = openssl::pkey::PKey::generate_ed25519().unwrap();
        let ed_key = COSEKey {
            type_: COSEAlgorithm::EDDSA,
            key: COSEKeyType::EC_OKP(COSEOKPKey {
                curve: EDDSACurve::ED25519,
                x: ed.raw_public_key().unwrap().into(),
            }),
        };
        for key in [rsa_key, ed_key] {
            let stored = rails_cose_public_key(&key).expect("Rails-compatible COSE encoding");
            let decoded = URL_SAFE_NO_PAD.decode(stored).unwrap();
            let value: serde_cbor_2::Value = serde_cbor_2::from_slice(&decoded).unwrap();
            assert_eq!(COSEKey::try_from(&value).unwrap(), key);
        }
    }

    #[test]
    fn passkey_registration_proof_has_a_five_minute_window() {
        assert!(fresh_registration(100, 399));
        assert!(!fresh_registration(100, 400));
        assert!(!fresh_registration(100, 99));
    }

    #[test]
    fn passkey_added_date_matches_the_rails_security_card() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 2)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(format_passkey_date(date), "October 02, 2026");
    }
}
use super::*;
use crate::entities::{
    account, active_session_key, otp_key, recovery_code, webauthn_key, webauthn_user_id,
};
use crate::oauth::AuthenticationClaim;
use axum::response::IntoResponse;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use medtracker_web::profile_security::{
    render_email_verification_page, render_passkey_registration_page, render_recovery_codes_page,
    render_recovery_prompt_page, render_security_section, render_totp_setup_page, SecurityPage,
    SecurityPasskey,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;
use totp_rs::{Algorithm, Builder, Secret};
use webauthn_rs_core::proto::{
    COSEKey, COSEKeyType, RegisterPublicKeyCredential, RegistrationState, UserVerificationPolicy,
};
use webauthn_rs_core::WebauthnCore;

static PASSWORD_WORKERS: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Deserialize, Serialize)]
struct RegistrationClaim {
    account_id: i64,
    csrf: String,
    issued_at: i64,
    nonce: String,
    user_handle: String,
    registration_state: RegistrationState,
}

impl AuthenticationClaim for RegistrationClaim {
    const PURPOSE: &'static str = "mt_passkey_registration";
}

fn fresh_registration(issued_at: i64, now: i64) -> bool {
    now.checked_sub(issued_at)
        .is_some_and(|age| (0..300).contains(&age))
}

fn format_passkey_date(created_at: chrono::NaiveDateTime) -> String {
    created_at.format("%B %d, %Y").to_string()
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/households/{slug}/settings/security/password",
            post(change_password),
        )
        .route(
            "/households/{slug}/settings/security/email",
            post(request_email_change),
        )
        .route(
            "/households/{slug}/settings/security/email/verify",
            get(show_email_verification).post(verify_email_change),
        )
        .route("/households/{slug}/settings/security/otp/new", get(new_otp))
        .route("/households/{slug}/settings/security/otp", post(enable_otp))
        .route(
            "/households/{slug}/settings/security/otp/disable",
            post(disable_otp),
        )
        .route(
            "/households/{slug}/settings/security/recovery",
            get(show_recovery_codes).post(view_recovery_codes),
        )
        .route(
            "/households/{slug}/settings/security/recovery/generate",
            post(generate_recovery_codes),
        )
        .route(
            "/households/{slug}/settings/security/passkeys/{id}/nickname",
            post(rename_passkey),
        )
        .route(
            "/households/{slug}/settings/security/passkeys/{id}/remove",
            post(remove_passkey),
        )
        .route(
            "/households/{slug}/settings/security/passkeys/new",
            get(new_passkey),
        )
        .route(
            "/households/{slug}/settings/security/passkeys",
            post(register_passkey),
        )
        .route("/profile-security.js", get(passkey_registration_script))
}

async fn passkey_registration_script() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../web/src/profile-security.js"),
    )
        .into_response()
}

fn sql(query: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}

fn valid_email(email: &str) -> bool {
    !email.is_empty() && email.len() <= 320 && email.parse::<lettre::Address>().is_ok()
}

fn new_otp_seed() -> Result<String, PageError> {
    let mut random = [0_u8; 32];
    getrandom::fill(&mut random).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let alphabet = b"abcdefghijklmnopqrstuvwxyz234567";
    Ok(random
        .iter()
        .map(|byte| alphabet[(byte & 31) as usize] as char)
        .collect())
}

fn pending_otp_id(seed: &str) -> String {
    format!("otp-setup:{}", hex::encode(Sha256::digest(seed.as_bytes())))
}

fn new_recovery_code() -> Result<String, PageError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn rails_cose_public_key(key: &COSEKey) -> Result<String, PageError> {
    let mut entries = vec![(3, serde_cbor_2::Value::Integer(key.type_ as i128))];
    match &key.key {
        COSEKeyType::EC_EC2(ec) => {
            entries.extend([
                (1, serde_cbor_2::Value::Integer(2)),
                (-1, serde_cbor_2::Value::Integer(ec.curve.clone() as i128)),
                (-2, serde_cbor_2::Value::Bytes(ec.x.as_slice().to_vec())),
                (-3, serde_cbor_2::Value::Bytes(ec.y.as_slice().to_vec())),
            ]);
        }
        COSEKeyType::RSA(rsa) => {
            entries.extend([
                (1, serde_cbor_2::Value::Integer(3)),
                (-1, serde_cbor_2::Value::Bytes(rsa.n.as_slice().to_vec())),
                (-2, serde_cbor_2::Value::Bytes(rsa.e.to_vec())),
            ]);
        }
        COSEKeyType::EC_OKP(okp) => {
            entries.extend([
                (1, serde_cbor_2::Value::Integer(1)),
                (-1, serde_cbor_2::Value::Integer(okp.curve.clone() as i128)),
                (-2, serde_cbor_2::Value::Bytes(okp.x.as_slice().to_vec())),
            ]);
        }
    }
    let map = entries
        .into_iter()
        .map(|(key, value)| (serde_cbor_2::Value::Integer(key), value))
        .collect();
    let value = serde_cbor_2::Value::Map(map);
    let decoded = COSEKey::try_from(&value).map_err(|_| error(StatusCode::BAD_REQUEST))?;
    if decoded != *key {
        return Err(error(StatusCode::BAD_REQUEST));
    }
    let bytes =
        serde_cbor_2::to_vec(&value).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn authenticator_uri(
    seed: &str,
    email: &str,
    hmac_secret: &[u8],
) -> Result<(String, String), PageError> {
    let derived = crate::auth_compatibility::derive_rodauth_otp_secret(seed, Some(hmac_secret))
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let secret = Secret::try_from_base32(derived.to_ascii_uppercase())
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let uri = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_step_duration(30)
        .with_secret(secret)
        .with_account_name(email.to_owned())
        .with_issuer(Some("MedTracker"))
        .build()
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
        .to_url()
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok((derived, uri))
}

fn status_location(slug: &str, status: &str) -> String {
    format!(
        "/households/{}/profile?section=security&security_status={status}",
        medtracker_web::household::path_segment(slug)
    )
}

async fn browser_form(
    state: &AppState,
    slug: &str,
    headers: HeaderMap,
    fields: &HashMap<String, String>,
) -> Result<WebApi, PageError> {
    if !oauth::trusted_cookie_origin(state, &headers) {
        return Err(error(StatusCode::FORBIDDEN));
    }
    let mut api = WebApi::authenticated(state.clone(), headers).await?;
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return Err(error(StatusCode::FORBIDDEN));
    }
    api.household(slug).await?;
    Ok(api)
}

pub(super) fn password_meets_requirements(value: &str) -> bool {
    value.chars().count() >= 12
        && value.len() <= 72
        && value.bytes().any(|byte| byte.is_ascii_digit())
        && value.bytes().any(|byte| !byte.is_ascii_alphanumeric())
}

pub(super) async fn verify_password_hash(
    password: String,
    hash: String,
) -> Result<bool, PageError> {
    let workers = PASSWORD_WORKERS
        .get_or_init(|| Arc::new(Semaphore::new(4)))
        .clone();
    let permit = workers
        .acquire_owned()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        bcrypt::verify(password, &hash).unwrap_or(false)
    })
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}

pub(crate) async fn verify_password_hash_for_close(
    password: String,
    hash: String,
) -> Result<bool, ()> {
    verify_password_hash(password, hash).await.map_err(|_| ())
}

async fn new_hash(password: String) -> Result<String, PageError> {
    let workers = PASSWORD_WORKERS
        .get_or_init(|| Arc::new(Semaphore::new(4)))
        .clone();
    let permit = workers
        .acquire_owned()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
    })
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}

async fn change_password(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let current = fields
        .get("current_password")
        .map(String::as_str)
        .unwrap_or("");
    let proposed = fields.get("new_password").map(String::as_str).unwrap_or("");
    let confirmation = fields
        .get("password_confirmation")
        .map(String::as_str)
        .unwrap_or("");
    if !password_meets_requirements(proposed) || proposed != confirmation || current.len() > 1024 {
        return redirect(status_location(&slug, "password_invalid"), api.cookie);
    }
    let result = async {
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let row = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if row.status != 2 {
            return Err(error(StatusCode::UNAUTHORIZED));
        }
        let Some(hash) = row.password_hash.as_ref() else {
            return Ok(false);
        };
        if !verify_password_hash(current.to_owned(), hash.clone()).await? {
            return Ok(false);
        }
        let mut active: account::ActiveModel = row.into();
        active.password_hash = Set(Some(new_hash(proposed.to_owned()).await?));
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active
            .update(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        active_session_key::Entity::delete_many()
            .filter(active_session_key::Column::AccountId.eq(api.account_id))
            .exec(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<bool, PageError>(true)
    }
    .await;
    match result {
        Ok(true) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Ok(false) => redirect(status_location(&slug, "password_invalid"), api.cookie),
        Err(error) => error.response(),
    }
}

async fn request_email_change(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let email = fields.get("email").map(|value| value.trim()).unwrap_or("");
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if !valid_email(email) || password.is_empty() || password.len() > 1024 {
        return redirect(status_location(&slug, "email_invalid"), api.cookie);
    }
    let key = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let email = email.to_owned();
    let result = async {
        let db = state.db.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 { return Err(error(StatusCode::UNAUTHORIZED)); }
        let Some(hash) = account.password_hash else { return Ok(false); };
        if !verify_password_hash(password.to_owned(), hash).await? { return Ok(false); }
        db.execute_raw(sql("DELETE FROM account_login_change_keys WHERE account_id=$1", [api.account_id.into()]))
            .await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.execute_raw(sql("INSERT INTO account_login_change_keys (account_id, key, login, deadline, created_at, updated_at) VALUES ($1, $2, $3, CURRENT_TIMESTAMP + interval '2 days', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)", [api.account_id.into(), key.clone().into(), email.clone().into()]))
            .await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<bool, PageError>(true)
    }.await;
    match result {
        Ok(false) => redirect(status_location(&slug, "email_invalid"), api.cookie),
        Err(error) => error.response(),
        Ok(true) => {
            let path = format!(
                "{}/email/verify?account_id={}&token={key}",
                security_base(&slug),
                api.account_id
            );
            let config = (*state.invitation_mail).clone();
            let target = email.clone();
            let mailed = tokio::task::spawn_blocking(move || {
                crate::invitations::send_account_email_change(config, target, path)
            })
            .await
            .is_ok_and(|result| result.is_ok());
            if !mailed {
                if let Ok(db) = state.db.begin().await {
                    if crate::restricted_role(&db).await.is_ok() {
                        let _ = db.execute_raw(sql("DELETE FROM account_login_change_keys WHERE account_id=$1 AND key=$2", [api.account_id.into(), key.into()])).await;
                        let _ = db.commit().await;
                    }
                }
                return failure(StatusCode::SERVICE_UNAVAILABLE);
            }
            redirect(status_location(&slug, "email_requested"), api.cookie)
        }
    }
}

fn security_base(slug: &str) -> String {
    format!(
        "/households/{}/settings/security",
        medtracker_web::household::path_segment(slug)
    )
}

async fn show_email_verification(
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let Some(account_id) = params
        .get("account_id")
        .and_then(|value| value.parse::<i64>().ok())
    else {
        return failure(StatusCode::BAD_REQUEST);
    };
    let Some(token) = params
        .get("token")
        .filter(|value| valid_change_token(value))
    else {
        return failure(StatusCode::BAD_REQUEST);
    };
    page_status(
        render_email_verification_page(&slug, account_id, token),
        None,
        StatusCode::OK,
    )
}

fn valid_change_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

async fn verify_email_change(
    State(state): State<AppState>,
    Path(_slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let Some(account_id) = fields
        .get("account_id")
        .and_then(|value| value.parse::<i64>().ok())
    else {
        return failure(StatusCode::BAD_REQUEST);
    };
    let Some(token) = fields
        .get("token")
        .filter(|value| valid_change_token(value))
    else {
        return failure(StatusCode::BAD_REQUEST);
    };
    let result = async {
        let db = state.db.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(account_id).lock_exclusive().one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 { return Ok(false); }
        let pending = db.query_one_raw(sql("SELECT login FROM account_login_change_keys WHERE account_id=$1 AND key=$2 AND deadline > CURRENT_TIMESTAMP FOR UPDATE", [account_id.into(), token.clone().into()]))
            .await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let Some(row) = pending else { return Ok(false); };
        let email: String = row.try_get("", "login").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let mut active: account::ActiveModel = account.into();
        active.email = Set(email);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active.update(&db).await.map_err(|_| error(StatusCode::CONFLICT))?;
        db.execute_raw(sql("DELETE FROM account_login_change_keys WHERE account_id=$1", [account_id.into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        active_session_key::Entity::delete_many().filter(active_session_key::Column::AccountId.eq(account_id))
            .exec(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<bool, PageError>(true)
    }.await;
    match result {
        Ok(true) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Ok(false) => failure(StatusCode::BAD_REQUEST),
        Err(error) => error.response(),
    }
}

async fn new_otp(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state.clone(), headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if let Err(error) = api.household(&slug).await {
        return error.response();
    }
    let secrets = match crate::rodauth_secrets::load() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    let seed = match new_otp_seed() {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let result = async {
        let db = state.db.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id).lock_exclusive().one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 { return Err(error(StatusCode::UNAUTHORIZED)); }
        if otp_key::Entity::find_by_id(api.account_id).one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.is_some() { return Err(error(StatusCode::CONFLICT)); }
        db.execute_raw(sql("DELETE FROM account_active_session_keys WHERE account_id=$1 AND session_id LIKE 'otp-setup:%'", [api.account_id.into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.execute_raw(sql("INSERT INTO account_active_session_keys (account_id, session_id, created_at, last_use) VALUES ($1,$2,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", [api.account_id.into(), pending_otp_id(&seed).into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<String, PageError>(account.email)
    }.await;
    let email = match result {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let (derived, uri) = match authenticator_uri(&seed, &email, secrets.current()) {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    page_status(
        render_totp_setup_page(&slug, &api.csrf, &seed, &derived, &uri),
        api.cookie.take(),
        StatusCode::OK,
    )
}

async fn enable_otp(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let seed = fields.get("secret").map(String::as_str).unwrap_or("");
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    let code = fields.get("code").map(String::as_str).unwrap_or("");
    if seed.len() != 32 || password.is_empty() || password.len() > 1024 || code.len() != 6 {
        return redirect(status_location(&slug, "otp_invalid"), api.cookie);
    }
    let secrets = match crate::rodauth_secrets::load() {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::SERVICE_UNAVAILABLE),
    };
    let result = async {
        let db = state.db.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id).lock_exclusive().one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 { return Err(error(StatusCode::UNAUTHORIZED)); }
        let Some(hash) = account.password_hash else { return Ok(false); };
        if !verify_password_hash(password.to_owned(), hash).await? { return Ok(false); }
        if otp_key::Entity::find_by_id(api.account_id).one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.is_some() { return Ok(false); }
        let verified = crate::auth_compatibility::verify_rodauth_otp(seed, Some(secrets.current()), secrets.old(), code, chrono::Utc::now().timestamp(), 0)
            .map_err(|_| error(StatusCode::BAD_REQUEST))?;
        let Some(verified) = verified else { return Ok(false); };
        let pending = db.query_one_raw(sql("DELETE FROM account_active_session_keys WHERE account_id=$1 AND session_id=$2 AND created_at > CURRENT_TIMESTAMP - interval '5 minutes' RETURNING session_id", [api.account_id.into(), pending_otp_id(seed).into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if pending.is_none() { return Ok(false); }
        db.execute_raw(sql("INSERT INTO account_otp_keys (id, key, last_use, num_failures) VALUES ($1,$2,to_timestamp($3) AT TIME ZONE 'UTC',0)", [api.account_id.into(), seed.to_owned().into(), (verified.matched_step as i64 * 30).into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        active_session_key::Entity::delete_many().filter(active_session_key::Column::AccountId.eq(api.account_id)).exec(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<bool, PageError>(true)
    }.await;
    match result {
        Ok(true) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Ok(false) => redirect(status_location(&slug, "otp_invalid"), api.cookie),
        Err(error) => error.response(),
    }
}

async fn disable_otp(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if password.is_empty() || password.len() > 1024 {
        return redirect(status_location(&slug, "otp_invalid"), api.cookie);
    }
    let result = async {
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 {
            return Err(error(StatusCode::UNAUTHORIZED));
        }
        let Some(hash) = account.password_hash else {
            return Ok(false);
        };
        if !verify_password_hash(password.to_owned(), hash).await? {
            return Ok(false);
        }
        let removed = db
            .execute_raw(sql(
                "DELETE FROM account_otp_keys WHERE id=$1",
                [api.account_id.into()],
            ))
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if removed.rows_affected() == 0 {
            return Ok(false);
        }
        let passkey_count = webauthn_key::Entity::find()
            .filter(webauthn_key::Column::AccountId.eq(api.account_id))
            .count(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if passkey_count == 0 {
            recovery_code::Entity::delete_many()
                .filter(recovery_code::Column::Id.eq(api.account_id))
                .exec(&db)
                .await
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        }
        active_session_key::Entity::delete_many()
            .filter(active_session_key::Column::AccountId.eq(api.account_id))
            .exec(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<bool, PageError>(true)
    }
    .await;
    match result {
        Ok(true) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Ok(false) => redirect(status_location(&slug, "otp_invalid"), api.cookie),
        Err(error) => error.response(),
    }
}

async fn show_recovery_codes(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state.clone(), headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if let Err(error) = api.household(&slug).await {
        return error.response();
    }
    page_status(
        render_recovery_prompt_page(&slug, &api.csrf),
        api.cookie.take(),
        StatusCode::OK,
    )
}

async fn view_recovery_codes(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let mut api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if password.is_empty() || password.len() > 1024 {
        return failure(StatusCode::BAD_REQUEST);
    }
    let result = async {
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 {
            return Err(error(StatusCode::UNAUTHORIZED));
        }
        let Some(hash) = account.password_hash else {
            return Err(error(StatusCode::FORBIDDEN));
        };
        if !verify_password_hash(password.to_owned(), hash).await? {
            return Err(error(StatusCode::FORBIDDEN));
        }
        let rows = recovery_code::Entity::find()
            .filter(recovery_code::Column::Id.eq(api.account_id))
            .all(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<Vec<String>, PageError>(rows.into_iter().map(|row| row.code).collect())
    }
    .await;
    match result {
        Ok(codes) => page_status(
            render_recovery_codes_page(&slug, &codes),
            api.cookie.take(),
            StatusCode::OK,
        ),
        Err(error) => error.response(),
    }
}

async fn generate_recovery_codes(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let mut api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if password.is_empty() || password.len() > 1024 {
        return failure(StatusCode::BAD_REQUEST);
    }
    let result = async {
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 {
            return Err(error(StatusCode::UNAUTHORIZED));
        }
        let Some(hash) = account.password_hash else {
            return Err(error(StatusCode::FORBIDDEN));
        };
        if !verify_password_hash(password.to_owned(), hash).await? {
            return Err(error(StatusCode::FORBIDDEN));
        }
        let otp = otp_key::Entity::find_by_id(api.account_id)
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .is_some();
        let passkey = webauthn_key::Entity::find()
            .filter(webauthn_key::Column::AccountId.eq(api.account_id))
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .is_some();
        if !otp && !passkey {
            return Err(error(StatusCode::CONFLICT));
        }
        let mut codes = Vec::with_capacity(16);
        for _ in 0..16 {
            codes.push(new_recovery_code()?);
        }
        recovery_code::Entity::delete_many()
            .filter(recovery_code::Column::Id.eq(api.account_id))
            .exec(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        for code in &codes {
            recovery_code::Entity::insert(recovery_code::ActiveModel {
                id: Set(api.account_id),
                code: Set(code.clone()),
            })
            .exec(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        }
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<Vec<String>, PageError>(codes)
    }
    .await;
    match result {
        Ok(codes) => page_status(
            render_recovery_codes_page(&slug, &codes),
            api.cookie.take(),
            StatusCode::OK,
        ),
        Err(error) => error.response(),
    }
}

async fn rename_passkey(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, i64)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let nickname = fields
        .get("nickname")
        .map(|value| value.trim())
        .unwrap_or("");
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if nickname.is_empty()
        || nickname.len() > 100
        || nickname.chars().any(char::is_control)
        || password.is_empty()
        || password.len() > 1024
    {
        return failure(StatusCode::BAD_REQUEST);
    }
    let result = async {
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 {
            return Err(error(StatusCode::UNAUTHORIZED));
        }
        let Some(hash) = account.password_hash else {
            return Err(error(StatusCode::FORBIDDEN));
        };
        if !verify_password_hash(password.to_owned(), hash).await? {
            return Err(error(StatusCode::FORBIDDEN));
        }
        let key = webauthn_key::Entity::find_by_id(id)
            .filter(webauthn_key::Column::AccountId.eq(api.account_id))
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::NOT_FOUND))?;
        let mut active: webauthn_key::ActiveModel = key.into();
        active.nickname = Set(Some(nickname.to_owned()));
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active
            .update(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<(), PageError>(())
    }
    .await;
    match result {
        Ok(()) => redirect(status_location(&slug, "passkey_renamed"), api.cookie),
        Err(error) => error.response(),
    }
}

async fn remove_passkey(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, i64)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if password.is_empty() || password.len() > 1024 {
        return failure(StatusCode::BAD_REQUEST);
    }
    let result = async {
        let db = state
            .db
            .begin()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id)
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 {
            return Err(error(StatusCode::UNAUTHORIZED));
        }
        let Some(hash) = account.password_hash else {
            return Err(error(StatusCode::FORBIDDEN));
        };
        if !verify_password_hash(password.to_owned(), hash).await? {
            return Err(error(StatusCode::FORBIDDEN));
        }
        let key = webauthn_key::Entity::find_by_id(id)
            .filter(webauthn_key::Column::AccountId.eq(api.account_id))
            .lock_exclusive()
            .one(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::NOT_FOUND))?;
        webauthn_key::Entity::delete_by_id(key.id)
            .exec(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let remaining = webauthn_key::Entity::find()
            .filter(webauthn_key::Column::AccountId.eq(api.account_id))
            .count(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if remaining == 0
            && otp_key::Entity::find_by_id(api.account_id)
                .one(&db)
                .await
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
                .is_none()
        {
            recovery_code::Entity::delete_many()
                .filter(recovery_code::Column::Id.eq(api.account_id))
                .exec(&db)
                .await
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        }
        active_session_key::Entity::delete_many()
            .filter(active_session_key::Column::AccountId.eq(api.account_id))
            .exec(&db)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit()
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<(), PageError>(())
    }
    .await;
    match result {
        Ok(()) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Err(error) => error.response(),
    }
}

fn passkey_core(state: &AppState) -> Result<WebauthnCore, PageError> {
    let (origin, rp_id) = state
        .oauth
        .passkey_origin_and_rp()
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let origin = url::Url::parse(&origin).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(WebauthnCore::new_unsafe_experts_only(
        "MedTracker",
        &rp_id,
        vec![origin],
        std::time::Duration::from_secs(300),
        Some(false),
        Some(false),
    ))
}

fn pending_passkey_id(nonce: &str) -> String {
    format!(
        "passkey-register:{}",
        hex::encode(Sha256::digest(nonce.as_bytes()))
    )
}

async fn new_passkey(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state.clone(), headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if let Err(error) = api.household(&slug).await {
        return error.response();
    }
    let core = match passkey_core(&state) {
        Ok(core) => core,
        Err(error) => return error.response(),
    };
    let nonce = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let result = async {
        let db = state.db.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id).lock_exclusive().one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 { return Err(error(StatusCode::UNAUTHORIZED)); }
        let handle = webauthn_user_id::Entity::find().filter(webauthn_user_id::Column::AccountId.eq(api.account_id))
            .one(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let handle = if let Some(row) = handle {
            row.webauthn_id
        } else {
            new_recovery_code()?
        };
        let handle_bytes = URL_SAFE_NO_PAD.decode(&handle).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let builder = core.new_challenge_register_builder(&handle_bytes, &account.email, &account.email)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .user_verification_policy(UserVerificationPolicy::Required)
            .require_resident_key(true);
        let (options, registration_state) = core.generate_challenge_register(builder)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let claim = RegistrationClaim {
            account_id: api.account_id,
            csrf: api.csrf.clone(),
            issued_at: chrono::Utc::now().timestamp(),
            nonce: nonce.clone(),
            user_handle: handle,
            registration_state,
        };
        let signed = state.oauth.sign(&claim).ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if signed.len() > 4096 { return Err(error(StatusCode::INTERNAL_SERVER_ERROR)); }
        db.execute_raw(sql("DELETE FROM account_active_session_keys WHERE account_id=$1 AND session_id LIKE 'passkey-register:%'", [api.account_id.into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.execute_raw(sql("INSERT INTO account_active_session_keys (account_id, session_id, created_at, last_use) VALUES ($1,$2,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", [api.account_id.into(), pending_passkey_id(&nonce).into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let options_json = serde_json::to_string(&options).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<(String, String), PageError>((options_json, signed))
    }.await;
    match result {
        Ok((options_json, claim)) => page_status(
            render_passkey_registration_page(&slug, &api.csrf, &options_json, &claim),
            api.cookie.take(),
            StatusCode::OK,
        ),
        Err(error) => error.response(),
    }
}

async fn register_passkey(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let api = match browser_form(&state, &slug, headers, &fields).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    let nickname = fields
        .get("nickname")
        .map(|value| value.trim())
        .unwrap_or("");
    let credential_json = fields
        .get("webauthn_credential")
        .map(String::as_str)
        .unwrap_or("");
    let signed = fields
        .get("registration_claim")
        .map(String::as_str)
        .unwrap_or("");
    if password.is_empty()
        || password.len() > 1024
        || nickname.is_empty()
        || nickname.len() > 100
        || nickname.chars().any(char::is_control)
        || credential_json.len() > 16_384
    {
        return failure(StatusCode::BAD_REQUEST);
    }
    let Some(claim) = state.oauth.verify::<RegistrationClaim>(signed) else {
        return failure(StatusCode::FORBIDDEN);
    };
    if claim.account_id != api.account_id
        || claim.csrf != api.csrf
        || !fresh_registration(claim.issued_at, chrono::Utc::now().timestamp())
    {
        return failure(StatusCode::FORBIDDEN);
    }
    let credential: RegisterPublicKeyCredential = match serde_json::from_str(credential_json) {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::BAD_REQUEST),
    };
    let core = match passkey_core(&state) {
        Ok(core) => core,
        Err(error) => return error.response(),
    };
    let result = async {
        let db = state.db.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        crate::restricted_role(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let account = account::Entity::find_by_id(api.account_id).lock_exclusive().one(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?.ok_or_else(|| error(StatusCode::UNAUTHORIZED))?;
        if account.status != 2 { return Err(error(StatusCode::UNAUTHORIZED)); }
        let Some(hash) = account.password_hash else { return Err(error(StatusCode::FORBIDDEN)); };
        if !verify_password_hash(password.to_owned(), hash).await? { return Err(error(StatusCode::FORBIDDEN)); }
        let pending = db.query_one_raw(sql("SELECT session_id FROM account_active_session_keys WHERE account_id=$1 AND session_id=$2 AND created_at > CURRENT_TIMESTAMP - interval '5 minutes' FOR UPDATE", [api.account_id.into(), pending_passkey_id(&claim.nonce).into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if pending.is_none() { return Err(error(StatusCode::FORBIDDEN)); }
        let registered = core.register_credential(&credential, &claim.registration_state, None)
            .map_err(|_| error(StatusCode::BAD_REQUEST))?;
        if !registered.user_verified || registered.registration_policy != UserVerificationPolicy::Required {
            return Err(error(StatusCode::BAD_REQUEST));
        }
        let credential_id = URL_SAFE_NO_PAD.encode(registered.cred_id.as_slice());
        let public_key = rails_cose_public_key(&registered.cred)?;
        let counter: i32 = registered.counter.try_into().map_err(|_| error(StatusCode::BAD_REQUEST))?;
        db.query_one_raw(sql("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))", [credential_id.clone().into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        let existing = db.query_one_raw(sql("SELECT id FROM account_webauthn_keys WHERE webauthn_id=$1 LIMIT 1", [credential_id.clone().into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if existing.is_some() { return Err(error(StatusCode::CONFLICT)); }
        let handle = webauthn_user_id::Entity::find().filter(webauthn_user_id::Column::AccountId.eq(api.account_id))
            .one(&db).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if let Some(handle) = handle {
            if handle.webauthn_id != claim.user_handle { return Err(error(StatusCode::CONFLICT)); }
        } else {
            db.execute_raw(sql("INSERT INTO account_webauthn_user_ids (account_id, webauthn_id, created_at, updated_at) VALUES ($1,$2,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", [api.account_id.into(), claim.user_handle.clone().into()])).await
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        }
        db.execute_raw(sql("DELETE FROM account_active_session_keys WHERE account_id=$1 AND session_id=$2", [api.account_id.into(), pending_passkey_id(&claim.nonce).into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.execute_raw(sql("INSERT INTO account_webauthn_keys (account_id, webauthn_id, public_key, nickname, sign_count, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", [api.account_id.into(), credential_id.into(), public_key.into(), nickname.to_owned().into(), counter.into()])).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        active_session_key::Entity::delete_many().filter(active_session_key::Column::AccountId.eq(api.account_id)).exec(&db).await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        db.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok::<(), PageError>(())
    }.await;
    match result {
        Ok(()) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Err(error) => error.response(),
    }
}

pub(super) async fn section(
    state: &AppState,
    api: &mut WebApi,
    household_id: i64,
    slug: &str,
    status: Option<&str>,
) -> Result<String, PageError> {
    let me = api
        .get(&format!("/api/v1/households/{household_id}/me"))
        .await?;
    let account_id = me
        .pointer("/data/account/id")
        .and_then(Value::as_i64)
        .filter(|id| *id == api.account_id)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let email = me
        .pointer("/data/account/email")
        .and_then(Value::as_str)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
        .to_owned();
    let db = state
        .db
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    crate::restricted_role(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let totp_enabled = otp_key::Entity::find_by_id(account_id)
        .one(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
        .is_some();
    let recovery_codes_count = recovery_code::Entity::find()
        .filter(recovery_code::Column::Id.eq(account_id))
        .count(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let keys = webauthn_key::Entity::find()
        .filter(webauthn_key::Column::AccountId.eq(account_id))
        .order_by_desc(webauthn_key::Column::CreatedAt)
        .all(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    db.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let passkeys = keys
        .into_iter()
        .map(|key| SecurityPasskey {
            id: key.id,
            nickname: key
                .nickname
                .unwrap_or_else(|| format!("Passkey {}", key.id)),
            added_on: format_passkey_date(key.created_at),
        })
        .collect();
    render_security_section(SecurityPage {
        locale: api.locale,
        slug: slug.to_owned(),
        csrf: api.csrf.clone(),
        email,
        totp_enabled,
        recovery_codes_count: recovery_codes_count
            .try_into()
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?,
        passkeys,
        status: status.map(str::to_owned),
    })
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}
