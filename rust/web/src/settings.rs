use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;
use std::collections::HashMap;

pub struct SettingsPage {
    pub household_name: String,
    pub slug: String,
    pub locale: Locale,
    pub csrf: String,
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
    let action = format!("/households/{}/settings", path_segment(&page.slug));
    let body = view! {
        <h1>{title.clone()}</h1>
        {(!page.notice.is_empty()).then(|| view! { <p class="med-success" role="status">{page.notice}</p> })}
        <form class="household-form med-panel" method="post" action=action>
            <input type="hidden" name="authenticity_token" value=page.csrf/>
            {(!other_errors.is_empty()).then(|| view! {
                <ul class="med-alert" role="alert">{other_errors.into_iter().map(|error| view! {<li>{error}</li>}).collect_view()}</ul>
            })}
            <div class="form-field">
                <label for="settings_time_zone">{zone_label}</label>
                <p id="settings-time-zone-hint">{hint}</p>
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
            {page.can_edit.then(|| view! { <div class="household-actions"><button class="med-primary" type="submit">{save}</button></div> })}
        </form>
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
