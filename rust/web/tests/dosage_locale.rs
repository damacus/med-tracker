use medtracker_web::household_i18n::Locale;
use medtracker_web::{
    MedicationDetail, MedicationDetailRender, render_medication_detail_with_management,
};

#[test]
fn dosage_dialog_labels_and_unknown_errors_follow_the_selected_locale() {
    for locale in [Locale::Cy, Locale::Ga, Locale::Es, Locale::Pt] {
        let html = render_medication_detail_with_management(
            MedicationDetailRender {
                household_name: "Synthetic household",
                slug: "synthetic",
                csrf: "synthetic-token",
                medication: MedicationDetail {
                    id: 42,
                    name: "Synthetic medicine".into(),
                    description: String::new(),
                    supply: "4.5".into(),
                    unit: "ml".into(),
                    location: "Synthetic location".into(),
                    sources: vec![],
                },
                stock_options: vec![],
                taken_at: "2026-10-01T12:00",
                client_uuid: "synthetic",
                notice: Some("private database diagnostic"),
                form_state: None,
            },
            true,
            locale,
        );
        assert!(!html.contains("private database diagnostic"));
        assert!(!html.contains("Choose the person and source."));
        assert!(!html.contains("Confirm the person, medication, time, and inventory source."));
        assert!(!html.contains(">Taken at<"));
        assert!(html.contains(&format!("lang=\"{}\"", locale.as_str())));
    }
}
