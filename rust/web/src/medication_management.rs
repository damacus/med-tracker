use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct MedicationDraft {
    pub name: String,
    pub friendly_name: String,
    pub description: String,
    pub barcode: String,
    pub dose_amount: String,
    pub dose_unit: String,
    pub current_supply: String,
    pub reorder_threshold: String,
    pub location_id: String,
    pub warnings: String,
    pub etag: String,
}

pub struct MedicationFormPage {
    pub household_name: String,
    pub slug: String,
    pub csrf: String,
    pub locale: Locale,
    pub medication_id: Option<String>,
    pub draft: MedicationDraft,
    pub locations: Vec<(String, String)>,
    pub errors: BTreeMap<String, Vec<String>>,
}

fn messages(text: Text, errors: &[String]) -> String {
    errors
        .iter()
        .map(|message| text.api_error(message).unwrap_or_else(|| message.clone()))
        .collect::<Vec<_>>()
        .join(". ")
}

fn field_error(name: &str, text: Text, errors: &[String]) -> String {
    if errors.is_empty() {
        return String::new();
    }
    let id = format!("medication-{name}-error");
    let message = messages(text, errors);
    view! { <p class="household-field-error" id=id>{message}</p> }.to_html()
}

fn input_field(
    name: &str,
    label: String,
    value: &str,
    kind: &str,
    required: bool,
    text: Text,
    errors: &[String],
) -> String {
    let id = format!("medication-{name}");
    let described = (!errors.is_empty()).then(|| format!("medication-{name}-error"));
    let decimal = kind == "number";
    let body = view! {
        <div class="household-field">
            <label for=id.clone()>{label}</label>
            <input id=id name=name.to_owned() type=kind.to_owned() value=value.to_owned()
                required=required step=decimal.then_some("any") min=decimal.then_some("0")
                inputmode=decimal.then_some("decimal")
                aria-invalid=if errors.is_empty() { "false" } else { "true" }
                aria-describedby=described/>
        </div>
    }
    .to_html();
    body.replacen(
        "</div>",
        &format!("{}</div>", field_error(name, text, errors)),
        1,
    )
}

fn textarea_field(name: &str, label: String, value: &str, text: Text, errors: &[String]) -> String {
    let id = format!("medication-{name}");
    let described = (!errors.is_empty()).then(|| format!("medication-{name}-error"));
    let error = field_error(name, text, errors);
    let value = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    view! {
        <div class="household-field">
            <label for=id.clone()>{label}</label>
            <textarea id=id name=name.to_owned() rows="3"
                aria-invalid=if errors.is_empty() { "false" } else { "true" }
                aria-describedby=described inner_html=value></textarea>
            <div inner_html=error></div>
        </div>
    }
    .to_html()
}

fn select_field(
    name: &str,
    label: String,
    selected: &str,
    options: Vec<(String, String)>,
    placeholder: String,
    text: Text,
    errors: &[String],
) -> String {
    let id = format!("medication-{name}");
    let described = (!errors.is_empty()).then(|| format!("medication-{name}-error"));
    let selected = selected.to_owned();
    let error = field_error(name, text, errors);
    view! {
        <div class="household-field">
            <label for=id.clone()>{label}</label>
            <select id=id name=name.to_owned() required
                aria-invalid=if errors.is_empty() { "false" } else { "true" }
                aria-describedby=described>
                <option value="" selected=selected.is_empty()>{placeholder}</option>
                {options.into_iter().map(|(value, label)| {
                    let is_selected = value == selected;
                    view! { <option value=value selected=is_selected>{label}</option> }
                }).collect_view()}
            </select>
            <div inner_html=error></div>
        </div>
    }
    .to_html()
}

pub fn render_medication_form(page: MedicationFormPage) -> Result<String, TranslationError> {
    render_medication_form_with_options(page, false)
}

pub fn render_medication_form_with_options(
    page: MedicationFormPage,
    options_mode: bool,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let editing = page.medication_id.is_some();
    let title = text.get(
        if editing {
            "medications.form.edit_title"
        } else {
            "medications.form.new_title"
        },
        &[],
    )?;
    let inventory = format!("/households/{}/medications", path_segment(&page.slug));
    let action = page.medication_id.as_ref().map_or_else(
        || inventory.clone(),
        |id| format!("{inventory}/{}", path_segment(id)),
    );
    let errors_for = |name: &str| page.errors.get(name).map(Vec::as_slice).unwrap_or(&[]);
    let mut fields = select_field(
        "location_id",
        text.get("medications.show.location", &[])?,
        &page.draft.location_id,
        page.locations,
        text.get("forms.medications.select_location", &[])?,
        text,
        errors_for("location_id"),
    );
    for (name, key, value, kind, required) in [
        ("name", "name", &page.draft.name, "text", true),
        (
            "friendly_name",
            "friendly_name",
            &page.draft.friendly_name,
            "text",
            false,
        ),
        (
            "dose_amount",
            "standard_dosage",
            &page.draft.dose_amount,
            "number",
            false,
        ),
        (
            "current_supply",
            if editing {
                "current_supply"
            } else {
                "starting_supply"
            },
            &page.draft.current_supply,
            "number",
            false,
        ),
        (
            "reorder_threshold",
            "reorder_threshold",
            &page.draft.reorder_threshold,
            "number",
            true,
        ),
    ] {
        if options_mode && matches!(name, "dose_amount" | "current_supply") {
            continue;
        }
        fields.push_str(&input_field(
            name,
            text.get(&format!("forms.medications.{key}"), &[])?,
            value,
            kind,
            required,
            text,
            errors_for(name),
        ));
    }
    fields.push_str(&input_field(
        "barcode",
        text.get("medications.finder.barcode", &[])?,
        &page.draft.barcode,
        "text",
        false,
        text,
        errors_for("barcode"),
    ));
    if options_mode {
        let explanation = text.get("forms.medications.dosage_options_read_only", &[])?;
        fields.push_str(&view! { <p class="household-note">{explanation}</p> }.to_html());
    } else {
        fields.push_str(&select_field(
            "dose_unit",
            text.get("forms.medications.unit", &[])?,
            &page.draft.dose_unit,
            [
                "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop",
                "sachet", "pad",
            ]
            .into_iter()
            .map(|unit| (unit.to_owned(), unit.to_owned()))
            .collect(),
            text.get("forms.medications.select_unit", &[])?,
            text,
            errors_for("dose_unit"),
        ));
    }
    for (name, value) in [
        ("description", &page.draft.description),
        ("warnings", &page.draft.warnings),
    ] {
        fields.push_str(&textarea_field(
            name,
            text.get(&format!("forms.medications.{name}"), &[])?,
            value,
            text,
            errors_for(name),
        ));
    }
    let count = page.errors.values().map(Vec::len).sum::<usize>();
    let error_summary = if count == 0 {
        String::new()
    } else {
        let heading = text.plural("forms.medications.validation_errors", count as u64, &[])?;
        view! {
            <section class="household-errors" role="alert" aria-labelledby="medication-errors-heading">
                <h2 id="medication-errors-heading">{heading}</h2>
                <ul>{page.errors.iter().map(|(name, errors)| {
                    let message = messages(text, errors);
                    if matches!(name.as_str(), "medication" | "base") {
                        view! { <li>{message}</li> }.into_any()
                    } else {
                        let target = format!("#medication-{name}");
                        view! { <li><a href=target>{message}</a></li> }.into_any()
                    }
                }).collect_view()}</ul>
            </section>
        }.to_html()
    };
    let save = text.get("forms.medications.save_medication", &[])?;
    let back = text.get("forms.medications.back", &[])?;
    let body = view! {
        <section class="household-content">
            <h1>{title.clone()}</h1>
            <div inner_html=error_summary></div>
            <form class="household-form" action=action method="post">
                <input type="hidden" name="authenticity_token" value=page.csrf/>
                <input type="hidden" name="etag" value=page.draft.etag/>
                <div class="household-fields" inner_html=fields></div>
                <div class="household-actions"><a href=inventory>{back}</a><button type="submit">{save}</button></div>
            </form>
        </section>
    }.to_html();
    Ok(household_document(
        &title,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
    ))
}
