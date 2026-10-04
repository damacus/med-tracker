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

pub struct ManagedPerson {
    pub id: i64,
    pub name: String,
    pub automatic: bool,
    pub selected: bool,
}

pub struct ProfileNotificationPage {
    pub slug: String,
    pub csrf: String,
    pub household_id: i64,
    pub locale: Locale,
    pub preferences: NotificationDraft,
    pub editable: bool,
    pub saved: bool,
    pub error: bool,
    pub push_configured: bool,
    pub times: [Option<String>; 4],
    pub managed: Vec<ManagedPerson>,
}

pub fn render_notification_profile(
    page: ProfileNotificationPage,
) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let action = format!(
        "/households/{}/profile/notifications",
        path_segment(&page.slug)
    );
    let title = text.get("profiles.notifications.title", &[])?;
    let description = text.get("profiles.notifications.description", &[])?;
    let browser_title = text.get("profiles.notifications.browser_title", &[])?;
    let browser_description = text.get("profiles.notifications.browser_description", &[])?;
    let master_label = text.get("profiles.notifications.enable_reminders", &[])?;
    let master_hint = text.get("profiles.notifications.enable_reminders_description", &[])?;
    let categories_title = text.get("notification_settings.categories.title", &[])?;
    let categories_hint = text.get("profiles.notifications.categories_description", &[])?;
    let times_title = text.get("profiles.notifications.reminder_times_title", &[])?;
    let times_description = text.get("profiles.notifications.reminder_times_description", &[])?;
    let managed_title = text.get("profiles.notifications.managed_people.title", &[])?;
    let managed_description = text.get("profiles.notifications.managed_people.description", &[])?;
    let automatic = text.get("profiles.notifications.managed_people.automatic", &[])?;
    let save = text.get("profiles.notifications.save", &[])?;
    let on = text.get("profiles.common.on", &[])?;
    let off = text.get("profiles.common.off", &[])?;
    let checked = [
        page.preferences.dose_due_enabled,
        page.preferences.missed_dose_enabled,
        page.preferences.low_stock_enabled,
        page.preferences.private_text_enabled,
    ];
    let reminder_state = if page.preferences.enabled { &on } else { &off };
    let reminder_summary = text.get(
        "profiles.sections.notifications.reminders",
        &[("state", reminder_state)],
    )?;
    let enabled_count = checked
        .iter()
        .filter(|enabled| **enabled)
        .count()
        .to_string();
    let category_summary = text.get(
        "profiles.sections.notifications.categories",
        &[("count", &enabled_count)],
    )?;
    let categories = CATEGORIES
        .into_iter()
        .zip(checked)
        .map(|(name, checked)| {
            Ok(profile_switch(
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
                &on,
                &off,
            )
            .into_any())
        })
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let times = ["morning", "afternoon", "evening", "night"]
        .into_iter()
        .zip(page.times)
        .map(|(period, time)| {
            Ok((
                period,
                text.get(&format!("profiles.notifications.periods.{period}"), &[])?,
                time.unwrap_or_default(),
            ))
        })
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let managed_visible = !page.managed.is_empty();
    let managed_rows = page.managed.into_iter().map(|person| {
        let id = format!("managed-person-{}", person.id);
        let label = text.get("profiles.notifications.managed_people.toggle_label", &[("name", &person.name)])
            .unwrap_or_else(|_| person.name.clone());
        let control = if person.automatic {
            view! { <span class="profile-badge">{automatic.clone()}</span> }.into_any()
        } else {
            view! { <label for=id.clone()>{label}</label><input id=id type="checkbox" name="managed_person_ids[]" value=person.id.to_string() checked=person.selected disabled=!page.editable/> }.into_any()
        };
        view! { <div class="profile-managed-person"><span>{person.name}</span>{control}</div> }
    }).collect::<Vec<_>>();
    let csrf = page.csrf;
    let body = view! {
        <div class="profile-section-header" data-testid="profile-notifications-header"><span class="profile-section-icon" aria-hidden="true"><svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9"/><path d="M10 21h4"/></svg></span>
            <div><h2>{text.get("profiles.sections.notifications.title", &[])?}</h2><div class="profile-summary"><span>{reminder_summary}</span><span>{category_summary}</span></div></div>
        </div>
        <div class="profile-section-body profile-notifications-body"><section class="profile-info-card" data-testid="profile-notifications-card">
            <h3>{title}</h3><p>{description}</p>
            <div data-profile-push data-household-id=page.household_id.to_string() data-slug=page.slug data-csrf=csrf.clone()>
                <h4>{browser_title}</h4><p>{browser_description}</p>
                <div class="profile-push-status-box"><div class="profile-push-status-row"><p data-push-status role="status" aria-live="polite">{text.get("profiles.notifications.checking_status", &[])?}</p>
                        <div class="household-actions"><button type="button" data-push-on class="profile-secondary" disabled=!page.push_configured>{text.get("profiles.common.on", &[])?}</button>
                            <button type="button" data-push-off class="profile-secondary" disabled=!page.push_configured>{text.get("profiles.common.off", &[])?}</button></div></div>
                    <button type="button" data-push-test class="profile-secondary" hidden>{text.get("profiles.notifications.send_test_notification", &[])?}</button>
                </div>
            </div>
            {page.saved.then(|| view! { <p role="status" class="med-success">{"Notification preferences saved."}</p> })}
            {page.error.then(|| view! { <p role="alert" class="med-alert">{"Notification preferences could not be saved."}</p> })}
            <form class="household-form" method="post" action=action>
                <input type="hidden" name="authenticity_token" value=csrf/>
                {(!page.editable).then(|| view! { <p role="status">{text.get("notifications.read_only", &[]).unwrap_or_default()}</p> })}
                {profile_switch("enabled", master_label, master_hint, page.preferences.enabled, !page.editable, &on, &off)}
                <fieldset><legend>{categories_title}</legend><p>{categories_hint}</p>{categories.into_iter().collect_view()}</fieldset>
                {managed_visible.then(|| view! { <details class="profile-info-card" data-testid="managed-notification-people"><summary><strong>{managed_title}</strong><small>{managed_description}</small></summary>
                    <input type="hidden" name="managed_person_ids[]" value=""/>
                    {managed_rows.into_iter().collect_view()}
                </details> })}
                <details class="profile-info-card" data-testid="notification-delivery-times"><summary><strong>{times_title}</strong><small>{times_description}</small></summary>
                    <div class="profile-time-grid">{times.into_iter().map(|(period, label, time)| { let id = format!("notification_{period}_time"); view! { <div class="form-field"><label for=id.clone()>{label}</label><input id=id type="time" name=format!("{period}_time") value=time disabled=!page.editable/></div> } }).collect_view()}</div>
                </details>
                {page.editable.then(|| view! { <button type="submit" class="med-primary">{save}</button> })}
            </form>
        </section></div>
    };
    Ok(body.to_html())
}

fn profile_switch(
    name: &str,
    label: String,
    hint: String,
    checked: bool,
    disabled: bool,
    on: &str,
    off: &str,
) -> impl IntoView {
    let id = format!("notification_{name}");
    let hint_id = format!("{id}_hint");
    view! {
        <div class="profile-notification-row">
            <div class="profile-notification-copy"><label for=id.clone()>{label}</label><small id=hint_id.clone()>{hint}</small></div>
            <div class="profile-toggle">
                <input id=id name=name.to_owned() type="checkbox" value="true" checked=checked disabled=disabled aria-describedby=hint_id/>
                <span aria-hidden="true">{on.to_owned()}</span><span aria-hidden="true">{off.to_owned()}</span>
            </div>
        </div>
    }
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
