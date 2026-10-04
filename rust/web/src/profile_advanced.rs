use crate::household::path_segment;
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

pub struct TokenSummary {
    pub id: i64,
    pub name: String,
    pub last_used_at: String,
    pub expires_at: String,
}

pub struct Variants {
    pub wizard: String,
    pub dashboard: String,
    pub medication_launcher: String,
}

pub struct AdvancedPage {
    pub locale: Locale,
    pub slug: String,
    pub csrf: String,
    pub membership_id: i64,
    pub can_edit: bool,
    pub can_manage_tokens: bool,
    pub tokens: Vec<TokenSummary>,
    pub variants: Variants,
    pub version: String,
    pub notice: Option<String>,
    pub new_token: Option<String>,
}

pub fn render_advanced(page: AdvancedPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let base = format!("/households/{}/profile", path_segment(&page.slug));
    let token_action = format!("{base}/api_tokens");
    let export_json = format!("{base}/data_exports/health_data_json");
    let export_zip = format!("{base}/data_exports/backup_zip");
    let experiments_action = format!("{base}/experiments");
    let close_action = format!("{base}/close_account");
    let token_title = text.get("profiles.api_tokens.title", &[])?;
    let token_description = text.get("profiles.api_tokens.description", &[])?;
    let token_name = text.get("profiles.api_tokens.name_label", &[])?;
    let token_create = text.get("profiles.api_tokens.create", &[])?;
    let token_empty = text.get("profiles.api_tokens.empty", &[])?;
    let token_revoke = text.get("profiles.api_tokens.revoke", &[])?;
    let export_title = text.get("profiles.sections.advanced.data_backup", &[])?;
    let export_description = text.get("profiles.sections.advanced.data_backup_description", &[])?;
    let experiments_title = text.get("profiles.sections.advanced.experiments", &[])?;
    let experiments_description =
        text.get("profiles.sections.advanced.experiments_description", &[])?;
    let dashboard_title = text.get("profiles.experiments.dashboard.title", &[])?;
    let system_title = text.get("profiles.version_info.title", &[])?;
    let system_description = text.get("profiles.version_info.description", &[])?;
    let version_label = text.get("profiles.version_info.app_version", &[])?;
    let docs_label = text.get("profiles.version_info.documentation", &[])?;
    let view_docs = text.get("profiles.version_info.view_docs", &[])?;
    let close_title = text.get("profiles.close_account.title", &[])?;
    let close_warning = text.get("profiles.close_account.retention_description", &[])?;
    let close_confirm = text.get("profiles.close_account.confirm_retention_button", &[])?;
    let close_dialog_title = text.get("profiles.close_account.dialog_title", &[])?;
    let close_cancel = text.get("profiles.close_account.cancel_button", &[])?;
    let section_title = text.get("profiles.sections.advanced.title", &[])?;
    let token_count = page.tokens.len().to_string();
    let token_summary = text.get(
        "profiles.sections.advanced.tokens",
        &[("count", &token_count)],
    )?;
    let export_summary = text.get("profiles.sections.advanced.exports", &[])?;
    let password_label = text.get("profiles.password_sheet.current_password_label", &[])?;
    let dashboard_options = ["current", "time_first", "family_lanes", "calm_focus"]
        .into_iter()
        .map(|value| {
            Ok((
                value,
                text.get(
                    &format!("profiles.experiments.dashboard.options.{value}.label"),
                    &[],
                )?,
                text.get(
                    &format!("profiles.experiments.dashboard.options.{value}.description"),
                    &[],
                )?,
            ))
        })
        .collect::<Result<Vec<_>, TranslationError>>()?;
    let wizard_options = [
        ("fullpage", "Full page", "Opens as a dedicated page."),
        ("modal", "Modal", "Opens over the current page."),
        ("slideover", "Slide-over", "Opens beside the current page."),
    ];
    let launcher_options = [
        (
            "current",
            "Current launcher",
            "Keep the existing Add Medication choices.",
        ),
        (
            "context_aware",
            "Context-aware",
            "Start with the person whose Add Medication action you chose.",
        ),
    ];
    let new_token_open = page.new_token.is_some();
    let can_edit = page.can_edit;
    let can_manage_tokens = page.can_manage_tokens;
    let csrf = page.csrf;
    let membership_id = page.membership_id.to_string();
    let variants = page.variants;
    let section = view! {
        <div class="profile-section-header" data-testid="profile-advanced-header">
            <span class="profile-section-icon" aria-hidden="true">{"⚙"}</span>
            <div><h2>{section_title}</h2><div class="profile-summary"><span>{token_summary}</span><span>{export_summary}</span></div></div>
        </div>
        <div class="profile-section-body"><div class="profile-advanced" data-testid="profile-advanced-section">
            {page.notice.map(|notice| view! { <p role="status" class="med-success">{notice}</p> })}
            <details class="profile-info-card" open=new_token_open>
                <summary><h3>{token_title}</h3><p>{token_description}</p></summary>
                {page.new_token.map(|token| view! { <div class="profile-new-token" role="status">
                    <p>{"Copy this token now. It will not be shown again."}</p>
                    <code>{token}</code>
                </div> })}
                {can_manage_tokens.then(|| view! {
                    <form method="post" action=token_action.clone() class="household-form">
                        <input type="hidden" name="authenticity_token" value=csrf.clone()/>
                        <input type="hidden" name="api_app_token[household_membership_id]" value=membership_id/>
                        <div class="form-field"><label for="advanced-token-name">{token_name}</label>
                            <input id="advanced-token-name" type="text" name="api_app_token[name]" maxlength="120" required/>
                        </div>
                        <button type="submit" class="med-primary">{token_create}</button>
                    </form>
                })}
                {page.tokens.is_empty().then(|| view! { <p>{token_empty}</p> })}
                <ul class="profile-advanced-list">
                    {page.tokens.into_iter().map(|token| {
                        let revoke_action = format!("{token_action}/{}", token.id);
                        view! { <li><strong>{token.name}</strong>
                            <span>{format!("Last used: {}", token.last_used_at)}</span>
                            <span>{format!("Expires: {}", token.expires_at)}</span>
                            {can_manage_tokens.then(|| view! { <form method="post" action=revoke_action>
                                <input type="hidden" name="authenticity_token" value=csrf.clone()/>
                                <button type="submit" class="profile-secondary">{token_revoke.clone()}</button>
                            </form> })}
                        </li> }
                    }).collect_view()}
                </ul>
            </details>
            <details class="profile-info-card">
                <summary><h3>{export_title}</h3><p>{export_description}</p></summary>
                <p class="med-alert">{"Unencrypted ZIP exports are not password protected. Store them somewhere private."}</p>
                <div class="household-actions">
                    <a href=export_json class="profile-secondary">{"Health data JSON"}</a>
                    <a href=export_zip class="profile-secondary">{"Unencrypted ZIP"}</a>
                </div>
            </details>
            <details class="profile-info-card">
                <summary><h3>{experiments_title}</h3><p>{experiments_description}</p></summary>
                {can_edit.then(|| view! {
                    <div class="profile-advanced-experiments">
                        <form method="post" action=experiments_action.clone() class="household-form">
                            <input type="hidden" name="authenticity_token" value=csrf.clone()/>
                            <fieldset><legend>{"Add Medication Wizard Style"}</legend>
                                {wizard_options.into_iter().map(|(value,label,description)| view! { <label class="profile-choice">
                                    <input type="radio" name="wizard_variant" value=value checked=variants.wizard == value required/>
                                    <span><strong>{label}</strong><small>{description}</small></span>
                                </label> }).collect_view()}
                            </fieldset><button type="submit" class="med-primary">{"Save wizard style"}</button>
                        </form>
                        <form method="post" action=experiments_action.clone() class="household-form">
                            <input type="hidden" name="authenticity_token" value=csrf.clone()/>
                            <fieldset><legend>{dashboard_title}</legend>
                                {dashboard_options.into_iter().map(|(value,label,description)| view! { <label class="profile-choice">
                                    <input type="radio" name="dashboard_variant" value=value checked=variants.dashboard == value required/>
                                    <span><strong>{label}</strong><small>{description}</small></span>
                                </label> }).collect_view()}
                            </fieldset><button type="submit" class="med-primary">{"Save dashboard layout"}</button>
                        </form>
                        <form method="post" action=experiments_action class="household-form">
                            <input type="hidden" name="authenticity_token" value=csrf.clone()/>
                            <fieldset><legend>{"Add Medication launcher"}</legend>
                                {launcher_options.into_iter().map(|(value,label,description)| view! { <label class="profile-choice">
                                    <input type="radio" name="medication_launcher_variant" value=value checked=variants.medication_launcher == value required/>
                                    <span><strong>{label}</strong><small>{description}</small></span>
                                </label> }).collect_view()}
                            </fieldset><button type="submit" class="med-primary">{"Save launcher"}</button>
                        </form>
                    </div>
                })}
                {(!can_edit).then(|| view! { <p>{"You do not have permission to change these preferences."}</p> })}
            </details>
            <details class="profile-info-card">
                <summary><h3>{system_title}</h3><p>{system_description}</p></summary>
                <dl><div><dt>{version_label}</dt><dd>{format!("v{}", page.version)}</dd></div>
                    <div><dt>{docs_label}</dt><dd><a href="https://damacus.github.io/med-tracker" target="_blank" rel="noopener">{view_docs}</a></dd></div>
                </dl>
            </details>
            <section class="profile-danger-zone"><h3>{"Danger Zone"}</h3>
                <div class="profile-danger-row profile-info-card"><div><strong>{close_title.clone()}</strong><p>{close_warning.clone()}</p></div>
                    <button type="button" class="profile-destructive" data-profile-dialog="profile-close-modal">{close_title}</button>
                </div>
            </section>
        </div></div>
        <dialog id="profile-close-modal" class="profile-dialog" data-testid="profile-close-dialog" aria-labelledby="profile-close-title">
            <div class="profile-dialog-heading"><div><h2 id="profile-close-title">{close_dialog_title}</h2><p>{close_warning}</p></div>
                <button type="button" class="profile-dialog-close" aria-label="Close" data-profile-close="profile-close-modal">{"×"}</button>
            </div>
            <form method="post" action=close_action class="household-form">
                <input type="hidden" name="authenticity_token" value=csrf/>
                <div class="form-field"><label for="advanced-close-password">{password_label}</label>
                    <input id="advanced-close-password" type="password" name="password" required autocomplete="current-password"/>
                </div>
                <div class="household-actions"><button type="button" class="profile-secondary" data-profile-close="profile-close-modal">{close_cancel}</button>
                    <button type="submit" class="profile-destructive">{close_confirm}</button></div>
            </form>
        </dialog>
    };
    Ok(section.to_html())
}
