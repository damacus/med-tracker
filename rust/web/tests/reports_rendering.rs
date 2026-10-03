use medtracker_web::household_i18n::Locale;
use medtracker_web::reports::{ReportChoice, ReportDraft, render_reports};

fn page(choices: Vec<ReportChoice>, draft: ReportDraft, errors: Vec<(String, String)>) -> String {
    render_reports(
        "Test household",
        "test-slug",
        Locale::En,
        choices,
        draft,
        errors,
    )
    .expect("reports page renders")
}

fn choices() -> Vec<ReportChoice> {
    vec![
        ReportChoice {
            id: "7".into(),
            name: "Managed Person".into(),
        },
        ReportChoice {
            id: "9".into(),
            name: "Second Person".into(),
        },
    ]
}

#[test]
fn reports_form_offers_manageable_people_with_a_get_download_action() {
    let html = page(choices(), ReportDraft::default(), vec![]);
    assert!(html.contains("<h1"));
    assert!(html.contains("Reports"));
    assert!(html.contains("href=\"/households/test-slug/reports\""));
    assert!(html.contains("action=\"/households/test-slug/reports/health-history.pdf\""));
    assert!(html.contains("method=\"get\""));
    assert!(html.contains("for=\"report_person_id\""));
    assert!(html.contains("id=\"report_person_id\""));
    assert!(html.contains("name=\"person_id\""));
    assert!(html.contains("Select a person"));
    assert!(html.contains("value=\"7\""));
    assert!(html.contains("Managed Person"));
    assert!(html.contains("value=\"9\""));
    assert!(html.contains("Second Person"));
    assert!(!html.contains("value=\"42\""));
    assert!(html.contains("for=\"report_start_date\""));
    assert!(html.contains("id=\"report_start_date\""));
    assert!(html.contains("name=\"start_date\""));
    assert!(html.contains("for=\"report_end_date\""));
    assert!(html.contains("id=\"report_end_date\""));
    assert!(html.contains("name=\"end_date\""));
    assert_eq!(html.matches("type=\"date\"").count(), 2);
    assert!(html.contains("for=\"report_include_medication_takes\""));
    assert!(html.contains("id=\"report_include_medication_takes\""));
    assert!(html.contains("name=\"include_medication_takes\""));
    assert!(html.contains("value=\"1\""));
    assert!(!html.contains("checked"));
    assert!(html.contains("Download PDF"));
    assert!(html.contains("type=\"submit\""));
    assert!(html.contains("required"));
}

#[test]
fn reports_form_defaults_include_medication_takes_to_unchecked() {
    let html = page(
        choices(),
        ReportDraft {
            person_id: "7".into(),
            start_date: "2026-02-01".into(),
            end_date: "2026-02-26".into(),
            include_medication_takes: false,
        },
        vec![],
    );
    assert!(!html.contains("checked"));
    assert!(html.contains("value=\"1\""));
}

#[test]
fn reports_form_escapes_untrusted_person_names() {
    let html = page(
        vec![ReportChoice {
            id: "5".into(),
            name: "Unsafe <script> Name".into(),
        }],
        ReportDraft::default(),
        vec![],
    );
    assert!(html.contains("Unsafe &lt;script&gt; Name"));
    assert!(!html.contains("Unsafe <script> Name"));
}

#[test]
fn reports_form_retains_filters_and_associates_errors_with_fields() {
    let html = page(
        choices(),
        ReportDraft {
            person_id: "9".into(),
            start_date: "2026-02-01".into(),
            end_date: "2026-02-03".into(),
            include_medication_takes: true,
        },
        vec![
            ("start_date".into(), "start_date_invalid".into()),
            ("end_date".into(), "date_order".into()),
            ("download".into(), "failed".into()),
        ],
    );
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("value=\"2026-02-01\""));
    assert!(html.contains("value=\"2026-02-03\""));
    assert!(html.contains("checked"));
    assert!(html.contains("selected"));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("id=\"report_start_date_error\""));
    assert!(html.contains("aria-describedby=\"report_start_date_error\""));
    assert!(html.contains("id=\"report_end_date_error\""));
    assert!(html.contains("aria-describedby=\"report_end_date_error\""));
}

#[test]
fn reports_form_without_manageable_people_disables_download_and_explains() {
    let html = page(vec![], ReportDraft::default(), vec![]);
    assert!(html.contains("do not manage anyone"));
    assert!(html.contains("disabled"));
    assert!(!html.contains("<option value=\""));
}

#[test]
fn reports_form_renders_localized_in_every_supported_locale() {
    let expected = [
        (Locale::En, "en", "Reports", "Download PDF"),
        (Locale::Cy, "cy", "Adroddiadau", "Lawrlwytho PDF"),
        (Locale::Ga, "ga", "Tuarascálacha", "Íoslódáil PDF"),
        (Locale::Es, "es", "Informes", "Descargar PDF"),
        (Locale::Pt, "pt", "Relatórios", "Transferir PDF"),
    ];
    for (locale, code, title, download) in expected {
        let html = render_reports(
            "Test household",
            "test-slug",
            locale,
            choices(),
            ReportDraft::default(),
            vec![],
        )
        .unwrap_or_else(|error| panic!("{code} reports render failed: {error}"));
        assert!(
            html.contains(&format!("lang=\"{code}\"")),
            "missing {code} lang"
        );
        assert!(html.contains(title), "missing {code} title");
        assert!(html.contains(download), "missing {code} download label");
    }
}
