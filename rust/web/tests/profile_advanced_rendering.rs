use medtracker_web::household_i18n::Locale;
use medtracker_web::profile_advanced::{AdvancedPage, TokenSummary, Variants, render_advanced};

fn page() -> AdvancedPage {
    AdvancedPage {
        locale: Locale::En,
        slug: "family name".to_owned(),
        csrf: "contract-csrf".to_owned(),
        membership_id: 17,
        can_edit: true,
        can_manage_tokens: true,
        tokens: vec![TokenSummary {
            id: 42,
            name: "Kitchen tablet".to_owned(),
            last_used_at: "2026-10-03".to_owned(),
            expires_at: "2026-11-03".to_owned(),
        }],
        variants: Variants {
            wizard: "modal".to_owned(),
            dashboard: "family_lanes".to_owned(),
            medication_launcher: "context_aware".to_owned(),
        },
        version: "0.5.35".to_owned(),
        notice: None,
        new_token: None,
    }
}

#[test]
fn one_time_token_is_visible_and_html_escaped() {
    let mut page = page();
    page.new_token = Some("mt_app_<unsafe>".to_owned());
    let html = render_advanced(page).expect("one-time token page");
    assert!(html.contains("mt_app_&lt;unsafe&gt;"));
    assert!(!html.contains("mt_app_<unsafe>"));
}

#[test]
fn advanced_section_has_live_token_export_experiment_and_closure_controls() {
    let html = render_advanced(page()).expect("Advanced section");
    assert!(html.contains("data-testid=\"profile-advanced-header\""));
    assert!(html.contains("1 active API tokens"));
    assert!(html.contains("Data export available"));
    assert!(html.contains("data-profile-dialog=\"profile-close-modal\""));
    assert!(html.contains("<section class=\"profile-danger-zone\"><h3>Danger Zone</h3><div class=\"profile-danger-row profile-info-card\""));
    assert!(html.contains("<dialog id=\"profile-close-modal\""));
    assert!(html.contains("aria-labelledby=\"profile-close-title\""));
    assert!(html.contains("<h2 id=\"profile-close-title\""));
    assert!(html.contains("data-profile-close=\"profile-close-modal\""));
    assert!(html.contains("Closing your account removes your sign-in and household access."));
    assert!(!html.contains("Permanently delete your account and all associated data"));
    assert!(html.contains("/households/family%20name/profile/api_tokens"));
    assert!(html.contains("Kitchen tablet"));
    assert!(html.contains("/households/family%20name/profile/data_exports/health_data_json"));
    assert!(html.contains("/households/family%20name/profile/data_exports/backup_zip"));
    assert!(html.contains("name=\"wizard_variant\""));
    assert!(html.contains("name=\"dashboard_variant\""));
    assert!(html.contains("name=\"medication_launcher_variant\""));
    assert!(html.contains("/households/family%20name/profile/close_account"));
    assert!(html.contains("type=\"password\""));
    assert!(html.contains("v0.5.35"));
    assert!(html.contains("https://damacus.github.io/med-tracker"));
}

#[test]
fn advanced_section_disables_mutations_without_profile_permission() {
    let mut page = page();
    page.can_edit = false;
    page.can_manage_tokens = false;
    let html = render_advanced(page).expect("read-only Advanced section");
    assert!(!html.contains("name=\"api_app_token[name]\""));
    assert!(!html.contains("name=\"wizard_variant\""));
    assert!(html.contains("profile/close_account"));
    assert!(html.contains("Kitchen tablet"));
    assert!(html.contains("data_exports/health_data_json"));
}

#[test]
fn advanced_section_renders_for_every_supported_locale() {
    for locale in Locale::ALL {
        let mut page = page();
        page.locale = locale;
        let html = render_advanced(page).expect("translated Advanced section");
        assert!(html.contains("data-testid=\"profile-advanced-section\""));
        assert!(html.contains("name=\"password\""));
    }
}
