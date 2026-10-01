use crate::household::path_segment;
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

pub struct TreatmentRow {
    pub id: String,
    pub resource: String,
    pub medication_name: String,
    pub dose_amount: String,
    pub dose_unit: String,
    pub description: String,
    pub notes: String,
    pub paused: bool,
}

pub fn render_treatment_overview(
    slug: &str,
    person: i64,
    locale: Locale,
    can_manage: bool,
    rows: Vec<TreatmentRow>,
) -> Result<String, TranslationError> {
    let text = Text::new(locale);
    let base = format!("/households/{}/people/{person}", path_segment(slug));
    let title = text.get("treatments.overview.title", &[])?;
    let add_assignment = text.get("person_medications.form.add_medication", &[])?;
    let add_schedule = text.get("schedules.form.add_plan", &[])?;
    let empty = text.get("treatments.overview.empty", &[])?;
    let edit = text.get("person_medications.card.edit", &[])?;
    let pause = text.get("person_medications.card.pause", &[])?;
    let resume = text.get("person_medications.card.resume", &[])?;
    let history = text.get("medication_pauses.history", &[])?;
    let paused = text.get("person_medications.card.paused", &[])?;
    Ok(view! {
        <section aria-labelledby="treatments-heading"><div class="household-heading"><h2 id="treatments-heading">{title}</h2>
            {can_manage.then(|| view! { <div class="household-actions"><a class="med-button" href=format!("{base}/assignments/new")>{add_assignment}</a><a class="med-button" href=format!("{base}/schedules/new")>{add_schedule}</a></div> })}
        </div>
        {rows.is_empty().then(|| view! { <p>{empty}</p> })}
        <div class="household-grid">{rows.into_iter().map(|row| {
            let source = format!("{base}/{}/{}", row.resource, path_segment(&row.id));
            let edit = edit.clone();
            let action = if row.paused { "resume" } else { "pause" };
            let action_label = if row.paused { resume.clone() } else { pause.clone() };
            let history = history.clone();
            let paused_label = paused.clone();
            view! { <article class="med-card" data-treatment-id=row.id><h3>{row.medication_name}</h3>
                <p>{row.dose_amount}" "{row.dose_unit}</p><p>{row.description}</p><p>{row.notes}</p>
                {row.paused.then(|| view! { <p class="med-badge">{paused_label}</p> })}
                <div class="household-actions">{can_manage.then(|| view! { <a class="med-text-button" href=format!("{source}/edit")>{edit}</a><a class="med-text-button" href=format!("{source}/{action}")>{action_label}</a> })}
                <a class="med-text-button" href=format!("{source}/history")>{history}</a></div>
            </article> }
        }).collect_view()}</div></section>
    }.to_html())
}
