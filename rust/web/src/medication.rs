use crate::document::medication_document;
use crate::{household, household_i18n};
use leptos::prelude::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct MedicationCard {
    pub id: i64,
    pub name: String,
    pub supply: String,
    pub unit: String,
}

#[derive(Clone)]
pub struct DoseSource {
    pub id: i64,
    pub kind: String,
    pub person_name: String,
    pub amount: String,
    pub unit: String,
    pub portable_id: String,
    pub can_record: bool,
    pub eligible_stock_ids: Vec<i64>,
}

pub struct MedicationDetail {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub supply: String,
    pub unit: String,
    pub location: String,
    pub sources: Vec<DoseSource>,
}

pub struct DoseFormState {
    pub source_type: String,
    pub source_id: String,
    pub dose_amount: String,
    pub dose_unit: String,
    pub taken_at: String,
    pub stock_id: String,
}

pub fn render_medication_list(
    household_name: &str,
    slug: &str,
    csrf: &str,
    medications: Vec<MedicationCard>,
) -> String {
    render_medication_list_with_management(
        household_name,
        slug,
        csrf,
        medications,
        false,
        household_i18n::Locale::En,
    )
}

pub fn render_medication_list_with_management(
    household_name: &str,
    slug: &str,
    csrf: &str,
    medications: Vec<MedicationCard>,
    can_create: bool,
    locale: household_i18n::Locale,
) -> String {
    render_medication_list_with_stock(
        household_name,
        slug,
        csrf,
        medications,
        can_create,
        locale,
        &std::collections::HashMap::new(),
    )
}

pub fn render_medication_list_with_stock(
    household_name: &str,
    slug: &str,
    csrf: &str,
    medications: Vec<MedicationCard>,
    can_create: bool,
    locale: household_i18n::Locale,
    option_stock: &std::collections::HashMap<i64, Vec<crate::stock::OptionStock>>,
) -> String {
    let prefix = format!("/households/{}", household::path_segment(slug));
    let text = household_i18n::Text::new(locale);
    let title = text
        .get("medications.index.title", &[])
        .expect("catalogue key");
    let dashboard_label = text
        .get("layouts.sidebar.dashboard", &[])
        .expect("catalogue key");
    let inventory_label = text
        .get("layouts.sidebar.inventory", &[])
        .expect("catalogue key");
    let view_label = text
        .get("medications.index.view", &[])
        .expect("catalogue key");
    let people_label = text
        .get("layouts.sidebar.people", &[])
        .expect("catalogue key");
    let locations_label = text
        .get("layouts.sidebar.locations", &[])
        .expect("catalogue key");
    let add_label = text
        .get("medications.index.add_medication", &[])
        .expect("catalogue key");
    let body = view! {
        <main class="med-app">
            <header class="med-topbar"><a class="med-brand" href=format!("{prefix}/dashboard")>"MedTracker"</a><span>{household_name.to_owned()}</span></header>
            <div class="med-layout">
                <nav class="med-sidebar" aria-label=household_name.to_owned()><a href=format!("{prefix}/dashboard")>{dashboard_label}</a><a aria-current="page" href=format!("{prefix}/medications")>{inventory_label.clone()}</a><a href=format!("{prefix}/people")>{people_label}</a><a href=format!("{prefix}/locations")>{locations_label}</a></nav>
                <section class="med-content">
                    <p class="med-eyebrow">{inventory_label}</p>
                    <h1>{title.clone()}</h1>
                    {can_create.then(|| view! { <a class="med-button" href=format!("{prefix}/medications/new")>{add_label}</a> })}
                    <div class="med-grid">
                        {medications.into_iter().map(|medication| {
                            let href = format!("{prefix}/medications/{}", medication.id);
                            let rows = option_stock.get(&medication.id).map(Vec::as_slice).unwrap_or(&[]);
                            let stock = if rows.is_empty() {
                                let remaining = text.get("medications.index.stock_remaining", &[("amount", &medication.supply), ("unit", &medication.unit)]).expect("catalogue key");
                                view! { <p>{remaining}</p> }.to_html()
                            } else {
                                crate::stock::inventory_quantities(rows, &medication.supply, &medication.unit, locale)
                            };
                            let stock_path = format!("{href}/stock");
                            let stock_label = text.get("medications.stock.title", &[]).expect("catalogue key");
                            view! {
                                <article class="med-card">
                                    <h2>{medication.name.clone()}</h2>
                                    <div inner_html=stock></div>
                                    <a href=href>{view_label.clone()}</a>
                                    <a href=stock_path>{stock_label}</a>
                                </article>
                            }
                        }).collect_view()}
                    </div>
                </section>
            </div>
        </main>
    }.to_html();
    medication_document(&title, csrf, body).replacen(
        "lang=\"en\"",
        &format!("lang=\"{}\"", locale.as_str()),
        1,
    )
}

pub struct MedicationDetailRender<'a> {
    pub household_name: &'a str,
    pub slug: &'a str,
    pub csrf: &'a str,
    pub medication: MedicationDetail,
    pub stock_options: Vec<MedicationCard>,
    pub taken_at: &'a str,
    pub client_uuid: &'a str,
    pub notice: Option<&'a str>,
    pub form_state: Option<DoseFormState>,
}

pub fn render_medication_detail(input: MedicationDetailRender<'_>) -> String {
    render_medication_detail_with_management(input, false, household_i18n::Locale::En)
}

pub fn render_medication_detail_with_management(
    input: MedicationDetailRender<'_>,
    can_edit: bool,
    locale: household_i18n::Locale,
) -> String {
    render_medication_detail_with_stock(input, can_edit, locale, &[])
}

pub fn render_medication_detail_with_stock(
    input: MedicationDetailRender<'_>,
    can_edit: bool,
    locale: household_i18n::Locale,
    option_stock: &[crate::stock::OptionStock],
) -> String {
    let stock = HashMap::from([(input.medication.id, option_stock.to_vec())]);
    render_medication_detail_with_stock_inventory(input, can_edit, locale, &stock)
}

pub fn render_medication_detail_with_stock_inventory(
    input: MedicationDetailRender<'_>,
    can_edit: bool,
    locale: household_i18n::Locale,
    stock_inventory: &HashMap<i64, Vec<crate::stock::OptionStock>>,
) -> String {
    let MedicationDetailRender {
        household_name,
        slug,
        csrf,
        medication,
        stock_options,
        taken_at,
        client_uuid,
        notice,
        form_state,
    } = input;
    let option_stock = stock_inventory
        .get(&medication.id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let prefix = format!("/households/{}", household::path_segment(slug));
    let text = household_i18n::Text::new(locale);
    let people_label = text
        .get("layouts.sidebar.people", &[])
        .expect("catalogue key");
    let locations_label = text
        .get("layouts.sidebar.locations", &[])
        .expect("catalogue key");
    let edit_label = text
        .get("medications.form.edit_title", &[])
        .expect("catalogue key");
    let dashboard_label = text
        .get("layouts.sidebar.dashboard", &[])
        .expect("catalogue key");
    let inventory_label = text
        .get("layouts.sidebar.inventory", &[])
        .expect("catalogue key");
    let profile_label = text
        .get("medications.show.profile", &[])
        .expect("catalogue key");
    let overview_label = text
        .get("medications.show.overview", &[])
        .expect("catalogue key");
    let stock_label = text
        .get("medications.show.inventory_status", &[])
        .expect("catalogue key");
    let dose_action = format!("{prefix}/medications/{}/doses", medication.id);
    let stock_path = format!("{prefix}/medications/{}/stock", medication.id);
    let stock_action_label = text
        .get("medications.stock.title", &[])
        .expect("catalogue key");
    let stock_html = if option_stock.is_empty() {
        let remaining = text
            .get("medications.dose_dialog.remaining", &[])
            .expect("catalogue key");
        view! { <div class="med-stock-number"><strong>{medication.supply.clone()}</strong><span>{medication.unit.clone()}" "{remaining}</span></div> }.to_html()
    } else {
        crate::stock::inventory_quantities(
            option_stock,
            &medication.supply,
            &medication.unit,
            locale,
        )
    };
    let label = |key: &str| {
        text.get(&format!("medications.dose_dialog.{key}"), &[])
            .expect("catalogue key")
    };
    let administration_title = text
        .get(
            "medications.dose_dialog.administration_title",
            &[("name", &medication.name)],
        )
        .expect("catalogue key");
    let notice = notice.map(|message| {
        match message {
            "Medication taken successfully." => text.get("schedules.medication_taken", &[]),
            "Invalid dose configured" => text.get("schedules.invalid_dose_configured", &[]),
            "Taken at is invalid." => Ok(label("invalid_time")),
            "This source does not belong to this medication." => Ok(label("foreign_source")),
            "You cannot record this dose." => Ok(label("forbidden")),
            "This dose request conflicts with a previous record." => Ok(label("conflict")),
            _ => text.form_error(message),
        }
        .expect("catalogue notice")
    });
    let notice = notice.as_deref();
    let dosage_label = text
        .get("medications.show.dosages_heading", &[])
        .expect("catalogue key");
    let first = if let Some(form) = form_state.as_ref() {
        medication
            .sources
            .iter()
            .find(|source| source.kind == form.source_type && source.portable_id == form.source_id)
            .cloned()
    } else {
        medication.sources.first().cloned()
    };
    let source_unavailable = form_state.is_some() && first.is_none();
    let source_type = form_state
        .as_ref()
        .map(|form| form.source_type.clone())
        .or_else(|| first.as_ref().map(|source| source.kind.clone()))
        .unwrap_or_default();
    let source_id = form_state
        .as_ref()
        .map(|form| form.source_id.clone())
        .or_else(|| first.as_ref().map(|source| source.portable_id.clone()))
        .unwrap_or_default();
    let dose_amount = form_state
        .as_ref()
        .map(|form| form.dose_amount.clone())
        .or_else(|| first.as_ref().map(|source| source.amount.clone()))
        .unwrap_or_default();
    let dose_unit = form_state
        .as_ref()
        .map(|form| form.dose_unit.clone())
        .or_else(|| first.as_ref().map(|source| source.unit.clone()))
        .unwrap_or_default();
    let taken_at = form_state
        .as_ref()
        .map(|form| form.taken_at.clone())
        .unwrap_or_else(|| taken_at.to_owned());
    let selected_stock = form_state.as_ref().map(|form| form.stock_id.clone());
    let displayed_dose = if source_type == "schedule" {
        label("calculated")
    } else {
        format!("{} {}", dose_amount, dose_unit)
    };
    let initial_stock_ids = first
        .as_ref()
        .map(|source| source.eligible_stock_ids.clone())
        .unwrap_or_default();
    let first_stock_id = initial_stock_ids.first().copied();
    let has_recordable_source = medication
        .sources
        .iter()
        .any(|source| source.can_record && !source.eligible_stock_ids.is_empty());
    let body = view! {
        <main class="med-app">
            <header class="med-topbar"><a class="med-brand" href=format!("{prefix}/dashboard")>"MedTracker"</a><span>{household_name.to_owned()}</span></header>
            <div class="med-layout">
                <nav class="med-sidebar" aria-label=household_name.to_owned()><a href=format!("{prefix}/dashboard")>{dashboard_label}</a><a href=format!("{prefix}/medications")>{inventory_label}</a><a href=format!("{prefix}/people")>{people_label}</a><a href=format!("{prefix}/locations")>{locations_label}</a></nav>
                <section class="med-content">
                    <p class="med-eyebrow">{profile_label}</p>
                    <h1>{medication.name.clone()}</h1>
                    <a href=format!("{prefix}/medications/{}/dosage_options", medication.id)>{dosage_label}</a>
                    <a class="med-button" href=stock_path>{stock_action_label}</a>
                    {can_edit.then(|| view! { <a class="med-button" href=format!("{prefix}/medications/{}/edit", medication.id)>{edit_label}</a> })}
                    <p class="med-location">{medication.location.clone()}</p>
                    {form_state.is_none().then_some(notice).flatten().map(|text| view! { <p class="med-alert" role="alert">{text.to_owned()}</p> })}
                    <div class="med-detail-grid">
                        {(!medication.description.is_empty()).then(|| view! { <section class="med-card"><h2>{overview_label}</h2><p>{medication.description.clone()}</p></section> })}
                        <section class="med-card med-stock"><h2>{stock_label}</h2><div inner_html=stock_html></div><p>{label("stock_source")}": "{medication.location.clone()}</p></section>
                    </div>
                    {has_recordable_source.then(|| view! { <a class="med-button med-log-link" href="#administration" data-open-administration>{label("log")}</a> })}
                </section>
            </div>
            <dialog id="administration-dialog" aria-label=administration_title.clone()>
                <div class="med-dialog-heading"><div><h2>{administration_title.clone()}</h2><p>{label("choose_person_source")}</p></div><button type="button" class="med-close" data-close-dialog aria-label=label("close")>"×"</button></div>
                <div class="med-source-list">
                    {medication.sources.into_iter().map(|source| {
                        let test_id = format!("log-administration-{}-{}", source.kind, source.id);
                        let stock_ids = source.eligible_stock_ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
                        view! {
                            <article class="med-source-card"><div><p class="med-eyebrow">{label("source")}</p><strong>{source.person_name.clone()}</strong><p>{medication.name.clone()}</p><small>{if source.kind == "schedule" { label("dose_calculated") } else { format!("{} {}", source.amount, source.unit) }}</small></div>
                                {(source.can_record && !source.eligible_stock_ids.is_empty()).then(|| view! { <button type="button" class="med-button" data-open-dose data-testid=test_id data-source-id=source.portable_id data-source-kind=source.kind data-person-name=source.person_name data-dose-amount=source.amount data-dose-unit=source.unit data-stock-ids=stock_ids>{label("log")}</button> })}
                                {(!source.can_record).then(|| view! { <span class="med-source-note">{label("view_only")}</span> })}
                                {(source.can_record && source.eligible_stock_ids.is_empty()).then(|| view! { <span class="med-source-note">{label("no_stock")}</span> })}
                            </article>
                        }
                    }).collect_view()}
                </div>
            </dialog>
            <dialog id="dose-dialog" aria-label=label("record") data-calculated-dose=label("calculated") data-reopen=form_state.is_some()>
                <div class="med-dialog-heading"><div><h2>{label("record")}</h2><p>{label("confirm")}</p></div><button type="button" class="med-close" data-close-dialog aria-label=label("close")>"×"</button></div>
                {form_state.is_some().then(|| view! { <p class="med-alert" role="alert">{notice.map(str::to_owned).unwrap_or_else(|| label("review"))}</p> })}
                {source_unavailable.then(|| view! { <p class="med-source-note">{label("unavailable")}</p><button type="button" class="med-text-button" data-choose-source>{label("choose_source")}</button> })}
                <div class="med-dose-summary"><div><small>{label("person")}</small><strong id="dose-person">{first.as_ref().map(|source| source.person_name.clone()).unwrap_or_else(|| label("choose_source"))}</strong></div><div><small>{label("medication")}</small><strong>{medication.name.clone()}</strong></div><div><small>{label("dose")}</small><strong id="dose-display">{displayed_dose}</strong></div></div>
                <form class="med-dose-form" method="post" action=dose_action>
                    <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                    <input type="hidden" name="client_uuid" value=client_uuid.to_owned()/>
                    <input type="hidden" name="source_type" value=source_type/>
                    <input type="hidden" name="source_id" value=source_id/>
                    <input type="hidden" name="dose_amount" value=dose_amount/>
                    <input type="hidden" name="dose_unit" value=dose_unit/>
                    <label for="taken-at">{label("taken_at")}</label><input id="taken-at" name="taken_at" type="datetime-local" value=taken_at required/>
                    <label for="stock-source">{label("stock_source")}</label>
                    <select id="stock-source" name="taken_from_medication_id">
                        {stock_options.into_iter().map(|option| { let selected = selected_stock.as_ref().is_some_and(|stock| stock == &option.id.to_string()) || selected_stock.is_none() && first_stock_id == Some(option.id); let available = initial_stock_ids.contains(&option.id); let rows = stock_inventory.get(&option.id).map(Vec::as_slice).unwrap_or(&[]); let contents = crate::stock::selector_quantities(rows, &option.supply, &option.unit, locale); view! { <option value=option.id selected=selected hidden=!available disabled=!available>{option.name}" — "{contents}" "{label("remaining")}</option> } }).collect_view()}
                    </select>
                    <div class="med-dialog-actions"><button class="med-button" type="submit" disabled=source_unavailable>{label("log")}</button></div>
                </form>
            </dialog>
        </main>
    }.to_html();
    medication_document(&medication.name, csrf, body).replacen(
        "lang=\"en\"",
        &format!("lang=\"{}\"", locale.as_str()),
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DoseFormState, DoseSource, MedicationCard, MedicationDetail, MedicationDetailRender,
        render_medication_detail,
    };

    #[test]
    fn rejected_dose_preserves_form_snapshot_and_reopens_dialog() {
        let html = render_medication_detail(MedicationDetailRender {
            household_name: "Home",
            slug: "home",
            csrf: "csrf",
            medication: MedicationDetail {
                id: 1,
                name: "Example".to_owned(),
                description: String::new(),
                supply: "20".to_owned(),
                unit: "ml".to_owned(),
                location: "Cabinet".to_owned(),
                sources: vec![DoseSource {
                    id: 2,
                    kind: "person_medication".to_owned(),
                    person_name: "Sam".to_owned(),
                    amount: "1.25".to_owned(),
                    unit: "ml".to_owned(),
                    portable_id: "source-2".to_owned(),
                    can_record: true,
                    eligible_stock_ids: vec![1],
                }],
            },
            stock_options: vec![MedicationCard {
                id: 1,
                name: "Example".to_owned(),
                supply: "20".to_owned(),
                unit: "ml".to_owned(),
            }],
            taken_at: "2026-03-30T08:00",
            client_uuid: "retry-uuid",
            notice: Some("Invalid dose configured"),
            form_state: Some(DoseFormState {
                source_type: "person_medication".to_owned(),
                source_id: "source-2".to_owned(),
                dose_amount: "invalid".to_owned(),
                dose_unit: "ml".to_owned(),
                taken_at: "2026-03-30T09:00".to_owned(),
                stock_id: "1".to_owned(),
            }),
        });
        assert!(html.contains("data-reopen>"));
        assert!(html.contains("name=\"client_uuid\" value=\"retry-uuid\""));
        assert!(html.contains("name=\"dose_amount\" value=\"invalid\""));
        assert!(html.contains("id=\"dose-display\">invalid ml"));
        assert!(
            html.contains("name=\"taken_at\" type=\"datetime-local\" value=\"2026-03-30T09:00\"")
        );
    }
}
