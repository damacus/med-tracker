use super::*;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct OAuthState {
    secret: Arc<[u8]>,
    pub(super) base_url: Url,
    pub(super) password_workers: Arc<Semaphore>,
}

impl OAuthState {
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

    pub(super) fn sign<T: Serialize>(&self, data: &T) -> Option<String> {
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(data).ok()?);
        let mut mac = HmacSha256::new_from_slice(&self.secret).ok()?;
        mac.update(payload.as_bytes());
        Some(format!(
            "{payload}.{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        ))
    }

    pub(super) fn verify<T: for<'a> Deserialize<'a>>(&self, value: &str) -> Option<T> {
        if value.len() > 4096 {
            return None;
        }
        let (payload, signature) = value.split_once('.')?;
        let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).ok()?;
        mac.update(payload.as_bytes());
        mac.verify_slice(&signature).ok()?;
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
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
