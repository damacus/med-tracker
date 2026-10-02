use super::*;
use crate::household_i18n::{Text, TranslationError};
use crate::medication_management::field_error;
use leptos::prelude::*;

fn button(page: &TreatmentFormPage, intent: &str, key: &str) -> Result<String, TranslationError> {
    let label = Text::new(page.locale).get(key, &[])?;
    Ok(view! { <button type="submit" name="intent" value=intent.to_owned() class="med-button">{label}</button> }.to_html())
}

fn times(
    page: &TreatmentFormPage,
    prefix: &str,
    initial: usize,
) -> Result<String, TranslationError> {
    let mut html = String::new();
    for index in page.draft.indices(&format!("{prefix}time_"), initial) {
        html.push_str(&fields::input(
            page,
            &format!("{prefix}time_{index}"),
            if prefix.is_empty() {
                "treatments.form.time"
            } else {
                "treatments.form.step_time_instruction"
            },
        )?);
        html.push_str(&button(
            page,
            &format!("remove_{prefix}time_{index}"),
            "treatments.form.remove_time",
        )?);
    }
    html.push_str(&button(
        page,
        &format!("add_{prefix}time"),
        "treatments.form.add_time",
    )?);
    Ok(html)
}

fn weekdays(page: &TreatmentFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let mut html = String::new();
    for day in [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ] {
        let name = format!("weekday_{day}");
        let id = format!("medication-{name}");
        let label = text.get(&format!("treatments.weekdays.{day}"), &[])?;
        let errors = fields::error_slice(page, &name);
        let error = field_error(&name, text, errors);
        html.push_str(&view! {
            <div class="household-field">
                <label for=id.clone()>{label}</label>
                <input id=id name=name type="checkbox" value="true" checked=page.draft.value(&format!("weekday_{day}")) == "true"
                    aria-invalid=if errors.is_empty() { "false" } else { "true" }
                    aria-describedby=(!errors.is_empty()).then(|| format!("medication-weekday_{day}-error"))/>
                <div inner_html=error></div>
            </div>
        }.to_html());
    }
    Ok(html)
}

fn dates(page: &TreatmentFormPage) -> Result<String, TranslationError> {
    let mut html = String::new();
    for index in page.draft.indices("date_", 2) {
        html.push_str(&fields::input(
            page,
            &format!("date_{index}"),
            "treatments.form.date",
        )?);
        html.push_str(&button(
            page,
            &format!("remove_date_{index}"),
            "treatments.form.remove_date",
        )?);
    }
    html.push_str(&button(page, "add_date", "treatments.form.add_date")?);
    Ok(html)
}

fn taper(page: &TreatmentFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let mut html = String::new();
    for index in page.draft.indices("step_", 2) {
        let label = text.get(
            "treatments.form.step",
            &[("number", &(index + 1).to_string())],
        )?;
        let mut step = String::new();
        for (name, key) in [
            ("start_date", "schedules.form.start_date"),
            ("end_date", "schedules.form.end_date"),
            ("dose_amount", "forms.medications.standard_dosage"),
            (
                "max_daily_doses",
                "person_medications.form.max_doses_per_cycle",
            ),
            (
                "min_hours_between_doses",
                "person_medications.form.min_hours_apart",
            ),
        ] {
            step.push_str(&fields::input(page, &format!("step_{index}_{name}"), key)?);
        }
        step.push_str(&fields::select(
            page,
            &format!("step_{index}_dose_unit"),
            "forms.medications.unit",
            page.units.clone(),
            "forms.medications.select_unit",
        )?);
        let instruction_hint = text.get("treatments.form.step_time_hint", &[])?;
        step.push_str(&view! { <p class="household-note">{instruction_hint}</p> }.to_html());
        step.push_str(&times(page, &format!("step_{index}_"), 1)?);
        step.push_str(&button(
            page,
            &format!("remove_step_{index}"),
            "treatments.form.remove_step",
        )?);
        html.push_str(&view! { <fieldset><legend>{label}</legend><div class="household-fields" inner_html=step></div></fieldset> }.to_html());
    }
    html.push_str(&button(page, "add_step", "treatments.form.add_step")?);
    Ok(html)
}

pub(super) fn render(page: &TreatmentFormPage) -> Result<String, TranslationError> {
    let mut html = String::new();
    match page.draft.value("schedule_type") {
        "prn" => {
            let hint = Text::new(page.locale).get("treatments.form.as_needed_hint", &[])?;
            html.push_str(&view! { <p class="household-note">{hint}</p> }.to_html());
        }
        "tapering" => {
            let hint = Text::new(page.locale).get("treatments.form.taper_times_hint", &[])?;
            html.push_str(&view! { <p class="household-note">{hint}</p> }.to_html());
            html.push_str(&times(page, "", 1)?);
            html.push_str(&taper(page)?);
        }
        kind => {
            html.push_str(&times(
                page,
                "",
                if kind == "multiple_daily" { 2 } else { 1 },
            )?);
            if kind == "weekly" {
                html.push_str(&weekdays(page)?);
            }
            if kind == "specific_dates" {
                html.push_str(&dates(page)?);
            }
        }
    }
    Ok(html)
}
