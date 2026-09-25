use axum::{Router, http::header, response::Html, routing::get};
use leptos::prelude::*;

#[component]
fn BrandPanel() -> impl IntoView {
    view! {
        <section class="brand-panel" aria-label="MedTracker">
            <div class="brand-name"><span class="brand-mark" aria-hidden="true">"M"</span><span>"MedTracker"</span></div>
            <div class="brand-intro">
                <p class="eyebrow">"YOUR HEALTH, IN ONE PLACE"</p>
                <p class="brand-heading">"A clearer view of every medicine."</p>
                <p class="brand-description">"Keep your household's medication details close at hand."</p>
            </div>
            <div class="brand-accent" aria-hidden="true"><span></span><span></span><span></span></div>
        </section>
    }
}

#[component]
fn LoginPage(csrf: String, error: String) -> impl IntoView {
    let focus_password = !error.is_empty();
    view! {
        <main class="auth-page">
            <div class="auth-shell">
                <BrandPanel/>
                <section class="form-panel" aria-label="Sign in">
                    <p class="eyebrow">"SECURE SIGN IN"</p>
                    <h1>"Welcome back"</h1>
                    <p class="form-intro">"Sign in to continue to your mobile app."</p>
                    {(!error.is_empty()).then(|| view! { <p class="form-alert" role="alert">{error}</p> })}
                    <form class="auth-form" action="/login" method="post">
                        <input type="hidden" name="authenticity_token" value=csrf/>
                        <div class="form-field">
                            <label for="email">"Email address"</label>
                            <input id="email" name="email" type="email" autocomplete="username" required/>
                        </div>
                        <div class="form-field">
                            <div class="field-heading"><label for="password">"Password"</label><a href="/reset-password-request">"Forgot?"</a></div>
                            <input id="password" name="password" type="password" autocomplete="current-password" autofocus=focus_password required/>
                        </div>
                        <button class="primary-button" type="submit">"Sign In to Dashboard"</button>
                    </form>
                </section>
            </div>
        </main>
    }
}

#[component]
fn PublicLoginPage(heading: String, message: String) -> impl IntoView {
    view! {
        <main class="auth-page">
            <div class="auth-shell">
                <BrandPanel/>
                <section class="form-panel" aria-label="Sign in information">
                    <p class="eyebrow">"SECURE SIGN IN"</p>
                    <h1>{heading}</h1>
                    <p class="form-intro">{message}</p>
                </section>
            </div>
        </main>
    }
}

#[component]
fn ConsentPage(
    csrf: String,
    client_name: String,
    scopes: Vec<String>,
    fields: Vec<(String, String)>,
) -> impl IntoView {
    view! {
        <main class="auth-page">
            <div class="auth-shell">
                <BrandPanel/>
                <section class="form-panel" aria-label="Application authorization">
                    <p class="eyebrow">"MOBILE ACCESS"</p>
                    <h1>"Authorize application"</h1>
                    <p class="form-intro">{client_name}" requests access to your MedTracker data."</p>
                    <form class="auth-form" id="authorize-form" action="/authorize" method="post">
                        <input type="hidden" name="authenticity_token" value=csrf/>
                        {fields.into_iter().map(|(name, value)| view! {
                            <input type="hidden" name=name value=value/>
                        }).collect_view()}
                        <fieldset class="scope-list">
                            <legend>"Access requested"</legend>
                            {scopes.into_iter().map(|scope| {
                                let (label, description) = scope_copy(&scope);
                                view! {
                                    <label class="scope-row">
                                        <input type="checkbox" name="scope[]" value=scope checked/>
                                        <span class="scope-copy"><strong>{label}</strong><small>{description}</small></span>
                                    </label>
                                }
                            }).collect_view()}
                        </fieldset>
                        <button class="primary-button" type="submit">"Authorize"</button>
                    </form>
                </section>
            </div>
        </main>
    }
}

fn scope_copy(scope: &str) -> (&'static str, &'static str) {
    match scope {
        "medtracker" => (
            "MedTracker data",
            "Read and update your MedTracker account data.",
        ),
        "offline_access" => ("Stay signed in", "Keep access active between visits."),
        _ => ("Other access", "Access requested by this application."),
    }
}

fn document(title: &str, body: String) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"theme-color\" content=\"#f3f8fb\"><link rel=\"stylesheet\" href=\"/auth.css\"><title>{title} | MedTracker</title></head><body>{body}</body></html>"
    )
}

pub fn render_login(csrf: &str, error: &str) -> String {
    if csrf.is_empty() {
        return document("Sign in", view! { <PublicLoginPage heading="Sign in through your app".to_owned() message="Open MedTracker from your registered mobile app to begin sign-in.".to_owned()/> }.to_html());
    }
    document(
        "Sign in",
        view! { <LoginPage csrf=csrf.to_owned() error=error.to_owned()/> }.to_html(),
    )
}

pub fn render_reset_unavailable() -> String {
    document("Password reset", view! { <PublicLoginPage heading="Password reset is unavailable".to_owned() message="Password reset is not yet supported in this sign-in service.".to_owned()/> }.to_html())
}

pub fn stylesheet() -> &'static str {
    include_str!("auth.css")
}

pub fn render_consent(
    csrf: &str,
    client_name: &str,
    scopes: &[String],
    fields: &[(String, String)],
) -> String {
    document("Authorize", view! { <ConsentPage csrf=csrf.to_owned() client_name=client_name.to_owned() scopes=scopes.to_vec() fields=fields.to_vec()/> }.to_html())
}

async fn login() -> Html<String> {
    Html(render_login("", ""))
}

async fn styles() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        stylesheet(),
    )
}

async fn health() -> &'static str {
    "ok"
}

pub fn app() -> Router {
    Router::new()
        .route("/login", get(login))
        .route("/auth.css", get(styles))
        .route("/health", get(health))
}

#[cfg(test)]
mod tests {
    use super::render_login;

    #[test]
    fn login_is_a_server_rendered_document() {
        let html = render_login("csrf", "");
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("Welcome back"));
        assert!(html.contains("authenticity_token"));
        assert!(!html.contains("disabled"));
        assert!(!html.contains("<script"));
    }
}
