use crate::household_i18n::{Locale, Text};
use leptos::prelude::*;

pub fn household_document(
    title: &str,
    household_name: &str,
    slug: &str,
    locale: &str,
    body: String,
    notifications_visible: bool,
) -> String {
    let locale = Locale::resolve(Some(locale), None);
    let text = Text::new(locale);
    let prefix = format!("/households/{}", path_segment(slug));
    let nav = [
        ("dashboard", "layouts.sidebar.dashboard"),
        ("medications", "layouts.sidebar.inventory"),
        ("people", "layouts.sidebar.people"),
        ("locations", "layouts.sidebar.locations"),
        ("reports", "layouts.sidebar.reports"),
        ("settings/notifications", "notifications.title"),
    ];
    let navigation = view! {
        <nav class="med-sidebar" aria-label=text.get("layouts.mobile_rail.primary_navigation", &[]).unwrap_or_else(|_| "Primary navigation".into())>
            {nav.into_iter().filter(|(path, _)| {
                notifications_visible || *path != "settings/notifications"
            }).map(|(path, key)| {
                let label = text.get(key, &[]).unwrap_or_else(|_| path.to_owned());
                view! { <a href=format!("{prefix}/{path}")>{label}</a> }
            }).collect_view()}
        </nav>
    }.to_html();
    let heading = view! { <p class="med-eyebrow">{household_name.to_owned()}</p> }.to_html();
    let title = view! { <title>{format!("{title} | MedTracker")}</title> }.to_html();
    let skip_label = text
        .get("layouts.navigation.skip_to_content", &[])
        .unwrap_or_else(|_| "Skip to content".into());
    let skip =
        view! { <a class="household-skip" href="#household-content">{skip_label}</a> }.to_html();
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">{title}<link rel=\"stylesheet\" href=\"/medication.css\"><link rel=\"stylesheet\" href=\"/household.css\"></head><body>{skip}<div class=\"med-layout\">{navigation}<main class=\"med-content\" id=\"household-content\">{heading}{body}</main></div></body></html>",
        locale.as_str()
    )
}

pub fn path_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
