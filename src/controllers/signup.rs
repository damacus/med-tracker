use axum::{
    Extension,
    extract::{Form as AxumForm, Query},
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::json;

use crate::models::{
    care::invitations,
    identity::{
        browser,
        signup::{self, SignupError, SignupInput},
    },
};

#[derive(Deserialize)]
struct Create {
    #[serde(flatten)]
    input: SignupInput,
    authenticity_token: String,
}

#[derive(Deserialize)]
struct VerificationQuery {
    key: Option<String>,
}

#[derive(Deserialize)]
struct Verify {
    authenticity_token: String,
}

#[derive(Deserialize)]
struct Resend {
    email: String,
    authenticity_token: String,
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/create-account", get(create_form).post(create))
        .add("/verify-account", get(verification).post(verify))
        .add("/verify-account-resend", get(resend_form).post(resend))
}

async fn resend_form(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match browser::authenticate(&ctx.db, &session).await {
        Ok(_) => return redirect("/"),
        Err(crate::models::identity::resource::AuthenticationError::Unavailable) => {
            return unavailable();
        }
        Err(_) => {}
    }
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = json!("Resend Verification Email");
    data["email"] = json!("");
    data["error"] = serde_json::Value::Null;
    data["authenticity_token"] = json!(authenticity_token);
    (
        token,
        render(&view, "auth/verification_resend.html", data, StatusCode::OK),
    )
        .into_response()
}

async fn resend(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    AxumForm(form): AxumForm<Resend>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match browser::authenticate(&ctx.db, &session).await {
        Ok(_) => return redirect("/"),
        Err(crate::models::identity::resource::AuthenticationError::Unavailable) => {
            return unavailable();
        }
        Err(_) => {}
    }
    let notice = match signup::resend_verification(
        &ctx.db,
        &form.email,
        &ctx.config.server.full_url(),
    )
    .await
    {
        Ok(signup::VerificationResend::Sent) => {
            "An email has been sent to you with a link to verify your account"
        }
        Ok(signup::VerificationResend::RecentlySent) => {
            "An email has recently been sent to you with a link to verify your account"
        }
        Ok(signup::VerificationResend::UnavailableAccount) => {
            "Unable to resend verify account email"
        }
        Err(_) => return unavailable(),
    };
    session.set("auth_notice", notice);
    session.set_store(true);
    redirect("/")
}

async fn create_form(
    State(ctx): State<AppContext>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match signup::registration_open(&ctx.db).await {
        Ok(true) => {}
        Ok(false) => return redirect("/login"),
        Err(_) => return unavailable(),
    }
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = json!("Create Account");
    data["name"] = json!("");
    data["date_of_birth"] = json!("");
    data["email"] = json!("");
    data["errors"] = json!({});
    data["authenticity_token"] = json!(authenticity_token);
    (
        token,
        render(&view, "auth/create_account.html", data, StatusCode::OK),
    )
        .into_response()
}

async fn create(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<Create>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if browser::authenticate(&ctx.db, &session).await.is_ok() {
        return redirect("/");
    }
    if form.input.invitation_token.is_empty() {
        match signup::registration_open(&ctx.db).await {
            Ok(true) => {}
            Ok(false) => return redirect("/login"),
            Err(_) => return unavailable(),
        }
    }
    let request_id = super::medications::request_id(request);
    match signup::create(
        &ctx.db,
        &form.input,
        &ctx.config.server.full_url(),
        Some(&request_id),
    )
    .await
    {
        Ok(()) => {
            session.remove("invitation_pending");
            let mut data = super::medications::rendering::appearance_context();
            data["title"] = json!("Verify Account");
            render(&view, "auth/verification_sent.html", data, StatusCode::OK)
        }
        Err(SignupError::Invalid(errors)) => {
            let invited = !form.input.invitation_token.is_empty();
            let invitation = if invited {
                match invitations::preview(&ctx.db, &form.input.invitation_token).await {
                    Ok(value) => value,
                    Err(_) => return StatusCode::NOT_FOUND.into_response(),
                }
            } else {
                serde_json::Value::Null
            };
            let Ok(authenticity_token) = token.authenticity_token() else {
                return unavailable();
            };
            let mut data = super::medications::rendering::appearance_context();
            data["title"] = json!(if invited {
                "Complete Your Account"
            } else {
                "Create Account"
            });
            if invited {
                data["invitation"] = invitation;
            }
            data["invitation_token"] = json!(form.input.invitation_token);
            data["signed_in"] = json!(false);
            data["name"] = json!(form.input.name);
            data["date_of_birth"] = json!(form.input.date_of_birth);
            data["email"] = json!(form.input.email);
            data["errors"] = json!(errors);
            data["authenticity_token"] = json!(authenticity_token);
            (
                token,
                render(
                    &view,
                    if invited {
                        "invitations/accept.html"
                    } else {
                        "auth/create_account.html"
                    },
                    data,
                    StatusCode::UNPROCESSABLE_ENTITY,
                ),
            )
                .into_response()
        }
        Err(SignupError::RegistrationClosed) => redirect("/login"),
        Err(_) => unavailable(),
    }
}

async fn verification(
    State(ctx): State<AppContext>,
    Query(query): Query<VerificationQuery>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if let Some(key) = query.key {
        match signup::valid_key(&ctx.db, &key).await {
            Ok(true) => {
                session.set("verify_account_key", key);
                session.set_store(true);
                return redirect("/verify-account");
            }
            Ok(false) => {
                session.remove("verify_account_key");
                return redirect("/login");
            }
            Err(_) => return unavailable(),
        }
    }
    let Some(key) = session.get::<String>("verify_account_key") else {
        return redirect("/login");
    };
    match signup::valid_key(&ctx.db, &key).await {
        Ok(true) => {}
        Ok(false) => {
            session.remove("verify_account_key");
            return redirect("/login");
        }
        Err(_) => return unavailable(),
    }
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = json!("Verify Account");
    data["authenticity_token"] = json!(authenticity_token);
    data["error"] = serde_json::Value::Null;
    (
        token,
        render(&view, "auth/verify_account.html", data, StatusCode::OK),
    )
        .into_response()
}

async fn verify(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    AxumForm(form): AxumForm<Verify>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(key) = session.get::<String>("verify_account_key") else {
        return redirect("/login");
    };
    let request_id = super::medications::request_id(request);
    match signup::verify(&ctx.db, &session, &key, Some(&request_id)).await {
        Ok(outcome) => {
            session.remove("verify_account_key");
            match outcome {
                browser::SignInOutcome::Authenticated => {
                    super::auth::authenticated_redirect(&session)
                }
                browser::SignInOutcome::OtpRequired => redirect("/otp-auth"),
                browser::SignInOutcome::RecoveryRequired => redirect("/recovery-auth"),
                browser::SignInOutcome::PasskeyRequired => redirect("/webauthn-auth"),
            }
        }
        Err(SignupError::InvalidKey) => {
            session.remove("verify_account_key");
            redirect("/login")
        }
        Err(_) => unavailable(),
    }
}

fn render(view: &TeraView, path: &str, data: serde_json::Value, status: StatusCode) -> Response {
    match format::render().view(view, path, data) {
        Ok(response) => (status, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}

fn redirect(path: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, path),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

fn unavailable() -> Response {
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}
