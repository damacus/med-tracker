use crate::controllers::medications::{authentication_error, operation_error, unavailable};
use crate::models::{
    care::report_pdf,
    identity::{better_auth::IdentityService, browser},
    ids::AccountId,
    profile::self_account,
};
use axum::{
    Extension,
    extract::{Query, State},
    http::{HeaderMap, header, request::Parts},
    response::{IntoResponse, Response},
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{
    controller::{middleware::request_id::LocoRequestId, views::ViewRenderer},
    prelude::{AppContext, TeraView},
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FlowKind {
    Email,
    Password,
    Reset,
    Totp,
    Passkey,
    Keys,
    Provider,
    Operation,
    Support,
    History,
    Plans,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    Continue,
    Invalid,
    Complete,
    SecretResult,
}

impl Outcome {
    fn for_body(body: &str, data: &Value) -> Self {
        if data
            .get("error")
            .is_some_and(|value| !value.is_null() && value != false)
        {
            Self::Invalid
        } else if body == "key_result" || body == "totp" && data["mode"] == "backup" {
            Self::SecretResult
        } else if body == "complete" {
            Self::Complete
        } else {
            Self::Continue
        }
    }
}

impl FlowKind {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "email" => Self::Email,
            "password" => Self::Password,
            "reset" => Self::Reset,
            "totp" => Self::Totp,
            "passkey" => Self::Passkey,
            "keys" => Self::Keys,
            "provider" => Self::Provider,
            "operation" => Self::Operation,
            "support" => Self::Support,
            "history" => Self::History,
            "plans" => Self::Plans,
            _ => return None,
        })
    }
    fn name(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Password => "password",
            Self::Reset => "reset",
            Self::Totp => "totp",
            Self::Passkey => "passkey",
            Self::Keys => "keys",
            Self::Provider => "provider",
            Self::Operation => "operation",
            Self::Support => "support",
            Self::History => "history",
            Self::Plans => "plans",
        }
    }
}

#[derive(Clone, Serialize)]
pub(crate) struct Presentation {
    pub kind: FlowKind,
    pub household: Option<String>,
    pub fragment: bool,
    pub complete: bool,
}

impl Presentation {
    pub(crate) fn from_request(parts: &Parts) -> Result<Option<Self>, Box<Response>> {
        let path = parts.uri.path();
        let canonical = path == "/profile"
            || path
                .strip_prefix("/households/")
                .is_some_and(|value| value.ends_with("/profile"));
        let mut selected = None;
        let mut household = None;
        let mut complete = false;
        for (key, value) in
            url::form_urlencoded::parse(parts.uri.query().unwrap_or_default().as_bytes())
        {
            match key.as_ref() {
                "panel" | "profile_flow" | "profile_done" => {
                    if key != "profile_flow" && !canonical {
                        continue;
                    }
                    if selected.is_some() {
                        return Err(Box::new(
                            axum::http::StatusCode::BAD_REQUEST.into_response(),
                        ));
                    }
                    selected = Some(FlowKind::parse(&value).ok_or_else(|| {
                        Box::new(axum::http::StatusCode::BAD_REQUEST.into_response())
                    })?);
                    complete = key == "profile_done";
                }
                "profile_household" => {
                    if household.is_some()
                        || value.is_empty()
                        || value.len() > 128
                        || !value.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
                        })
                    {
                        return Err(Box::new(
                            axum::http::StatusCode::BAD_REQUEST.into_response(),
                        ));
                    }
                    household = Some(value.into_owned());
                }
                _ => {}
            }
        }
        if selected.is_none() && path == "/account/security/provider/confirm" {
            selected = Some(FlowKind::Operation);
        }
        let Some(kind) = selected else {
            return Ok(None);
        };
        if !canonical
            && !(path.starts_with("/account/security/")
                || path == "/account/support"
                || path.starts_with("/account/support/")
                || path == "/profile/history"
                || path == "/profile/schedules"
                || path.starts_with("/profile/schedules/")
                || path == "/reset-password"
                || path == "/reset-password/confirm")
        {
            return Err(Box::new(
                axum::http::StatusCode::BAD_REQUEST.into_response(),
            ));
        }
        if let Some(slug) = path
            .strip_prefix("/households/")
            .and_then(|path| path.strip_suffix("/profile"))
        {
            household = Some(slug.to_owned());
        }
        Ok(Some(Self {
            kind,
            household,
            complete,
            fragment: parts
                .headers
                .get(header::ACCEPT)
                .is_some_and(|value| value == "text/vnd.medtracker.profile-flow+html"),
        }))
    }
    pub(crate) fn profile(&self) -> String {
        self.household.as_ref().map_or_else(
            || "/profile".into(),
            |slug| format!("/households/{slug}/profile"),
        )
    }
    pub(crate) fn suffix(&self) -> String {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.append_pair("profile_flow", self.kind.name());
        if let Some(slug) = &self.household {
            query.append_pair("profile_household", slug);
        }
        format!("?{}", query.finish())
    }
    pub(crate) fn destination(&self, path: &str) -> String {
        match path {
            "/account/security" | "/profile#security" => format!(
                "{}?profile_done={}#security",
                self.profile(),
                self.kind.name()
            ),
            "/account/security/keys" => format!("{}?panel=keys#security", self.profile()),
            "/account/support" => format!("{}?panel=support#security", self.profile()),
            "/profile/schedules" => format!("{}?panel=plans#profile", self.profile()),
            _ => path.to_owned(),
        }
    }
}

pub(crate) struct Frame {
    pub presentation: Presentation,
    name: String,
    email: String,
    labels: Value,
    locale: &'static str,
}

impl Frame {
    pub(crate) fn rejected(&self) -> &str {
        self.labels["profiles"]["dialogs"]["rejected"]
            .as_str()
            .unwrap_or_default()
    }
    pub(crate) async fn load(
        ctx: &AppContext,
        session: &Session<SessionPgPool>,
        headers: &HeaderMap,
        presentation: Presentation,
    ) -> Result<Option<Self>, Box<Response>> {
        use crate::models::{errors::OperationError, identity::resource::AuthenticationError};

        let principal =
            match browser::authenticate(&ctx.db, session, ctx.config.settings.as_ref()).await {
                Ok(principal) => principal,
                Err(
                    AuthenticationError::Unauthenticated
                    | AuthenticationError::Forbidden
                    | AuthenticationError::NotFound
                    | AuthenticationError::InsufficientScope { .. },
                ) => return Ok(None),
                Err(error @ AuthenticationError::Unavailable) => {
                    return Err(Box::new(authentication_error(error)));
                }
            };
        let transaction = match principal.account_transaction(&ctx.db, false).await {
            Ok(transaction) => transaction,
            Err(
                AuthenticationError::Unauthenticated
                | AuthenticationError::Forbidden
                | AuthenticationError::NotFound
                | AuthenticationError::InsufficientScope { .. },
            ) => return Ok(None),
            Err(error @ AuthenticationError::Unavailable) => {
                return Err(Box::new(authentication_error(error)));
            }
        };
        let snapshot = match self_account::read(
            &transaction,
            AccountId::from_database(principal.account_id()),
            ctx.config.settings.as_ref(),
        )
        .await
        {
            Ok(snapshot) => snapshot,
            Err(OperationError::Unauthenticated) => return Ok(None),
            Err(error) => return Err(Box::new(operation_error(error))),
        };
        transaction
            .commit()
            .await
            .map_err(|error| operation_error(error.into()))?;
        let locale = report_pdf::locale(
            headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("en"),
        );
        let labels = report_pdf::translations(locale).map_err(|_| unavailable())?;
        Ok(Some(Self {
            presentation,
            name: snapshot.name.unwrap_or_else(|| snapshot.email.clone()),
            email: snapshot.email,
            labels,
            locale,
        }))
    }
    pub(crate) fn render(
        &self,
        engine: &TeraView,
        key: &str,
        mut data: Value,
    ) -> loco_rs::Result<String> {
        let body = match key {
            "identity_onboarding/email_change.html" => "email_change",
            "identity_onboarding/password.html" => "password",
            "identity_onboarding/password_reset.html" => "password_reset",
            "identity_onboarding/password_email.html" => "password_email",
            "identity_onboarding/totp.html" => "totp",
            "identity_onboarding/add_passkey.html" => "add_passkey",
            "identity_onboarding/keys.html" => "keys",
            "identity_onboarding/key_result.html" => "key_result",
            "identity_onboarding/provider_link.html" => "provider_link",
            "identity_onboarding/support.html" => "support",
            "profile/history.html" => "history",
            "profile/schedules.html" => "schedules",
            "profile/show.html" if self.presentation.complete => "complete",
            "profile/flow-error.html" => "error",
            _ => return engine.render(key, data),
        };
        let secret = body == "key_result"
            || data.get("secret").is_some_and(|value| !value.is_null())
            || data
                .get("backup_codes")
                .and_then(Value::as_array)
                .is_some_and(|codes| !codes.is_empty());
        let outcome = Outcome::for_body(body, &data);
        data["labels"] = self.labels.clone();
        data["lang"] = json!(self.locale);
        data["profile_flow"] = json!({"kind":self.presentation.kind,"profile":self.presentation.profile(),"body":body,"outcome":outcome,"secret":secret,"name":self.name,"email":self.email});
        data["profile_flow_query"] = json!(self.presentation.suffix());
        data["profile_flow_tail"] = json!(self.presentation.suffix().trim_start_matches('?'));
        data["profile_path"] = json!(self.presentation.profile());
        if data.get("flow").is_none() {
            data["flow"] = self.labels["profiles"]["security_flow"].clone();
            data["flow_json"] = json!(data["flow"].to_string());
        }
        engine.render(
            if self.presentation.fragment {
                "profile/flow-fragment.html"
            } else {
                "profile/flow-page.html"
            },
            data,
        )
    }
}

pub(crate) struct Open {
    pub ctx: AppContext,
    pub session: Session<SessionPgPool>,
    pub headers: HeaderMap,
    pub token: CsrfToken,
    pub view: crate::controllers::shell::ShellView,
    pub request: Option<Extension<LocoRequestId>>,
}

pub(crate) async fn open(input: Open) -> Response {
    let Open {
        ctx,
        session,
        headers,
        token,
        view,
        request,
    } = input;
    let Some(kind) = view.profile_presentation().map(|value| value.kind) else {
        return unavailable();
    };
    let Some(service) = ctx.shared_store.get::<IdentityService>() else {
        return unavailable();
    };
    use crate::controllers::identity_onboarding as identity;
    use crate::controllers::shell::ShellViewEngine;
    match kind {
        FlowKind::Email => {
            identity::profile_email(Extension(service), headers, token, ShellViewEngine(view)).await
        }
        FlowKind::Password => {
            identity::profile_password(Extension(service), headers, token, ShellViewEngine(view))
                .await
        }
        FlowKind::Reset => identity::profile_reset(token, ShellViewEngine(view)).await,
        FlowKind::Totp => {
            identity::profile_totp(Extension(service), headers, token, ShellViewEngine(view)).await
        }
        FlowKind::Passkey => {
            identity::profile_passkey(token, Extension(service), headers, ShellViewEngine(view))
                .await
        }
        FlowKind::Keys => {
            identity::profile_keys(
                Extension(service),
                headers,
                request,
                token,
                ShellViewEngine(view),
            )
            .await
        }
        FlowKind::Provider => {
            identity::profile_provider(Extension(service), headers, token, ShellViewEngine(view))
                .await
        }
        FlowKind::Support => {
            identity::profile_support(
                State(ctx),
                Extension(service),
                headers,
                token,
                ShellViewEngine(view),
            )
            .await
        }
        FlowKind::History => {
            super::account_profile::history::index(
                State(ctx),
                session,
                token,
                headers,
                ShellViewEngine(view),
                request,
                Ok(Query(Default::default())),
            )
            .await
        }
        FlowKind::Plans => {
            super::account_profile::person_schedules::index(
                State(ctx),
                session,
                headers,
                token,
                ShellViewEngine(view),
            )
            .await
        }
        FlowKind::Operation => unavailable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(uri: &str) -> Parts {
        axum::http::Request::builder()
            .uri(uri)
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    #[test]
    fn profile_presentation_keeps_household_and_protected_continuations() {
        let selected = Presentation::from_request(&request("/households/home/profile?panel=keys"))
            .unwrap()
            .unwrap();
        assert_eq!(selected.profile(), "/households/home/profile");
        assert_eq!(
            selected.suffix(),
            "?profile_flow=keys&profile_household=home"
        );
        assert_eq!(
            selected.destination("/account/security"),
            "/households/home/profile?profile_done=keys#security"
        );
        assert_eq!(selected.destination("/login"), "/login");
        let proof = Presentation::from_request(&request(
            "/account/security/password/confirm?profile_flow=keys&profile_household=home",
        ))
        .unwrap()
        .unwrap();
        assert_eq!(proof.profile(), selected.profile());
        assert!(!proof.complete);
    }

    #[test]
    fn presentation_rejects_ambiguous_or_cross_application_context() {
        for uri in [
            "/profile?panel=email&profile_done=password",
            "/profile?panel=unknown",
            "/profile?panel=email&profile_household=../other",
            "/api/auth/sign-in/email?profile_flow=email",
            "/platform/users?profile_flow=operation",
        ] {
            assert!(Presentation::from_request(&request(uri)).is_err(), "{uri}");
        }
        assert!(
            Presentation::from_request(&request("/unrelated?panel=email"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejected_proofs_are_invalid_and_one_time_results_never_become_saved_notices() {
        assert_eq!(
            Outcome::for_body("error", &json!({"error":"expired proof"})),
            Outcome::Invalid
        );
        assert_eq!(
            Outcome::for_body("password", &json!({"error":"invalid proof"})),
            Outcome::Invalid
        );
        assert_eq!(
            Outcome::for_body("totp", &json!({"mode":"finish","error":"invalid code"})),
            Outcome::Invalid
        );
        assert_eq!(
            Outcome::for_body("totp", &json!({"mode":"backup","error":null})),
            Outcome::SecretResult
        );
        assert_eq!(
            Outcome::for_body("key_result", &json!({"api_key":"synthetic"})),
            Outcome::SecretResult
        );
        assert_eq!(Outcome::for_body("complete", &json!({})), Outcome::Complete);
    }
}
