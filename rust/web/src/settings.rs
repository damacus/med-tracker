use crate::household::path_segment;
use crate::household_i18n::{Locale, Text, TranslationError};
use damacus_web_ui::{Button, ButtonType};
use leptos::prelude::*;
use std::collections::HashMap;

pub struct SettingsPage {
    pub household_name: String,
    pub slug: String,
    pub household_id: i64,
    pub locale: Locale,
    pub csrf: String,
    pub person_name: String,
    pub person_id: i64,
    pub email: String,
    pub date_of_birth: Option<String>,
    pub age: Option<i64>,
    pub person_type: String,
    pub has_capacity: bool,
    pub mobile_shortcuts: Vec<String>,
    pub gravatar_enabled: bool,
    pub avatar_attached: bool,
    pub gravatar_url: Option<String>,
    pub vapid_public_key: Option<String>,
    pub active_section: String,
    pub security_html: String,
    pub notifications_html: String,
    pub advanced_html: String,
    pub time_zone: String,
    pub can_edit: bool,
    pub errors: HashMap<String, Vec<String>>,
    pub notice: String,
}

fn profile_icon_svg(name: &str) -> &'static str {
    match name {
        "dashboard" | "locations" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/></svg>"#
        }
        "inventory" => {
            r#"<svg viewBox="0 -960 960 960" width="24" height="24" fill="currentColor"><path d="M620-163 450-333l56-56 114 114 226-226 56 56-282 282Zm220-397h-80v-200h-80v120H280v-120h-80v560h240v80H200q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h167q11-35 43-57.5t70-22.5q40 0 71.5 22.5T594-840h166q33 0 56.5 23.5T840-760v200ZM480-760q17 0 28.5-11.5T520-800q0-17-11.5-28.5T480-840q-17 0-28.5 11.5T440-800q0 17 11.5 28.5T480-760Z"/></svg>"#
        }
        "people" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>"#
        }
        "profile" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>"#
        }
        "notifications" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>"#
        }
        "search" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>"#
        }
        "logout" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="m16 17 5-5-5-5"/><line x1="21" x2="9" y1="12" y2="12"/></svg>"#
        }
        "menu" => {
            r#"<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 6h18M3 12h18M3 18h18"/></svg>"#
        }
        _ => "",
    }
}

pub fn render_settings(page: SettingsPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get("profiles.show.title", &[])?;
    let zone_label = text.get("profiles.time_zone.title", &[])?;
    let hint = text.get("profiles.time_zone.description", &[])?;
    let save = text.get("profiles.time_zone.save", &[])?;
    let read_only = text.get("profiles.time_zone.read_only", &[])?;
    let zone_errors = translated_errors(text, page.errors.get("time_zone"))?;
    let mut other_errors: Vec<_> = page
        .errors
        .iter()
        .filter(|(field, _)| field.as_str() != "time_zone")
        .collect();
    other_errors.sort_by_key(|(field, _)| *field);
    let other_errors: Vec<_> = other_errors
        .into_iter()
        .flat_map(|(_, errors)| errors)
        .map(|message| text.form_error(message))
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let invalid_zone = !zone_errors.is_empty();
    let described_by = if invalid_zone {
        "settings-time-zone-hint settings-time-zone-errors"
    } else {
        "settings-time-zone-hint"
    };
    let zones = time_zone_options(&page.time_zone);
    let action = format!("/households/{}/profile", path_segment(&page.slug));
    let document_slug = page.slug.clone();
    let document_locale = page.locale;
    let document_csrf = page.csrf.clone();
    let document_person_name = page.person_name.clone();
    let document_vapid_key = page.vapid_public_key.clone();
    let document_shortcuts = page.mobile_shortcuts.clone();
    let avatar_action = format!("/api/v1/households/{}/profile/avatar", page.household_id);
    let gravatar_action = format!("{action}/gravatar");
    let shortcuts_action = format!("{action}/shortcuts");
    let avatar_url = if page.avatar_attached {
        Some(format!(
            "/api/v1/households/{}/profile/avatar",
            page.household_id
        ))
    } else {
        page.gravatar_url.clone()
    };
    let base = action.clone();
    let profile_active = page.active_section == "profile";
    let security_active = page.active_section == "security";
    let notifications_active = page.active_section == "notifications";
    let advanced_active = page.active_section == "advanced";
    let initials: String = page
        .person_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect();
    let age = page.age.map(|value| value.to_string());
    let date_of_birth = page
        .date_of_birth
        .as_deref()
        .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .map(|value| value.format("%B %d, %Y").to_string())
        .unwrap_or_else(|| "Not set".to_owned());
    let person_type = match page.person_type.as_str() {
        "adult" => "Adult",
        "minor" => "Minor",
        "dependent_adult" => "Dependent adult",
        other => other,
    };
    let capacity = if page.has_capacity { "Yes" } else { "No" };
    let avatar_title = text.get("profiles.avatar.title", &[])?;
    let avatar_description = text.get("profiles.avatar.description", &[])?;
    let shortcuts_title = text.get("profiles.mobile_shortcuts.title", &[])?;
    let shortcuts_description = text.get("profiles.mobile_shortcuts.description", &[])?;
    let unavailable_option = text.get("profiles.mobile_shortcuts.unavailable_option", &[])?;
    let appearance_title = text.get("profiles.appearance.title", &[])?;
    let appearance_description = text.get("profiles.appearance.description", &[])?;
    let shortcut_items = [
        ("dashboard", text.get("layouts.mobile_rail.home", &[])?),
        ("inventory", text.get("layouts.mobile_rail.inventory", &[])?),
        ("locations", text.get("layouts.sidebar.locations", &[])?),
        ("people", text.get("layouts.sidebar.people", &[])?),
        ("profile", text.get("profiles.sections.profile.title", &[])?),
    ];
    let body = view! {
        <div class="profile-page">
        <header class="profile-hero" data-testid="profile-hero">
            <div class="profile-identity"><span class="profile-avatar" aria-hidden="true"><span>{initials.clone()}</span>{avatar_url.clone().map(|url| view! { <img src=url alt="" data-profile-avatar-image="true"/> })}</span><div><p class="profile-eyebrow">{text.get("profiles.show.eyebrow", &[])?}</p><h1>{title.clone()}</h1><p class="profile-email">{page.email.clone()}</p></div></div>
            <p class="profile-description">{text.get("profiles.show.description", &[])?}</p>
        </header>
        <nav class="profile-tabs" aria-label=title.clone() role="tablist">
            <a id="profile-tab-profile" role="tab" aria-selected=profile_active.to_string() aria-controls="profile-profile-panel" tabindex=if profile_active { "0" } else { "-1" } href=base.clone()>{text.get("profiles.sections.profile.title", &[])?}</a>
            <a id="profile-tab-security" role="tab" aria-selected=security_active.to_string() aria-controls="profile-security-panel" tabindex=if security_active { "0" } else { "-1" } href=format!("{base}?section=security")>{text.get("profiles.sections.security.title", &[])?}</a>
            <a id="profile-tab-notifications" role="tab" aria-selected=notifications_active.to_string() aria-controls="profile-notifications-panel" tabindex=if notifications_active { "0" } else { "-1" } href=format!("{base}?section=notifications")>{text.get("profiles.sections.notifications.title", &[])?}</a>
            <a id="profile-tab-advanced" role="tab" aria-selected=advanced_active.to_string() aria-controls="profile-advanced-panel" tabindex=if advanced_active { "0" } else { "-1" } href=format!("{base}?section=advanced")>{text.get("profiles.sections.advanced.title", &[])?}</a>
        </nav>
        {(!page.notice.is_empty()).then(|| view! { <p class="med-success" role="status">{page.notice}</p> })}
        {(!other_errors.is_empty()).then(|| view! { <ul class="med-alert" role="alert">{other_errors.into_iter().map(|error| view! { <li>{error}</li> }).collect_view()}</ul> })}
        <section id="profile-profile-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-profile" tabindex="0" hidden=!profile_active>
            <div class="profile-section-header"><span class="profile-section-icon" aria-hidden="true" inner_html=profile_icon_svg("profile")/><div><h2>{text.get("profiles.sections.profile.title", &[])?}</h2><div class="profile-summary"><span>{page.person_name.clone()}</span><span>{page.email.clone()}</span><span>{text.get("profiles.appearance.modes.system", &[])?}</span></div></div></div>
            <div class="profile-grid">
                <section class="profile-info-card" data-testid="profile-personal-info-card"><h3>{text.get("profiles.show.personal_information.title", &[])?}</h3><p>{text.get("profiles.show.personal_information.description", &[])?}</p><dl>
                    <div><dt>Name</dt><dd>{page.person_name.clone()}</dd></div>
                    <div><dt>Email</dt><dd>{page.email.clone()}</dd></div>
                    <div><dt>Time Zone</dt><dd>{page.time_zone.clone()}</dd></div>
                    <div><dt>Date of Birth</dt><dd>{date_of_birth}</dd></div>
                    {age.map(|value| view! { <div><dt>Age</dt><dd>{value}</dd></div> })}
                    <div><dt>Person Type</dt><dd>{person_type}</dd></div>
                    <div><dt>Has Capacity</dt><dd>{capacity}</dd></div>
                </dl></section>
                <div class="profile-settings">
                    <button type="button" class="profile-setting-row" data-profile-dialog="profile-avatar-modal"><span><strong>{avatar_title.clone()}</strong><small>{avatar_description.clone()}</small></span><span aria-hidden="true">{"›"}</span></button>
                    <button type="button" class="profile-setting-row" data-profile-dialog="profile-time-zone-modal"><span><strong>{zone_label.clone()}</strong><small>{hint.clone()}</small></span><span aria-hidden="true">{"›"}</span></button>
                    <button type="button" class="profile-setting-row" data-profile-dialog="profile-shortcuts-modal"><span><strong>{shortcuts_title.clone()}</strong><small>{shortcuts_description.clone()}</small></span><span aria-hidden="true">{"›"}</span></button>
                    <button type="button" class="profile-setting-row" data-profile-dialog="profile-appearance-modal"><span><strong>{appearance_title.clone()}</strong><small>{appearance_description.clone()}</small></span><span aria-hidden="true">{"›"}</span></button>
                </div>
            </div>
        </section>
        <section id="profile-security-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-security" tabindex="0" hidden=!security_active inner_html=page.security_html></section>
        <section id="profile-notifications-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-notifications" tabindex="0" hidden=!notifications_active inner_html=page.notifications_html></section>
        <section id="profile-advanced-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-advanced" tabindex="0" hidden=!advanced_active inner_html=page.advanced_html></section>
        </div>
        <dialog id="profile-time-zone-modal" class="profile-dialog profile-dialog-compact" data-testid="profile-time-zone-dialog" aria-labelledby="profile-time-zone-heading" open=invalid_zone>
            <div class="profile-dialog-heading profile-overlay-heading"><h2 id="profile-time-zone-heading">{zone_label.clone()}</h2><p>{hint.clone()}</p><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-time-zone-modal">{"×"}</button></div>
        <form class="household-form profile-overlay-body" method="post" action=action>
            <input type="hidden" name="authenticity_token" value=page.csrf.clone()/>
            <div class="form-field">
                <label for="settings_time_zone">{zone_label}</label>
                <p id="settings-time-zone-hint" class="profile-sr-only">{hint}</p>
                <select id="settings_time_zone" name="time_zone" required disabled=!page.can_edit
                    aria-invalid=invalid_zone.to_string() aria-describedby=described_by>
                    {zones.into_iter().map(|zone| { let current = zone == page.time_zone; view! { <option value=zone selected=current>{zone.clone()}</option> } }).collect_view()}
                </select>
                {invalid_zone.then(|| view! {
                    <ul id="settings-time-zone-errors" class="field-errors" role="alert">
                        {zone_errors.into_iter().map(|error| view! { <li>{error}</li> }).collect_view()}
                    </ul>
                })}
            </div>
            {(!page.can_edit).then(|| view! { <p>{read_only}</p> })}
            {page.can_edit.then(|| view! { <div class="household-actions"><Button class="profile-secondary" attr:data-profile-close="profile-time-zone-modal">{"Close"}</Button><Button class="med-primary" kind=ButtonType::Submit>{save}</Button></div> })}
        </form>
        </dialog>
        <dialog id="profile-avatar-modal" class="profile-dialog profile-sheet" data-testid="profile-avatar-sheet" aria-labelledby="profile-avatar-heading">
            <div class="profile-dialog-heading profile-overlay-heading"><h2 id="profile-avatar-heading">{avatar_title.clone()}</h2><p>{avatar_description.clone()}</p><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-avatar-modal">{"×"}</button></div>
            <div class="profile-sheet-body profile-overlay-body">
                <div class="profile-avatar-preview"><span class="profile-avatar" aria-hidden="true"><span>{initials}</span>{avatar_url.map(|url| view! { <img src=url alt="" data-profile-avatar-image="true"/> })}</span><strong>{page.person_name.clone()}</strong></div>
                <p>{text.get("profiles.avatar.supported_formats", &[])?}</p>
                <form class="profile-stack" method="post" action=avatar_action.clone() enctype="multipart/form-data" data-profile-avatar-upload="true">
                    <input type="hidden" name="authenticity_token" value=page.csrf.clone()/>
                    <label for="person_avatar">{text.get("profiles.avatar.upload_label", &[])?}</label>
                    <input id="person_avatar" type="file" name="avatar" accept="image/png,image/jpeg,image/webp" disabled=!page.can_edit/>
                    {page.can_edit.then(|| view! { <button class="med-primary" type="submit">{text.get("profiles.avatar.upload", &[]).unwrap_or_default()}</button> })}
                </form>
                {(page.can_edit && page.avatar_attached).then(|| view! { <form method="post" action=avatar_action data-profile-avatar-remove="true"><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><button class="profile-secondary" type="submit">{text.get("profiles.avatar.remove", &[]).unwrap_or_default()}</button></form> })}
                <p id="profile-avatar-errors" class="field-errors" role="alert" hidden></p>
                <form class="profile-gravatar" method="post" action=gravatar_action>
                    <input type="hidden" name="authenticity_token" value=page.csrf.clone()/>
                    <div><strong>{text.get("profiles.avatar.gravatar_label", &[])?}</strong><p>{text.get("profiles.avatar.gravatar_description", &[])?}</p></div>
                    <select name="gravatar_enabled" aria-label= text.get("profiles.avatar.gravatar_label", &[])? disabled=!page.can_edit><option value="1" selected=page.gravatar_enabled>{text.get("profiles.common.on", &[])?}</option><option value="0" selected=!page.gravatar_enabled>{text.get("profiles.common.off", &[])?}</option></select>
                    {page.can_edit.then(|| view! { <button class="med-primary" type="submit">{text.get("profiles.avatar.save_gravatar", &[]).unwrap_or_default()}</button> })}
                </form>
            </div>
        </dialog>
        <dialog id="profile-shortcuts-modal" class="profile-dialog profile-sheet" data-testid="profile-mobile-shortcuts-sheet" aria-labelledby="profile-shortcuts-heading">
            <div class="profile-dialog-heading profile-overlay-heading"><h2 id="profile-shortcuts-heading">{shortcuts_title}</h2><p>{shortcuts_description}</p><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-shortcuts-modal">{"×"}</button></div>
            <form class="profile-sheet-body profile-overlay-body profile-stack" method="post" action=shortcuts_action>
                <input type="hidden" name="authenticity_token" value=page.csrf.clone()/>
                <p>{text.get("profiles.mobile_shortcuts.rust_unavailable", &[])?}</p>
                {(0..3).map(|index| { let value = page.mobile_shortcuts.get(index).cloned().unwrap_or_default(); let unavailable = match value.as_str() { "finder" => Some("Medication Finder"), "medicine_reviews" => Some("Medicine reviews"), "reports" => Some("Reports"), "administration" => Some("Administration"), _ => None }; view! { <div class="form-field"><label for=format!("profile-shortcut-{index}")>{text.get("profiles.mobile_shortcuts.slot", &[("number", &(index+1).to_string())]).unwrap_or_default()}</label><select id=format!("profile-shortcut-{index}") name=format!("mobile_shortcuts_{index}") disabled=!page.can_edit><option value="" selected=value.is_empty()>{text.get("profiles.mobile_shortcuts.none", &[]).unwrap_or_default()}</option>{unavailable.map(|label| view! { <option value=value.clone() selected>{format!("{label} ({unavailable_option})")}</option> })}{shortcut_items.iter().map(|(key, label)| view! { <option value=*key selected=value == *key>{label.clone()}</option> }).collect_view()}</select></div> } }).collect_view()}
                {page.can_edit.then(|| view! { <div class="household-actions"><button type="button" class="profile-secondary" data-profile-close="profile-shortcuts-modal">{"Close"}</button><button class="med-primary" type="submit">{text.get("profiles.mobile_shortcuts.save", &[]).unwrap_or_default()}</button></div> })}
            </form>
        </dialog>
        <dialog id="profile-appearance-modal" class="profile-dialog profile-sheet profile-sheet-wide" data-testid="profile-appearance-sheet" aria-labelledby="profile-appearance-heading">
            <div class="profile-dialog-heading profile-overlay-heading"><h2 id="profile-appearance-heading">{appearance_title}</h2><p>{appearance_description}</p><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-appearance-modal">{"×"}</button></div>
            <div class="profile-sheet-body profile-overlay-body profile-stack"><h3>{text.get("profiles.appearance.mode_label", &[])?}</h3><div class="profile-choice-grid">{[("light", "Light"), ("dark", "Dark"), ("system", "System")].into_iter().map(|(key,label)| view! { <button type="button" class="profile-choice" data-appearance=key aria-pressed="false">{label}</button> }).collect_view()}</div><h3>{text.get("profiles.theme_picker.title", &[])?}</h3><p>{text.get("profiles.theme_picker.description", &[])?}</p><div class="profile-theme-grid">{[("default", "Command Centre"), ("serene-sage", "Serene Sage"), ("modern-clinical", "Modern Clinical"), ("warm-earth", "Warm Earth"), ("deep-lavender", "Deep Lavender"), ("forest-care", "Forest Care"), ("sunset-support", "Sunset Support"), ("tech-indigo", "Tech Indigo"), ("soft-rose", "Soft Rose"), ("minty-fresh", "Minty Fresh")].into_iter().map(|(key,label)| view! { <button type="button" class="profile-theme" data-theme=key aria-pressed="false"><span class="profile-theme-swatch"></span>{label}</button> }).collect_view()}</div></div>
        </dialog>
    }.to_html();
    Ok(profile_document(ProfileDocument {
        title: &title,
        household_name: &page.household_name,
        slug: &document_slug,
        locale: document_locale,
        csrf: &document_csrf,
        person_name: &document_person_name,
        vapid_public_key: document_vapid_key.as_deref(),
        mobile_shortcuts: &document_shortcuts,
        body,
    }))
}

struct ProfileDocument<'a> {
    title: &'a str,
    household_name: &'a str,
    slug: &'a str,
    locale: Locale,
    csrf: &'a str,
    person_name: &'a str,
    vapid_public_key: Option<&'a str>,
    mobile_shortcuts: &'a [String],
    body: String,
}

fn profile_document(page: ProfileDocument<'_>) -> String {
    let ProfileDocument {
        title,
        household_name,
        slug,
        locale,
        csrf,
        person_name,
        vapid_public_key,
        mobile_shortcuts,
        body,
    } = page;
    let text = Text::new(locale);
    let prefix = format!("/households/{}", path_segment(slug));
    let profile = format!("{prefix}/profile");
    let dashboard = format!("{prefix}/dashboard");
    let inventory = format!("{prefix}/medications");
    let locations = format!("{prefix}/locations");
    let people = format!("{prefix}/people");
    let notifications = format!("{prefix}/profile?section=notifications");
    let initials: String = person_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect();
    let rail_items: Vec<_> = mobile_shortcuts
        .iter()
        .filter_map(|key| {
            let (path, label, icon) = match key.as_str() {
                "dashboard" => (
                    "dashboard",
                    text.get("layouts.mobile_rail.home", &[]).ok()?,
                    profile_icon_svg("dashboard"),
                ),
                "inventory" => (
                    "medications",
                    text.get("layouts.mobile_rail.inventory", &[]).ok()?,
                    profile_icon_svg("inventory"),
                ),
                "locations" => (
                    "locations",
                    text.get("layouts.sidebar.locations", &[]).ok()?,
                    profile_icon_svg("locations"),
                ),
                "people" => (
                    "people",
                    text.get("layouts.sidebar.people", &[]).ok()?,
                    profile_icon_svg("people"),
                ),
                "profile" => (
                    "profile",
                    text.get("profiles.sections.profile.title", &[]).ok()?,
                    profile_icon_svg("profile"),
                ),
                _ => return None,
            };
            Some((format!("{prefix}/{path}"), label, icon, key == "profile"))
        })
        .collect();
    let shell = view! {
        <a class="profile-skip" href="#profile-content">{text.get("layouts.navigation.skip_to_content", &[]).unwrap_or_else(|_| "Skip to content".into())}</a>
        <div class="profile-shell">
            <aside class="profile-sidebar">
                <div class="profile-brand"><span aria-hidden="true">{"M"}</span><strong>"MedTracker"</strong></div>
                <a class="profile-search" href=format!("{dashboard}#dashboard-search-island")><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("search")/>"Search"<kbd>"Ctrl K"</kbd></a>
                <nav aria-label="Main navigation">
                    <a href=dashboard.clone()><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("dashboard")/><span>"Dashboard"</span></a>
                    <a href=inventory.clone()><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("inventory")/><span>"Inventory"</span></a>
                    <a href=locations><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("locations")/><span>"Locations"</span></a>
                    <a href=people><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("people")/><span>"People"</span></a>
                    <a href=notifications><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("notifications")/><span>{text.get("profiles.sections.notifications.title", &[]).unwrap_or_else(|_| "Notifications".into())}</span></a>
                </nav>
                <div class="profile-sidebar-bottom"><a class="profile-current" href=profile.clone()><span class="profile-sidebar-avatar" aria-hidden="true">{initials.clone()}</span><span><strong>{person_name.to_owned()}</strong><small>{household_name.to_owned()}</small></span></a><form method="post" action="/logout"><input type="hidden" name="authenticity_token" value=csrf.to_owned()/><button type="submit"><span class="profile-nav-icon" aria-hidden="true" inner_html=profile_icon_svg("logout")/><span>"Sign Out"</span></button></form></div>
            </aside>
            <header class="profile-mobile-topbar"><button type="button" aria-label="Open menu" data-profile-dialog="profile-mobile-menu"><span aria-hidden="true" inner_html=profile_icon_svg("menu")/></button><a href=dashboard.clone()><strong>"MedTracker"</strong></a><a href=format!("{dashboard}#dashboard-search-island") aria-label="Search dashboard"><span aria-hidden="true" inner_html=profile_icon_svg("search")/></a></header>
            <dialog id="profile-mobile-menu" class="profile-dialog profile-mobile-menu" aria-label="Navigation menu"><div class="profile-dialog-heading profile-overlay-heading"><h2>"MedTracker"</h2><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-mobile-menu">"×"</button></div><nav aria-label="Main navigation"><a href=dashboard.clone()>"Dashboard"</a><a href=inventory.clone()>"Inventory"</a><a href=format!("{prefix}/locations")>"Locations"</a><a href=format!("{prefix}/people")>"People"</a><a href=profile.clone()>"Profile"</a><a href=format!("{prefix}/profile?section=notifications")>"Notifications"</a></nav></dialog>
            <main id="profile-content" class="profile-main" inner_html=body></main>
            <nav class="profile-mobile-rail" aria-label="Primary navigation">{rail_items.into_iter().map(|(href,label,icon,current)| view! { <a href=href aria-current=current.then_some("page")><span class="profile-rail-icon" aria-hidden="true" inner_html=icon/><span>{label}</span></a> }).collect_view()}</nav>
        </div>
    }.to_html();
    let page_title = view! { <title>{format!("{title} | MedTracker")}</title> }.to_html();
    let vapid_meta = vapid_public_key
        .map(|key| view! { <meta name="vapid-public-key" content=key.to_owned()/> }.to_html())
        .unwrap_or_default();
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">{page_title}{vapid_meta}<link rel=\"stylesheet\" href=\"/medication.css\"><link rel=\"stylesheet\" href=\"/household.css\"><link rel=\"stylesheet\" href=\"/profile.css\"><script defer src=\"/profile.js\"></script><script defer src=\"/profile-notifications.js\"></script></head><body>{shell}</body></html>",
        locale.as_str()
    )
}

fn time_zone_options(current: &str) -> Vec<String> {
    let mut zones: Vec<String> = crate::rails_time_zones::labels()
        .map(str::to_owned)
        .collect();
    if !current.is_empty() && !zones.iter().any(|zone| zone == current) {
        zones.insert(0, current.to_owned());
    }
    zones
}

fn translated_errors(
    text: Text,
    errors: Option<&Vec<String>>,
) -> Result<Vec<String>, TranslationError> {
    errors
        .into_iter()
        .flatten()
        .map(|message| text.form_error(message))
        .collect()
}
