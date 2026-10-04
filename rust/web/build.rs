use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..");
    let css_path = root.join("app/assets/tailwind/application.css");
    let overlays_path = root.join("app/assets/tailwind/profile-overlays.css");
    println!("cargo:rerun-if-changed={}", overlays_path.display());
    let picker_path = root.join("app/views/profiles/theme_picker_card.rb");
    println!("cargo:rerun-if-changed={}", css_path.display());
    println!("cargo:rerun-if-changed={}", picker_path.display());
    let source = fs::read_to_string(css_path).expect("canonical Rails stylesheet is required");
    let first_font = source
        .find("@font-face")
        .expect("Rails font declarations missing");
    let theme = source
        .find("@theme inline")
        .expect("Rails Tailwind theme boundary missing");
    let palette = source
        .find(":root[data-allow-palette=\"true\"]")
        .expect("Rails palette boundary missing");
    let components = source
        .find("@layer components")
        .expect("Rails component boundary missing");
    assert!(first_font < theme && theme < palette && palette < components);
    let mut css = format!(
        "{}{}",
        &source[first_font..theme],
        &source[palette..components]
    );
    assert!(!css.contains("@apply") && !css.contains("@theme"));
    css.push_str(
        &fs::read_to_string(overlays_path).expect("canonical Profile overlay styles are required"),
    );
    let picker = fs::read_to_string(picker_path).expect("canonical Rails theme picker is required");
    let mut theme_count = 0;
    for line in picker.lines().filter(|line| line.contains("color: 'bg-[#")) {
        let id = line
            .split("id: '")
            .nth(1)
            .unwrap()
            .split("'")
            .next()
            .unwrap();
        let colour = line
            .split("color: 'bg-[")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap();
        css.push_str(&format!("\n:root[data-theme=\"{id}\"]{{--profile-palette:{colour}}}\n.profile-theme[data-theme=\"{id}\"] .profile-theme-swatch{{background:{colour}}}\n"));
        theme_count += 1;
    }
    assert_eq!(
        theme_count, 10,
        "review the Rust picker when Rails changes its themes"
    );
    let mut fonts =
        String::from("pub fn rails_font(path: &str) -> Option<&'static [u8]> { match path {\n");
    for suffix in source.split("url('").skip(1) {
        let url = suffix.split("')").next().unwrap();
        if !url.starts_with("/fonts/") {
            continue;
        }
        let relative = format!("../../app/assets{url}");
        assert!(
            manifest.join(&relative).is_file(),
            "missing canonical font {url}"
        );
        println!(
            "cargo:rerun-if-changed={}",
            manifest.join(&relative).display()
        );
        fonts.push_str(&format!("{url:?} => Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{relative}\"))),\n"));
    }
    fonts.push_str("_ => None, } }\n");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out.join("rails-design.css"), css).unwrap();
    fs::write(out.join("rails-fonts.rs"), fonts).unwrap();
}
