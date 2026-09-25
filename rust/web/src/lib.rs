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
                    <p class="form-intro">"Sign in to your household dashboard or continue to your mobile app."</p>
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
    let title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"theme-color\" content=\"#f3f8fb\"><link rel=\"stylesheet\" href=\"/auth.css\"><title>{title} | MedTracker</title></head><body>{body}</body></html>"
    )
}

fn authenticated_document(title: &str, csrf: &str, body: String) -> String {
    document(title, body).replacen(
        "<head>",
        &format!("<head><meta name=\"csrf-token\" content=\"{csrf}\">"),
        1,
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

pub fn render_dashboard(household_name: &str, csrf: &str, empty: bool) -> String {
    let body = view! {
        <main class="auth-page">
            <div class="auth-shell">
                <BrandPanel/>
                <section class="form-panel" aria-label="Household dashboard">
                    <p class="eyebrow">"YOUR HOUSEHOLD"</p>
                    <h1>{household_name.to_owned()}</h1>
                    {empty.then(|| view! { <p class="form-intro">"You do not have an active household yet."</p> })}
                    <form action="/logout" method="post">
                        <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                        <button class="primary-button" type="submit">"Sign out"</button>
                    </form>
                </section>
            </div>
        </main>
    }
    .to_html();
    authenticated_document("Dashboard", csrf, body)
}

#[derive(Clone)]
pub struct MedicationCard {
    pub id: i64,
    pub name: String,
    pub supply: String,
    pub unit: String,
}

#[derive(Clone)]
pub struct DoseSource {
    pub id: i64,
    pub kind: String,
    pub person_name: String,
    pub amount: String,
    pub unit: String,
    pub portable_id: String,
    pub can_record: bool,
    pub eligible_stock_ids: Vec<i64>,
}

#[derive(Clone)]
pub struct HistoryRow {
    pub medication_name: String,
    pub amount: String,
    pub unit: String,
}

pub struct MedicationDetail {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub supply: String,
    pub unit: String,
    pub location: String,
    pub sources: Vec<DoseSource>,
}

pub struct DoseFormState {
    pub source_type: String,
    pub source_id: String,
    pub dose_amount: String,
    pub dose_unit: String,
    pub taken_at: String,
    pub stock_id: String,
}

fn medication_document(title: &str, csrf: &str, body: String) -> String {
    authenticated_document(title, csrf, body)
        .replacen("</head>", "<link rel=\"stylesheet\" href=\"/medication.css\"><script defer src=\"/medication.js\"></script></head>", 1)
}

pub fn render_medication_list(
    household_name: &str,
    slug: &str,
    csrf: &str,
    medications: Vec<MedicationCard>,
) -> String {
    let prefix = format!("/households/{slug}");
    let body = view! {
        <main class="med-app">
            <header class="med-topbar"><a class="med-brand" href=format!("{prefix}/dashboard")>"MedTracker"</a><span>{household_name.to_owned()}</span></header>
            <div class="med-layout">
                <nav class="med-sidebar" aria-label="Household"><a href=format!("{prefix}/dashboard")>"Dashboard"</a><a aria-current="page" href=format!("{prefix}/medications")>"Inventory"</a></nav>
                <section class="med-content">
                    <p class="med-eyebrow">"HOUSEHOLD INVENTORY"</p>
                    <h1>"Medications"</h1>
                    <div class="med-grid">
                        {medications.into_iter().map(|medication| {
                            let href = format!("{prefix}/medications/{}", medication.id);
                            view! {
                                <article class="med-card">
                                    <h2>{medication.name.clone()}</h2>
                                    <p>{medication.supply}" "{medication.unit}" remaining"</p>
                                    <a href=href>"View medication"</a>
                                </article>
                            }
                        }).collect_view()}
                    </div>
                </section>
            </div>
        </main>
    }.to_html();
    medication_document("Medications", csrf, body)
}

pub fn render_medication_detail(
    household_name: &str,
    slug: &str,
    csrf: &str,
    medication: MedicationDetail,
    stock_options: Vec<MedicationCard>,
    taken_at: &str,
    client_uuid: &str,
    notice: Option<&str>,
    form_state: Option<DoseFormState>,
) -> String {
    let prefix = format!("/households/{slug}");
    let dose_action = format!("{prefix}/medications/{}/doses", medication.id);
    let first = if let Some(form) = form_state.as_ref() {
        medication
            .sources
            .iter()
            .find(|source| source.kind == form.source_type && source.portable_id == form.source_id)
            .cloned()
    } else {
        medication.sources.first().cloned()
    };
    let source_unavailable = form_state.is_some() && first.is_none();
    let source_type = form_state
        .as_ref()
        .map(|form| form.source_type.clone())
        .or_else(|| first.as_ref().map(|source| source.kind.clone()))
        .unwrap_or_default();
    let source_id = form_state
        .as_ref()
        .map(|form| form.source_id.clone())
        .or_else(|| first.as_ref().map(|source| source.portable_id.clone()))
        .unwrap_or_default();
    let dose_amount = form_state
        .as_ref()
        .map(|form| form.dose_amount.clone())
        .or_else(|| first.as_ref().map(|source| source.amount.clone()))
        .unwrap_or_default();
    let dose_unit = form_state
        .as_ref()
        .map(|form| form.dose_unit.clone())
        .or_else(|| first.as_ref().map(|source| source.unit.clone()))
        .unwrap_or_default();
    let taken_at = form_state
        .as_ref()
        .map(|form| form.taken_at.clone())
        .unwrap_or_else(|| taken_at.to_owned());
    let selected_stock = form_state.as_ref().map(|form| form.stock_id.clone());
    let displayed_dose = if source_type == "schedule" {
        "Calculated for selected time".to_owned()
    } else {
        format!("{} {}", dose_amount, dose_unit)
    };
    let initial_stock_ids = first
        .as_ref()
        .map(|source| source.eligible_stock_ids.clone())
        .unwrap_or_default();
    let first_stock_id = initial_stock_ids.first().copied();
    let has_recordable_source = medication
        .sources
        .iter()
        .any(|source| source.can_record && !source.eligible_stock_ids.is_empty());
    let body = view! {
        <main class="med-app">
            <header class="med-topbar"><a class="med-brand" href=format!("{prefix}/dashboard")>"MedTracker"</a><span>{household_name.to_owned()}</span></header>
            <div class="med-layout">
                <nav class="med-sidebar" aria-label="Household"><a href=format!("{prefix}/dashboard")>"Dashboard"</a><a href=format!("{prefix}/medications")>"Inventory"</a></nav>
                <section class="med-content">
                    <p class="med-eyebrow">"MEDICATION PROFILE"</p>
                    <h1>{medication.name.clone()}</h1>
                    <p class="med-location">{medication.location.clone()}</p>
                    {form_state.is_none().then(|| notice).flatten().map(|text| view! { <p class="med-alert" role="alert">{text.to_owned()}</p> })}
                    <div class="med-detail-grid">
                        {(!medication.description.is_empty()).then(|| view! { <section class="med-card"><h2>"Overview"</h2><p>{medication.description.clone()}</p></section> })}
                        <section class="med-card med-stock"><h2>"Inventory Status"</h2><div class="med-stock-number"><strong>{medication.supply.clone()}</strong><span>{medication.unit.clone()}" remaining"</span></div><p>"Stock source: "{medication.location.clone()}</p></section>
                    </div>
                    {has_recordable_source.then(|| view! { <a class="med-button med-log-link" href="#administration" data-open-administration>"Log"</a> })}
                </section>
            </div>
            <dialog id="administration-dialog" aria-label=format!("Log administration for {}", medication.name)>
                <div class="med-dialog-heading"><div><h2>"Log administration for "{medication.name.clone()}</h2><p>"Choose the person and source."</p></div><button type="button" class="med-close" data-close-dialog aria-label="Close">"×"</button></div>
                <div class="med-source-list">
                    {medication.sources.into_iter().map(|source| {
                        let test_id = format!("log-administration-{}-{}", source.kind, source.id);
                        let stock_ids = source.eligible_stock_ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
                        view! {
                            <article class="med-source-card"><div><p class="med-eyebrow">"MEDICATION SOURCE"</p><strong>{source.person_name.clone()}</strong><p>{medication.name.clone()}</p><small>{if source.kind == "schedule" { "Dose calculated for selected time".to_owned() } else { format!("{} {}", source.amount, source.unit) }}</small></div>
                                {(source.can_record && !source.eligible_stock_ids.is_empty()).then(|| view! { <button type="button" class="med-button" data-open-dose data-testid=test_id data-source-id=source.portable_id data-source-kind=source.kind data-person-name=source.person_name data-dose-amount=source.amount data-dose-unit=source.unit data-stock-ids=stock_ids>"Log"</button> })}
                                {(!source.can_record).then(|| view! { <span class="med-source-note">"View only"</span> })}
                                {(source.can_record && source.eligible_stock_ids.is_empty()).then(|| view! { <span class="med-source-note">"No eligible stock"</span> })}
                            </article>
                        }
                    }).collect_view()}
                </div>
            </dialog>
            <dialog id="dose-dialog" aria-label="Record dose" data-reopen=form_state.is_some()>
                <div class="med-dialog-heading"><div><h2>"Record dose"</h2><p>"Confirm the person, medication, time, and inventory source."</p></div><button type="button" class="med-close" data-close-dialog aria-label="Close">"×"</button></div>
                {form_state.is_some().then(|| view! { <p class="med-alert" role="alert">{notice.unwrap_or("Please review this dose.").to_owned()}</p> })}
                {source_unavailable.then(|| view! { <p class="med-source-note">"This source is unavailable. Choose another source before submitting."</p><button type="button" class="med-text-button" data-choose-source>"Choose source"</button> })}
                <div class="med-dose-summary"><div><small>"PERSON"</small><strong id="dose-person">{first.as_ref().map(|source| source.person_name.clone()).unwrap_or_else(|| "Choose source".to_owned())}</strong></div><div><small>"MEDICATION"</small><strong>{medication.name.clone()}</strong></div><div><small>"DOSE"</small><strong id="dose-display">{displayed_dose}</strong></div></div>
                <form class="med-dose-form" method="post" action=dose_action>
                    <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                    <input type="hidden" name="client_uuid" value=client_uuid.to_owned()/>
                    <input type="hidden" name="source_type" value=source_type/>
                    <input type="hidden" name="source_id" value=source_id/>
                    <input type="hidden" name="dose_amount" value=dose_amount/>
                    <input type="hidden" name="dose_unit" value=dose_unit/>
                    <label for="taken-at">"Taken at"</label><input id="taken-at" name="taken_at" type="datetime-local" value=taken_at required/>
                    <label for="stock-source">"Stock source"</label>
                    <select id="stock-source" name="taken_from_medication_id">
                        {stock_options.into_iter().map(|option| { let selected = selected_stock.as_ref().is_some_and(|stock| stock == &option.id.to_string()) || selected_stock.is_none() && first_stock_id == Some(option.id); let available = initial_stock_ids.contains(&option.id); view! { <option value=option.id selected=selected hidden=!available disabled=!available>{option.name}" — "{option.supply}" "{option.unit}" remaining"</option> } }).collect_view()}
                    </select>
                    <div class="med-dialog-actions"><button class="med-button" type="submit" disabled=source_unavailable>"Log"</button></div>
                </form>
            </dialog>
        </main>
    }.to_html();
    medication_document(&medication.name, csrf, body)
}

pub fn render_journey_dashboard(
    household_name: &str,
    slug: &str,
    csrf: &str,
    history: Vec<HistoryRow>,
) -> String {
    let prefix = format!("/households/{slug}");
    let body = view! {
        <main class="med-app">
            <header class="med-topbar"><a class="med-brand" href=format!("{prefix}/dashboard")>"MedTracker"</a><span>{household_name.to_owned()}</span></header>
            <div class="med-layout"><nav class="med-sidebar" aria-label="Household"><a aria-current="page" href=format!("{prefix}/dashboard")>"Dashboard"</a><a href=format!("{prefix}/medications")>"Inventory"</a></nav>
                <section class="med-content"><p class="med-eyebrow">"TODAY"</p><h1>{household_name.to_owned()}</h1><a class="med-button" href=format!("{prefix}/medications")>"View inventory"</a>
                    <section class="med-card med-history" data-testid="dashboard-today-dose-history"><h2>"Previous Doses Today"</h2>
                        {history.into_iter().map(|row| view! { <div class="med-history-row"><strong>{row.medication_name}</strong><span>{row.amount}" "{row.unit}</span></div> }).collect_view()}
                    </section>
                    <form action="/logout" method="post"><input type="hidden" name="authenticity_token" value=csrf.to_owned()/><button type="submit" class="med-text-button">"Sign out"</button></form>
                </section>
            </div>
        </main>
    }.to_html();
    medication_document("Dashboard", csrf, body)
}

pub fn stylesheet() -> &'static str {
    include_str!("auth.css")
}

pub fn medication_stylesheet() -> &'static str {
    include_str!("medication.css")
}

pub fn medication_script() -> &'static str {
    include_str!("medication.js")
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
    use super::{
        DoseFormState, DoseSource, MedicationCard, MedicationDetail, render_login,
        render_medication_detail,
    };

    #[test]
    fn login_is_a_server_rendered_document() {
        let html = render_login("csrf", "");
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("Welcome back"));
        assert!(html.contains("authenticity_token"));
        assert!(!html.contains("disabled"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn rejected_dose_preserves_form_snapshot_and_reopens_dialog() {
        let html = render_medication_detail(
            "Home",
            "home",
            "csrf",
            MedicationDetail {
                id: 1,
                name: "Example".to_owned(),
                description: String::new(),
                supply: "20".to_owned(),
                unit: "ml".to_owned(),
                location: "Cabinet".to_owned(),
                sources: vec![DoseSource {
                    id: 2,
                    kind: "person_medication".to_owned(),
                    person_name: "Sam".to_owned(),
                    amount: "1.25".to_owned(),
                    unit: "ml".to_owned(),
                    portable_id: "source-2".to_owned(),
                    can_record: true,
                    eligible_stock_ids: vec![1],
                }],
            },
            vec![MedicationCard {
                id: 1,
                name: "Example".to_owned(),
                supply: "20".to_owned(),
                unit: "ml".to_owned(),
            }],
            "2026-03-30T08:00",
            "retry-uuid",
            Some("Invalid dose configured"),
            Some(DoseFormState {
                source_type: "person_medication".to_owned(),
                source_id: "source-2".to_owned(),
                dose_amount: "invalid".to_owned(),
                dose_unit: "ml".to_owned(),
                taken_at: "2026-03-30T09:00".to_owned(),
                stock_id: "1".to_owned(),
            }),
        );
        assert!(html.contains("data-reopen>"));
        assert!(html.contains("name=\"client_uuid\" value=\"retry-uuid\""));
        assert!(html.contains("name=\"dose_amount\" value=\"invalid\""));
        assert!(html.contains("id=\"dose-display\">invalid ml"));
        assert!(
            html.contains("name=\"taken_at\" type=\"datetime-local\" value=\"2026-03-30T09:00\"")
        );
    }
}
