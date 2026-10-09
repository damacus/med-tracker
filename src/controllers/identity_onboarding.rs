use axum::{
    Extension,
    extract::{Form as AxumForm, Query},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};

use crate::models::{
    care::invitations,
    identity::{
        better_auth::{BrowserIdentity, IdentityService, browser_identity, dispatch},
        signup,
    },
};

mod confirmation;
mod credentials;
mod email_change;
mod forms;
mod keys;
mod oidc;
mod passkeys;
mod password;
mod password_email;
mod recovery;
mod rendering;
mod security;
mod support;
mod totp;
mod verification_resend;

pub(crate) use password::render_named;

pub fn routes() -> Routes {
    Routes::new()
        .add("/create-account", get(create_form).post(create))
        .add("/auth/passkey/setup", get(passkeys::setup))
        .add("/auth/passkey/complete", get(passkeys::complete))
        .add("/recovery-login", get(recovery::form).post(recovery::login))
        .add("/account/security", get(security::settings))
        .add("/account/security/keys", get(keys::show).post(keys::create))
        .add("/account/security/keys/revoke", post(keys::revoke))
        .add("/auth/zitadel", post(oidc::start))
        .add("/auth/zitadel/callback", get(oidc::callback))
        .add(
            "/auth/zitadel/register",
            get(oidc::registration).post(oidc::register),
        )
        .add("/account/security/provider/link", get(oidc::link))
        .add(
            "/account/security/email",
            get(email_change::show).post(email_change::start),
        )
        .add(
            "/account/security/email/pending",
            get(email_change::pending),
        )
        .add(
            "/account/security/email/confirm",
            get(email_change::confirmation).post(email_change::confirm),
        )
        .add("/account/security/session/revoke", post(security::revoke))
        .add(
            "/verify-account-confirm",
            get(confirmation::show).post(confirmation::confirm),
        )
        .add(
            "/verify-account-resend",
            get(verification_resend::show).post(verification_resend::resend),
        )
        .add(
            "/account/security/password",
            get(password::show).post(password::start),
        )
        .add(
            "/account/security/password/confirm",
            post(password::confirm),
        )
        .add(
            "/account/security/password/email",
            get(password_email::show).post(password_email::confirm),
        )
        .add("/account/security/operation", post(credentials::start))
        .add("/account/support", get(support::page))
        .add("/account/support/approve", post(support::approve))
        .add("/account/support/end", post(support::end))
        .add("/account/security/passkey", get(credentials::passkey))
        .add("/account/security/totp", get(totp::show).post(totp::start))
        .add("/account/security/totp/finish", post(totp::finish))
        .add("/auth/totp", get(totp::login_form).post(totp::login))
        .add("/account/security/totp/disable", post(totp::disable_start))
        .add(
            "/account/security/totp/disable/confirm",
            post(totp::disable_confirm),
        )
}

async fn create_form(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    Query(query): Query<forms::InvitationQuery>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match browser_identity(&service, &passkeys::request(&headers)).await {
        Ok(BrowserIdentity::Authenticated { .. }) => return rendering::redirect("/"),
        Ok(BrowserIdentity::Enrolment { .. }) => return rendering::redirect("/auth/passkey/setup"),
        Ok(BrowserIdentity::PendingFactor) => return rendering::redirect("/auth/totp"),
        Ok(BrowserIdentity::SignedOut) => {}
        Err(_) => return rendering::unavailable(),
    }
    let invitation = if let Some(value) = query
        .invitation_token
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        match invitations::preview(&ctx.db, value).await {
            Ok(invitation) => Some(invitation),
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        match signup::registration_open(&ctx.db).await {
            Ok(true) => {}
            Ok(false) => return rendering::redirect("/login"),
            Err(_) => return rendering::unavailable(),
        }
        None
    };
    rendering::account(
        &view,
        token,
        &forms::Create::empty(query.invitation_token),
        invitation,
        &Default::default(),
        StatusCode::OK,
    )
}

async fn create(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<forms::Create>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser_identity(&service, &passkeys::request(&headers)).await {
        Ok(BrowserIdentity::Authenticated { .. }) => return rendering::redirect("/"),
        Ok(BrowserIdentity::Enrolment { .. }) => return rendering::redirect("/auth/passkey/setup"),
        Ok(BrowserIdentity::PendingFactor) => return rendering::redirect("/auth/totp"),
        Ok(BrowserIdentity::SignedOut) => {}
        Err(_) => return rendering::unavailable(),
    }
    let invitation = if let Some(value) = form
        .invitation_token
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        match invitations::preview(&ctx.db, value).await {
            Ok(invitation) => Some(invitation),
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        match signup::registration_open(&ctx.db).await {
            Ok(true) => {}
            Ok(false) => return rendering::redirect("/login"),
            Err(_) => return rendering::unavailable(),
        }
        None
    };
    let profile = match form.profile() {
        Ok(profile) => profile,
        Err(errors) => {
            return rendering::account(
                &view,
                token,
                &form,
                invitation,
                &errors,
                StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let request_id = super::medications::request_id(request_id);
    if form.credential == "password"
        && let Err(message) =
            crate::models::identity::better_auth::validate_new_password(&form.password)
    {
        return rendering::account(
            &view,
            token,
            &form,
            invitation,
            &std::collections::BTreeMap::from([("password".into(), vec![message.into()])]),
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    let mut request = better_auth_core::AuthRequest::new(
        better_auth_core::HttpMethod::Post,
        "/onboarding/signup",
    );
    request.headers = passkeys::request(&headers).headers;
    request.body = serde_json::to_vec(&serde_json::json!({"name":profile.name,"email":profile.email,"date_of_birth":profile.date_of_birth,"invitation_token":profile.invitation_token,"credential":form.credential,"password":if form.credential=="password" {Some(form.password.clone())}else{None}})).ok();
    match dispatch(&service, request, request_id).await {
        Ok(response) if response.status < 400 => rendering::sent(&view),
        Ok(_) | Err(_) => rendering::account(
            &view,
            token,
            &form,
            invitation,
            &std::collections::BTreeMap::from([(
                "account".into(),
                vec!["Check your details and try again.".into()],
            )]),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    }
}
