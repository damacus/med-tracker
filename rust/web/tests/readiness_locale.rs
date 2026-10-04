use medtracker_web::household_i18n::{Locale, Text};
use medtracker_web::medication_management::{
    MedicationDraft, MedicationFormPage, render_medication_form,
};
use medtracker_web::treatments::{TreatmentDraft, TreatmentFormPage, render_assignment_form};
use std::collections::BTreeMap;

fn missing_version(locale: Locale) {
    let page = |message: &str| MedicationFormPage {
        household_name: "Synthetic household".into(),
        slug: "synthetic".into(),
        csrf: "synthetic-token".into(),
        locale,
        medication_id: Some("42".into()),
        draft: MedicationDraft {
            name: "Synthetic medicine".into(),
            ..Default::default()
        },
        locations: vec![("1".into(), "Synthetic location".into())],
        errors: BTreeMap::from([("base".into(), vec![message.into()])]),
        notifications_visible: true,
    };
    let html = render_medication_form(page("If-Match is required")).unwrap();
    let expected = Text::new(locale)
        .get("medications.stock.original_token", &[])
        .unwrap();
    assert!(
        html.contains(&expected),
        "Missing version needs reopen guidance in {}",
        locale.as_str()
    );
    assert!(!html.contains("If-Match is required"));
    let unknown = render_medication_form(page("private database diagnostic")).unwrap();
    assert!(!unknown.contains("private database diagnostic"));
    assert!(
        !unknown.contains(&expected),
        "Unknown errors must not be labelled as missing versions"
    );
}

fn assignment_choice(locale: Locale, placeholder: &str, guidance: &str) {
    let page = |editing| TreatmentFormPage {
        household_name: "Synthetic household".into(),
        slug: "synthetic".into(),
        person_id: "7".into(),
        person_name: "Synthetic person".into(),
        csrf: "synthetic-token".into(),
        locale,
        action: "/synthetic".into(),
        editing,
        schedule: false,
        medications: vec![("42".into(), "Synthetic medicine".into())],
        dosages: vec![("12".into(), "1.25 ml".into())],
        units: vec![("ml".into(), "ml".into())],
        draft: TreatmentDraft {
            fields: BTreeMap::from([
                ("medication_id".into(), "42".into()),
                ("dose_amount".into(), "1.25".into()),
                ("dose_unit".into(), "ml".into()),
                ("administration_kind".into(), "as_needed".into()),
            ]),
        },
        errors: BTreeMap::new(),
        notifications_visible: true,
    };
    let edit = render_assignment_form(page(true)).unwrap();
    assert!(
        edit.contains(placeholder),
        "Blank option needs an honest editing placeholder in {}",
        locale.as_str()
    );
    assert!(
        edit.contains(guidance),
        "Linked dose replacement needs guidance in {}",
        locale.as_str()
    );
    assert!(
        !edit.contains("value=\"12\" selected"),
        "No unknown link identity may be fabricated"
    );
    let new = render_assignment_form(page(false)).unwrap();
    assert!(
        new.contains(
            &Text::new(locale)
                .get("treatments.form.default_dose", &[])
                .unwrap()
        )
    );
    assert!(!new.contains(guidance));
}

#[test]
fn missing_version_en() {
    missing_version(Locale::En);
}
#[test]
fn missing_version_cy() {
    missing_version(Locale::Cy);
}
#[test]
fn missing_version_ga() {
    missing_version(Locale::Ga);
}
#[test]
fn missing_version_es() {
    missing_version(Locale::Es);
}
#[test]
fn missing_version_pt() {
    missing_version(Locale::Pt);
}

#[test]
fn assignment_choice_en() {
    assignment_choice(
        Locale::En,
        "Keep current dose choice",
        "Leave the option blank to keep any existing link. If the dose is linked, choose a replacement option before changing its amount or unit.",
    );
}
#[test]
fn assignment_choice_cy() {
    assignment_choice(
        Locale::Cy,
        "Cadw'r dewis dos presennol",
        "Gadewch y dewis yn wag i gadw unrhyw gyswllt presennol. Os yw'r dos wedi'i gysylltu, dewiswch opsiwn newydd cyn newid ei swm neu ei uned.",
    );
}
#[test]
fn assignment_choice_ga() {
    assignment_choice(
        Locale::Ga,
        "Coinnigh an rogha dáileoige reatha",
        "Fág an rogha bán chun aon nasc atá ann a choinneáil. Má tá an dáileog nasctha, roghnaigh rogha ionaid sula n-athraíonn tú a méid nó a haonad.",
    );
}
#[test]
fn assignment_choice_es() {
    assignment_choice(
        Locale::Es,
        "Mantener la elección de dosis actual",
        "Deje la opción en blanco para conservar cualquier vínculo existente. Si la dosis está vinculada, elija una opción de reemplazo antes de cambiar su cantidad o unidad.",
    );
}
#[test]
fn assignment_choice_pt() {
    assignment_choice(
        Locale::Pt,
        "Manter a escolha de dose atual",
        "Deixe a opção em branco para manter qualquer ligação existente. Se a dose estiver ligada, escolha uma opção de substituição antes de alterar a quantidade ou a unidade.",
    );
}
