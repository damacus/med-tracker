#![recursion_limit = "256"]

use leptodon::button::{Button, ButtonAppearance};
use leptodon::input::TextInput;
use leptodon::modal::{Modal, ModalFooterChildren};
use leptodon::radio::FormValue;
use leptodon::select::Select;
#[cfg(feature = "hydrate")]
use leptos::ev;
use leptos::html;
use leptos::oco::Oco;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PersonChoice {
    pub id: String,
    pub name: String,
}

impl fmt::Display for PersonChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(formatter)
    }
}

impl FormValue for PersonChoice {
    fn value(&self) -> Oco<'static, str> {
        Oco::Owned(self.id.clone())
    }
}

#[component]
pub fn PersonSelector(choices: Vec<PersonChoice>, selected_id: String) -> impl IntoView {
    let selected = choices
        .iter()
        .find(|choice| choice.id == selected_id)
        .cloned()
        .unwrap_or_else(|| choices[0].clone());
    view! {
        <label for="dashboard-native-person-select">"Switch person"</label>
        <Select id="dashboard-native-person-select" name="dashboard_person_id" required=true selected=RwSignal::new(selected) options=RwSignal::new(choices)/>
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchItem {
    pub label: String,
    pub detail: String,
    pub target_id: String,
}

pub fn search_items(items: &[SearchItem], query: &str) -> Vec<SearchItem> {
    let term = query.trim().to_lowercase();
    items
        .iter()
        .filter(|item| {
            term.is_empty()
                || item.label.to_lowercase().contains(&term)
                || item.detail.to_lowercase().contains(&term)
        })
        .cloned()
        .collect()
}

#[component]
pub fn SearchPalette(items: Vec<SearchItem>) -> impl IntoView {
    let visible = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let selected = RwSignal::new(0_usize);
    let input_ref = NodeRef::<html::Input>::new();
    let results = Memo::new({
        let items = items.clone();
        move |_| search_items(&items, &query.get())
    });

    #[cfg(feature = "hydrate")]
    {
        use leptos_dom::helpers::window_event_listener;
        Effect::new(move |_| {
            let matches = results.get();
            let index = selected.get();
            let expanded = visible.get();
            if let Some(input) = input_ref.get() {
                let _ = input.set_attribute("role", "combobox");
                let _ = input.set_attribute("aria-autocomplete", "list");
                let _ = input.set_attribute("aria-controls", "dashboard-search-results");
                let _ =
                    input.set_attribute("aria-expanded", if expanded { "true" } else { "false" });
                if let Some(item) = matches.get(index) {
                    let _ = input.set_attribute(
                        "aria-activedescendant",
                        &format!("dashboard-search-option-{}", item.target_id),
                    );
                } else {
                    let _ = input.remove_attribute("aria-activedescendant");
                }
            }
        });
        let listener = window_event_listener(ev::keydown, move |event| {
            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
                event.prevent_default();
                remember_focus();
                visible.set(true);
                query.set(String::new());
                selected.set(0);
            } else if visible.get_untracked() {
                match event.key().as_str() {
                    "Escape" => {
                        visible.set(false);
                        restore_focus();
                    }
                    "ArrowDown" => {
                        event.prevent_default();
                        let count = results.get_untracked().len();
                        if count > 0 {
                            selected.update(|value| *value = (*value + 1) % count);
                        }
                    }
                    "ArrowUp" => {
                        event.prevent_default();
                        let count = results.get_untracked().len();
                        if count > 0 {
                            selected.update(|value| *value = (*value + count - 1) % count);
                        }
                    }
                    "Enter" => {
                        event.prevent_default();
                        if let Some(item) = results.get_untracked().get(selected.get_untracked()) {
                            forget_focus();
                            visible.set(false);
                            activate_result(&item.target_id);
                        }
                    }
                    _ => {}
                }
            }
        });
        on_cleanup(move || listener.remove());
        Effect::watch(
            move || visible.get(),
            move |open, previous, _| {
                if *open {
                    leptos_dom::helpers::set_timeout(
                        move || {
                            if let Some(input) = input_ref.get() {
                                let _ = input.focus();
                            }
                        },
                        std::time::Duration::from_millis(25),
                    );
                } else if previous == Some(&true) {
                    restore_focus();
                }
            },
            false,
        );
    }

    let dataset = serde_json::to_string(&items).expect("serialize dashboard search items");
    view! {
        <div data-dashboard-search=dataset>
            <Button class="dashboard-search" appearance=ButtonAppearance::Secondary on_click=move |_| {
                #[cfg(feature = "hydrate")]
                remember_focus();
                visible.set(true);
                query.set(String::new());
                selected.set(0);
            }>
                <span data-testid="dashboard-search-trigger">"Search"</span><kbd>"Ctrl K"</kbd>
            </Button>
            <Modal id="dashboard-search-dialog" title="Search this dashboard" visible=visible footer=ModalFooterChildren { children: Box::new(|| view! { <span class="text-sm text-gray-500">"Visible medicines and stock only · Esc to close"</span> }.into_any()) }>
                <div on:input=move |_| selected.set(0)><TextInput label="Search this dashboard" name="dashboard-search" value=query input_ref=input_ref placeholder="Medicine or stock name"/></div>
                <div id="dashboard-search-results" role="listbox" aria-label="Search results" class="dashboard-search-results">
                    {move || { let matches = results.get(); if matches.is_empty() { view! { <p class="dashboard-empty">"No matching items in this dashboard."</p> }.into_any() } else { matches.into_iter().enumerate().map(|(index, item)| { let target_id = item.target_id.clone(); let option_id = format!("dashboard-search-option-{}", target_id); view! { <div id=option_id class="dashboard-search-result" role="option" aria-selected=move || if selected.get() == index { "true" } else { "false" } on:click=move |_| { #[cfg(feature = "hydrate")] { forget_focus(); visible.set(false); activate_result(&target_id); } #[cfg(not(feature = "hydrate"))] { let _ = &target_id; visible.set(false); } }><strong>{item.label}" "</strong><small>{item.detail}</small></div> } }).collect_view().into_any() } }}
                </div>
            </Modal>
        </div>
    }
}

#[cfg(feature = "hydrate")]
thread_local! { static PRIOR_FOCUS: std::cell::RefCell<Option<web_sys::Element>> = const { std::cell::RefCell::new(None) }; }

#[cfg(feature = "hydrate")]
fn remember_focus() {
    PRIOR_FOCUS.with(|focus| {
        *focus.borrow_mut() = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element()
    });
}

#[cfg(feature = "hydrate")]
fn forget_focus() {
    PRIOR_FOCUS.with(|focus| {
        focus.borrow_mut().take();
    });
}

#[cfg(feature = "hydrate")]
fn restore_focus() {
    use wasm_bindgen::JsCast;
    PRIOR_FOCUS.with(|focus| {
        if let Some(element) = focus.borrow_mut().take()
            && let Some(element) = element.dyn_ref::<web_sys::HtmlElement>()
        {
            let _ = element.focus();
        }
    });
}

#[cfg(feature = "hydrate")]
fn activate_result(id: &str) {
    use wasm_bindgen::JsCast;
    if let Some(element) = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
    {
        if let Ok(Some(disclosure)) = element.closest("details") {
            let _ = disclosure.set_attribute("open", "");
        }
        element.scroll_into_view();
        if let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() {
            let _ = element.focus();
        }
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document
        .get_element_by_id("dashboard-search-island")
        .unwrap();
    let dataset = root
        .first_element_child()
        .unwrap()
        .get_attribute("data-dashboard-search")
        .unwrap();
    let items: Vec<SearchItem> = serde_json::from_str(&dataset).unwrap();
    leptos::mount::hydrate_from(root.unchecked_into(), move || {
        SearchPalette(SearchPaletteProps { items })
    })
    .forget();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_filters_only_supplied_items() {
        let items = vec![SearchItem {
            label: "Salbutamol inhaler".into(),
            detail: "Jamie · As needed".into(),
            target_id: "schedule".into(),
        }];
        assert_eq!(search_items(&items, "salb"), items);
        assert!(search_items(&items, "amoxicillin").is_empty());
    }
}
