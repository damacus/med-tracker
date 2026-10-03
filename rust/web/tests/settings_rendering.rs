use medtracker_web::household_i18n::Locale;
use medtracker_web::settings::{SettingsPage, render_settings};
use std::collections::HashMap;

fn page(time_zone: &str) -> SettingsPage {
    SettingsPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        locale: Locale::En,
        csrf: "test-csrf".into(),
        time_zone: time_zone.into(),
        can_edit: true,
        errors: HashMap::new(),
        notice: String::new(),
    }
}

#[test]
fn read_only_profile_shows_timezone_without_a_save_control() {
    let mut read_only = page("Europe/London");
    read_only.can_edit = false;
    let html = render_settings(read_only).unwrap();
    assert!(html.contains("id=\"settings_time_zone\""));
    assert!(html.contains("disabled"));
    assert!(html.contains("Europe/London"));
    assert!(!html.contains("Save time zone"));
    assert!(html.contains("You do not have permission to change this time zone."));
}

fn selected_option<'a>(html: &'a str, value: &str) -> &'a str {
    html.split("<option")
        .skip(1)
        .find(|option| {
            option
                .split('>')
                .next()
                .unwrap()
                .contains(&format!("value=\"{value}\""))
        })
        .unwrap_or_else(|| panic!("missing option {value}"))
        .split('>')
        .next()
        .unwrap()
}

#[test]
fn settings_form_renders_a_labelled_supported_timezone_select_with_the_saved_choice() {
    let html = render_settings(page("Europe/London")).unwrap();
    assert!(html.contains("<h1"));
    assert!(html.contains("My Profile"));
    assert!(html.contains("action=\"/households/test/settings\""));
    assert!(html.contains("method=\"post\""));
    assert!(html.contains("name=\"authenticity_token\""));
    assert!(html.contains("value=\"test-csrf\""));
    assert!(html.contains("for=\"settings_time_zone\""));
    assert!(html.contains("id=\"settings_time_zone\""));
    assert!(html.contains("name=\"time_zone\""));
    assert!(html.contains("aria-describedby=\"settings-time-zone-hint\""));
    assert!(html.contains("id=\"settings-time-zone-hint\""));
    assert!(
        selected_option(&html, "Europe/London").contains("selected"),
        "saved timezone must be the selected option"
    );
    assert!(html.contains("value=\"UTC\""));
    assert!(html.contains("value=\"America/Los_Angeles\""));
    assert_eq!(html.matches(" selected").count(), 1);
}

#[test]
fn settings_form_keeps_an_unlisted_saved_timezone_as_the_current_choice() {
    let html = render_settings(page("Pacific Time (US & Canada)")).unwrap();
    assert!(!html.contains("Pacific Time (US & Canada)<"));
    assert!(html.contains("Pacific Time (US &amp; Canada)"));
    assert!(
        selected_option(&html, "Pacific Time (US &amp; Canada)").contains("selected"),
        "a stored value outside the supported list must stay selected"
    );
}

#[test]
fn settings_form_associates_inline_errors_and_retains_the_attempted_timezone() {
    let mut draft = page("Mars/Olympus <base>");
    draft.errors = HashMap::from([(
        "time_zone".into(),
        vec!["is not included in the list".into()],
    )]);
    let html = render_settings(draft).unwrap();
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(
        html.contains("aria-describedby=\"settings-time-zone-hint settings-time-zone-errors\"")
    );
    assert!(html.contains("id=\"settings-time-zone-errors\""));
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("is not included in the list"));
    assert!(!html.contains("Mars/Olympus <base>"));
    assert!(
        selected_option(&html, "Mars/Olympus &lt;base&gt;").contains("selected"),
        "the rejected selection must be retained and escaped"
    );
}

#[test]
fn settings_page_announces_a_saved_profile_without_an_error_state() {
    let mut saved = page("UTC");
    saved.notice = "Profile updated successfully.".into();
    let html = render_settings(saved).unwrap();
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("Profile updated successfully."));
    assert!(!html.contains("aria-invalid=\"true\""));
    assert!(!html.contains("role=\"alert\""));
}

#[test]
fn settings_form_uses_supported_locale_labels() {
    for (locale, heading, label, save) in [
        (Locale::En, "My Profile", "Time Zone", "Save time zone"),
        (
            Locale::Cy,
            "Fy Mhroffil",
            "Parth Amser",
            "Cadw'r parth amser",
        ),
        (
            Locale::Ga,
            "Mo Phróifíl",
            "Crios Ama",
            "Sábháil an crios ama",
        ),
        (
            Locale::Es,
            "Mi perfil",
            "Zona horaria",
            "Guardar zona horaria",
        ),
        (
            Locale::Pt,
            "O Meu Perfil",
            "Fuso horário",
            "Guardar fuso horário",
        ),
    ] {
        let mut draft = page("UTC");
        draft.locale = locale;
        let html = render_settings(draft).unwrap();
        assert!(html.contains(&format!("lang=\"{}\"", locale.as_str())));
        assert!(
            html.contains(heading),
            "missing {heading} for {}",
            locale.as_str()
        );
        assert!(
            html.contains(label),
            "missing {label} for {}",
            locale.as_str()
        );
        assert!(
            html.contains(save),
            "missing {save} for {}",
            locale.as_str()
        );
    }
}
