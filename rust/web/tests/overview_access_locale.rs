use medtracker_web::household_i18n::{Locale, Text};
use medtracker_web::treatments::{TreatmentOverview, render_treatment_overview_with_access};

#[test]
fn unavailable_schedules_do_not_claim_that_an_empty_assignment_list_proves_no_treatments() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        let html = render_treatment_overview_with_access(
            "synthetic",
            7,
            locale,
            false,
            TreatmentOverview {
                rows: Vec::new(),
                schedules_unavailable: true,
            },
        )
        .unwrap();
        assert!(
            html.contains(
                &text
                    .get("treatments.overview.schedules_unavailable", &[])
                    .unwrap()
            ),
            "Unavailable schedules need truthful guidance in {}",
            locale.as_str()
        );
        assert!(
            !html.contains(&text.get("treatments.overview.empty", &[]).unwrap()),
            "Restricted schedules cannot be reported absent in {}",
            locale.as_str()
        );
    }
}

#[test]
fn fully_readable_empty_treatments_retain_the_existing_empty_message() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        let html = render_treatment_overview_with_access(
            "synthetic",
            7,
            locale,
            false,
            TreatmentOverview::default(),
        )
        .unwrap();
        assert!(html.contains(&text.get("treatments.overview.empty", &[]).unwrap()));
        assert!(
            !html.contains(
                &text
                    .get("treatments.overview.schedules_unavailable", &[])
                    .unwrap()
            )
        );
    }
}
