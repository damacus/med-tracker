use super::*;
use crate::household_i18n::{Text, TranslationError};

pub fn render_assignment_form(page: TreatmentFormPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get(
        if page.editing {
            "person_medications.form.edit_medication_for"
        } else {
            "person_medications.form.add_medication_for"
        },
        &[("person", &page.person_name)],
    )?;
    let submit = text.get(
        if page.editing {
            "person_medications.form.save_changes_button"
        } else {
            "person_medications.form.add_medication_button"
        },
        &[],
    )?;
    let mut html = fields::common(&page)?;
    let kinds = ["routine", "as_needed"]
        .into_iter()
        .map(|kind| {
            text.get(
                &format!("person_medications.form.administration_kinds.{kind}.label"),
                &[],
            )
            .map(|label| (kind.to_owned(), label))
        })
        .collect::<Result<Vec<_>, _>>()?;
    html.push_str(&fields::select(
        &page,
        "administration_kind",
        "person_medications.form.administration_kind",
        kinds,
        "person_medications.form.optional",
    )?);
    html.push_str(&fields::notes(&page)?);
    fields::document(page, title, submit, html)
}
