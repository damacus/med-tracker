use leptos::attribute_interceptor::AttributeInterceptor;
use leptos::prelude::*;

pub const BROWSER_RUNTIME: &str = include_str!("runtime.js");

#[derive(Clone, Copy, Default)]
pub enum ButtonType {
    #[default]
    Button,
    Submit,
    Reset,
}

impl ButtonType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Reset => "reset",
        }
    }
}

#[component]
pub fn Button(
    children: Children,
    #[prop(optional)] kind: ButtonType,
    #[prop(optional)] disabled: bool,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    view! { <button type=kind.as_str() disabled=disabled class=class>{children()}</button> }
}

#[component]
pub fn Field(
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(into)] label: String,
    #[prop(optional, into)] value: String,
    #[prop(default = String::from("text"), into)] input_type: String,
    #[prop(optional, into)] hint: String,
    #[prop(optional, into)] error: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] required: bool,
    #[prop(optional, into)] class: String,
    #[prop(optional, into)] wrapper_class: String,
    #[prop(optional, into)] label_class: String,
    #[prop(optional, into)] message_class: String,
) -> impl IntoView {
    let hint_id = format!("{id}-hint");
    let error_id = format!("{id}-error");
    let has_hint = !hint.is_empty();
    let invalid = !error.is_empty();
    let mut descriptions = Vec::new();
    if has_hint {
        descriptions.push(hint_id.clone());
    }
    if invalid {
        descriptions.push(error_id.clone());
    }
    let described_by = (!descriptions.is_empty()).then(|| descriptions.join(" "));
    view! {
        <AttributeInterceptor let:attrs>
            <div class=wrapper_class.clone()>
                <label for=id.clone() class=label_class.clone()>{label.clone()}</label>
                <input id=id.clone() name=name.clone() type=input_type.clone() value=value.clone()
                    disabled=disabled required=required class=class.clone()
                    aria-describedby=described_by.clone() aria-invalid=invalid.then_some("true") {..attrs}/>
                {has_hint.then(|| view! { <p id=hint_id.clone() class=message_class.clone()>{hint.clone()}</p> })}
                {invalid.then(|| view! { <p id=error_id.clone() class=message_class.clone() role="alert">{error.clone()}</p> })}
            </div>
        </AttributeInterceptor>
    }
}

#[derive(Clone, Copy, Default)]
pub enum TabsMode {
    #[default]
    Local,
    Navigation,
}

#[component]
pub fn Tabs(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    children: Children,
    #[prop(optional)] mode: TabsMode,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    let mode = match mode {
        TabsMode::Local => "local",
        TabsMode::Navigation => "navigation",
    };
    view! { <nav id=id role="tablist" aria-label=label class=class data-ui-tabs="" data-ui-mode=mode>{children()}</nav> }
}

#[component]
pub fn Tab(
    #[prop(into)] id: String,
    #[prop(into)] panel_id: String,
    children: Children,
    #[prop(optional)] selected: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional, into)] href: String,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    let selected = selected && !disabled;
    let tab_index = if selected { "0" } else { "-1" };
    if href.is_empty() {
        view! { <button id=id type="button" role="tab" aria-controls=panel_id
            aria-selected=selected.to_string() tabindex=tab_index disabled=disabled class=class>{children()}</button> }.into_any()
    } else {
        view! { <a id=id href=(!disabled).then_some(href) role="tab" aria-controls=panel_id
        aria-selected=selected.to_string() aria-disabled=disabled.then_some("true")
        tabindex=tab_index class=class>{children()}</a> }
        .into_any()
    }
}

#[component]
pub fn TabPanel(
    #[prop(into)] id: String,
    #[prop(into)] tab_id: String,
    children: Children,
    #[prop(optional)] selected: bool,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    view! { <section id=id role="tabpanel" aria-labelledby=tab_id hidden=!selected class=class>{children()}</section> }
}

fn overlay(
    id: String,
    labelled_by: String,
    described_by: String,
    open: bool,
    class: String,
    kind: &'static str,
    children: Children,
) -> impl IntoView {
    view! { <dialog id=id role="dialog" aria-labelledby=labelled_by
    aria-describedby=(!described_by.is_empty()).then_some(described_by) aria-modal="true"
    open=open class=class data-ui-modal="" data-ui-kind=kind>{children()}</dialog> }
}

#[component]
pub fn Dialog(
    #[prop(into)] id: String,
    #[prop(into)] labelled_by: String,
    children: Children,
    #[prop(optional, into)] described_by: String,
    #[prop(optional)] open: bool,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    overlay(
        id,
        labelled_by,
        described_by,
        open,
        class,
        "dialog",
        children,
    )
}

#[component]
pub fn Sheet(
    #[prop(into)] id: String,
    #[prop(into)] labelled_by: String,
    children: Children,
    #[prop(optional, into)] described_by: String,
    #[prop(optional)] open: bool,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    overlay(
        id,
        labelled_by,
        described_by,
        open,
        class,
        "sheet",
        children,
    )
}
