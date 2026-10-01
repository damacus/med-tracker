use super::*;
use crate::household::{household_document, path_segment};
use crate::household_i18n::{Text, TranslationError};
use crate::medication_management::textarea_field;
use leptos::prelude::*;

pub fn render_pause_form(
    page: TreatmentFormPage,
    resuming: bool,
    medication_name: String,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get(
        if resuming {
            "treatments.form.resume"
        } else {
            "medication_pauses.title"
        },
        &[],
    )?;
    let submit = text.get(
        if resuming {
            "treatments.form.resume"
        } else {
            "medication_pauses.submit"
        },
        &[],
    )?;
    let mut html = view! { <p>{medication_name}</p> }.to_html();
    if resuming {
        let hint = text.get("treatments.form.resume_hint", &[])?;
        html.push_str(&view! { <p class="household-note">{hint}</p> }.to_html());
    } else {
        let reasons = [
            "out_of_supply",
            "temporarily_not_needed",
            "clinician_advice",
            "side_effects",
            "other",
        ]
        .into_iter()
        .map(|reason| {
            text.get(&format!("medication_pauses.reasons.{reason}"), &[])
                .map(|label| (reason.to_owned(), label))
        })
        .collect::<Result<Vec<_>, _>>()?;
        html.push_str(&fields::select(
            &page,
            "reason",
            "medication_pauses.reason",
            reasons,
            "medication_pauses.choose_reason",
        )?);
        html.push_str(&textarea_field(
            "note",
            text.get("medication_pauses.note", &[])?,
            page.draft.value("note"),
            text,
            fields::error_slice(&page, "note"),
        ));
        let timing = text.get("medication_pauses.timing", &[])?;
        html.push_str(&view! { <p class="household-note">{timing}</p> }.to_html());
    }
    fields::document(page, title, submit, html)
}

pub struct PauseHistoryPage {
    pub household_name: String,
    pub slug: String,
    pub person_id: String,
    pub medication_name: String,
    pub locale: Locale,
    pub rows: Vec<PauseHistoryRow>,
}

pub struct PauseHistoryRow {
    pub reason: String,
    pub note: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub recorded_by: Option<String>,
    pub resumed_by: Option<String>,
}

pub fn render_pause_history(page: PauseHistoryPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get("medication_pauses.history", &[])?;
    let unknown_actor = text.get("medication_pauses.actor_unknown", &[])?;
    let unknown_start = text.get("medication_pauses.start_unknown", &[])?;
    let active = text.get("treatments.form.pause_open", &[])?;
    let back = text.get("person_medications.form.back", &[])?;
    let empty = text.get("treatments.form.no_pause_history", &[])?;
    let mut rows = String::new();
    for row in page.rows {
        let reason = text
            .get(&format!("medication_pauses.reasons.{}", row.reason), &[])
            .or_else(|_| text.get("medication_pauses.reasons.reason_not_recorded", &[]))?;
        let started = text.get(
            "medication_pauses.started",
            &[
                ("date", row.started_at.as_deref().unwrap_or(&unknown_start)),
                (
                    "actor",
                    row.recorded_by.as_deref().unwrap_or(&unknown_actor),
                ),
            ],
        )?;
        let ended = match row.ended_at {
            Some(date) => text.get(
                "medication_pauses.resumed",
                &[
                    ("date", &date),
                    ("actor", row.resumed_by.as_deref().unwrap_or(&unknown_actor)),
                ],
            )?,
            None => active.clone(),
        };
        rows.push_str(&view! { <article class="med-card"><h2>{reason}</h2><p>{started}</p><p>{ended}</p><p>{row.note}</p></article> }.to_html());
    }
    let body = view! {
        <section class="household-content"><h1>{title.clone()}</h1><p>{page.medication_name}</p>
            {(rows.is_empty()).then(|| view! { <p>{empty}</p> })}
            <div class="household-grid" inner_html=rows></div>
            <a class="med-text-button" href=format!("/households/{}/people/{}", path_segment(&page.slug), path_segment(&page.person_id))>{back}</a>
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
