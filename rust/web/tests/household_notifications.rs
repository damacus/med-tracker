use medtracker_web::household::household_document;
use medtracker_web::household_i18n::{Locale, Text};
use medtracker_web::notifications::{
    NotificationDraft, NotificationPage, render_notification_settings,
};

fn page(draft: Option<NotificationDraft>, saved: bool, error: bool) -> NotificationPage {
    NotificationPage {
        household_name: "Test household".into(),
        slug: "test-house".into(),
        csrf: "test-csrf".into(),
        locale: Locale::En,
        preferences: draft,
        editable: true,
        saved,
        error,
        notifications_visible: true,
    }
}

fn checkbox(html: &str, name: &str) -> String {
    html.split("<input")
        .skip(1)
        .find(|input| {
            input
                .split('>')
                .next()
                .unwrap()
                .contains(&format!("name=\"{name}\""))
        })
        .unwrap_or_else(|| panic!("missing checkbox {name}"))
        .split('>')
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn first_use_defaults_match_notification_preference_storage() {
    let defaults = NotificationDraft::default();
    assert!(defaults.enabled);
    assert!(defaults.dose_due_enabled);
    assert!(defaults.missed_dose_enabled);
    assert!(defaults.low_stock_enabled);
    assert!(!defaults.private_text_enabled);
}

#[test]
fn view_only_preferences_show_values_without_a_save_action() {
    let mut view_only = page(Some(NotificationDraft::default()), false, false);
    view_only.editable = false;
    let html = render_notification_settings(view_only).unwrap();
    assert!(html.contains("You can view these preferences"));
    assert!(html.contains("role=\"status\""));
    assert!(!html.contains("type=\"submit\""));
    for name in [
        "enabled",
        "dose_due_enabled",
        "missed_dose_enabled",
        "low_stock_enabled",
        "private_text_enabled",
    ] {
        assert!(checkbox(&html, name).contains("disabled"));
    }
}

#[test]
fn notification_form_posts_five_labelled_switches_and_a_csrf_token() {
    let html = render_notification_settings(page(
        Some(NotificationDraft {
            enabled: true,
            dose_due_enabled: false,
            missed_dose_enabled: true,
            low_stock_enabled: false,
            private_text_enabled: true,
        }),
        false,
        false,
    ))
    .unwrap();
    assert!(html.contains("<h1"));
    assert!(html.contains("My notifications"));
    assert!(html.contains("action=\"/households/test-house/settings/notifications\""));
    assert!(html.contains("method=\"post\""));
    assert!(html.contains("name=\"authenticity_token\""));
    assert!(html.contains("value=\"test-csrf\""));
    for (name, checked) in [
        ("enabled", true),
        ("dose_due_enabled", false),
        ("missed_dose_enabled", true),
        ("low_stock_enabled", false),
        ("private_text_enabled", true),
    ] {
        let input = checkbox(&html, name);
        assert!(
            input.contains("type=\"checkbox\""),
            "{name} is not a checkbox"
        );
        assert_eq!(input.contains(" checked"), checked, "{name} checked state");
        let id = input
            .split("id=\"")
            .nth(1)
            .and_then(|rest| rest.split('\"').next())
            .unwrap_or_else(|| panic!("{name} has no id"));
        assert!(
            html.contains(&format!("for=\"{id}\"")),
            "{name} has no associated label"
        );
    }
    assert!(html.contains("Save preferences"));
    assert!(html.contains("Dose Due Reminders"));
    assert!(html.contains("Missed Dose Reminders"));
    assert!(html.contains("Low Stock Warnings"));
    assert!(html.contains("Private Notification Content"));
    assert!(html.contains("Hide sensitive information"));
}

#[test]
fn notification_form_keeps_editable_categories_when_the_master_switch_is_off() {
    let html = render_notification_settings(page(
        Some(NotificationDraft {
            enabled: false,
            dose_due_enabled: true,
            missed_dose_enabled: false,
            low_stock_enabled: true,
            private_text_enabled: false,
        }),
        false,
        false,
    ))
    .unwrap();
    for (name, checked) in [
        ("enabled", false),
        ("dose_due_enabled", true),
        ("missed_dose_enabled", false),
        ("low_stock_enabled", true),
        ("private_text_enabled", false),
    ] {
        let input = checkbox(&html, name);
        assert_eq!(input.contains(" checked"), checked, "{name} checked state");
        assert!(!input.contains("disabled"), "{name} must stay editable");
    }
}

#[test]
fn unavailable_preferences_render_a_neutral_state_without_editable_defaults() {
    let html = render_notification_settings(page(None, false, false)).unwrap();
    assert!(html.contains("My notifications"));
    assert!(!html.contains("name=\"enabled\""));
    assert!(!html.contains("method=\"post\""));
    assert!(!html.contains("authenticity_token"));
    assert!(html.contains("role=\"status\""));
}

#[test]
fn saved_preferences_render_an_accessible_success_region() {
    let html = render_notification_settings(page(
        Some(NotificationDraft {
            enabled: true,
            dose_due_enabled: true,
            missed_dose_enabled: true,
            low_stock_enabled: true,
            private_text_enabled: true,
        }),
        true,
        false,
    ))
    .unwrap();
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("Notification settings saved."));
}

#[test]
fn failed_save_keeps_attempted_values_with_an_associated_error_and_no_success() {
    let html = render_notification_settings(page(
        Some(NotificationDraft {
            enabled: false,
            dose_due_enabled: true,
            missed_dose_enabled: true,
            low_stock_enabled: false,
            private_text_enabled: false,
        }),
        false,
        true,
    ))
    .unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("Failed to save notification settings."));
    assert!(!html.contains("Notification settings saved."));
    assert!(checkbox(&html, "dose_due_enabled").contains(" checked"));
    assert!(!checkbox(&html, "enabled").contains(" checked"));
    let alert = html
        .split("<div")
        .skip(1)
        .find(|tag| tag.contains("role=\"alert\""))
        .expect("failed-save alert");
    assert!(
        alert.contains("id=\"notification-preferences-error\""),
        "failed-save alert must carry a stable id"
    );
    let form = html
        .split("<form")
        .nth(1)
        .and_then(|rest| rest.split('>').next())
        .expect("notification form");
    assert!(
        form.contains("aria-describedby=\"notification-preferences-error\""),
        "form must reference the failed-save alert"
    );
    let clean =
        render_notification_settings(page(Some(NotificationDraft::default()), false, false))
            .unwrap();
    let clean_form = clean
        .split("<form")
        .nth(1)
        .and_then(|rest| rest.split('>').next())
        .expect("notification form");
    assert!(!clean_form.contains("notification-preferences-error"));
}

#[test]
fn notification_settings_copy_exists_in_every_authoritative_locale() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        for key in [
            "notifications.title",
            "notifications.description",
            "notifications.enabled_label",
            "notifications.enabled_hint",
            "notifications.categories_hint",
            "notifications.unavailable",
            "notifications.read_only",
            "notifications.save",
            "notification_settings.categories.title",
            "notification_settings.categories.dose_due_enabled.title",
            "notification_settings.categories.dose_due_enabled.description",
            "notification_settings.categories.missed_dose_enabled.title",
            "notification_settings.categories.missed_dose_enabled.description",
            "notification_settings.categories.low_stock_enabled.title",
            "notification_settings.categories.low_stock_enabled.description",
            "notification_settings.categories.private_text_enabled.title",
            "notification_settings.categories.private_text_enabled.description",
            "notification_preferences.updated",
            "notification_preferences.update_failed",
        ] {
            assert_eq!(
                text.source_locale(key).unwrap(),
                locale,
                "{key} must be translated for {}",
                locale.as_str()
            );
            assert!(!text.get(key, &[]).unwrap().is_empty());
        }
        let html = render_notification_settings(NotificationPage {
            household_name: "Test".into(),
            slug: "test".into(),
            csrf: "csrf".into(),
            locale,
            preferences: Some(NotificationDraft {
                enabled: true,
                dose_due_enabled: true,
                missed_dose_enabled: true,
                low_stock_enabled: true,
                private_text_enabled: true,
            }),
            editable: true,
            saved: false,
            error: false,
            notifications_visible: true,
        })
        .unwrap();
        assert!(html.contains(&format!("lang=\"{}\"", locale.as_str())));
        assert!(!html.contains("My notifications") || locale == Locale::En);
    }
}

#[test]
fn household_shell_links_to_my_notifications() {
    let html = household_document("People", "Test", "test-house", "en", String::new(), true);
    assert!(html.contains("href=\"/households/test-house/settings/notifications\""));
    assert!(html.contains("My notifications"));
}

#[test]
fn household_shell_hides_my_notifications_without_read_access() {
    let html = household_document("People", "Test", "test-house", "en", String::new(), false);
    assert!(!html.contains("settings/notifications"));
    assert!(!html.contains("My notifications"));
    assert!(html.contains("href=\"/households/test-house/people\""));
}
