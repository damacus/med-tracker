use crate::household::household_document;
use crate::household_i18n::{Locale, Text, TranslationError};
use leptos::prelude::*;

pub struct DmdImportRun {
    pub id: i64,
    pub filename: String,
    pub status: String,
    pub active: bool,
    pub total_records: i64,
    pub processed_records: i64,
    pub progress_percentage: i64,
    pub imported_count: i64,
    pub skipped_count: i64,
    pub created_count: i64,
    pub updated_count: i64,
    pub unchanged_count: i64,
    pub skipped_expired_count: i64,
    pub skipped_missing_name_count: i64,
    pub skipped_invalid_count: i64,
    pub error_message: String,
    pub log: String,
    pub created_at: String,
}

pub struct DmdImportPage {
    pub household_name: String,
    pub slug: String,
    pub locale: Locale,
    pub csrf: String,
    pub action: String,
    pub runs: Vec<DmdImportRun>,
    pub alert: String,
    pub notice: String,
    pub notifications_visible: bool,
}

pub fn render_dmd_import(page: DmdImportPage) -> Result<String, TranslationError> {
    let text = Text::new(page.locale);
    let title = text.get("admin.nhs_dmd_imports.title", &[])?;
    let subtitle = text.get("admin.nhs_dmd_imports.subtitle", &[])?;
    let file_label = text.get("admin.nhs_dmd_imports.form.release_zip", &[])?;
    let help = text.get("admin.nhs_dmd_imports.form.help", &[])?;
    let submit = text.get("admin.nhs_dmd_imports.form.submit", &[])?;
    let latest_title = text.get("admin.nhs_dmd_imports.latest_run.title", &[])?;
    let progress_label = text.get("admin.nhs_dmd_imports.latest_run.progress", &[])?;
    let percentage_label = text.get("admin.nhs_dmd_imports.latest_run.percentage", &[])?;
    let breakdown_heading = text.get("admin.nhs_dmd_imports.latest_run.breakdown_heading", &[])?;
    let skipped_heading = text.get("admin.nhs_dmd_imports.latest_run.skipped_heading", &[])?;
    let log_label = text.get("admin.nhs_dmd_imports.latest_run.log", &[])?;
    let no_log = text.get("admin.nhs_dmd_imports.latest_run.no_log", &[])?;
    let labels: Vec<(&str, String)> = [
        "imported",
        "skipped",
        "created",
        "updated",
        "unchanged",
        "skipped_expired",
        "skipped_missing_name",
        "skipped_invalid",
    ]
    .into_iter()
    .map(|key| {
        text.get(&format!("admin.nhs_dmd_imports.latest_run.{key}"), &[])
            .map(|label| (key, label))
    })
    .collect::<Result<Vec<_>, _>>()?;
    let label = |key: &str| {
        labels
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| key.to_owned())
    };
    let mut runs = page.runs.into_iter();
    let latest = runs.next();
    let history: Vec<DmdImportRun> = runs.collect();
    let active = latest.as_ref().is_some_and(|run| run.active);
    let body = view! {
        <header class="med-heading"><h1>{title.clone()}</h1><p>{subtitle}</p></header>
        {(!page.alert.is_empty()).then(|| view! {
            <p class="med-alert" role="alert">{page.alert.clone()}</p>
        })}
        {(!page.notice.is_empty()).then(|| view! {
            <p class="med-notice" role="status">{page.notice.clone()}</p>
        })}
        <form class="household-form med-panel" method="post" action=page.action enctype="multipart/form-data">
            <input type="hidden" name="authenticity_token" value=page.csrf/>
            <div class="med-field">
                <label for="release_zip">{file_label}</label>
                <input id="release_zip" name="release_zip" type="file" accept=".zip,application/zip" required/>
                <p class="med-hint">{help}</p>
            </div>
            <button class="med-primary" type="submit" disabled=active>{submit}</button>
        </form>
        {latest.map(|run| {
            let status_label = text
                .get(&format!("admin.nhs_dmd_imports.statuses.{}", run.status), &[])
                .unwrap_or_else(|_| run.status.clone());
            let log = if run.log.is_empty() { no_log.clone() } else { run.log.clone() };
            view! {
                <section class="med-panel" data-testid="dmd-latest-run">
                    <h2>{latest_title.clone()}</h2>
                    <dl class="med-facts">
                        <div><dt>{progress_label.clone()}</dt><dd>{format!("{} / {}", run.processed_records, run.total_records)}</dd></div>
                        <div><dt>{percentage_label.clone()}</dt><dd>{format!("{}%", run.progress_percentage)}</dd></div>
                    </dl>
                    <p><strong>{status_label}</strong>{format!(" — {}", run.filename)}</p>
                    {(!run.error_message.is_empty()).then(|| view! {
                        <p class="med-alert" role="alert">{run.error_message.clone()}</p>
                    })}
                    <h3>{breakdown_heading.clone()}</h3>
                    <table class="med-table">
                        <tbody>
                            <tr><th>{label("imported")}</th><td>{run.imported_count}</td></tr>
                            <tr><th>{label("created")}</th><td>{run.created_count}</td></tr>
                            <tr><th>{label("updated")}</th><td>{run.updated_count}</td></tr>
                            <tr><th>{label("unchanged")}</th><td>{run.unchanged_count}</td></tr>
                        </tbody>
                    </table>
                    <h3>{skipped_heading.clone()}</h3>
                    <table class="med-table">
                        <tbody>
                            <tr><th>{label("skipped")}</th><td>{run.skipped_count}</td></tr>
                            <tr><th>{label("skipped_expired")}</th><td>{run.skipped_expired_count}</td></tr>
                            <tr><th>{label("skipped_missing_name")}</th><td>{run.skipped_missing_name_count}</td></tr>
                            <tr><th>{label("skipped_invalid")}</th><td>{run.skipped_invalid_count}</td></tr>
                        </tbody>
                    </table>
                    <h3>{log_label.clone()}</h3>
                    <pre class="med-log">{log}</pre>
                </section>
            }
        })}
        {(!history.is_empty()).then(|| view! {
            <section class="med-panel">
                <table class="med-table">
                    <thead><tr><th>#</th><th>File</th><th>Status</th></tr></thead>
                    <tbody>
                        {history.into_iter().map(|run| {
                            let status_label = text
                                .get(&format!("admin.nhs_dmd_imports.statuses.{}", run.status), &[])
                                .unwrap_or_else(|_| run.status.clone());
                            view! {
                                <tr>
                                    <td>{run.id}</td>
                                    <td>{run.filename}</td>
                                    <td>{status_label}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </section>
        })}
    }
    .to_html();
    let document = household_document(
        &title,
        &page.household_name,
        &page.slug,
        page.locale.as_str(),
        body,
        page.notifications_visible,
    );
    if active {
        return Ok(document.replacen(
            "</head>",
            "<meta http-equiv=\"refresh\" content=\"5\"></head>",
            1,
        ));
    }
    Ok(document)
}
