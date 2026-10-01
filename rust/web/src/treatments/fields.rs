use super::*;
use crate::household::{household_document, path_segment};
use crate::household_i18n::{Text, TranslationError};
use crate::medication_management::{
    field_error, input_field, messages, select_field, textarea_field,
};
use leptos::prelude::*;

pub(super) fn error_slice<'a>(page: &'a TreatmentFormPage, name: &str) -> &'a [String] {
    page.errors.get(name).map(Vec::as_slice).unwrap_or_default()
}

pub(super) fn input(
    page: &TreatmentFormPage,
    name: &str,
    key: &str,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    Ok(input_field(
        name,
        text.get(key, &[])?,
        page.draft.value(name),
        "text",
        name == "end_date",
        text,
        error_slice(page, name),
    ))
}

pub(super) fn select(
    page: &TreatmentFormPage,
    name: &str,
    key: &str,
    mut options: Vec<(String, String)>,
    placeholder: &str,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let selected = page.draft.value(name);
    if !selected.is_empty() && !options.iter().any(|(value, _)| value == selected) {
        options.push((
            selected.to_owned(),
            text.get("treatments.form.unavailable_selection", &[])?,
        ));
    }
    Ok(select_field(
        name,
        text.get(key, &[])?,
        page.draft.value(name),
        options,
        text.get(placeholder, &[])?,
        text,
        error_slice(page, name),
    ))
}

fn input_hint(
    page: &TreatmentFormPage,
    name: &str,
    key: &str,
    hint_key: &str,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let label = text.get(key, &[])?;
    let hint = text.get(hint_key, &[])?;
    let id = format!("medication-{name}");
    let hint_id = format!("{id}-hint");
    let errors = error_slice(page, name);
    let described = if errors.is_empty() {
        hint_id.clone()
    } else {
        format!("{hint_id} {id}-error")
    };
    let error = field_error(name, text, errors);
    Ok(view! { <div class="household-field"><label for=id.clone()>{label}</label>
        <input id=id name=name.to_owned() type="text" inputmode="numeric" value=page.draft.value(name).to_owned() aria-invalid=if errors.is_empty() { "false" } else { "true" } aria-describedby=described/>
        <p class="household-note" id=hint_id>{hint}</p><div inner_html=error></div>
    </div> }.to_html())
}

pub(super) fn common(page: &TreatmentFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let mut html = select(
        page,
        "medication_id",
        "person_medications.form.medication",
        page.medications.clone(),
        "person_medications.form.select_medication",
    )?;
    html.push_str(&select(
        page,
        "source_dosage_option_id",
        "person_medications.form.dose",
        page.dosages.clone(),
        "treatments.form.default_dose",
    )?);
    let apply_dose = text.get("treatments.form.apply_dose", &[])?;
    html.push_str(&view! {
        <button type="submit" name="intent" value="apply_dose" class="med-button">{apply_dose}</button>
    }.to_html());
    html.push_str(&input(
        page,
        "dose_amount",
        "forms.medications.standard_dosage",
    )?);
    html.push_str(&select(
        page,
        "dose_unit",
        "forms.medications.unit",
        page.units.clone(),
        "forms.medications.select_unit",
    )?);
    let maximum_hint = if page.editing {
        "treatments.form.keep_maximum"
    } else if page.schedule {
        "treatments.form.schedule_maximum"
    } else {
        "treatments.form.optional_maximum"
    };
    html.push_str(&input_hint(
        page,
        "max_daily_doses",
        "person_medications.form.max_doses_per_cycle",
        maximum_hint,
    )?);
    html.push_str(&input(
        page,
        "min_hours_between_doses",
        "person_medications.form.min_hours_apart",
    )?);
    let cycles = ["daily", "weekly", "monthly"]
        .into_iter()
        .map(|cycle| {
            text.get(&format!("dosages.management.{cycle}"), &[])
                .map(|label| (cycle.to_owned(), label))
        })
        .collect::<Result<Vec<_>, _>>()?;
    html.push_str(&select(
        page,
        "dose_cycle",
        "person_medications.form.dose_cycle",
        cycles,
        if page.editing {
            "treatments.form.keep_cycle"
        } else {
            "schedules.form.select_cycle"
        },
    )?);
    Ok(html)
}

pub(super) fn notes(page: &TreatmentFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    Ok(textarea_field(
        "notes",
        text.get("person_medications.form.notes", &[])?,
        page.draft.value("notes"),
        text,
        error_slice(page, "notes"),
    ))
}

pub(super) fn document(
    page: TreatmentFormPage,
    title: String,
    submit: String,
    fields: String,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let cancel = text.get("person_medications.form.cancel", &[])?;
    let back = format!(
        "/households/{}/people/{}",
        path_segment(&page.slug),
        path_segment(&page.person_id)
    );
    let summary = if page.errors.is_empty() {
        String::new()
    } else {
        let heading = text.plural(
            "person_medications.form.validation_errors",
            page.errors.values().map(Vec::len).sum::<usize>() as u64,
            &[],
        )?;
        view! {
            <section class="household-errors" role="alert" id="treatment-errors" aria-labelledby="treatment-errors-heading">
                <h2 id="treatment-errors-heading">{heading}</h2>
                <ul>{page.errors.iter().map(|(name, errors)| {
                    let message = messages(text, errors);
                    if matches!(name.as_str(), "base" | "person_medication" | "schedule" | "schedule_config" | "medication_pause_period") {
                        view! { <li>{message}</li> }.into_any()
                    } else {
                        view! { <li><a href=format!("#medication-{name}")>{message}</a></li> }.into_any()
                    }
                }).collect_view()}</ul>
            </section>
        }.to_html()
    };
    let body = view! {
        <section class="household-content">
            <h1>{title.clone()}</h1><p>{page.person_name}</p>
            <div inner_html=summary></div>
            <form class="household-form" action=page.action method="post" novalidate aria-describedby=(!page.errors.is_empty()).then_some("treatment-errors")>
                <input type="hidden" name="authenticity_token" value=page.csrf/>
                <input type="hidden" name="etag" value=page.draft.value("etag").to_owned()/>
                <input type="hidden" name="source_type" value=page.draft.value("source_type").to_owned()/>
                <input type="hidden" name="source_id" value=page.draft.value("source_id").to_owned()/>
                <input type="hidden" name="period_etag" value=page.draft.value("period_etag").to_owned()/>
                <input type="hidden" name="pause_period_id" value=page.draft.value("pause_period_id").to_owned()/>
                <input type="hidden" name="submission_id" value=page.draft.value("submission_id").to_owned()/>
                <input type="hidden" name="config_original" value=page.draft.value("config_original").to_owned()/>
                <input type="hidden" name="config_type_original" value=page.draft.value("config_type_original").to_owned()/>
                <div class="household-fields" inner_html=fields></div>
                <div class="household-actions"><a href=back>{cancel}</a><button class="med-primary" type="submit">{submit}</button></div>
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
