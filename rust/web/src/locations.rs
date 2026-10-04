use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct LocationRow {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Default)]
pub struct LocationDraft {
    pub name: String,
    pub description: String,
    pub etag: String,
    pub idempotency_key: String,
}

pub struct LocationsPage {
    pub household_name: String,
    pub slug: String,
    pub locale: Locale,
    pub can_create: bool,
    pub locations: Vec<LocationRow>,
    pub notifications_visible: bool,
}

pub struct LocationFormPage {
    pub household_name: String,
    pub slug: String,
    pub locale: Locale,
    pub csrf: String,
    pub action: String,
    pub title: String,
    pub draft: LocationDraft,
    pub errors: HashMap<String, Vec<String>>,
    pub notifications_visible: bool,
}

pub struct LocationMedication {
    pub id: i64,
    pub name: String,
    pub supply: String,
    pub unit: String,
}

pub struct LocationDetailPage {
    pub household_name: String,
    pub slug: String,
    pub locale: Locale,
    pub location: LocationRow,
    pub can_update: bool,
    pub medications: Vec<LocationMedication>,
    pub notice: String,
    pub notifications_visible: bool,
}

pub fn render_location_list(page: LocationsPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get("locations.index.title", &[])?;
    let add = text.get("locations.index.add_location", &[])?;
    let prefix = format!("/households/{}/locations", path_segment(&page.slug));
    let new_path = format!("{prefix}/new");
    let body = view! {
        <header class="med-heading"><h1>{title.clone()}</h1>
            {page.can_create.then(|| view! { <a class="med-primary" href=new_path>{add}</a> })}
        </header>
        <ul class="household-list">
            {page.locations.into_iter().map(|location| view! {
                <li class="med-panel">
                    <h2><a href=format!("{prefix}/{}", location.id)>{location.name}</a></h2>
                    <p>{location.description}</p>
                </li>
            }).collect_view()}
        </ul>
    }
    .to_html();
    Ok(household_document(
        &title,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
        page.notifications_visible,
    ))
}

pub fn render_location_form(page: LocationFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let name_label = text.get("forms.medications.name", &[])?;
    let description_label = text.get("locations.form.description_optional", &[])?;
    let save = text.get("locations.form.save", &[])?;
    let back = text.get("locations.show.all_locations", &[])?;
    let name_placeholder = text.get("forms.locations.name_placeholder", &[])?;
    let description_placeholder = text.get("forms.locations.description_placeholder", &[])?;
    let name_errors = translated_errors(text, page.errors.get("name"))?;
    let description_errors = translated_errors(text, page.errors.get("description"))?;
    let mut other_errors: Vec<_> = page
        .errors
        .iter()
        .filter(|(field, _)| field.as_str() != "name" && field.as_str() != "description")
        .collect();
    other_errors.sort_by_key(|(field, _)| *field);
    let other_errors: Vec<_> = other_errors
        .into_iter()
        .flat_map(|(_, errors)| errors)
        .map(|message| text.form_error(message))
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let invalid_name = !name_errors.is_empty();
    let invalid_description = !description_errors.is_empty();
    let description = page
        .draft
        .description
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let prefix = format!("/households/{}/locations", path_segment(&page.slug));
    let body = view! {
        <h1>{page.title.clone()}</h1>
        <form class="household-form med-panel" method="post" action=page.action>
            <input type="hidden" name="authenticity_token" value=page.csrf/>
            <input type="hidden" name="etag" value=page.draft.etag/>
            <input type="hidden" name="idempotency_key" value=page.draft.idempotency_key/>
            {(!other_errors.is_empty()).then(|| view! {
                <ul class="med-alert" role="alert">{other_errors.into_iter().map(|error| view! {<li>{error}</li>}).collect_view()}</ul>
            })}
            <div class="form-field">
                <label for="location-name">{name_label}</label>
                <input id="location-name" name="name" type="text" value=page.draft.name placeholder=name_placeholder
                    required aria-invalid=invalid_name.to_string() aria-describedby=invalid_name.then_some("location-name-errors")/>
                {invalid_name.then(|| view! {
                    <ul id="location-name-errors" class="field-errors" role="alert">
                        {name_errors.into_iter().map(|error| view! { <li>{error}</li> }).collect_view()}
                    </ul>
                })}
            </div>
            <div class="form-field">
                <label for="location-description">{description_label}</label>
                <textarea id="location-description" name="description" rows="4" placeholder=description_placeholder
                    aria-invalid=invalid_description.to_string() aria-describedby=invalid_description.then_some("location-description-errors") inner_html=description></textarea>
                {invalid_description.then(|| view! {
                    <ul id="location-description-errors" class="field-errors" role="alert">
                        {description_errors.into_iter().map(|error| view! { <li>{error}</li> }).collect_view()}
                    </ul>
                })}
            </div>
            <div class="household-actions"><button class="med-primary" type="submit">{save}</button><a href=prefix>{back}</a></div>
        </form>
    }.to_html();
    Ok(household_document(
        &page.title,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
        page.notifications_visible,
    ))
}

pub fn render_location_detail(page: LocationDetailPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let edit = text.get("locations.show.edit_location", &[])?;
    let back = text.get("locations.show.all_locations", &[])?;
    let details = text.get("locations.show.details", &[])?;
    let no_details = text.get("locations.show.no_details", &[])?;
    let medications_heading = text.get("locations.show.medications_heading", &[])?;
    let no_medications = text.get("locations.show.no_medications", &[])?;
    let prefix = format!("/households/{}", path_segment(&page.slug));
    let edit_path = format!("{prefix}/locations/{}/edit", page.location.id);
    let empty = page.medications.is_empty();
    let description = if page.location.description.is_empty() {
        no_details
    } else {
        page.location.description
    };
    let body = view! {
        <header class="med-heading"><h1>{page.location.name.clone()}</h1>
            {page.can_update.then(|| view! { <a class="med-primary" href=edit_path>{edit}</a> })}
        </header>
        {(!page.notice.is_empty()).then(|| view! { <p class="med-success" role="status">{page.notice}</p> })}
        <section class="med-panel"><h2>{details}</h2><p>{description}</p></section>
        <section class="med-panel"><h2>{medications_heading}</h2>
            {empty.then(|| view! { <p>{no_medications}</p> })}
            <ul class="household-list">{page.medications.into_iter().map(|medication| view! {
                <li><a href=format!("{prefix}/medications/{}", medication.id)>{medication.name}</a>
                    <span>{format!(" {} {}", medication.supply, medication.unit)}</span></li>
            }).collect_view()}</ul>
        </section>
        <a href=format!("/households/{}/locations", path_segment(&page.slug))>{back}</a>
    }.to_html();
    Ok(household_document(
        &page.location.name,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
        page.notifications_visible,
    ))
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
