use medtracker_web::household_i18n::{Locale, Text};
use medtracker_web::stock::{OptionStock, StockDraft, StockPage, render_stock};
use std::collections::BTreeMap;

fn render(locale: Locale, supply: Option<String>) -> String {
    render_stock(StockPage {
        household_name: "Home".into(),
        slug: "home".into(),
        medication_id: "1".into(),
        medication_name: "Example".into(),
        supply,
        unit: "ml".into(),
        status: "not_needed".into(),
        options: vec![OptionStock {
            id: 2,
            quantity: None,
            unit: "tablet".into(),
        }],
        can_manage: true,
        csrf: "csrf".into(),
        locale,
        action: None,
        draft: StockDraft::default(),
        errors: BTreeMap::new(),
        notifications_visible: true,
    })
    .unwrap()
}

#[test]
fn untracked_parent_is_distinct_from_zero_scalar_fallback_in_every_locale() {
    for locale in Locale::ALL {
        let text = Text::new(locale);
        let untracked = render(locale, None);
        assert!(
            untracked.contains(&text.get("stock_removals.errors.untracked", &[]).unwrap()),
            "null parent needs explicit no tracked stock message in {}",
            locale.as_str()
        );
        assert!(
            !untracked.contains(&text.get("medications.stock.scalar_fallback", &[]).unwrap()),
            "null parent must not claim an available scalar balance"
        );
        let empty = render(locale, Some("0.0".into()));
        assert!(empty.contains(&text.get("medications.stock.scalar_fallback", &[]).unwrap()));
        assert!(empty.contains("0.0"));
    }
}
