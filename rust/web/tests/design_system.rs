use medtracker_web::{rails_design_stylesheet, rails_font};

#[test]
fn design_assets_preserve_the_canonical_rails_declarations() {
    let source = include_str!("../../../app/assets/tailwind/application.css");
    let exported = rails_design_stylesheet();
    let first_font = source.find("@font-face").unwrap();
    let theme = source.find("@theme inline").unwrap();
    assert!(exported.contains(&source[first_font..theme]));
    let palette = source.find(":root[data-allow-palette=\"true\"]").unwrap();
    let components = source.find("@layer components").unwrap();
    assert!(exported.contains(&source[palette..components]));
    assert!(!exported.contains("@apply"));
    assert!(!exported.contains("@theme"));
}

#[test]
fn every_declared_font_is_embedded_from_the_rails_asset() {
    let source = include_str!("../../../app/assets/tailwind/application.css");
    for suffix in source.split("url('").skip(1) {
        let url = suffix.split("')").next().unwrap();
        if !url.starts_with("/fonts/") {
            continue;
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../app/assets")
            .join(url.trim_start_matches('/'));
        assert_eq!(rails_font(url).unwrap(), std::fs::read(path).unwrap());
    }
    assert!(rails_font("/fonts/../secrets").is_none());
    assert!(rails_font("/fonts/unknown.woff2").is_none());
}

#[test]
fn profile_overlays_are_delivered_with_the_shared_design_assets() {
    assert!(rails_design_stylesheet().contains(include_str!(
        "../../../app/assets/tailwind/profile-overlays.css"
    )));
    assert!(rails_design_stylesheet().contains(".profile-dialog[open]"));
    assert!(rails_design_stylesheet().contains(".profile-sheet[open]"));
}
