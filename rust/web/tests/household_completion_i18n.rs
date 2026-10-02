use medtracker_web::household_i18n::{Locale, Text};
use medtracker_web::locations::{LocationDraft, LocationFormPage, render_location_form};
use medtracker_web::people::{PersonDraft, render_person_form};
use std::collections::HashMap;

fn person(locale: Locale, message: &str) -> String {
    render_person_form(
        "Test household",
        "test",
        "real-csrf",
        locale,
        Some(42),
        PersonDraft {
            name: "Retained <person>".into(),
            email: "draft@example.test".into(),
            date_of_birth: "1980-02-03".into(),
            person_type: "adult".into(),
            has_capacity: "true".into(),
        },
        vec![
            ("name".into(), message.into()),
            ("base".into(), message.into()),
        ],
    )
    .unwrap()
}

fn location(locale: Locale, message: &str) -> String {
    render_location_form(LocationFormPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        locale,
        csrf: "real-csrf".into(),
        action: "/households/test/locations/42".into(),
        title: Text::new(locale)
            .get("locations.show.edit_location", &[])
            .unwrap(),
        draft: LocationDraft {
            name: "Retained <cupboard>".into(),
            description: "Retained </textarea><script>alert('draft')</script> & description".into(),
            etag: "\"version-1\"".into(),
            idempotency_key: "retained-key".into(),
        },
        errors: HashMap::from([
            ("name".into(), vec![message.into()]),
            ("base".into(), vec![message.into()]),
        ]),
    })
    .unwrap()
}

#[test]
fn unknown_people_and_location_errors_use_safe_localised_messages_and_retain_drafts() {
    let unknown = "Unrecognised backend diagnostic: private clinical detail";
    for (locale, expected) in [
        (Locale::En, "This value could not be saved."),
        (Locale::Cy, "Ni ellid cadw'r gwerth hwn."),
        (Locale::Ga, "Níorbh fhéidir an luach seo a shábháil."),
        (Locale::Es, "No se pudo guardar este valor."),
        (Locale::Pt, "Não foi possível guardar este valor."),
    ] {
        for (resource, html) in [
            ("person", person(locale, unknown)),
            ("location", location(locale, unknown)),
        ] {
            assert!(
                !html.contains(unknown),
                "{resource} {} must not expose unrecognised API diagnostics",
                locale.as_str()
            );
            assert!(
                html.contains(expected),
                "{resource} {} needs a useful translated safe error",
                locale.as_str()
            );
            assert!(html.contains("aria-invalid=\"true\""));
            assert!(html.contains("aria-describedby="));
            assert!(html.contains("name=\"authenticity_token\""));
            assert!(html.contains(&format!("lang=\"{}\"", locale.as_str())));
        }
        let person = person(locale, unknown);
        assert!(person.contains("Retained &lt;person&gt;"));
        assert!(person.contains("draft@example.test"));
        assert!(person.contains("1980-02-03"));
        let location = location(locale, unknown);
        assert!(location.contains("Retained &lt;cupboard&gt;"));
        assert!(location.contains("Retained &lt;/textarea&gt;&lt;script&gt;alert('draft')&lt;/script&gt; &amp; description"));
        assert!(!location.contains("<script>alert('draft')</script>"));
        assert!(location.contains("version-1"));
        assert!(location.contains("retained-key"));
    }
}

#[test]
fn known_and_pretranslated_people_and_location_errors_preserve_their_translation() {
    for (locale, expected) in [
        (Locale::En, "can't be blank"),
        (Locale::Cy, "ni all fod yn wag"),
        (Locale::Ga, "ní féidir a bheith folamh"),
        (Locale::Es, "no puede estar en blanco"),
        (Locale::Pt, "não pode ficar em branco"),
    ] {
        for message in ["can't be blank", expected] {
            for (resource, html) in [
                ("person", person(locale, message)),
                ("location", location(locale, message)),
            ] {
                assert!(
                    html.contains(expected),
                    "{resource} {} must preserve known validation text",
                    locale.as_str()
                );
                assert!(html.contains("aria-invalid=\"true\""));
                assert!(html.contains("aria-describedby="));
            }
        }
    }
}
