#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_section_shows_account_and_factor_management() {
        let html = render_security_section(SecurityPage {
            locale: Locale::En,
            slug: "test house".into(),
            csrf: "token".into(),
            email: "person@example.test".into(),
            totp_enabled: true,
            recovery_codes_count: 4,
            passkeys: vec![SecurityPasskey {
                id: 7,
                nickname: "Phone key".into(),
                added_on: "2026-10-03".into(),
            }],
            status: None,
        })
        .unwrap();
        assert!(html.contains("Account Security"));
        assert!(html.contains("profile-section-header"));
        assert!(html.contains("2FA: On"));
        assert!(html.contains("1 passkeys"));
        assert!(html.contains("Change Email Address"));
        assert!(html.contains("Change Password"));
        assert!(html.contains("Authenticator App (TOTP)"));
        assert!(html.contains("Recovery Codes"));
        assert!(html.contains("Passkeys"));
        assert!(html.contains("Phone key"));
        assert!(html.contains("/households/test%20house/settings/security"));
        assert!(html.contains("name=\"nickname\""));
        assert!(
            !html.contains("href=\"/households/test%20house/settings/security/passkeys/7/remove\"")
        );
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn security_section_escapes_untrusted_passkey_nicknames() {
        let html = render_security_section(SecurityPage {
            locale: Locale::En,
            slug: "test".into(),
            csrf: "token".into(),
            email: "person@example.test".into(),
            totp_enabled: false,
            recovery_codes_count: 0,
            passkeys: vec![SecurityPasskey {
                id: 7,
                nickname: "<img src=x onerror=alert(1)>".into(),
                added_on: "2026-10-03".into(),
            }],
            status: None,
        })
        .unwrap();
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;img"));
    }

    #[test]
    fn security_cards_match_the_account_and_two_factor_hierarchy() {
        let html = render_security_section(SecurityPage {
            locale: Locale::En,
            slug: "house".into(),
            csrf: "csrf".into(),
            email: "person@example.test".into(),
            totp_enabled: false,
            recovery_codes_count: 0,
            passkeys: vec![SecurityPasskey {
                id: 7,
                nickname: "Phone key".into(),
                added_on: "October 02, 2026".into(),
            }],
            status: None,
        })
        .unwrap();
        assert_eq!(
            html.matches("class=\"profile-security-account-row\"")
                .count(),
            2
        );
        assert!(html.contains("Update the email address you use to sign in"));
        assert!(html.contains("<h3>Two-Factor Authentication</h3>"));
        assert_eq!(
            html.matches("class=\"profile-security-factor-row\"")
                .count(),
            2
        );
        assert!(html.contains("class=\"profile-security-passkey-row\""));
        assert!(html.contains("Phone key"));
        assert!(html.contains("October 02, 2026"));
        assert!(html.contains("class=\"profile-security-remove\""));
        assert!(html.contains("Add a passkey"));
        assert!(html.contains("aria-labelledby=\"security-email-heading\""));
        assert!(html.contains("aria-labelledby=\"security-password-heading\""));
        assert!(html.contains("aria-labelledby=\"security-passkey-remove-7-heading\""));
    }

    #[test]
    fn otp_setup_page_shows_secret_and_requires_password_and_code() {
        let html = render_totp_setup_page(
            "house",
            "csrf",
            "abcdefghijklmnop",
            "DERIVEDSECRET",
            "otpauth://totp/MedTracker:person%40example.test?secret=DERIVEDSECRET",
        );
        assert!(html.contains("DERIVEDSECRET"));
        assert!(!html.contains("<code>abcdefghijklmnop</code>"));
        assert!(html.contains("Current password"));
        assert!(html.contains("Authenticator code"));
        assert!(html.contains("otpauth://totp/"));
        assert!(html.contains("name=\"authenticity_token\""));
    }

    #[test]
    fn recovery_page_shows_codes_only_on_the_authenticated_view() {
        let html = render_recovery_codes_page("house", &["safe-code".into()]);
        assert!(html.contains("safe-code"));
        assert!(html.contains("Recovery codes"));
        assert!(html.contains("Back to Security"));
    }

    #[test]
    fn recovery_prompt_requires_password_without_exposing_codes() {
        let html = render_recovery_prompt_page("house", "csrf-token");
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("href=\"/auth.css\""));
        assert!(html.contains("src=\"/profile.js\""));
        assert!(html.contains("action=\"/households/house/settings/security/recovery\""));
        assert!(html.contains("name=\"authenticity_token\" value=\"csrf-token\""));
        assert!(html.contains("name=\"password\" type=\"password\""));
        assert!(!html.contains("private-code"));
    }

    #[test]
    fn first_use_recovery_codes_offer_generation() {
        let html = render_security_section(SecurityPage {
            locale: Locale::En,
            slug: "house".into(),
            csrf: "csrf".into(),
            email: "person@example.test".into(),
            totp_enabled: true,
            recovery_codes_count: 0,
            passkeys: vec![],
            status: None,
        })
        .unwrap();
        assert!(html.contains("Generate recovery codes"));
        assert!(html.contains("data-profile-dialog=\"security-recovery-dialog\""));
        assert!(!html.contains("href=\"/households/house/settings/security/recovery\""));
    }

    #[test]
    fn passkey_registration_page_exposes_browser_credential_ceremony() {
        let html = render_passkey_registration_page(
            "house",
            "csrf",
            r#"{"publicKey":{"challenge":"YQ"}}"#,
            "signed-claim",
        );
        assert!(html.contains("/profile-security.js"));
        assert!(html.contains("data-passkey-options"));
        assert!(html.contains("name=\"webauthn_credential\""));
        assert!(html.contains("name=\"registration_claim\""));
        assert!(html.contains("Current password"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn security_subflows_load_the_profile_font_and_dialog_bootstrap() {
        let pages = [
            render_totp_setup_page("house", "csrf", "seed", "DERIVED", "otpauth://totp/example"),
            render_recovery_codes_page("house", &["private-code".into()]),
            render_passkey_registration_page(
                "house",
                "csrf",
                r#"{"publicKey":{"challenge":"YQ"}}"#,
                "signed",
            ),
        ];
        for html in pages {
            assert!(html.starts_with("<!doctype html>"));
            assert!(html.contains("href=\"/auth.css\""));
            assert!(html.contains("src=\"/profile.js\""));
            assert!(html.contains("class=\"auth-shell\""));
        }
    }

    #[test]
    fn security_subflows_use_the_auth_form_input_and_button_styles() {
        let otp =
            render_totp_setup_page("house", "csrf", "seed", "DERIVED", "otpauth://totp/example");
        let recovery = render_recovery_prompt_page("house", "csrf");
        let passkey = render_passkey_registration_page(
            "house",
            "csrf",
            r#"{"publicKey":{"challenge":"YQ"}}"#,
            "signed",
        );
        for html in [otp, recovery, passkey.clone()] {
            assert!(html.contains("class=\"auth-form\""));
            assert!(html.contains("class=\"primary-button\""));
            assert!(html.contains("class=\"form-field\""));
        }
        assert_eq!(passkey.matches("class=\"form-field\"").count(), 2);
    }

    #[test]
    fn email_confirmation_uses_the_profile_document_and_preserves_token_fields() {
        let html = render_email_verification_page("house", 7, "a-token");
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("href=\"/auth.css\""));
        assert!(html.contains("src=\"/profile.js\""));
        assert!(html.contains("class=\"auth-shell\""));
        assert!(html.contains("name=\"account_id\" value=\"7\""));
        assert!(html.contains("name=\"token\" value=\"a-token\""));
    }

    #[test]
    fn security_controls_follow_all_profile_locales() {
        for locale in Locale::ALL {
            let html = render_security_section(SecurityPage {
                locale,
                slug: "house".into(),
                csrf: "csrf".into(),
                email: "person@example.test".into(),
                totp_enabled: false,
                recovery_codes_count: 0,
                passkeys: vec![],
                status: None,
            })
            .unwrap();
            let text = Text::new(locale);
            assert!(html.contains(&text.get("profiles.account_security.title", &[]).unwrap()));
            assert!(
                html.contains(
                    &text
                        .get("rodauth.views.two_factor_manage.methods.totp_title", &[])
                        .unwrap()
                )
            );
            for key in [
                "profiles.sections.security.methods_title",
                "profiles.sections.security.methods_description",
                "profiles.sections.security.totp_description",
                "profiles.sections.security.recovery_description",
                "profiles.sections.security.passkeys_description",
                "profiles.sections.security.passkeys_setup",
            ] {
                assert!(
                    html.contains(&text.get(key, &[]).unwrap()),
                    "missing {key} for {locale:?}"
                );
            }
        }
    }
}
use crate::auth::BrandPanel;
use crate::document::document;
use crate::household::path_segment;
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

#[derive(Clone)]
pub struct SecurityPasskey {
    pub id: i64,
    pub nickname: String,
    pub added_on: String,
}

pub struct SecurityPage {
    pub locale: Locale,
    pub slug: String,
    pub csrf: String,
    pub email: String,
    pub totp_enabled: bool,
    pub recovery_codes_count: usize,
    pub passkeys: Vec<SecurityPasskey>,
    pub status: Option<String>,
}

pub fn render_email_verification_page(slug: &str, account_id: i64, token: &str) -> String {
    let action = format!(
        "/households/{}/settings/security/email/verify",
        path_segment(slug)
    );
    document("Confirm email change", view! {
        <main class="auth-page profile-security-email"><div class="auth-shell"><BrandPanel/><section class="form-panel">
            <h1>"Confirm email change"</h1>
            <p>"Confirm the new email address for your MedTracker account."</p>
            <form method="post" action=action class="auth-form">
                <input type="hidden" name="account_id" value=account_id.to_string()/>
                <input type="hidden" name="token" value=token.to_owned()/>
                <button type="submit" class="primary-button">"Confirm email address"</button>
            </form>
        </section></div></main>
    }.to_html())
}

pub fn render_totp_setup_page(
    slug: &str,
    csrf: &str,
    seed: &str,
    authenticator_secret: &str,
    uri: &str,
) -> String {
    let action = format!("/households/{}/settings/security/otp", path_segment(slug));
    document("Set up authenticator app", view! {
        <main class="auth-page profile-security-setup"><div class="auth-shell"><BrandPanel/><section class="form-panel">
            <h1>"Set up authenticator app"</h1>
            <p>"Add this secret to your authenticator app, then enter its six-digit code."</p>
            <p><strong>"Secret: "</strong><code>{authenticator_secret.to_owned()}</code></p>
            <p><a href=uri.to_owned()>"Open in authenticator app"</a></p>
            <form method="post" action=action class="auth-form">
                <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                <input type="hidden" name="secret" value=seed.to_owned()/>
                <div class="form-field"><label for="otp-password">"Current password"</label><input id="otp-password" name="password" type="password" autocomplete="current-password" required/></div>
                <div class="form-field"><label for="otp-code">"Authenticator code"</label><input id="otp-code" name="code" inputmode="numeric" autocomplete="one-time-code" pattern="[0-9]{6}" required/></div>
                <button type="submit" class="primary-button">"Enable authenticator app"</button>
            </form>
        </section></div></main>
    }.to_html())
}

pub fn render_recovery_prompt_page(slug: &str, csrf: &str) -> String {
    let action = format!(
        "/households/{}/settings/security/recovery",
        path_segment(slug)
    );
    let back = format!(
        "/households/{}/profile?section=security",
        path_segment(slug)
    );
    document("View recovery codes", view! {
        <main class="auth-page profile-security-recovery"><div class="auth-shell"><BrandPanel/><section class="form-panel">
            <h1>"View recovery codes"</h1>
            <p>"Enter your current password to view your remaining recovery codes."</p>
            <form method="post" action=action class="auth-form">
                <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                <div class="form-field"><label for="recovery-password">"Current password"</label><input id="recovery-password" name="password" type="password" autocomplete="current-password" required/></div>
                <button type="submit" class="primary-button">"View recovery codes"</button>
            </form>
            <a href=back>"Back to Security"</a>
        </section></div></main>
    }.to_html())
}

pub fn render_recovery_codes_page(slug: &str, codes: &[String]) -> String {
    let back = format!(
        "/households/{}/profile?section=security",
        path_segment(slug)
    );
    document("Recovery codes", view! {
        <main class="auth-page profile-security-recovery"><div class="auth-shell"><BrandPanel/><section class="form-panel">
            <h1>"Recovery codes"</h1>
            <p>"Keep these codes in a secure place. Each code works once."</p>
            <ol>{codes.iter().map(|code| view! { <li><code>{code.clone()}</code></li> }).collect_view()}</ol>
            <a href=back>"Back to Security"</a>
        </section></div></main>
    }.to_html())
}

pub fn render_passkey_registration_page(
    slug: &str,
    csrf: &str,
    options_json: &str,
    signed_claim: &str,
) -> String {
    let action = format!(
        "/households/{}/settings/security/passkeys",
        path_segment(slug)
    );
    let back = format!(
        "/households/{}/profile?section=security",
        path_segment(slug)
    );
    let page = document("Add a passkey", view! {
        <main class="auth-page profile-security-passkey"><div class="auth-shell"><BrandPanel/><section class="form-panel">
            <h1>"Add a passkey"</h1>
            <p>"Use your device or security key to create a passkey."</p>
            <form method="post" action=action class="auth-form" data-passkey-options=options_json.to_owned() data-profile-passkey-register="true">
                <input type="hidden" name="authenticity_token" value=csrf.to_owned()/>
                <input type="hidden" name="registration_claim" value=signed_claim.to_owned()/>
                <input type="hidden" name="webauthn_credential" value=""/>
                <div class="form-field"><label for="passkey-nickname">"Nickname"</label><input id="passkey-nickname" name="nickname" maxlength="100" required/></div>
                <div class="form-field"><label for="passkey-password">"Current password"</label><input id="passkey-password" name="password" type="password" autocomplete="current-password" required/></div>
                <button type="button" class="primary-button" data-passkey-create="true">"Create passkey"</button>
                <p role="alert" data-passkey-error="true"></p>
            </form>
            <a href=back>"Back to Security"</a>
        </section></div></main>
    }.to_html());
    page.replacen(
        "</head>",
        "<script defer src=\"/profile-security.js\"></script></head>",
        1,
    )
}

pub fn render_security_section(page: SecurityPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let status = match page.status.as_deref() {
        Some("password_changed") => Some((
            text.get("profiles.sections.security.password_updated", &[])?,
            false,
        )),
        Some("password_invalid") => Some((
            "Check your current password and new password requirements.".to_owned(),
            true,
        )),
        Some("email_requested") => Some((
            text.get("profiles.email_change_requires_verification", &[])?,
            false,
        )),
        Some("email_changed") => Some((text.get("profiles.updated", &[])?, false)),
        Some("email_invalid") => Some((
            "Check your email address and current password.".to_owned(),
            true,
        )),
        Some("otp_enabled") => Some((
            text.get("rodauth.views.two_factor_manage.status_enabled", &[])?,
            false,
        )),
        Some("otp_disabled") => Some((
            text.get("profiles.sections.security.two_factor_off", &[])?,
            false,
        )),
        Some("otp_invalid") => Some((
            "Check your password and authentication code.".to_owned(),
            true,
        )),
        Some("recovery_regenerated") => Some((
            text.get("rodauth.views.add_recovery_codes.notice_title", &[])?,
            false,
        )),
        Some("passkey_added") => {
            Some((text.get("rodauth.views.webauthn_setup.submit", &[])?, false))
        }
        Some("passkey_renamed") => Some((
            text.get(
                "rodauth.views.two_factor_manage.methods.passkeys_manage",
                &[],
            )?,
            false,
        )),
        Some("passkey_removed") => Some((
            text.get("rodauth.views.webauthn_remove.card_title", &[])?,
            false,
        )),
        _ => None,
    };
    let base = format!("/households/{}/settings/security", path_segment(&page.slug));
    let email_action = format!("{base}/email");
    let password_action = format!("{base}/password");
    let otp_setup = format!("{base}/otp/new");
    let otp_disable = format!("{base}/otp/disable");
    let recovery = format!("{base}/recovery");
    let recovery_generate = format!("{base}/recovery/generate");
    let passkey_setup = format!("{base}/passkeys/new");
    let account_title = text.get("profiles.account_security.title", &[])?;
    let account_description = text.get("profiles.account_security.description", &[])?;
    let email_title = text.get("profiles.account_security.change_email_title", &[])?;
    let email_description = text.get("profiles.account_security.change_email_description", &[])?;
    let password_title = text.get("profiles.account_security.change_password_title", &[])?;
    let password_description =
        text.get("profiles.account_security.change_password_description", &[])?;
    let methods_title = text.get("profiles.sections.security.methods_title", &[])?;
    let methods_description = text.get("profiles.sections.security.methods_description", &[])?;
    let totp_title = text.get("rodauth.views.two_factor_manage.methods.totp_title", &[])?;
    let totp_description = text.get("profiles.sections.security.totp_description", &[])?;
    let totp_setup_label = text.get("profiles.sections.security.totp_setup", &[])?;
    let totp_disable_label = text.get("profiles.sections.security.totp_disable", &[])?;
    let recovery_title = text.get(
        "rodauth.views.two_factor_manage.methods.recovery_codes_title",
        &[],
    )?;
    let recovery_description = text.get("profiles.sections.security.recovery_description", &[])?;
    let recovery_view_label = text.get("profiles.sections.security.recovery_view", &[])?;
    let passkeys_title = text.get(
        "rodauth.views.two_factor_manage.methods.passkeys_title",
        &[],
    )?;
    let passkeys_description = text.get("profiles.sections.security.passkeys_description", &[])?;
    let passkeys_setup_label = text.get("profiles.sections.security.passkeys_setup", &[])?;
    let nickname_label = text.get("rodauth.views.webauthn_setup.nickname_label", &[])?;
    let password_label = text.get("profiles.password_sheet.current_password_label", &[])?;
    let update_password_label = text.get("profiles.password_sheet.submit", &[])?;
    let change_label = text.get("profiles.account_security.change_button", &[])?;
    let new_password_label = text.get("profiles.password_sheet.new_password_label", &[])?;
    let confirm_password_label = text.get("profiles.password_sheet.confirm_password_label", &[])?;
    let totp_status = if page.totp_enabled {
        text.get("profiles.sections.security.totp_active", &[])?
    } else {
        text.get("profiles.sections.security.not_configured", &[])?
    };
    let recovery_status = if page.recovery_codes_count == 0 {
        text.get("profiles.sections.security.not_generated", &[])?
    } else {
        text.get(
            "profiles.sections.security.recovery_codes_available",
            &[("count", &page.recovery_codes_count.to_string())],
        )?
    };
    let recovery_action_label = if page.recovery_codes_count == 0 {
        text.get("profiles.sections.security.recovery_generate", &[])?
    } else {
        text.get("profiles.sections.security.recovery_regenerate", &[])?
    };
    let no_passkeys = text.get("profiles.sections.security.no_passkeys", &[])?;
    let remove_label = text.get("profiles.sections.security.remove", &[])?;
    let security_title = text.get("profiles.sections.security.title", &[])?;
    let password_summary = text.get("profiles.sections.security.password_updated", &[])?;
    let two_factor_summary = text.get(
        if page.totp_enabled {
            "profiles.sections.security.two_factor_on"
        } else {
            "profiles.sections.security.two_factor_off"
        },
        &[],
    )?;
    let passkey_count = page.passkeys.len().to_string();
    let passkey_summary = text.get(
        "profiles.sections.security.passkeys",
        &[("count", &passkey_count)],
    )?;
    let passkeys = page
        .passkeys
        .iter()
        .map(|passkey| {
            Ok((
                passkey.clone(),
                text.get(
                    "profiles.sections.security.passkey_added_on",
                    &[("date", &passkey.added_on)],
                )?,
            ))
        })
        .collect::<Result<Vec<_>, TranslationError>>()?;
    Ok(view! {
        <div class="profile-section-header profile-security-header"><span class="profile-section-icon" aria-hidden="true"><svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/></svg></span><div><h2>{security_title}</h2><div class="profile-summary"><span>{password_summary}</span><span>{two_factor_summary}</span><span>{passkey_summary}</span></div></div></div>
        <div class="profile-grid profile-security-grid">
            {status.map(|(message, is_error)| view! { <p class="med-alert" role=if is_error { "alert" } else { "status" }>{message}</p> })}
            <section class="profile-info-card">
                <h3>{account_title}</h3>
                <p>{account_description}</p>
                <div class="profile-security-account-list">
                    <div class="profile-security-account-row"><div><h4>{email_title.clone()}</h4><p>{email_description.clone()}</p></div><button type="button" data-profile-dialog="security-email-dialog">{change_label.clone()}</button></div>
                    <div class="profile-security-account-row"><div><h4>{password_title.clone()}</h4><p>{password_description}</p></div><button type="button" data-profile-dialog="security-password-dialog">{change_label.clone()}</button></div>
                </div>
            </section>
            <section class="profile-info-card profile-security-factor-card">
                <h3>{methods_title}</h3>
                <p>{methods_description}</p>
                <div class="profile-security-factors">
                    <section class="profile-security-factor-section"><h4>{totp_title}</h4><p>{totp_description}</p>
                        <div class="profile-security-factor-row"><span class="profile-security-factor-status"><span aria-hidden="true">{if page.totp_enabled { "✓" } else { "⊗" }}</span>{totp_status}</span>
                            {if page.totp_enabled { view! { <button type="button" data-profile-dialog="security-otp-disable-dialog">{totp_disable_label.clone()}</button> }.into_any() } else { view! { <a href=otp_setup>{totp_setup_label}</a> }.into_any() }}
                        </div>
                    </section>
                    <section class="profile-security-factor-section"><h4>{recovery_title}</h4><p>{recovery_description}</p>
                        <div class="profile-security-factor-row"><span class="profile-security-factor-status"><span aria-hidden="true">{if page.recovery_codes_count > 0 { "✓" } else { "⊗" }}</span>{recovery_status}</span>
                            <div class="profile-security-factor-actions">{(page.recovery_codes_count > 0).then(|| view! { <a href=recovery>{recovery_view_label}</a> })}<button type="button" data-profile-dialog="security-recovery-dialog">{recovery_action_label.clone()}</button></div>
                        </div>
                    </section>
                    <section class="profile-security-factor-section"><h4>{passkeys_title}</h4><p>{passkeys_description}</p>
                        {if passkeys.is_empty() { view! { <div class="profile-security-factor-row"><span class="profile-security-factor-status"><span aria-hidden="true">"⊗"</span>{no_passkeys}</span><a href=passkey_setup.clone()>{passkeys_setup_label.clone()}</a></div> }.into_any() } else { view! { <div class="profile-security-passkey-group"><ul class="profile-security-passkeys">{passkeys.into_iter().map(|(passkey, added_on)| {
                            let remove = format!("{base}/passkeys/{}/remove", passkey.id);
                            let rename = format!("{base}/passkeys/{}/nickname", passkey.id);
                            let nickname_id = format!("passkey-{}-nickname", passkey.id);
                            let rename_password_id = format!("passkey-{}-rename-password", passkey.id);
                            let remove_password_id = format!("passkey-{}-remove-password", passkey.id);
                            let remove_dialog_id = format!("security-passkey-remove-{}", passkey.id);
                            let remove_heading_id = format!("{remove_dialog_id}-heading");
                            let remove_button_id = remove_dialog_id.clone();
                            let remove_close_id = remove_dialog_id.clone();
                            let heading_label_id = remove_heading_id.clone();
                            view! { <li class="profile-security-passkey-row"><span class="profile-security-passkey-label"><span aria-hidden="true"><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="15" r="3"/><path d="M10.5 13L19 4l2 2-2 2 1.5 1.5-2 2L17 10l-4.5 4.5"/></svg></span><span><strong>{passkey.nickname}</strong><small>{added_on}</small></span></span>
                                <div class="profile-security-passkey-actions"><details><summary>"Manage passkey"</summary>
                                    <form method="post" action=rename><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><label for=nickname_id.clone()>{nickname_label.clone()}</label><input id=nickname_id name="nickname" maxlength="100" required/><label for=rename_password_id.clone()>{password_label.clone()}</label><input id=rename_password_id name="password" type="password" autocomplete="current-password" required/><button type="submit">{change_label.clone()}</button></form>
                                </details><button type="button" class="profile-security-remove" data-profile-dialog=remove_button_id>{remove_label.clone()}</button></div>
                                <dialog id=remove_dialog_id aria-labelledby=remove_heading_id class="profile-dialog"><div class="profile-dialog-heading"><h2 id=heading_label_id>{remove_label.clone()}</h2><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close=remove_close_id>"×"</button></div><form class="household-form" method="post" action=remove><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><div class="form-field"><label for=remove_password_id.clone()>{password_label.clone()}</label><input id=remove_password_id name="password" type="password" autocomplete="current-password" required/></div><div class="household-actions"><button class="med-primary" type="submit">{remove_label.clone()}</button></div></form></dialog>
                            </li> }
                        }).collect_view()}</ul><a class="profile-security-add-passkey" href=passkey_setup>{passkeys_setup_label}</a></div> }.into_any() }}
                    </section>
                </div>
            </section>
        </div>
        <dialog id="security-email-dialog" aria-labelledby="security-email-heading" class="profile-dialog"><div class="profile-dialog-heading"><h2 id="security-email-heading">{email_title}</h2><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="security-email-dialog">"×"</button></div>
            <form class="household-form" method="post" action=email_action><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><div class="form-field"><label for="security-email">{email_description}</label><input id="security-email" name="email" type="email" autocomplete="email" required/></div><div class="form-field"><label for="security-email-password">{password_label.clone()}</label><input id="security-email-password" name="password" type="password" autocomplete="current-password" required/></div><div class="household-actions"><button class="med-primary" type="submit">{change_label.clone()}</button></div></form>
        </dialog>
        <dialog id="security-password-dialog" aria-labelledby="security-password-heading" class="profile-dialog"><div class="profile-dialog-heading"><h2 id="security-password-heading">{password_title}</h2><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="security-password-dialog">"×"</button></div>
            <form class="household-form" method="post" action=password_action><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><div class="form-field"><label for="security-current-password">{password_label.clone()}</label><input id="security-current-password" name="current_password" type="password" autocomplete="current-password" required/></div><div class="form-field"><label for="security-new-password">{new_password_label}</label><input id="security-new-password" name="new_password" type="password" autocomplete="new-password" minlength="12" required/></div><div class="form-field"><label for="security-password-confirmation">{confirm_password_label}</label><input id="security-password-confirmation" name="password_confirmation" type="password" autocomplete="new-password" required/></div><div class="household-actions"><button class="med-primary" type="submit">{update_password_label}</button></div></form>
        </dialog>
        <dialog id="security-otp-disable-dialog" aria-labelledby="security-otp-disable-heading" class="profile-dialog"><div class="profile-dialog-heading"><h2 id="security-otp-disable-heading">{totp_disable_label.clone()}</h2><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="security-otp-disable-dialog">"×"</button></div>
            <form class="household-form" method="post" action=otp_disable><input type="hidden" name="authenticity_token" value=page.csrf.clone()/><div class="form-field"><label for="security-disable-password">{password_label.clone()}</label><input id="security-disable-password" name="password" type="password" autocomplete="current-password" required/></div><div class="household-actions"><button class="med-primary" type="submit">{totp_disable_label}</button></div></form>
        </dialog>
        <dialog id="security-recovery-dialog" aria-labelledby="security-recovery-heading" class="profile-dialog"><div class="profile-dialog-heading"><h2 id="security-recovery-heading">{recovery_action_label.clone()}</h2><button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="security-recovery-dialog">"×"</button></div>
            <p>{text.get("rodauth.views.recovery_codes.safety_description", &[])?}</p><form class="household-form" method="post" action=recovery_generate><input type="hidden" name="authenticity_token" value=page.csrf/><div class="form-field"><label for="security-recovery-password">{password_label}</label><input id="security-recovery-password" name="password" type="password" autocomplete="current-password" required/></div><div class="household-actions"><button class="med-primary" type="submit">{recovery_action_label}</button></div></form>
        </dialog>
    }.to_html())
}
