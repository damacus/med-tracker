use crate::models::care::reports::{
    GpHistoryReport, MedicationReviewReport, OrdinaryHistoryReport,
};
use loco_rs::controller::views::{ViewRenderer, engines::TeraView};
use serde::Serialize;
use serde_json::{Value, json};
use sghtmltopdf::{Converter, with_render_stack};
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub enum Kind {
    GpHistory,
    OrdinaryHistory,
    MedicationReview,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Document {
    Gp(GpHistoryReport),
    Ordinary(OrdinaryHistoryReport),
    Review(MedicationReviewReport),
}

impl Document {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Gp(_) => Kind::GpHistory,
            Self::Ordinary(_) => Kind::OrdinaryHistory,
            Self::Review(_) => Kind::MedicationReview,
        }
    }
}

impl Kind {
    fn title_key(self) -> &'static str {
        match self {
            Self::GpHistory => "reports.health_history.title",
            Self::OrdinaryHistory => "reports.health_history.title",
            Self::MedicationReview => "reports.medication_review.title",
        }
    }

    fn subject_key(self) -> &'static str {
        match self {
            Self::GpHistory | Self::OrdinaryHistory => {
                "reports.health_history.disclaimer.entered_information"
            }
            Self::MedicationReview => "reports.medication_review.boundary",
        }
    }

    fn template(self) -> &'static str {
        match self {
            Self::GpHistory => "reports/gp_history.html",
            Self::OrdinaryHistory => "reports/ordinary_history.html",
            Self::MedicationReview => "reports/medication_review.html",
        }
    }
}

#[derive(Debug)]
pub enum RenderError {
    Font,
    Locale,
    Template,
    Engine,
}

pub fn locale(value: &str) -> &'static str {
    value
        .split(',')
        .filter_map(|part| {
            let mut pieces = part.trim().split(';');
            let primary = pieces
                .next()?
                .trim()
                .split(['-', '_'])
                .next()?
                .to_ascii_lowercase();
            let language = match primary.as_str() {
                "cy" => "cy",
                "ga" => "ga",
                "pt" => "pt",
                "es" => "es",
                "en" => "en",
                _ => return None,
            };
            let quality = pieces
                .find_map(|piece| {
                    piece
                        .trim()
                        .strip_prefix("q=")
                        .and_then(|value| value.parse::<f32>().ok())
                })
                .unwrap_or(1.0);
            (quality > 0.0 && quality <= 1.0).then_some((language, quality))
        })
        .reduce(|left, right| if right.1 > left.1 { right } else { left })
        .map_or("en", |(language, _)| language)
}

#[cfg(test)]
mod locale_tests {
    use super::Document;
    use super::{locale, localize_report, translations};
    use crate::models::care::reports::{
        GpHistoryReport, GpPerson, MedicationReviewReport, ReportUtc, ReviewPerson, ReviewPromptRow,
    };
    use chrono::{NaiveDate, Utc};
    use serde_json::Value;
    use serde_json::json;
    use std::collections::BTreeSet;

    pub(super) fn gp_fixture() -> GpHistoryReport {
        GpHistoryReport {
            person: GpPerson {
                id: 1,
                name: "Example person".into(),
                date_of_birth: None,
            },
            start_date: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            generated_at: ReportUtc(
                NaiveDate::from_ymd_opt(2026, 6, 30)
                    .unwrap()
                    .and_hms_opt(23, 30, 0)
                    .unwrap()
                    .and_utc(),
            ),
            current_medicines: Vec::new(),
            chronology: Vec::new(),
            medication_takes: Vec::new(),
            time_zone: Some("Europe/London".into()),
            include_medication_takes: Some(false),
            generated_label: None,
            reporting_period: None,
            birthday_label: None,
        }
    }

    fn legacy_prompt() -> ReviewPromptRow {
        ReviewPromptRow {
            id: 1,
            person_id: 1,
            primary_medication_id: 2,
            interacting_medication_id: 3,
            evidence_record_id: 4,
            primary_medication_name: "One".into(),
            interacting_medication_name: "Two".into(),
            evidence_source_name: "Legacy source".into(),
            evidence_source_url: "https://example.test/legacy".into(),
            evidence_source_version: "1".into(),
            matched_term: "legacy".into(),
            match_type: "unclassified_legacy".into(),
            source_instruction: "unclassified_legacy".into(),
            match_reason: "Legacy source".into(),
            evidence_text: "Legacy evidence".into(),
            etag: "fixture".into(),
            evidence_source_checked_on: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            evidence_source_effective_on: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            risk_level: "unknown".into(),
            match_confidence: "unknown".into(),
            status: "needs_review".into(),
            practitioner_name: None,
            practitioner_role: None,
            review_note: None,
            reviewed_on: None,
            reviewed_by_membership_id: None,
            updated_at: ReportUtc(Utc::now()),
            person_name: Some("Synthetic adult".into()),
            risk_label: None,
            confidence_label: None,
            status_label: None,
            match_type_label: None,
            source_instruction_label: None,
        }
    }

    fn leaf_paths(value: &Value, prefix: &str, paths: &mut BTreeSet<String>) {
        if let Some(object) = value.as_object() {
            for (key, value) in object {
                leaf_paths(value, &format!("{prefix}.{key}"), paths);
            }
        } else if let Some(values) = value.as_array() {
            assert!(
                !values.is_empty()
                    && values
                        .iter()
                        .all(|value| value.as_str().is_some_and(|text| !text.is_empty())),
                "{prefix}"
            );
            paths.insert(format!("{prefix}[{}]", values.len()));
        } else {
            assert!(
                value.as_str().is_some_and(|text| !text.is_empty()),
                "{prefix}"
            );
            paths.insert(prefix.to_owned());
        }
    }

    #[test]
    fn bundled_report_locales_have_the_same_required_labels() {
        let english = translations("en").unwrap();
        let groups = [
            "reports",
            "administration",
            "smart_insights",
            "health_events.kinds",
            "health_events.severities",
        ];
        for group in groups {
            let mut required = BTreeSet::new();
            let base = group
                .split('.')
                .fold(&english, |value, segment| &value[segment]);
            leaf_paths(base, group, &mut required);
            assert!(!required.is_empty(), "{group}");
            for language in ["cy", "ga", "pt", "es"] {
                let labels = translations(language).unwrap();
                let value = group
                    .split('.')
                    .fold(&labels, |value, segment| &value[segment]);
                let mut found = BTreeSet::new();
                leaf_paths(value, group, &mut found);
                assert_eq!(found, required, "{language} {group}");
            }
        }
    }

    #[test]
    fn selects_supported_language_from_weighted_accept_language_header() {
        assert_eq!(locale("cy;q=0.9,en;q=0.8"), "cy");
        assert_eq!(locale("fr,es;q=0.8,en;q=0.2"), "es");
        assert_eq!(locale("cy;q=0.9,en;q=0.9"), "cy");
        assert_eq!(locale("ES,en;q=0.1"), "es");
    }

    #[test]
    fn gp_generated_time_uses_the_accounts_local_date_and_zone() {
        let mut report = Document::Gp(gp_fixture());
        localize_report(&mut report, &translations("en").unwrap());
        let report = json!(report);
        assert!(
            report["generated_label"]
                .as_str()
                .unwrap()
                .contains("2026-07-01 00:30 BST")
        );
    }

    #[test]
    fn review_html_renders_a_legacy_unclassified_match_without_dropping_the_report() {
        use loco_rs::controller::views::{ViewRenderer, engines::TeraView};
        let view = TeraView::build().unwrap();
        let labels = translations("en").unwrap();
        let mut report = Document::Review(MedicationReviewReport {
            person: ReviewPerson {
                id: None,
                name: "Synthetic adult".into(),
            },
            generated_at: ReportUtc(
                NaiveDate::from_ymd_opt(2026, 7, 1)
                    .unwrap()
                    .and_hms_opt(12, 0, 0)
                    .unwrap()
                    .and_utc(),
            ),
            prompts: vec![legacy_prompt()],
            people_count: Some(1),
            to_discuss: Some(1),
            reviewed: Some(0),
            time_zone: None,
            generated_label: None,
        });
        localize_report(&mut report, &labels);
        let report = json!(report);
        let html = view.render("reports/medication_review.html", json!({"lang":"en","title":"Medication review","labels":labels,"report":report,"page_counter_css":""}));
        assert!(
            html.is_ok(),
            "legacy review evidence should still render: {html:?}"
        );
    }
}

pub(crate) fn translations(locale: &str) -> Result<Value, RenderError> {
    let source = match locale {
        "cy" => include_str!("../../../assets/reports/locales/cy.yml"),
        "ga" => include_str!("../../../assets/reports/locales/ga.yml"),
        "pt" => include_str!("../../../assets/reports/locales/pt.yml"),
        "es" => include_str!("../../../assets/reports/locales/es.yml"),
        _ => include_str!("../../../assets/reports/locales/en.yml"),
    };
    let all: Value = serde_yaml_ng::from_str(source).map_err(|_| RenderError::Locale)?;
    Ok(all.get(locale).cloned().unwrap_or(Value::Null))
}

fn label(source: &Value, path: &str) -> String {
    path.split('.')
        .fold(source, |value, segment| &value[segment])
        .as_str()
        .unwrap_or(path)
        .to_owned()
}

fn interpolate(source: &Value, path: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(label(source, path), |text, (key, value)| {
            text.replace(&format!("%{{{key}}}"), value)
        })
}

fn localize_report(report: &mut Document, labels: &Value) {
    let (generated_at, zone, review) = match report {
        Document::Gp(data) => (data.generated_at.0, data.time_zone.as_deref(), false),
        Document::Ordinary(data) => (data.generated_at.0, None, false),
        Document::Review(data) => (data.generated_at.0, data.time_zone.as_deref(), true),
    };
    let zone = zone
        .and_then(|name| name.parse::<chrono_tz::Tz>().ok())
        .unwrap_or(chrono_tz::UTC);
    let generated = generated_at
        .with_timezone(&zone)
        .format("%Y-%m-%d %H:%M %Z")
        .to_string();
    let key = if review {
        "reports.medication_review.generated_at"
    } else {
        "reports.health_history.generated_at"
    };
    let generated_label = interpolate(labels, key, &[("timestamp", &generated)]);
    match report {
        Document::Ordinary(data) => {
            data.generated_label = Some(generated_label);
        }
        Document::Review(data) => {
            data.generated_label = Some(generated_label);
            let translations = &labels["reports"]["medication_review"];
            let translated = |group: &str, value: &str| {
                translations[group]
                    .get(value)
                    .and_then(Value::as_str)
                    .unwrap_or(value)
                    .to_owned()
            };
            for prompt in &mut data.prompts {
                prompt.risk_label = Some(translated("risk_levels", &prompt.risk_level));
                prompt.confidence_label =
                    Some(translated("confidence_levels", &prompt.match_confidence));
                prompt.status_label = Some(translated("statuses", &prompt.status));
                prompt.match_type_label = Some(translated("match_types", &prompt.match_type));
                prompt.source_instruction_label = Some(translated(
                    "source_instructions",
                    &prompt.source_instruction,
                ));
            }
        }
        Document::Gp(data) => {
            data.generated_label = Some(generated_label);
            data.reporting_period = Some(interpolate(
                labels,
                "reports.health_history.gp.reporting_period",
                &[
                    ("start_date", &data.start_date.to_string()),
                    ("end_date", &data.end_date.to_string()),
                ],
            ));
            let birthday = data.person.date_of_birth.map_or_else(
                || label(labels, "reports.health_history.gp.not_recorded"),
                |date| date.to_string(),
            );
            data.birthday_label = Some(interpolate(
                labels,
                "reports.health_history.gp.identity.date_of_birth",
                &[("date", &birthday)],
            ));
            for event in &mut data.chronology {
                event.kind_label = Some(label(
                    labels,
                    &format!("health_events.kinds.{}", event.event_kind),
                ));
                let severity = event.severity.unwrap_or("");
                event.severity_label = Some(label(
                    labels,
                    &format!("health_events.severities.{severity}"),
                ));
                let started = event.started_on.to_string();
                event.date_range_label = Some(event.ended_on.map_or_else(
                    || {
                        interpolate(
                            labels,
                            "reports.health_history.ongoing_from",
                            &[("started_on", &started)],
                        )
                    },
                    |end| {
                        interpolate(
                            labels,
                            "reports.health_history.event_date_range",
                            &[("started_on", &started), ("ended_on", &end.to_string())],
                        )
                    },
                ));
                let duration = event.duration_days.map_or_else(
                    || label(labels, "reports.health_history.gp.chronology.ongoing"),
                    |days| {
                        interpolate(
                            labels,
                            "reports.health_history.gp.chronology.days",
                            &[("count", &days.to_string())],
                        )
                    },
                );
                let mut details = vec![interpolate(
                    labels,
                    "reports.health_history.gp.chronology.duration",
                    &[("duration", &duration)],
                )];
                if !severity.is_empty() {
                    details.push(interpolate(
                        labels,
                        "reports.health_history.gp.chronology.severity",
                        &[(
                            "severity",
                            &label(labels, &format!("health_events.severities.{severity}")),
                        )],
                    ));
                }
                if let Some(note) = event.notes.as_deref().filter(|value| !value.is_empty()) {
                    details.push(interpolate(
                        labels,
                        "reports.health_history.gp.chronology.notes",
                        &[("notes", note)],
                    ));
                }
                if let Some(action) = event
                    .action_taken
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    details.push(interpolate(
                        labels,
                        "reports.health_history.gp.chronology.action",
                        &[("action", action)],
                    ));
                }
                let help = if event.medical_help_sought {
                    "yes"
                } else {
                    "no"
                };
                details.push(interpolate(
                    labels,
                    "reports.health_history.gp.chronology.medical_help",
                    &[(
                        "medical_help",
                        &label(labels, &format!("reports.health_history.gp.{help}")),
                    )],
                ));
                let medications = event.medication_names.join(", ");
                if !medications.is_empty() {
                    details.push(interpolate(
                        labels,
                        "reports.health_history.gp.chronology.medications",
                        &[("medications", &medications)],
                    ));
                }
                event.details = Some(details);
            }
        }
    }
}

fn page_counter_css(labels: &Value) -> Result<String, RenderError> {
    let translation = label(labels, "reports.health_history.gp.page_number");
    let (before_page, remainder) = translation
        .split_once("<page>")
        .ok_or(RenderError::Locale)?;
    let (between, after_total) = remainder.split_once("<total>").ok_or(RenderError::Locale)?;
    let before_page = serde_json::to_string(before_page).map_err(|_| RenderError::Locale)?;
    let between = serde_json::to_string(between).map_err(|_| RenderError::Locale)?;
    let after_total = serde_json::to_string(after_total).map_err(|_| RenderError::Locale)?;
    Ok(format!(
        "{before_page} counter(page) {between} counter(pages) {after_total}"
    ))
}

fn font_directory() -> Result<PathBuf, RenderError> {
    let directory = PathBuf::from("assets/reports/fonts")
        .canonicalize()
        .map_err(|_| RenderError::Font)?;
    directory
        .join("NotoSans-Regular.ttf")
        .is_file()
        .then_some(directory)
        .ok_or(RenderError::Font)
}

pub fn render(
    view: &TeraView,
    mut report: Document,
    language: &str,
) -> Result<Vec<u8>, RenderError> {
    let kind = report.kind();
    let language = locale(language);
    let labels = translations(language)?;
    let title = label(&labels, kind.title_key());
    let subject = label(&labels, kind.subject_key());
    let page_counter_css = page_counter_css(&labels)?;
    localize_report(&mut report, &labels);
    let font_directory = font_directory()?;
    let font = font_directory.join("NotoSans-Regular.ttf");
    let html = view
        .render(
            kind.template(),
            json!({
                "lang": language,
                "title": title,
                "labels": labels,
                "page_counter_css": page_counter_css,
                "report": report,
            }),
        )
        .map_err(|_| RenderError::Template)?;
    let args = vec![
        "--page-size".to_owned(),
        "A4".to_owned(),
        "--title".to_owned(),
        title,
        "--author".to_owned(),
        "MedTracker".to_owned(),
        "--subject".to_owned(),
        subject,
        "--gothic-font".to_owned(),
        font.to_string_lossy().into_owned(),
        "--serif-font".to_owned(),
        font.to_string_lossy().into_owned(),
        "--mono-font".to_owned(),
        font.to_string_lossy().into_owned(),
        "--base-url".to_owned(),
        font_directory.to_string_lossy().into_owned(),
        "--allow-path".to_owned(),
        font_directory.to_string_lossy().into_owned(),
    ];
    let converter = Converter::from_args(args).map_err(|_| RenderError::Engine)?;
    let bytes = with_render_stack(|| converter.render_to_vec(html.as_bytes()))
        .map_err(|_| RenderError::Engine)?;
    bytes
        .starts_with(b"%PDF-")
        .then_some(bytes)
        .ok_or(RenderError::Engine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gp_html_preserves_clinical_details_and_localizes_event_labels() {
        let view = TeraView::build().unwrap();
        for language in ["en", "cy"] {
            let labels = translations(language).unwrap();
            let page_counter = page_counter_css(&labels).unwrap();
            let mut report = locale_tests::gp_fixture();
            report.person.date_of_birth =
                Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap());
            report.start_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
            report.end_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
            report
                .chronology
                .push(crate::models::care::reports::GpEvent {
                    id: 1,
                    event_kind: "suspected_side_effect",
                    title: "Example symptom".into(),
                    started_on: chrono::NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(),
                    ended_on: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()),
                    ongoing: false,
                    duration_days: Some(3),
                    severity: Some("moderate"),
                    notes: Some("Observed after lunch".into()),
                    action_taken: Some("Recorded event".into()),
                    medical_help_sought: true,
                    medication_names: vec!["Example medicine".into()],
                    kind_label: None,
                    severity_label: None,
                    date_range_label: None,
                    details: None,
                });
            let mut report = Document::Gp(report);
            localize_report(&mut report, &labels);
            let html = view.render("reports/gp_history.html", json!({"lang":language,"title":label(&labels,"reports.health_history.title"),"labels":labels.clone(),"page_counter_css":page_counter,"report":report})).unwrap();
            assert!(
                html.contains(label(&labels, "health_events.kinds.suspected_side_effect").as_str())
            );
            assert!(html.contains(label(&labels, "health_events.severities.moderate").as_str()));
            assert!(html.contains("Observed after lunch"));
            assert!(html.contains("Recorded event"));
            assert!(html.contains("Example medicine"));
            if language == "en" {
                assert!(html.contains("Duration: 3 days"));
                assert!(html.contains("Medical help sought: Yes"));
            }
        }
    }
}
