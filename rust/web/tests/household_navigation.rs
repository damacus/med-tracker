use medtracker_web::household_i18n::Locale;
use medtracker_web::{
    MedicationCard, MedicationDetail, MedicationDetailRender,
    render_medication_detail_with_management, render_medication_list_with_management,
};

fn medication() -> MedicationDetail {
    MedicationDetail {
        id: 42,
        name: "Test medication".into(),
        description: "Test description".into(),
        supply: "20.75".into(),
        unit: "ml".into(),
        location: "Test cupboard".into(),
        sources: Vec::new(),
    }
}

fn detail() -> MedicationDetailRender<'static> {
    MedicationDetailRender {
        household_name: "Test household",
        slug: "test",
        csrf: "test-csrf",
        medication: medication(),
        stock_options: Vec::new(),
        taken_at: "2026-10-01T09:00",
        client_uuid: "test-client",
        notice: None,
        form_state: None,
    }
}

#[test]
fn inventory_navigation_reaches_people_locations_and_only_permitted_creation() {
    for can_create in [false, true] {
        let html = render_medication_list_with_management(
            "Test household",
            "test",
            "test-csrf",
            Vec::new(),
            can_create,
            Locale::En,
        );
        for destination in ["people", "locations"] {
            assert!(html.contains(&format!("href=\"/households/test/{destination}\"")));
        }
        assert_eq!(
            html.contains("href=\"/households/test/medications/new\""),
            can_create
        );
        if can_create {
            assert!(html.contains("Add Medication"));
        }
    }
}

#[test]
fn medication_detail_navigation_reaches_people_locations_and_only_permitted_editing() {
    for can_edit in [false, true] {
        let html = render_medication_detail_with_management(detail(), can_edit, Locale::En);
        for destination in ["people", "locations"] {
            assert!(html.contains(&format!("href=\"/households/test/{destination}\"")));
        }
        assert_eq!(
            html.contains("href=\"/households/test/medications/42/edit\""),
            can_edit
        );
        assert!(html.contains("Test medication"));
    }
}

fn inventory(locale: Locale) -> String {
    render_medication_list_with_management(
        "Test household",
        "test",
        "test-csrf",
        vec![MedicationCard {
            id: 42,
            name: "Test medication".into(),
            supply: "1.25".into(),
            unit: "ml".into(),
        }],
        true,
        locale,
    )
}

#[test]
fn returned_inventory_uses_the_selected_locale_for_its_document_and_heading() {
    for (locale, title) in [
        (Locale::En, "Medications"),
        (Locale::Cy, "Meddyginiaethau"),
        (Locale::Ga, "Leigheasanna"),
        (Locale::Es, "Medicamentos"),
        (Locale::Pt, "Medicamentos"),
    ] {
        let html = inventory(locale);
        assert!(
            html.contains(&format!("lang=\"{}\"", locale.as_str())),
            "inventory document must preserve {}",
            locale.as_str()
        );
        assert!(html.contains(&format!("<title>{title} | MedTracker</title>")));
        assert!(html.contains(&format!("<h1>{title}</h1>")));
    }
}

#[test]
fn inventory_stock_keeps_the_exact_amount_and_unit_in_each_locale() {
    for (locale, remaining) in [
        (Locale::En, "1.25 ml remaining"),
        (Locale::Cy, "1.25 ml yn weddill"),
        (Locale::Ga, "1.25 ml fágtha"),
        (Locale::Es, "1.25 ml restantes"),
        (Locale::Pt, "1.25 ml restantes"),
    ] {
        assert!(
            inventory(locale).replace("<!>", "").contains(remaining),
            "stock text must retain the exact decimal amount and unit in {}",
            locale.as_str()
        );
    }
}

#[test]
fn inventory_navigation_and_view_links_use_each_locales_catalogue_labels() {
    for (locale, dashboard, label, view) in [
        (Locale::En, "Dashboard", "Inventory", "View"),
        (Locale::Cy, "Dangosfwrdd", "Rhestr Eiddo", "Gweld"),
        (Locale::Ga, "Dashboard", "Stoc", "Amharc"),
        (Locale::Es, "Panel", "Inventario", "Ver"),
        (Locale::Pt, "Painel", "Inventário", "Ver"),
    ] {
        let html = inventory(locale);
        for (path, text) in [
            ("dashboard", dashboard),
            ("medications", label),
            ("medications/42", view),
        ] {
            assert!(
                html.contains(&format!("href=\"/households/test/{path}\">{text}</a>")),
                "inventory {path} link must be labelled {text} in {}",
                locale.as_str()
            );
        }
    }
}

#[test]
fn management_detail_document_preserves_each_selected_locale() {
    for locale in [Locale::En, Locale::Cy, Locale::Ga, Locale::Es, Locale::Pt] {
        let html = render_medication_detail_with_management(detail(), true, locale);
        assert!(
            html.contains(&format!("lang=\"{}\"", locale.as_str())),
            "management detail document must preserve {}",
            locale.as_str()
        );
        assert!(html.contains("<title>Test medication | MedTracker</title>"));
        assert!(html.contains("<h1>Test medication</h1>"));
    }
}

#[test]
fn management_detail_headings_use_each_locales_catalogue_text() {
    for (locale, profile, overview, inventory_status) in [
        (
            Locale::En,
            "Medication Profile",
            "Overview",
            "Inventory Status",
        ),
        (
            Locale::Cy,
            "Proffil Meddyginiaeth",
            "Trosolwg",
            "Statws Rhestr",
        ),
        (
            Locale::Ga,
            "Próifíl Leigheas",
            "Forbhreathnú",
            "Stádas Inventár",
        ),
        (
            Locale::Es,
            "Perfil del Medicamento",
            "Resumen",
            "Estado del Inventario",
        ),
        (
            Locale::Pt,
            "Perfil do Medicamento",
            "Visão Geral",
            "Estado do Inventário",
        ),
    ] {
        let html = render_medication_detail_with_management(detail(), true, locale);
        assert!(
            html.contains(profile),
            "profile text must use {}",
            locale.as_str()
        );
        for heading in [overview, inventory_status] {
            assert!(
                html.contains(&format!("<h2>{heading}</h2>")),
                "management detail heading {heading} must use {}",
                locale.as_str()
            );
        }
    }
}

#[test]
fn management_detail_navigation_and_edit_label_use_each_locale() {
    for (locale, dashboard, inventory, edit) in [
        (Locale::En, "Dashboard", "Inventory", "Edit Medication"),
        (
            Locale::Cy,
            "Dangosfwrdd",
            "Rhestr Eiddo",
            "Golygu Meddyginiaeth",
        ),
        (Locale::Ga, "Dashboard", "Stoc", "Cuir Leigheas in Eagar"),
        (Locale::Es, "Panel", "Inventario", "Editar Medicamento"),
        (Locale::Pt, "Painel", "Inventário", "Editar Medicamento"),
    ] {
        let html = render_medication_detail_with_management(detail(), true, locale);
        for (path, label) in [("dashboard", dashboard), ("medications", inventory)] {
            assert!(
                html.contains(&format!("href=\"/households/test/{path}\">{label}</a>")),
                "management detail {path} link must use {}",
                locale.as_str()
            );
        }
        assert!(html.contains("href=\"/households/test/medications/42/edit\""));
        assert!(html.contains(edit));
    }
}
