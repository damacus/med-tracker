#[path = "../../support/showcase.rs"]
mod showcase;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    let document = web_sys::window().unwrap().document().unwrap();
    let open = document
        .document_element()
        .unwrap()
        .get_attribute("data-initial-open")
        .unwrap();
    leptos::mount::hydrate_body(move || showcase::app(&open));
    document
        .document_element()
        .unwrap()
        .set_attribute("data-hydrated", "true")
        .unwrap();
}
