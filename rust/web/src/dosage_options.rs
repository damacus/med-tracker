use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use crate::medication_management::{
    field_error, input_field, messages, select_field, textarea_field,
};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct DosageDraft {
    pub amount: String,
    pub unit: String,
    pub frequency: String,
    pub description: String,
    pub default_for_adults: bool,
    pub default_for_children: bool,
    pub default_max_daily_doses: String,
    pub default_min_hours_between_doses: String,
    pub default_dose_cycle: String,
    pub current_supply: String,
    pub reorder_threshold: String,
    pub etag: String,
    pub confirm_option_mode: bool,
}

pub struct DosageFormPage {
    pub household_name: String,
    pub slug: String,
    pub medication_id: String,
    pub medication_name: String,
    pub csrf: String,
    pub locale: Locale,
    pub option_id: Option<String>,
    pub draft: DosageDraft,
    pub first_option: bool,
    pub errors: BTreeMap<String, Vec<String>>,
}

pub struct DosageListPage {
    pub household_name: String,
    pub slug: String,
    pub medication_id: String,
    pub medication_name: String,
    pub locale: Locale,
    pub options: Vec<DosageOption>,
    pub can_manage: bool,
    pub notice: Option<String>,
}

pub struct DosageOption {
    pub id: i64,
    pub amount: String,
    pub unit: String,
    pub frequency: String,
    pub description: String,
    pub current_supply: Option<String>,
    pub reorder_threshold: Option<String>,
    pub default_for_adults: bool,
    pub default_for_children: bool,
}

fn checkbox(name: &str, label: String, checked: bool, text: Text, errors: &[String]) -> String {
    let id = format!("medication-{name}");
    let described = (!errors.is_empty()).then(|| format!("medication-{name}-error"));
    let error = field_error(name, text, errors);
    view! {
        <div class="household-field">
            <label for=id.clone()>{label}</label>
            <input id=id type="checkbox" name=name.to_owned() value="true" checked=checked
                aria-invalid=if errors.is_empty() { "false" } else { "true" } aria-describedby=described/>
            <div inner_html=error></div>
        </div>
    }.to_html()
}

pub fn render_dosage_form(page: DosageFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let editing = page.option_id.is_some();
    let title = text.get(
        if editing {
            "dosages.form.update"
        } else {
            "dosages.form.add"
        },
        &[],
    )?;
    let base = format!(
        "/households/{}/medications/{}/dosage_options",
        path_segment(&page.slug),
        path_segment(&page.medication_id)
    );
    let action = page
        .option_id
        .as_ref()
        .map_or_else(|| base.clone(), |id| format!("{base}/{}", path_segment(id)));
    let errors_for = |name: &str| page.errors.get(name).map(Vec::as_slice).unwrap_or(&[]);
    let mut fields = String::new();
    for (name, key, value, kind, required) in [
        (
            "amount",
            "dosages.form.amount",
            &page.draft.amount,
            "number",
            true,
        ),
        ("unit", "dosages.form.unit", &page.draft.unit, "text", true),
        (
            "frequency",
            "dosages.form.frequency_label",
            &page.draft.frequency,
            "text",
            true,
        ),
        (
            "default_max_daily_doses",
            "dosages.form.max_doses_per_cycle",
            &page.draft.default_max_daily_doses,
            "number",
            true,
        ),
        (
            "default_min_hours_between_doses",
            "dosages.form.min_hours_apart",
            &page.draft.default_min_hours_between_doses,
            "number",
            true,
        ),
        (
            "current_supply",
            "forms.medications.current_supply",
            &page.draft.current_supply,
            "number",
            false,
        ),
        (
            "reorder_threshold",
            "forms.medications.reorder_threshold",
            &page.draft.reorder_threshold,
            "number",
            false,
        ),
    ] {
        fields.push_str(&input_field(
            name,
            text.get(key, &[])?,
            value,
            kind,
            required,
            text,
            errors_for(name),
        ));
    }
    fields.push_str(&textarea_field(
        "description",
        text.get("dosages.form.description", &[])?,
        &page.draft.description,
        text,
        errors_for("description"),
    ));
    fields.push_str(&select_field(
        "default_dose_cycle",
        text.get("dosages.form.dose_cycle", &[])?,
        &page.draft.default_dose_cycle,
        ["daily", "weekly", "monthly"]
            .into_iter()
            .map(|cycle| {
                text.get(&format!("dosages.management.{cycle}"), &[])
                    .map(|label| (cycle.to_owned(), label))
            })
            .collect::<Result<_, _>>()?,
        text.get("schedules.form.select_cycle", &[])?,
        text,
        errors_for("default_dose_cycle"),
    ));
    for (name, checked) in [
        ("default_for_adults", page.draft.default_for_adults),
        ("default_for_children", page.draft.default_for_children),
    ] {
        fields.push_str(&checkbox(
            name,
            text.get(&format!("dosages.form.{name}"), &[])?,
            checked,
            text,
            errors_for(name),
        ));
    }
    let stock_hint = text.get("dosages.management.stock_hint", &[])?;
    fields.push_str(&view! { <p class="household-note">{stock_hint}</p> }.to_html());
    if page.first_option {
        fields.push_str(&checkbox(
            "confirm_option_mode",
            text.get("dosages.management.confirm_mode", &[])?,
            page.draft.confirm_option_mode,
            text,
            errors_for("confirm_option_mode"),
        ));
    }
    if !editing {
        let retry_hint = text.get("dosages.management.retry_hint", &[])?;
        fields.push_str(&view! { <p class="household-note">{retry_hint}</p> }.to_html());
    }
    let error_summary = if page.errors.is_empty() {
        String::new()
    } else {
        let heading = text.plural(
            "forms.medications.validation_errors",
            page.errors.values().map(Vec::len).sum::<usize>() as u64,
            &[],
        )?;
        view! {
            <section id="dosage-errors" class="household-errors" role="alert" aria-labelledby="dosage-errors-heading">
                <h2 id="dosage-errors-heading">{heading}</h2>
                <ul>{page.errors.iter().map(|(name, errors)| {
                    let message = messages(text, errors);
                    if matches!(name.as_str(), "dosage_option" | "base") {
                        view! { <li>{message}</li> }.into_any()
                    } else {
                        view! { <li><a href=format!("#medication-{name}")>{message}</a></li> }.into_any()
                    }
                }).collect_view()}</ul>
            </section>
        }.to_html()
    };
    let described = (!page.errors.is_empty()).then_some("dosage-errors");
    let cancel = text.get("dosages.form.cancel", &[])?;
    let body = view! {
        <section class="household-content">
            <h1>{title.clone()}</h1><p>{page.medication_name}</p>
            <div inner_html=error_summary></div>
            <form class="household-form" action=action method="post" aria-describedby=described>
                <input type="hidden" name="authenticity_token" value=page.csrf/>
                <input type="hidden" name="etag" value=page.draft.etag/>
                <div class="household-fields" inner_html=fields></div>
                <div class="household-actions"><a href=base>{cancel}</a><button class="med-primary" type="submit">{title.clone()}</button></div>
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

pub fn render_dosage_list(page: DosageListPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get("medications.show.dosages_heading", &[])?;
    let add = text.get("dosages.form.add", &[])?;
    let edit = text.get("medications.show.edit_dosage", &[])?;
    let back = text.get("forms.medications.back", &[])?;
    let untracked = text.get("dosages.management.untracked", &[])?;
    let supply_label = text.get("forms.medications.current_supply", &[])?;
    let threshold_label = text.get("forms.medications.reorder_threshold", &[])?;
    let adults = text.get("dosages.form.default_for_adults", &[])?;
    let children = text.get("dosages.form.default_for_children", &[])?;
    let empty = text.get("medications.show.no_dosages", &[])?;
    let base = format!(
        "/households/{}/medications/{}",
        path_segment(&page.slug),
        path_segment(&page.medication_id)
    );
    let options_path = format!("{base}/dosage_options");
    let body = view! {
        <section class="household-content">
            <h1>{title.clone()}</h1><p>{page.medication_name}</p>
            {page.notice.map(|notice| view! { <p role="status">{notice}</p> })}
            <div class="household-actions"><a href=base>{back}</a>{page.can_manage.then(|| view! { <a class="med-primary" href=format!("{options_path}/new")>{add}</a> })}</div>
            {page.options.is_empty().then(|| view! { <p>{empty}</p> })}
            <ul class="household-list">{page.options.into_iter().map(|option| {
                let id = option.id;
                let title = format!("{} {}", option.amount, option.unit);
                let supply = option.current_supply.unwrap_or_else(|| untracked.clone());
                let threshold = option.reorder_threshold.unwrap_or_else(|| untracked.clone());
                view! {
                    <li class="household-card"><h2>{title}</h2><p>{option.frequency}</p><p>{option.description}</p>
                        <p>{supply_label.clone()}": "{supply}</p><p>{threshold_label.clone()}": "{threshold}</p>
                        {option.default_for_adults.then(|| view! { <p>{adults.clone()}</p> })}
                        {option.default_for_children.then(|| view! { <p>{children.clone()}</p> })}
                        {page.can_manage.then(|| view! { <a href=format!("{options_path}/{id}/edit")>{edit.clone()}</a> })}
                    </li>
                }
            }).collect_view()}</ul>
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
