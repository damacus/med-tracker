use axum::{Router, response::Html, routing::get};
use leptos::prelude::*;

#[component]
fn LoginPage() -> impl IntoView {
    view! {
        <main>
            <h1>"Welcome back"</h1>
            <form>
                <div>
                    <label for="email">"Email address"</label>
                    <input id="email" type="email" autocomplete="username" required/>
                </div>
                <div>
                    <label for="password">"Password"</label>
                    <a href="/reset-password-request">"Forgot?"</a>
                    <input id="password" type="password" autocomplete="current-password" required/>
                </div>
                <button type="submit" disabled>"Sign In to Dashboard"</button>
            </form>
        </main>
    }
}

fn render_login() -> String {
    let body = view! { <LoginPage/> }.to_html();
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Sign in | MedTracker</title></head><body>{body}</body></html>"
    )
}

async fn login() -> Html<String> {
    Html(render_login())
}

async fn health() -> &'static str {
    "ok"
}

pub fn app() -> Router {
    Router::new()
        .route("/login", get(login))
        .route("/health", get(health))
}

#[cfg(test)]
mod tests {
    use super::render_login;

    #[test]
    fn login_is_a_server_rendered_document() {
        let html = render_login();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("Welcome back"));
        assert!(html.contains("disabled"));
        assert!(!html.contains("<script"));
    }
}
