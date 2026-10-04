use medtracker_web::household::household_document;
use medtracker_web::household_i18n::Locale;
use medtracker_web::locations::{LocationDraft, LocationFormPage, render_location_form};
use medtracker_web::medication_management::{
    MedicationDraft, MedicationFormPage, render_medication_form,
    render_medication_form_with_options,
};
use medtracker_web::people::{PersonDraft, render_person_form};
use std::collections::BTreeMap;
use std::collections::HashMap;

#[test]
fn household_shell_escapes_untrusted_labels_and_preserves_the_page_body() {
    let html = household_document(
        "<script>alert('title')</script>",
        "Household <img src=x onerror=alert(1)>",
        "safe-household",
        "en",
        "<h1>People</h1><form method=\"post\"><input name=\"name\" value=\"Existing value\"></form>".into(),
    );
    assert!(!html.contains("<script>alert('title')</script>"));
    assert!(!html.contains("<img src=x onerror=alert(1)>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("Household &lt;img"));
    assert!(html.contains("<h1>People</h1>"));
    assert!(html.contains("name=\"name\" value=\"Existing value\""));
}

#[test]
fn household_shell_keeps_navigation_in_the_current_household() {
    let html = household_document(
        "People",
        "Test household",
        "test-household",
        "en",
        String::new(),
    );
    for destination in [
        "dashboard",
        "medications",
        "people",
        "locations",
        "profile",
        "settings/notifications",
    ] {
        assert!(html.contains(&format!(
            "href=\"/households/test-household/{destination}\""
        )));
    }
    assert!(html.contains("<nav"));
    assert!(html.contains("aria-label="));
    assert!(html.contains("name=\"viewport\""));
    assert!(html.contains("width=device-width"));
}

#[test]
fn household_shell_sets_supported_document_languages_and_rejects_injected_language() {
    for locale in ["en", "es", "pt", "cy", "ga"] {
        let html = household_document("People", "Test", "test", locale, String::new());
        assert!(
            html.contains(&format!("lang=\"{locale}\"")),
            "missing document locale {locale}"
        );
    }
    let html = household_document(
        "People",
        "Test",
        "test",
        "en\" onload=\"alert(1)",
        String::new(),
    );
    assert!(html.contains("lang=\"en\""));
    assert!(!html.contains("onload="));
}

#[test]
fn person_edit_renders_initial_values_native_submission_and_associated_errors() {
    let html = render_person_form(
        "Test household",
        "test",
        "test-csrf",
        Locale::En,
        Some(42),
        PersonDraft {
            name: "Draft <person>".into(),
            email: "draft@example.test".into(),
            date_of_birth: "1980-02-03".into(),
            person_type: "adult".into(),
            has_capacity: "true".into(),
        },
        vec![("name".into(), "can't be blank".into())],
    )
    .unwrap();
    assert!(html.contains("<h1"));
    assert!(html.contains("Edit Person"));
    assert!(html.contains("action=\"/households/test/people/42\""));
    assert!(html.contains("method=\"post\""));
    assert!(html.contains("name=\"authenticity_token\""));
    assert!(html.contains("value=\"test-csrf\""));
    assert!(html.contains("Draft &lt;person&gt;"));
    assert!(!html.contains("Draft <person>"));
    assert!(html.contains("value=\"draft@example.test\""));
    assert!(html.contains("value=\"1980-02-03\""));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("aria-describedby=\"person_name_error\""));
    assert!(html.contains("id=\"person_name_error\""));
    assert!(html.contains("Update Person"));
}

#[test]
fn location_edit_renders_retained_values_precondition_and_associated_errors() {
    let html = render_location_form(LocationFormPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        locale: Locale::En,
        csrf: "test-csrf".into(),
        action: "/households/test/locations/42".into(),
        title: "Edit Location".into(),
        draft: LocationDraft {
            name: "Draft <cupboard>".into(),
            description: "Retained </textarea><script>alert('draft')</script> & description".into(),
            etag: "\"version-1\"".into(),
            idempotency_key: "location-test-key".into(),
        },
        errors: HashMap::from([("name".into(), vec!["can't be blank".into()])]),
    })
    .unwrap();
    assert!(html.contains("Edit Location"));
    assert!(html.contains("action=\"/households/test/locations/42\""));
    assert!(html.contains("method=\"post\""));
    assert!(html.contains("name=\"authenticity_token\""));
    assert!(html.contains("value=\"test-csrf\""));
    assert!(html.contains("Draft &lt;cupboard&gt;"));
    assert!(html.contains(
        "Retained &lt;/textarea&gt;&lt;script&gt;alert('draft')&lt;/script&gt; &amp; description"
    ));
    assert!(!html.contains("Retained </textarea><script>"));
    assert!(!html.contains("Draft <cupboard>"));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("aria-describedby="));
    assert!(html.contains("name=\"etag\""));
    assert!(html.contains("version-1"));
    assert!(html.contains("Save Location"));
}

#[test]
fn medication_edit_preserves_decimal_drafts_identity_and_native_error_association() {
    let html = render_medication_form(MedicationFormPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        csrf: "test-csrf".into(),
        locale: Locale::En,
        medication_id: Some("42".into()),
        draft: MedicationDraft {
            name: "Draft <medicine>".into(),
            friendly_name: "Draft display name".into(),
            description: "Retained description".into(),
            barcode: "123456789".into(),
            dose_amount: "2.50".into(),
            dose_unit: "ml".into(),
            current_supply: "20.75".into(),
            reorder_threshold: "3.5".into(),
            location_id: "9".into(),
            warnings: "Retained </textarea><script>alert('warning')</script> & warning".into(),
            etag: "\"version-1\"".into(),
        },
        locations: vec![("9".into(), "Test cupboard".into())],
        errors: BTreeMap::from([("name".into(), vec!["can't be blank".into()])]),
    })
    .unwrap();
    assert!(html.contains("Edit Medication"));
    assert!(html.contains("action=\"/households/test/medications/42\""));
    assert!(html.contains("method=\"post\""));
    assert!(html.contains("name=\"authenticity_token\""));
    assert!(html.contains("value=\"test-csrf\""));
    assert!(html.contains("Draft &lt;medicine&gt;"));
    assert!(!html.contains("Draft <medicine>"));
    assert!(html.contains("value=\"Draft display name\""));
    assert!(html.contains("value=\"2.50\""));
    assert!(html.contains("value=\"20.75\""));
    assert!(html.contains(
        "Retained &lt;/textarea&gt;&lt;script&gt;alert('warning')&lt;/script&gt; &amp; warning"
    ));
    assert!(!html.contains("Retained </textarea><script>"));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("aria-describedby=\"medication-name-error\""));
    assert!(html.contains("id=\"medication-name-error\""));
    assert!(html.contains("name=\"etag\""));
    assert!(html.contains("version-1"));
    assert!(html.contains("Save Medication"));
}

#[test]
fn household_person_and_medication_forms_render_authoritative_locales_without_losing_drafts() {
    for (locale, person_heading, medication_heading, blank_error) in [
        (
            Locale::En,
            "Edit Person",
            "Edit Medication",
            "can't be blank",
        ),
        (
            Locale::Cy,
            "Golygu Person",
            "Golygu Meddyginiaeth",
            "ni all fod yn wag",
        ),
        (
            Locale::Ga,
            "Cuir Duine in Eagar",
            "Cuir Leigheas in Eagar",
            "ní féidir a bheith folamh",
        ),
        (
            Locale::Es,
            "Editar Persona",
            "Editar Medicamento",
            "no puede estar en blanco",
        ),
        (
            Locale::Pt,
            "Editar Pessoa",
            "Editar Medicamento",
            "não pode ficar em branco",
        ),
    ] {
        let person = render_person_form(
            "Test",
            "test",
            "csrf",
            locale,
            Some(42),
            PersonDraft {
                name: "Retained draft".into(),
                email: "draft@example.test".into(),
                date_of_birth: "1980-02-03".into(),
                person_type: "adult".into(),
                has_capacity: "true".into(),
            },
            vec![("name".into(), "can't be blank".into())],
        )
        .unwrap();
        assert!(person.contains(person_heading));
        assert!(person.contains(&format!("lang=\"{}\"", locale.as_str())));
        assert!(person.contains("value=\"Retained draft\""));
        assert!(person.contains("aria-describedby=\"person_name_error\""));
        assert!(
            person.contains(blank_error),
            "person field error must use {}",
            locale.as_str()
        );
        let medication = render_medication_form(MedicationFormPage {
            household_name: "Test".into(),
            slug: "test".into(),
            csrf: "csrf".into(),
            locale,
            medication_id: Some("42".into()),
            draft: MedicationDraft {
                name: "Retained draft".into(),
                dose_amount: "2.50".into(),
                dose_unit: "ml".into(),
                ..MedicationDraft::default()
            },
            locations: vec![("9".into(), "Test cupboard".into())],
            errors: BTreeMap::from([("name".into(), vec!["can't be blank".into()])]),
        })
        .unwrap();
        assert!(medication.contains(medication_heading));
        assert!(medication.contains(&format!("lang=\"{}\"", locale.as_str())));
        assert!(medication.contains("value=\"2.50\""));
        assert!(medication.contains("aria-describedby=\"medication-name-error\""));
        assert!(
            medication.contains(blank_error),
            "medication field error must use {}",
            locale.as_str()
        );
    }
}

#[test]
fn person_form_native_type_selection_and_capacity_checkbox_keep_the_initial_draft() {
    for (person_type, capacity) in [
        ("minor", "false"),
        ("dependent_adult", "false"),
        ("adult", "true"),
    ] {
        let html = render_person_form(
            "Test",
            "test",
            "csrf",
            Locale::En,
            Some(42),
            PersonDraft {
                name: "Retained draft".into(),
                email: String::new(),
                date_of_birth: "1980-02-03".into(),
                person_type: person_type.into(),
                has_capacity: capacity.into(),
            },
            Vec::new(),
        )
        .unwrap();
        let selected = html
            .split("<option")
            .skip(1)
            .find(|option| {
                option
                    .split('>')
                    .next()
                    .unwrap()
                    .contains(&format!("value=\"{person_type}\""))
            })
            .expect("native option for initial person type");
        assert!(selected.split('>').next().unwrap().contains("selected"));
        let checkbox = html
            .split("<input")
            .skip(1)
            .find(|input| {
                input
                    .split('>')
                    .next()
                    .unwrap()
                    .contains("id=\"person_has_capacity\"")
            })
            .expect("native capacity checkbox")
            .split('>')
            .next()
            .unwrap();
        assert!(checkbox.contains("type=\"checkbox\""));
        assert!(checkbox.contains("name=\"has_capacity\""));
        assert_eq!(checkbox.contains(" checked"), capacity == "true");
    }
}

#[test]
fn existing_dosage_options_cannot_be_overwritten_by_a_scalar_medication_edit_form() {
    let html = render_medication_form_with_options(
        MedicationFormPage {
            household_name: "Test".into(),
            slug: "test".into(),
            csrf: "csrf".into(),
            locale: Locale::En,
            medication_id: Some("42".into()),
            draft: MedicationDraft {
                name: "Tracked medication".into(),
                friendly_name: "Friendly tracked medication".into(),
                dose_amount: "2.50".into(),
                dose_unit: "ml".into(),
                current_supply: "20.75".into(),
                ..MedicationDraft::default()
            },
            locations: vec![("9".into(), "Test cupboard".into())],
            errors: BTreeMap::new(),
        },
        true,
    )
    .unwrap();
    assert!(html.contains("name=\"name\""));
    assert!(html.contains("name=\"friendly_name\""));
    assert!(!html.contains("name=\"dose_amount\""));
    assert!(!html.contains("name=\"dose_unit\""));
    assert!(!html.contains("name=\"current_supply\""));
    assert!(html.contains("dosage options"));
    assert!(html.contains("Save Medication"));
}
