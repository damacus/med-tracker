use medtracker_web::household_i18n::Locale;
use medtracker_web::settings::{SettingsPage, render_settings};
use std::collections::HashMap;

fn page(time_zone: &str) -> SettingsPage {
    SettingsPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        household_id: 7,
        locale: Locale::En,
        csrf: "test-csrf".into(),
        person_name: "Alex Taylor".into(),
        person_id: 42,
        email: "alex@example.test".into(),
        date_of_birth: Some("1990-02-03".into()),
        age: Some(36),
        person_type: "adult".into(),
        has_capacity: true,
        mobile_shortcuts: vec!["dashboard".into(), "inventory".into(), "finder".into()],
        gravatar_enabled: false,
        avatar_attached: false,
        gravatar_url: None,
        vapid_public_key: None,
        active_section: "profile".into(),
        security_html: String::new(),
        notifications_html: String::new(),
        advanced_html: String::new(),
        time_zone: time_zone.into(),
        can_edit: true,
        errors: HashMap::new(),
        notice: String::new(),
    }
}

#[test]
fn profile_page_matches_the_rails_profile_structure() {
    let html = render_settings(page("Europe/London")).unwrap();
    assert!(html.contains("data-testid=\"profile-hero\""));
    assert!(html.contains("id=\"profile-tab-profile\""));
    assert!(html.contains("id=\"profile-tab-security\""));
    assert!(html.contains("id=\"profile-tab-notifications\""));
    assert!(html.contains("id=\"profile-tab-advanced\""));
    assert!(html.contains("id=\"profile-tab-profile\" role=\"tab\" aria-selected=\"true\" aria-controls=\"profile-profile-panel\" tabindex=\"0\""));
    assert!(html.contains("id=\"profile-tab-security\" role=\"tab\" aria-selected=\"false\" aria-controls=\"profile-security-panel\" tabindex=\"-1\""));
    assert!(html.contains("data-testid=\"profile-personal-info-card\""));
    assert!(html.contains("data-testid=\"profile-time-zone-dialog\""));
    assert!(html.contains("class=\"profile-shell\""));
    assert!(html.contains("class=\"profile-mobile-topbar\""));
    assert!(html.contains("aria-label=\"Open menu\""));
    assert!(html.contains("aria-label=\"Search dashboard\""));
    assert!(html.contains("id=\"profile-mobile-menu\""));
    assert!(html.contains("class=\"profile-mobile-rail\""));
    for class in [
        "profile-section-icon",
        "profile-nav-icon",
        "profile-rail-icon",
    ] {
        let fragment = html
            .split(class)
            .nth(1)
            .unwrap()
            .split("</span>")
            .next()
            .unwrap();
        assert!(fragment.contains("<svg"), "{class}: {fragment}");
    }
    assert!(html.contains("Alex Taylor"));
    assert!(html.contains("alex@example.test"));
    assert!(html.contains("February 03, 1990"));
    assert!(html.contains("<dd>Adult</dd>"));
}

#[test]
fn profile_shell_renders_for_every_supported_locale() {
    for locale in Locale::ALL {
        let mut fixture = page("London");
        fixture.locale = locale;
        fixture.security_html = "<section>Security</section>".into();
        fixture.notifications_html = "<section>Notifications</section>".into();
        fixture.advanced_html = "<section>Advanced</section>".into();
        let html = render_settings(fixture).expect("full profile shell");
        assert!(html.contains("<section>Security</section>"));
        assert!(html.contains("<section>Notifications</section>"));
        assert!(html.contains("<section>Advanced</section>"));
    }
}

#[test]
fn timezone_picker_includes_the_saved_rails_label() {
    let html = render_settings(page("London")).unwrap();
    assert!(html.contains("<option value=\"London\" selected>London</option>"));
    assert!(html.contains("Pacific Time (US &amp; Canada)"));
}

#[test]
fn rust_pages_use_the_rails_profile_font() {
    assert!(include_str!("../src/medication.css").contains("Plus Jakarta Sans"));
    assert!(include_str!("../src/dashboard.css").contains("Plus Jakarta Sans"));
}

#[test]
fn profile_settings_have_working_avatar_shortcut_and_appearance_controls() {
    let html = render_settings(page("Europe/London")).unwrap();
    for dialog in [
        "profile-avatar-modal",
        "profile-time-zone-modal",
        "profile-shortcuts-modal",
        "profile-appearance-modal",
    ] {
        assert!(html.contains(&format!("id=\"{dialog}\"")));
    }
    assert!(html.contains("name=\"mobile_shortcuts_0\""));
    assert!(html.contains("name=\"gravatar_enabled\""));
    assert!(html.contains("data-appearance=\"dark\""));
    assert!(html.contains("data-theme=\"serene-sage\""));
}

#[test]
fn profile_dialogs_name_their_headings_for_assistive_technology() {
    let html = render_settings(page("Europe/London")).unwrap();
    for (dialog, heading) in [
        ("profile-time-zone-modal", "profile-time-zone-heading"),
        ("profile-avatar-modal", "profile-avatar-heading"),
        ("profile-shortcuts-modal", "profile-shortcuts-heading"),
        ("profile-appearance-modal", "profile-appearance-heading"),
    ] {
        assert!(html.contains(&format!("id=\"{dialog}\"")));
        assert!(html.contains(&format!("aria-labelledby=\"{heading}\"")));
        assert!(html.contains(&format!("<h2 id=\"{heading}\"")));
    }
}

#[test]
fn mobile_rail_uses_only_saved_shortcuts_with_rust_destinations() {
    let mut with_shortcuts = page("Europe/London");
    with_shortcuts.mobile_shortcuts = vec!["dashboard".into(), "inventory".into(), "finder".into()];
    let html = render_settings(with_shortcuts).unwrap();
    let rail = html
        .split("class=\"profile-mobile-rail\"")
        .nth(1)
        .unwrap()
        .split("</nav>")
        .next()
        .unwrap();
    assert!(rail.contains("href=\"/households/test/dashboard\""));
    assert!(rail.contains("href=\"/households/test/medications\""));
    assert!(!rail.contains("href=\"/households/test/finder\""));
    assert!(!rail.contains("href=\"/households/test/profile\""));
    assert!(html.contains("value=\"finder\" selected"));
}

#[test]
fn attached_avatar_uses_the_authenticated_profile_image_route() {
    let mut with_avatar = page("Europe/London");
    with_avatar.avatar_attached = true;
    let html = render_settings(with_avatar).unwrap();
    assert!(html.contains("src=\"/api/v1/households/7/profile/avatar\""));
    assert!(!html.contains("/people/42/avatar"));
}

#[test]
fn opted_in_gravatar_has_an_initials_fallback() {
    let mut with_gravatar = page("Europe/London");
    with_gravatar.gravatar_enabled = true;
    with_gravatar.gravatar_url = Some("https://www.gravatar.com/avatar/example?d=404&s=64".into());
    let html = render_settings(with_gravatar).unwrap();
    assert!(html.contains("src=\"https://www.gravatar.com/avatar/example?d=404&amp;s=64\""));
    assert!(html.contains("data-profile-avatar-image"));
    assert!(html.contains("<span>AT</span>"));
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
    assert!(html.contains("action=\"/households/test/profile\""));
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
    assert!(html.contains("value=\"Pacific Time (US &amp; Canada)\""));
    let time_zone_select = html
        .split("<select id=\"settings_time_zone\"")
        .nth(1)
        .unwrap()
        .split("</select>")
        .next()
        .unwrap();
    assert_eq!(time_zone_select.matches(" selected").count(), 1);
}

#[test]
fn settings_form_keeps_a_saved_iana_timezone_as_the_current_choice() {
    let html = render_settings(page("Europe/London")).unwrap();
    assert!(
        selected_option(&html, "Europe/London").contains("selected"),
        "an existing IANA preference must stay selected"
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
    assert!(!html.contains("id=\"settings-time-zone-errors\""));
    assert!(!html.contains("<ul class=\"med-alert\""));
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
