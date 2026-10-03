use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct ReportChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct ReportDraft {
    pub person_id: String,
    pub start_date: String,
    pub end_date: String,
    pub include_medication_takes: bool,
}

pub fn render_reports(
    household_name: &str,
    slug: &str,
    locale: Locale,
    choices: Vec<ReportChoice>,
    draft: ReportDraft,
    errors: Vec<(String, String)>,
) -> Result<String, TranslationError> {
    let raw_slug = slug;
    let slug = path_segment(slug);
    let text = Text::new(locale);
    let errors = errors
        .into_iter()
        .map(|(field, key)| {
            text.get(&format!("reports.browser.errors.{key}"), &[])
                .map(|message| (field, message))
        })
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let title = text.get("layouts.sidebar.reports", &[])?;
    let intro = text.get("reports.browser.intro", &[])?;
    let person_label = text.get("reports.browser.person_label", &[])?;
    let person_prompt = text.get("reports.browser.person_prompt", &[])?;
    let start_label = text.get("reports.browser.start_date_label", &[])?;
    let end_label = text.get("reports.browser.end_date_label", &[])?;
    let takes_label = text.get("reports.browser.include_medication_takes", &[])?;
    let download = text.get("reports.browser.download", &[])?;
    let no_people = text.get("reports.browser.no_people", &[])?;
    let action = format!("/households/{slug}/reports/health-history.pdf");
    let error_for = |field: &str| {
        errors
            .iter()
            .filter(|(key, _)| key == field)
            .map(|(_, message)| message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    };
    let person_error = error_for("person_id");
    let start_error = error_for("start_date");
    let end_error = error_for("end_date");
    let takes_error = error_for("include_medication_takes");
    let empty = choices.is_empty();
    let masked =
        !draft.person_id.is_empty() && choices.iter().all(|choice| choice.id != draft.person_id);
    let body = view! {
        <div class="household-heading"><h1>{title.clone()}</h1></div>
        <section class="med-card">
            <p class="household-note">{intro}</p>
            {(!errors.is_empty()).then(|| view! { <div class="med-alert" role="alert"><ul>{errors.iter().map(|(_, message)| view! { <li>{message.clone()}</li> }).collect_view()}</ul></div> })}
            {empty.then(|| view! { <p class="household-note">{no_people.clone()}</p> })}
            <form id="health_history_report_form" class="household-form" action=action method="get">
                <div class="form-field"><label for="report_person_id">{person_label}</label>
                    <select id="report_person_id" name="person_id" required disabled=empty
                        aria-invalid=(!person_error.is_empty()).then_some("true")
                        aria-describedby=(!person_error.is_empty()).then_some("report_person_id_error")>
                        {(!empty).then(|| view! { <option value="" selected=draft.person_id.is_empty()>{person_prompt.clone()}</option> })}
                        {masked.then(|| view! { <option value=draft.person_id.clone() selected>{draft.person_id.clone()}</option> })}
                        {choices.iter().map(|choice| view! { <option value=choice.id.clone() selected=draft.person_id == choice.id>{choice.name.clone()}</option> }).collect_view()}
                    </select>
                    {(!person_error.is_empty()).then(|| view! { <p class="household-field-error" id="report_person_id_error">{person_error}</p> })}
                </div>
                <div class="form-field"><label for="report_start_date">{start_label}</label>
                    <input id="report_start_date" name="start_date" type="date" value=draft.start_date required
                        aria-invalid=(!start_error.is_empty()).then_some("true")
                        aria-describedby=(!start_error.is_empty()).then_some("report_start_date_error")/>
                    {(!start_error.is_empty()).then(|| view! { <p class="household-field-error" id="report_start_date_error">{start_error}</p> })}
                </div>
                <div class="form-field"><label for="report_end_date">{end_label}</label>
                    <input id="report_end_date" name="end_date" type="date" value=draft.end_date required
                        aria-invalid=(!end_error.is_empty()).then_some("true")
                        aria-describedby=(!end_error.is_empty()).then_some("report_end_date_error")/>
                    {(!end_error.is_empty()).then(|| view! { <p class="household-field-error" id="report_end_date_error">{end_error}</p> })}
                </div>
                <div class="form-field"><label for="report_include_medication_takes">{takes_label}</label>
                    <input id="report_include_medication_takes" name="include_medication_takes" type="checkbox" value="1" checked=draft.include_medication_takes
                        aria-invalid=(!takes_error.is_empty()).then_some("true")
                        aria-describedby=(!takes_error.is_empty()).then_some("report_include_medication_takes_error")/>
                    {(!takes_error.is_empty()).then(|| view! { <p class="household-field-error" id="report_include_medication_takes_error">{takes_error}</p> })}
                </div>
                <div class="household-actions"><button class="med-primary" type="submit" disabled=empty>{download}</button></div>
            </form>
        </section>
    }.to_html();
    Ok(household_document(
        &title,
        household_name,
        raw_slug,
        locale.as_str(),
        body,
    ))
}
