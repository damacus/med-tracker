use loco_rs::controller::views::{ViewRenderer, engines::TeraView};
use serde_json::{Value, json};
use sghtmltopdf::{Converter, with_render_stack};
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub enum Kind {
    GpHistory,
    OrdinaryHistory,
    MedicationReview,
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
    use super::{Kind, locale, localize_report, translations};
    use serde_json::Value;
    use serde_json::json;
    use std::collections::BTreeSet;

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
        let mut report = json!({"generated_at":"2026-06-30T23:30:00Z","time_zone":"Europe/London","start_date":"2026-07-01","end_date":"2026-07-01","person":{"date_of_birth":null},"chronology":[]});
        localize_report(Kind::GpHistory, &mut report, &translations("en").unwrap());
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
        let mut report = json!({"generated_at":"2026-07-01T12:00:00Z","person":{"name":"Synthetic adult"},"to_discuss":1,"reviewed":0,"people_count":1,"prompts":[{"person_name":"Synthetic adult","primary_medication_name":"One","interacting_medication_name":"Two","risk_level":"unknown","match_confidence":"unknown","status":"needs_review","evidence_text":"Legacy evidence","match_reason":"Legacy source","matched_term":"legacy","match_type":"unclassified_legacy","source_instruction":"unclassified_legacy","evidence_source_name":"Legacy source","evidence_source_version":"1","evidence_source_effective_on":"2026-01-01","evidence_source_checked_on":"2026-01-01","evidence_source_url":"https://example.test/legacy","practitioner_name":null}]});
        localize_report(Kind::MedicationReview, &mut report, &labels);
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

fn localize_report(kind: Kind, report: &mut Value, labels: &Value) {
    let generated = report["generated_at"]
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map_or_else(
            || report["generated_at"].as_str().unwrap_or("").to_owned(),
            |value| {
                let zone = report["time_zone"]
                    .as_str()
                    .and_then(|name| name.parse::<chrono_tz::Tz>().ok())
                    .unwrap_or(chrono_tz::UTC);
                value
                    .with_timezone(&zone)
                    .format("%Y-%m-%d %H:%M %Z")
                    .to_string()
            },
        );
    let generated_key = if matches!(kind, Kind::MedicationReview) {
        "reports.medication_review.generated_at"
    } else {
        "reports.health_history.generated_at"
    };
    report["generated_label"] = json!(interpolate(
        labels,
        generated_key,
        &[("timestamp", &generated)]
    ));
    if matches!(kind, Kind::MedicationReview)
        && let Some(prompts) = report["prompts"].as_array_mut()
    {
        let translations = &labels["reports"]["medication_review"];
        for prompt in prompts {
            for (field, group, output) in [
                ("risk_level", "risk_levels", "risk_label"),
                ("match_confidence", "confidence_levels", "confidence_label"),
                ("status", "statuses", "status_label"),
                ("match_type", "match_types", "match_type_label"),
                (
                    "source_instruction",
                    "source_instructions",
                    "source_instruction_label",
                ),
            ] {
                let value = prompt[field].as_str().unwrap_or("");
                let translated = translations[group]
                    .get(value)
                    .and_then(Value::as_str)
                    .unwrap_or(value)
                    .to_owned();
                prompt[output] = json!(translated);
            }
        }
    }
    if !matches!(kind, Kind::GpHistory) {
        return;
    }
    let start = report["start_date"].as_str().unwrap_or("");
    let end = report["end_date"].as_str().unwrap_or("");
    report["reporting_period"] = json!(interpolate(
        labels,
        "reports.health_history.gp.reporting_period",
        &[("start_date", start), ("end_date", end)]
    ));
    let birthday = report["person"]["date_of_birth"].as_str().unwrap_or("");
    let birthday = if birthday.is_empty() {
        label(labels, "reports.health_history.gp.not_recorded")
    } else {
        birthday.to_owned()
    };
    report["birthday_label"] = json!(interpolate(
        labels,
        "reports.health_history.gp.identity.date_of_birth",
        &[("date", &birthday)]
    ));
    let Some(events) = report["chronology"].as_array_mut() else {
        return;
    };
    for event in events {
        let kind = event["event_kind"].as_str().unwrap_or("");
        event["kind_label"] = json!(label(labels, &format!("health_events.kinds.{kind}")));
        let severity = event["severity"].as_str().unwrap_or("").to_owned();
        event["severity_label"] = json!(label(
            labels,
            &format!("health_events.severities.{severity}")
        ));
        let started = event["started_on"].as_str().unwrap_or("").to_owned();
        let ended = event["ended_on"].as_str().map(str::to_owned);
        event["date_range_label"] = json!(ended.as_deref().map_or_else(
            || interpolate(
                labels,
                "reports.health_history.ongoing_from",
                &[("started_on", &started)]
            ),
            |end| interpolate(
                labels,
                "reports.health_history.event_date_range",
                &[("started_on", &started), ("ended_on", end)]
            )
        ));
        let duration = event["duration_days"].as_i64().map_or_else(
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
        for (field, key, argument) in [
            ("notes", "notes", "notes"),
            ("action_taken", "action", "action"),
        ] {
            if let Some(value) = event[field].as_str().filter(|value| !value.is_empty()) {
                details.push(interpolate(
                    labels,
                    &format!("reports.health_history.gp.chronology.{key}"),
                    &[(argument, value)],
                ));
            }
        }
        let help = if event["medical_help_sought"].as_bool().unwrap_or(false) {
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
        let medications = event["medication_names"]
            .as_array()
            .map(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        if !medications.is_empty() {
            details.push(interpolate(
                labels,
                "reports.health_history.gp.chronology.medications",
                &[("medications", &medications)],
            ));
        }
        event["details"] = json!(details);
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
    kind: Kind,
    mut report: Value,
    language: &str,
) -> Result<Vec<u8>, RenderError> {
    let language = locale(language);
    let labels = translations(language)?;
    let title = label(&labels, kind.title_key());
    let subject = label(&labels, kind.subject_key());
    let page_counter_css = page_counter_css(&labels)?;
    localize_report(kind, &mut report, &labels);
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
        let report = json!({
            "person":{"name":"Example person","date_of_birth":"1990-01-01"},
            "start_date":"2026-01-01","end_date":"2026-01-31","generated_at":"2026-02-01T10:00:00Z",
            "current_medicines":[],"include_medication_takes":false,"medication_takes":[],
            "chronology":[{"started_on":"2026-01-03","ended_on":"2026-01-05","event_kind":"suspected_side_effect","title":"Example symptom","duration_days":3,"severity":"moderate","notes":"Observed after lunch","action_taken":"Recorded event","medical_help_sought":true,"medication_names":["Example medicine"]}]
        });
        for language in ["en", "cy"] {
            let labels = translations(language).unwrap();
            let page_counter = page_counter_css(&labels).unwrap();
            let mut report = report.clone();
            localize_report(Kind::GpHistory, &mut report, &labels);
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
