use crate::household::{household_document, path_segment};
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct NotificationDraft {
    pub enabled: bool,
    pub dose_due_enabled: bool,
    pub missed_dose_enabled: bool,
    pub low_stock_enabled: bool,
    pub private_text_enabled: bool,
}

impl Default for NotificationDraft {
    fn default() -> Self {
        Self {
            enabled: true,
            dose_due_enabled: true,
            missed_dose_enabled: true,
            low_stock_enabled: true,
            private_text_enabled: false,
        }
    }
}

pub struct NotificationPage {
    pub household_name: String,
    pub slug: String,
    pub csrf: String,
    pub locale: Locale,
    pub preferences: Option<NotificationDraft>,
    pub editable: bool,
    pub saved: bool,
    pub error: bool,
}

const CATEGORIES: [&str; 4] = [
    "dose_due_enabled",
    "missed_dose_enabled",
    "low_stock_enabled",
    "private_text_enabled",
];

fn switch(name: &str, label: String, hint: String, checked: bool, disabled: bool) -> impl IntoView {
    let id = format!("notification_{name}");
    let hint_id = format!("{id}_hint");
    view! {
        <div class="form-field">
            <label for=id.clone()>{label}</label>
            <input id=id name=name.to_owned() type="checkbox" value="true" checked=checked disabled=disabled aria-describedby=hint_id.clone()/>
            <p id=hint_id>{hint}</p>
        </div>
    }
}

fn form(
    text: &Text,
    page: &NotificationPage,
    title: &str,
    draft: &NotificationDraft,
) -> Result<String, TranslationError> {
    let action = format!(
        "/households/{}/settings/notifications",
        path_segment(&page.slug)
    );
    let description = text.get("notifications.description", &[])?;
    let saved_message = text.get("notification_preferences.updated", &[])?;
    let failed_message = text.get("notification_preferences.update_failed", &[])?;
    let read_only_message = text.get("notifications.read_only", &[])?;
    let master = switch(
        "enabled",
        text.get("notifications.enabled_label", &[])?,
        text.get("notifications.enabled_hint", &[])?,
        draft.enabled,
        !page.editable,
    )
    .into_any();
    let categories_title = text.get("notification_settings.categories.title", &[])?;
    let categories_hint = text.get("notifications.categories_hint", &[])?;
    let category_values = [
        draft.dose_due_enabled,
        draft.missed_dose_enabled,
        draft.low_stock_enabled,
        draft.private_text_enabled,
    ];
    let mut switches = Vec::with_capacity(CATEGORIES.len());
    for (name, checked) in CATEGORIES.into_iter().zip(category_values) {
        switches.push(
            switch(
                name,
                text.get(
                    &format!("notification_settings.categories.{name}.title"),
                    &[],
                )?,
                text.get(
                    &format!("notification_settings.categories.{name}.description"),
                    &[],
                )?,
                checked,
                !page.editable,
            )
            .into_any(),
        );
    }
    let save = text.get("notifications.save", &[])?;
    Ok(view! {
        <h1>{title.to_owned()}</h1>
        <p>{description}</p>
        <form class="household-form med-card" id="notification_preferences_form" method="post" action=action>
            <input type="hidden" name="authenticity_token" value=page.csrf.clone()/>
            {page.saved.then(|| view! { <div class="med-alert" role="status"><p>{saved_message}</p></div> })}
            {page.error.then(|| view! { <div class="med-alert" role="alert"><p>{failed_message}</p></div> })}
            {(!page.editable).then(|| view! { <p role="status">{read_only_message}</p> })}
            {master}
            <fieldset class="form-field">
                <legend>{categories_title}</legend>
                <p>{categories_hint}</p>
                {switches.into_iter().collect_view()}
            </fieldset>
            {page.editable.then(|| view! { <div class="household-actions"><button class="med-button" type="submit">{save}</button></div> })}
        </form>
    }
    .to_html())
}

pub fn render_notification_settings(page: NotificationPage) -> Result<String, TranslationError> {
    let raw_slug = page.slug.as_str();
    let text = Text::new(page.locale);
    let title = text.get("notifications.title", &[])?;
    let description = text.get("notifications.description", &[])?;
    let body = match page.preferences.as_ref() {
        None => {
            let unavailable = text.get("notifications.unavailable", &[])?;
            view! {
                <h1>{title.clone()}</h1>
                <p>{description}</p>
                <section class="med-card"><p role="status">{unavailable}</p></section>
            }
            .to_html()
        }
        Some(draft) => form(&text, &page, &title, draft)?,
    };
    Ok(household_document(
        &title,
        &page.household_name,
        raw_slug,
        page.locale.as_str(),
        body,
    ))
}
