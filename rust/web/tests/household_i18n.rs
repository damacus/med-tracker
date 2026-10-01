use leptos::prelude::*;
use medtracker_web::household_i18n::{Locale, Text};

#[test]
fn resolves_allowlisted_cookie_then_weighted_header_then_english() {
    assert_eq!(Locale::resolve(Some("cy"), Some("es;q=1")), Locale::Cy);
    assert_eq!(
        Locale::resolve(Some("invalid"), Some("ga-IE;q=0.8,es;q=0.2")),
        Locale::Ga
    );
    assert_eq!(
        Locale::resolve(None, Some("es;q=0.1,pt-PT;q=0.9")),
        Locale::Pt
    );
    assert_eq!(Locale::resolve(None, Some("fr;q=1,cy;q=0")), Locale::En);
    assert_eq!(
        Locale::resolve(None, Some("es;q=invalid,ga;q=0.2")),
        Locale::Ga
    );
    assert_eq!(Locale::resolve(None, None), Locale::En);
}

#[test]
fn reads_authoritative_household_labels_in_all_five_locales() {
    for (locale, expected) in [
        (Locale::En, "Household Settings"),
        (Locale::Cy, "Gosodiadau aelwyd"),
        (Locale::Ga, "Socruithe teaghlaigh"),
        (Locale::Es, "Configuración del hogar"),
        (Locale::Pt, "Definições do agregado"),
    ] {
        assert_eq!(
            Text::new(locale)
                .get("admin.households.title", &[])
                .unwrap(),
            expected
        );
    }
}

#[test]
fn keeps_interpolation_values_literal_including_placeholder_characters() {
    let name = "<b>Ada</b> %{count}";
    let text = Text::new(Locale::En)
        .get("dashboard.greeting_morning", &[("name", name)])
        .unwrap();
    assert_eq!(text, "Good morning, <b>Ada</b> %{count}");
    let rendered = view! { <p>{text}</p> }.to_html();
    assert!(rendered.contains("&lt;b&gt;Ada&lt;/b&gt;"));
    assert!(!rendered.contains("<b>Ada</b>"));
}

#[test]
fn preserves_exact_zero_one_other_for_rails_catalogues() {
    let text = Text::new(Locale::En);
    assert_eq!(
        text.plural("dashboard.dose_progress.aria_given", 0, &[])
            .unwrap(),
        "No doses given today"
    );
    assert_eq!(
        text.plural("dashboard.dose_progress.aria_given", 1, &[])
            .unwrap(),
        "1 dose given today"
    );
    assert_eq!(
        text.plural("dashboard.dose_progress.aria_given", 2, &[])
            .unwrap(),
        "2 doses given today"
    );
    assert_eq!(
        text.plural("dashboard.variants.family_lanes.task_count", 0, &[])
            .unwrap(),
        "0 tasks today"
    );
    for locale in [Locale::En, Locale::Cy, Locale::Ga, Locale::Es, Locale::Pt] {
        for count in [0, 1, 2, 3, 6, 7, 10, 11, 21] {
            assert!(
                Text::new(locale)
                    .plural("dashboard.dose_progress.aria_given", count, &[])
                    .is_ok()
            );
        }
    }
}

#[test]
fn rejects_unknown_keys_and_missing_interpolation_values() {
    let text = Text::new(Locale::En);
    assert!(text.get("unknown.key", &[]).is_err());
    assert!(text.get("dashboard.greeting_morning", &[]).is_err());
    assert!(text.get("admin.households", &[]).is_err());
}

#[test]
fn different_request_locales_do_not_share_mutable_state() {
    let handles: Vec<_> = [Locale::Cy, Locale::Ga, Locale::Es, Locale::Pt]
        .into_iter()
        .map(|locale| {
            std::thread::spawn(move || {
                let text = Text::new(locale);
                (locale, text.get("admin.households.title", &[]).unwrap())
            })
        })
        .collect();
    for handle in handles {
        let (locale, title) = handle.join().unwrap();
        assert_ne!(locale, Locale::En);
        assert_ne!(title, "Household Settings");
    }
}

#[test]
fn translates_known_api_validation_messages_without_inventing_unknown_copy() {
    let text = Text::new(Locale::Es);
    assert_eq!(
        text.api_error("can't be blank"),
        Some(text.get("errors.messages.blank", &[]).unwrap())
    );
    assert_eq!(
        text.api_error("is invalid"),
        Some(text.get("errors.messages.invalid", &[]).unwrap())
    );
    assert_eq!(
        text.api_error("has already been taken"),
        Some(text.get("errors.messages.taken", &[]).unwrap())
    );
    assert_eq!(text.api_error("unrecognised upstream detail"), None);
}

#[test]
fn translates_medication_numeric_and_unit_validation_using_catalogue_messages() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        assert_eq!(
            text.api_error("is not a number"),
            Some(text.get("errors.messages.not_a_number", &[]).unwrap())
        );
        assert_eq!(
            text.api_error("is not included in the list"),
            Some(text.get("errors.messages.inclusion", &[]).unwrap())
        );
        assert_eq!(
            text.api_error("must be greater than 0"),
            Some(
                text.get("errors.messages.greater_than", &[("count", "0")])
                    .unwrap()
            )
        );
        assert_eq!(
            text.api_error("must be greater than or equal to 0"),
            Some(
                text.get(
                    "errors.messages.greater_than_or_equal_to",
                    &[("count", "0")]
                )
                .unwrap()
            )
        );
        assert_eq!(
            text.api_error("must be greater than a custom threshold"),
            None
        );
    }
}

#[test]
fn dosage_options_notice_is_present_in_every_authoritative_locale() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        let key = "forms.medications.dosage_options_read_only";
        assert_eq!(text.source_locale(key).unwrap(), locale);
        assert!(!text.get(key, &[]).unwrap().is_empty());
    }
}

#[test]
fn new_location_heading_is_present_in_every_authoritative_locale() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        let key = "forms.locations.new_title";
        assert_eq!(text.source_locale(key).unwrap(), locale);
        assert!(!text.get(key, &[]).unwrap().is_empty());
    }
    assert_eq!(
        Text::new(Locale::En)
            .get("forms.locations.new_title", &[])
            .unwrap(),
        "New Location"
    );
}

#[test]
fn stock_remaining_copy_keeps_amount_and_unit_placeholders_in_every_locale() {
    let key = "medications.index.stock_remaining";
    for locale in Locale::ALL {
        let text = Text::new(locale);
        assert_eq!(text.source_locale(key).unwrap(), locale);
        let rendered = text
            .get(key, &[("amount", "12.50"), ("unit", "mg")])
            .unwrap();
        assert!(rendered.contains("12.50"));
        assert!(rendered.contains("mg"));
        assert!(text.get(key, &[("amount", "12.50")]).is_err());
    }
    assert_eq!(
        Text::new(Locale::En)
            .get(key, &[("amount", "12.50"), ("unit", "mg")])
            .unwrap(),
        "12.50 mg remaining"
    );
}
