use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use crate::medication_management::{input_field, messages, select_field, textarea_field};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct OptionStock {
    pub id: i64,
    pub quantity: Option<String>,
    pub unit: String,
}

pub fn option_quantities(rows: &[OptionStock], locale: Locale) -> String {
    let text = Text::new(locale);
    rows.iter()
        .map(|row| {
            let quantity = row.quantity.clone().unwrap_or_else(|| {
                text.get("dosages.management.untracked", &[])
                    .expect("catalogue key")
            });
            view! { <p data-stock-option-id=row.id>{quantity}" "{row.unit.clone()}</p> }.to_html()
        })
        .collect()
}

pub fn selector_quantities(
    rows: &[OptionStock],
    supply: &str,
    unit: &str,
    locale: Locale,
) -> String {
    let text = Text::new(locale);
    let untracked = text
        .get("dosages.management.untracked", &[])
        .expect("catalogue key");
    if rows.is_empty() {
        return format!(
            "{} {unit}",
            if supply.is_empty() {
                untracked.as_str()
            } else {
                supply
            }
        );
    }
    let mut contents = rows
        .iter()
        .map(|row| {
            format!(
                "{} {}",
                row.quantity.as_deref().unwrap_or(&untracked),
                row.unit
            )
        })
        .collect::<Vec<_>>();
    if rows.iter().all(|row| row.quantity.is_none()) {
        contents.push(
            text.get(
                if supply.is_empty() {
                    "stock_removals.errors.untracked"
                } else {
                    "medications.stock.scalar_fallback"
                },
                &[],
            )
            .expect("catalogue key"),
        );
        if !supply.is_empty() {
            contents.push(format!("{supply} {unit}"));
        }
    }
    contents.join("; ")
}

pub fn inventory_quantities(
    rows: &[OptionStock],
    supply: &str,
    unit: &str,
    locale: Locale,
) -> String {
    if rows.is_empty() {
        return view! { <p>{supply.to_owned()}" "{unit.to_owned()}</p> }.to_html();
    }
    let mut html = option_quantities(rows, locale);
    if rows.iter().all(|row| row.quantity.is_none()) {
        let warning = Text::new(locale)
            .get(
                if supply.is_empty() {
                    "stock_removals.errors.untracked"
                } else {
                    "medications.stock.scalar_fallback"
                },
                &[],
            )
            .expect("catalogue key");
        html.push_str(&view! { <p data-stock-scalar-fallback>{warning}</p> }.to_html());
        if !supply.is_empty() {
            html.push_str(&view! { <p>{supply.to_owned()}" "{unit.to_owned()}</p> }.to_html());
        }
    }
    html
}

#[derive(Clone, Debug, Default)]
pub struct StockDraft {
    pub etag: String,
    pub new_quantity: String,
    pub reason: String,
    pub supplier: String,
    pub quantity: String,
    pub expected_arrival_on: String,
    pub note: String,
    pub dosage_id: String,
    pub submission_id: String,
}

pub struct StockPage {
    pub household_name: String,
    pub slug: String,
    pub medication_id: String,
    pub medication_name: String,
    pub supply: Option<String>,
    pub unit: String,
    pub status: String,
    pub options: Vec<OptionStock>,
    pub can_manage: bool,
    pub csrf: String,
    pub locale: Locale,
    pub action: Option<String>,
    pub draft: StockDraft,
    pub errors: BTreeMap<String, Vec<String>>,
}

pub fn render_stock(page: StockPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let label = |key: &str| {
        text.get(&format!("medications.stock.{key}"), &[])
            .expect("catalogue key")
    };
    let base = format!(
        "/households/{}/medications/{}/stock",
        path_segment(&page.slug),
        path_segment(&page.medication_id)
    );
    let options_mode = !page.options.is_empty();
    let fallback = options_mode && page.options.iter().all(|row| row.quantity.is_none());
    let option_html = option_quantities(&page.options, page.locale);
    let quantity = page.supply.clone().unwrap_or_else(|| {
        text.get("dosages.management.untracked", &[])
            .expect("catalogue key")
    });
    let title = match page.action.as_deref() {
        Some("adjust") => label("adjust"),
        Some("order") => label("order"),
        Some("remove") => text.get("stock_removals.title", &[])?,
        _ => label("title"),
    };
    let mut fields = String::new();
    let field = |name: &str, caption: String, value: &str, required: bool| {
        input_field(
            name,
            caption,
            value,
            "text",
            required,
            text,
            page.errors.get(name).map(Vec::as_slice).unwrap_or(&[]),
        )
    };
    match page.action.as_deref() {
        Some("adjust") => {
            fields.push_str(&field(
                "new_quantity",
                label("new_quantity"),
                &page.draft.new_quantity,
                true,
            ));
            fields.push_str(&field(
                "reason",
                text.get("stock_removals.reason", &[])?,
                &page.draft.reason,
                false,
            ));
        }
        Some("order") => {
            fields.push_str(&field(
                "supplier",
                text.get("medications.show.order_supplier", &[])?,
                &page.draft.supplier,
                false,
            ));
            fields.push_str(&field(
                "quantity",
                text.get("medications.show.order_quantity", &[])?,
                &page.draft.quantity,
                false,
            ));
            fields.push_str(&field(
                "expected_arrival_on",
                text.get("medications.show.expected_arrival", &[])?,
                &page.draft.expected_arrival_on,
                false,
            ));
            fields.push_str(&view! { <p>{label("date_hint")}</p> }.to_html());
        }
        Some("remove") => {
            fields.push_str(&field(
                "quantity",
                text.get("stock_removals.quantity", &[])?,
                &page.draft.quantity,
                true,
            ));
            let reasons = [
                "dropped",
                "damaged",
                "expired",
                "discarded",
                "lost",
                "transferred_out",
                "other",
            ]
            .into_iter()
            .map(|value| {
                Ok((
                    value.to_owned(),
                    text.get(&format!("stock_removals.reasons.{value}"), &[])?,
                ))
            })
            .collect::<Result<Vec<_>, TranslationError>>()?;
            fields.push_str(&select_field(
                "reason",
                text.get("stock_removals.reason", &[])?,
                &page.draft.reason,
                reasons,
                text.get("stock_removals.choose_reason", &[])?,
                text,
                page.errors.get("reason").map(Vec::as_slice).unwrap_or(&[]),
            ));
            if options_mode && !fallback {
                let choices = page
                    .options
                    .iter()
                    .filter_map(|row| {
                        row.quantity.as_ref().map(|quantity| {
                            (row.id.to_string(), format!("{quantity} {}", row.unit))
                        })
                    })
                    .collect();
                fields.push_str(&select_field(
                    "dosage_id",
                    text.get("stock_removals.source", &[])?,
                    &page.draft.dosage_id,
                    choices,
                    text.get("stock_removals.choose_source", &[])?,
                    text,
                    page.errors
                        .get("dosage_id")
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ));
            }
            fields.push_str(&textarea_field(
                "note",
                text.get("stock_removals.note", &[])?,
                &page.draft.note,
                text,
                page.errors.get("note").map(Vec::as_slice).unwrap_or(&[]),
            ));
        }
        _ => {}
    }
    let errors = page
        .errors
        .iter()
        .map(|(name, values)| {
            let message = messages(text, values);
            let href = format!("#medication-{name}");
            if matches!(name.as_str(), "stock" | "stock_removal") {
                view! { <li>{message}</li> }.into_any()
            } else {
                view! { <li><a href=href>{message}</a></li> }.into_any()
            }
        })
        .collect_view();
    let action = page.action.clone();
    let body = view! {
        <main class="household-page">
            <a href=format!("/households/{}/medications/{}", path_segment(&page.slug), path_segment(&page.medication_id))>{page.medication_name.clone()}</a>
            <h1>{title.clone()}</h1>
            {(!page.errors.is_empty()).then(|| view! { <div class="med-alert" role="alert"><ul>{errors}</ul></div> })}
            <section class="med-card med-stock">
                <h2>{label("title")}</h2>
                {options_mode.then(|| view! { <div inner_html=option_html></div><p>{label("option_total")}</p> })}
                {(!options_mode || fallback && page.supply.is_some()).then(|| view! { <p>{quantity}" "{page.unit.clone()}</p> })}
                {fallback.then(|| view! { <p data-stock-scalar-fallback>{if page.supply.is_some() { label("scalar_fallback") } else { text.get("stock_removals.errors.untracked", &[]).expect("catalogue key") }}</p> })}
            </section>
            {action.as_ref().map(|action| view! {
                <form class="household-form" method="post" action=format!("{base}/{action}") novalidate>
                    <input type="hidden" name="authenticity_token" value=page.csrf.clone()/>
                    {(action == "adjust").then(|| view! { <input type="hidden" name="etag" value=page.draft.etag.clone()/> })}
                    {(action == "remove").then(|| view! { <input type="hidden" name="submission_id" value=page.draft.submission_id.clone()/> })}
                    <div inner_html=fields></div>
                    <button class="med-primary" type="submit">{title.clone()}</button>
                    <a href=base.clone()>{text.get("stock_removals.cancel", &[]).expect("catalogue key")}</a>
                </form>
            })}
            {action.is_none().then(|| view! {
                <div class="household-actions">
                    {(page.can_manage && !options_mode).then(|| view! { <a class="med-button" href=format!("{base}/adjust")>{label("adjust")}</a> })}
                    {page.can_manage.then(|| view! {
                        <div>{page.options.iter().map(|row| view! { <a class="med-button" href=format!("/households/{}/medications/{}/dosage_options/{}/edit", path_segment(&page.slug), path_segment(&page.medication_id), row.id)>{label("edit_option")}" "{row.unit.clone()}</a> }).collect_view()}</div>
                        <a class="med-button" href=format!("{base}/remove")>{text.get("stock_removals.title", &[]).expect("catalogue key")}</a>
                    })}
                    <a class="med-button" href=format!("{base}/order")>{label("order")}</a>
                    <p>{match page.status.as_str() { "ordered" => label("ordered"), "received" => label("received"), _ => label("not_ordered") }}</p>
                    <form method="post" action=format!("{base}/receive")><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><button class="med-primary" type="submit">{label("receive")}</button></form>
                    <p>{label("receipt_hint")}</p>
                </div>
            })}
        </main>
    }.to_html();
    Ok(household_document(
        &title,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
    ))
}
