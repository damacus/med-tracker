use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;
use std::collections::HashMap;

pub struct SettingsPage {
    pub household_name: String,
    pub slug: String,
    pub locale: Locale,
    pub csrf: String,
    pub person_name: String,
    pub email: String,
    pub date_of_birth: Option<String>,
    pub age: Option<i64>,
    pub person_type: String,
    pub has_capacity: bool,
    pub active_section: String,
    pub security_html: String,
    pub notifications_html: String,
    pub advanced_html: String,
    pub time_zone: String,
    pub can_edit: bool,
    pub errors: HashMap<String, Vec<String>>,
    pub notice: String,
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
    let base = action.clone();
    let profile_active = page.active_section == "profile";
    let security_active = page.active_section == "security";
    let notifications_active = page.active_section == "notifications";
    let advanced_active = page.active_section == "advanced";
    let initials: String = page.person_name.split_whitespace().filter_map(|part| part.chars().next()).take(2).collect();
    let age = page.age.map(|value| value.to_string());
    let capacity = if page.has_capacity { "Yes" } else { "No" };
    let body = view! {
        <link rel="stylesheet" href="/profile.css"/>
        <div class="profile-page">
        <header class="profile-hero" data-testid="profile-hero">
            <div class="profile-identity"><span class="profile-avatar" aria-hidden="true">{initials}</span><div><p class="profile-eyebrow">{text.get("profiles.show.eyebrow", &[])?}</p><h1>{title.clone()}</h1><p class="profile-email">{page.email.clone()}</p></div></div>
            <p class="profile-description">{text.get("profiles.show.description", &[])?}</p>
        </header>
        <nav class="profile-tabs" aria-label=title.clone() role="tablist">
            <a id="profile-tab-profile" role="tab" aria-selected=profile_active.to_string() aria-controls="profile-profile-panel" href=base.clone()>{text.get("profiles.sections.profile.title", &[])?}</a>
            <a id="profile-tab-security" role="tab" aria-selected=security_active.to_string() aria-controls="profile-security-panel" href=format!("{base}?section=security")>{text.get("profiles.sections.security.title", &[])?}</a>
            <a id="profile-tab-notifications" role="tab" aria-selected=notifications_active.to_string() aria-controls="profile-notifications-panel" href=format!("{base}?section=notifications")>{text.get("profiles.sections.notifications.title", &[])?}</a>
            <a id="profile-tab-advanced" role="tab" aria-selected=advanced_active.to_string() aria-controls="profile-advanced-panel" href=format!("{base}?section=advanced")>{text.get("profiles.sections.advanced.title", &[])?}</a>
        </nav>
        {(!page.notice.is_empty()).then(|| view! { <p class="med-success" role="status">{page.notice}</p> })}
        <section id="profile-profile-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-profile" hidden=!profile_active>
            <div class="profile-section-header"><span class="profile-section-icon" aria-hidden="true">{"◉"}</span><div><h2>{text.get("profiles.sections.profile.title", &[])?}</h2><div class="profile-summary"><span>{page.person_name.clone()}</span><span>{page.email.clone()}</span><span>{text.get("profiles.appearance.modes.system", &[])?}</span></div></div></div>
            <div class="profile-grid">
                <section class="profile-info-card" data-testid="profile-personal-info-card"><h3>{text.get("profiles.show.personal_information.title", &[])?}</h3><p>{text.get("profiles.show.personal_information.description", &[])?}</p><dl>
                    <div><dt>Name</dt><dd>{page.person_name.clone()}</dd></div>
                    <div><dt>Email</dt><dd>{page.email.clone()}</dd></div>
                    <div><dt>Time Zone</dt><dd>{page.time_zone.clone()}</dd></div>
                    <div><dt>Date of Birth</dt><dd>{page.date_of_birth.clone().unwrap_or_else(|| "Not set".into())}</dd></div>
                    {age.map(|value| view! { <div><dt>Age</dt><dd>{value}</dd></div> })}
                    <div><dt>Person Type</dt><dd>{page.person_type}</dd></div>
                    <div><dt>Has Capacity</dt><dd>{capacity}</dd></div>
                </dl></section>
                <div class="profile-settings">
                    <button type="button" class="profile-setting-row" data-profile-dialog="profile-time-zone-modal"><span><strong>{zone_label.clone()}</strong><small>{hint.clone()}</small></span><span aria-hidden="true">{"›"}</span></button>
                </div>
            </div>
        </section>
        <section id="profile-security-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-security" hidden=!security_active inner_html=page.security_html></section>
        <section id="profile-notifications-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-notifications" hidden=!notifications_active inner_html=page.notifications_html></section>
        <section id="profile-advanced-panel" class="profile-section" role="tabpanel" aria-labelledby="profile-tab-advanced" hidden=!advanced_active inner_html=page.advanced_html></section>
        </div>
        <dialog id="profile-time-zone-modal" class="profile-dialog" data-testid="profile-time-zone-dialog" open=invalid_zone>
            <div class="profile-dialog-heading"><div><h2>{zone_label.clone()}</h2><p>{hint.clone()}</p></div><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-time-zone-modal">{"×"}</button></div>
        <form class="household-form" method="post" action=action>
            <input type="hidden" name="authenticity_token" value=page.csrf/>
            {(!other_errors.is_empty()).then(|| view! {
                <ul class="med-alert" role="alert">{other_errors.into_iter().map(|error| view! {<li>{error}</li>}).collect_view()}</ul>
            })}
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
            {page.can_edit.then(|| view! { <div class="household-actions"><button type="button" class="profile-secondary" data-profile-close="profile-time-zone-modal">{"Close"}</button><button class="med-primary" type="submit">{save}</button></div> })}
        </form>
        </dialog>
        <script src="/profile.js" defer></script>
    }.to_html();
    Ok(household_document(
        &title,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
    ))
}

fn time_zone_options(current: &str) -> Vec<String> {
    let mut zones: Vec<String> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|zone| zone.name().to_owned())
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
