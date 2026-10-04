#[path = "support/showcase.rs"]
mod showcase;
use leptos::prelude::*;

fn main() {
    let destination = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&destination).unwrap();
    for (name, open) in [("closed", "none"), ("dialog", "dialog"), ("sheet", "sheet")] {
        let body = Owner::new().with(|| showcase::app(open).to_html());
        let document = format!(
            "<!doctype html><html lang=\"en\" data-initial-open=\"{open}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><script defer src=\"/ui.js\"></script><script defer src=\"/start.js\"></script></head><body>{body}</body></html>"
        );
        std::fs::write(format!("{destination}/{name}.html"), document).unwrap();
    }
}
