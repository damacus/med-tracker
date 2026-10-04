use axum::http::HeaderValue;
use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use cookie::{Cookie, CookieJar, Key};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

pub(crate) trait AuthenticationClaim {
    const PURPOSE: &'static str;
}

#[derive(Clone)]
pub struct OAuthState {
    secret: Arc<[u8]>,
    pub(super) base_url: Url,
    pub(super) password_workers: Arc<Semaphore>,
}

impl OAuthState {
    pub(crate) fn passkey_origin_and_rp(&self) -> Option<(String, String)> {
        Some((
            self.base_url.origin().ascii_serialization(),
            self.base_url.host_str()?.to_owned(),
        ))
    }

    pub(crate) fn occurrence_key_secret(&self) -> Arc<[u8]> {
        self.secret.clone()
    }

    pub fn from_env() -> Result<Self, String> {
        let secret = std::env::var("AUTH_SESSION_SECRET")
            .map_err(|_| "AUTH_SESSION_SECRET missing".to_owned())?;
        if secret.len() < 32 {
            return Err("AUTH_SESSION_SECRET must contain at least 32 bytes".to_owned());
        }
        let base_url = Url::parse(
            &std::env::var("PUBLIC_BASE_URL").map_err(|_| "PUBLIC_BASE_URL missing".to_owned())?,
        )
        .map_err(|_| "PUBLIC_BASE_URL invalid".to_owned())?;
        let local = matches!(
            base_url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        );
        if (base_url.scheme() != "https" && !(base_url.scheme() == "http" && local))
            || base_url.path() != "/"
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
        {
            return Err(
                "PUBLIC_BASE_URL must be HTTPS origin or explicit loopback HTTP origin".to_owned(),
            );
        }
        Ok(Self {
            secret: Arc::from(secret.into_bytes()),
            base_url,
            password_workers: Arc::new(Semaphore::new(4)),
        })
    }

    pub(crate) fn sign<T: Serialize + AuthenticationClaim>(&self, data: &T) -> Option<String> {
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(data).ok()?);
        let mut jar = CookieJar::new();
        jar.private_mut(&Key::derive_from(&self.secret))
            .add(Cookie::new(T::PURPOSE, payload));
        Some(jar.get(T::PURPOSE)?.value().to_owned())
    }

    pub(crate) fn verify<T: for<'a> Deserialize<'a> + AuthenticationClaim>(
        &self,
        value: &str,
    ) -> Option<T> {
        if value.len() > 4096 {
            return None;
        }
        let jar = CookieJar::new();
        let cookie = jar
            .private(&Key::derive_from(&self.secret))
            .decrypt(Cookie::new(T::PURPOSE, value.to_owned()))?;
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(cookie.value()).ok()?).ok()
    }

    pub(super) fn cookie(&self, name: &str, value: &str, max_age: i64) -> HeaderValue {
        let secure = if self.base_url.scheme() == "https" {
            "; Secure"
        } else {
            ""
        };
        HeaderValue::from_str(&format!(
            "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
        ))
        .expect("signed cookie is valid header")
    }
}

pub(super) fn secret() -> String {
    format!(
        "{}{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub(super) fn digest(token: &str) -> String {
    URL_SAFE.encode(Sha256::digest(token.as_bytes()))
}

pub(crate) fn session_key_digest(token: &str) -> String {
    digest(token)
}

#[cfg(test)]
mod tests {
    use super::OAuthState;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    #[derive(Serialize, Deserialize)]
    struct First {
        value: String,
    }
    #[derive(Serialize, Deserialize)]
    struct Second {
        value: String,
    }

    impl super::AuthenticationClaim for First {
        const PURPOSE: &'static str = "test-first";
    }
    impl super::AuthenticationClaim for Second {
        const PURPOSE: &'static str = "test-second";
    }

    #[test]
    fn signed_claims_cannot_be_reused_for_another_purpose() {
        let state = OAuthState {
            secret: Arc::from([42_u8; 64]),
            base_url: "https://example.com".parse().unwrap(),
            password_workers: Arc::new(Semaphore::new(1)),
        };
        let signed = state
            .sign(&First {
                value: "test".into(),
            })
            .unwrap();
        assert!(state.verify::<First>(&signed).is_some());
        assert!(state.verify::<Second>(&signed).is_none());
    }
    #[test]
    fn pending_factor_cookie_cannot_authenticate_as_a_browser_session() {
        let state = OAuthState {
            secret: Arc::from([42_u8; 64]),
            base_url: "https://example.com".parse().unwrap(),
            password_workers: Arc::new(Semaphore::new(1)),
        };
        let claim: super::super::factor_completion::FactorIntent =
            serde_json::from_value(serde_json::json!({
                "account_id": 123,
                "nonce": "test-nonce",
                "csrf": "test-csrf",
                "issued_at": 1000,
                "authorization": null
            }))
            .unwrap();
        let signed = state.sign(&claim).unwrap();
        assert!(state
            .verify::<super::super::factor_completion::FactorIntent>(&signed)
            .is_some());
        assert!(state
            .verify::<super::super::sessions::BrowserSession>(&signed)
            .is_none());
    }
}
