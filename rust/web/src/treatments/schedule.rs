use super::*;
use crate::household_i18n::{Text, TranslationError};
use leptos::prelude::*;

pub fn render_schedule_form(page: TreatmentFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get(
        if page.editing {
            "schedules.modal.edit_title"
        } else {
            "schedules.modal.new_title"
        },
        &[("person", &page.person_name)],
    )?;
    let submit = text.get(
        if page.editing {
            "schedules.form.update_plan"
        } else {
            "schedules.form.add_plan"
        },
        &[],
    )?;
    let mut html = fields::common(&page)?;
    let kinds = [
        "daily",
        "multiple_daily",
        "weekly",
        "specific_dates",
        "prn",
        "tapering",
        "every_other_day",
    ]
    .into_iter()
    .map(|kind| {
        text.get(&format!("treatments.types.{kind}"), &[])
            .map(|label| (kind.to_owned(), label))
    })
    .collect::<Result<Vec<_>, _>>()?;
    html.push_str(&fields::select(
        &page,
        "schedule_type",
        "treatments.form.schedule_type",
        kinds,
        "person_medications.form.optional",
    )?);
    let change = text.get("treatments.form.show_fields", &[])?;
    html.push_str(&view! { <button type="submit" name="intent" value="change_type" class="med-button">{change}</button> }.to_html());
    html.push_str(&fields::input(
        &page,
        "frequency",
        "schedules.form.frequency",
    )?);
    html.push_str(&fields::input(
        &page,
        "start_date",
        "schedules.form.start_date",
    )?);
    html.push_str(&fields::input(
        &page,
        "end_date",
        "treatments.form.required_end_date",
    )?);
    html.push_str(&schedule_controls::render(&page)?);
    html.push_str(&fields::notes(&page)?);
    fields::document(page, title, submit, html)
}
