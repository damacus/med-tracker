use std::borrow::Cow;

use axum::{
    Form,
    body::Body,
    extract::FromRequest,
    http::{Request, header::AUTHORIZATION},
};
use headers::{Authorization, Header, authorization::Basic};
use oxide_auth::code_grant::{accesstoken, refresh};

use super::ExchangeError;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Method {
    None,
    Post,
    Basic,
}

impl Method {
    pub fn registered_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Post => "client_secret_post",
            Self::Basic => "client_secret_basic",
        }
    }
}

pub(super) struct Input {
    fields: Vec<(String, String)>,
    pub id: String,
    pub secret: Option<String>,
    pub method: Method,
}

fn decode_component(value: &str) -> Result<String, ExchangeError> {
    let mut decoded: Vec<(String, String)> = serde_urlencoded::from_str(&format!("value={value}"))
        .map_err(|_| ExchangeError::InvalidRequest)?;
    if decoded.len() != 1 {
        return Err(ExchangeError::InvalidRequest);
    }
    Ok(decoded.remove(0).1)
}

impl Input {
    pub async fn parse(request: Request<Body>) -> Result<Self, ExchangeError> {
        let auth = request
            .headers()
            .get_all(AUTHORIZATION)
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if auth.len() > 1 {
            return Err(ExchangeError::InvalidRequest);
        }
        let basic = if auth.is_empty() {
            None
        } else {
            Some(
                Authorization::<Basic>::decode(&mut auth.iter())
                    .map_err(|_| ExchangeError::InvalidClient)?,
            )
        };
        let Form(fields) = Form::<Vec<(String, String)>>::from_request(request, &())
            .await
            .map_err(|_| ExchangeError::InvalidRequest)?;
        for name in [
            "client_id",
            "client_secret",
            "grant_type",
            "refresh_token",
            "code",
            "redirect_uri",
            "code_verifier",
            "scope",
        ] {
            if fields.iter().filter(|(key, _)| key == name).count() > 1 {
                return Err(ExchangeError::InvalidRequest);
            }
        }
        let value = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        };
        let (id, secret, method) = if let Some(basic) = basic {
            if value("client_secret").is_some() {
                return Err(ExchangeError::InvalidRequest);
            }
            let id = decode_component(basic.username())?;
            if value("client_id")
                .filter(|value| !value.is_empty())
                .is_some_and(|value| value != id)
            {
                return Err(ExchangeError::InvalidRequest);
            }
            (id, Some(decode_component(basic.password())?), Method::Basic)
        } else {
            let secret = value("client_secret");
            let method = if secret.is_some() {
                Method::Post
            } else {
                Method::None
            };
            (
                value("client_id").ok_or(ExchangeError::InvalidClient)?,
                secret,
                method,
            )
        };
        Ok(Self {
            fields,
            id,
            secret,
            method,
        })
    }

    pub fn value(&self, name: &str) -> Option<Cow<'_, str>> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str().into())
    }
}

impl accesstoken::Request for Input {
    fn valid(&self) -> bool {
        true
    }
    fn code(&self) -> Option<Cow<'_, str>> {
        self.value("code")
    }
    fn authorization(&self) -> accesstoken::Authorization<'_> {
        match &self.secret {
            Some(secret) => accesstoken::Authorization::UsernamePassword(
                self.id.as_str().into(),
                secret.as_bytes().into(),
            ),
            None => accesstoken::Authorization::None,
        }
    }
    fn client_id(&self) -> Option<Cow<'_, str>> {
        self.secret.is_none().then(|| self.id.as_str().into())
    }
    fn redirect_uri(&self) -> Option<Cow<'_, str>> {
        self.value("redirect_uri")
    }
    fn grant_type(&self) -> Option<Cow<'_, str>> {
        self.value("grant_type")
    }
    fn extension(&self, name: &str) -> Option<Cow<'_, str>> {
        self.value(name)
    }
}

impl refresh::Request for Input {
    fn valid(&self) -> bool {
        true
    }
    fn refresh_token(&self) -> Option<Cow<'_, str>> {
        self.value("refresh_token")
    }
    fn scope(&self) -> Option<Cow<'_, str>> {
        self.value("scope")
    }
    fn grant_type(&self) -> Option<Cow<'_, str>> {
        self.value("grant_type")
    }
    fn authorization(&self) -> Option<(Cow<'_, str>, Cow<'_, [u8]>)> {
        self.secret
            .as_ref()
            .map(|secret| (self.id.as_str().into(), secret.as_bytes().into()))
    }
    fn extension(&self, name: &str) -> Option<Cow<'_, str>> {
        self.value(name)
    }
}
