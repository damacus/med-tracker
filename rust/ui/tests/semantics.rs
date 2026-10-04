use damacus_web_ui::{Button, ButtonType, Dialog, Field, Sheet, Tab, TabPanel, Tabs, TabsMode};
use leptos::prelude::*;

#[test]
fn controls_escape_app_values_and_forward_native_attributes() {
    let html = view! {
        <Button kind=ButtonType::Submit disabled=true class="caller-button" attr:data-action="save">
            "Save <record>"
        </Button>
        <Field id="account-name" name="display_name" label="Display name" value="<script>"
            hint="Shown to colleagues" error="Required" disabled=true class="caller-input"/>
    }
    .to_html();
    assert!(html.contains("type=\"submit\""));
    assert!(html.contains("data-action=\"save\""));
    assert!(html.contains("Save &lt;record&gt;"));
    assert!(html.contains("for=\"account-name\""));
    assert!(html.contains("id=\"account-name\""));
    assert!(html.contains("aria-describedby=\"account-name-hint account-name-error\""));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("<script>"));
}

#[test]
fn tabs_and_initial_overlays_render_without_browser_access() {
    let html = view! {
        <Tabs id="preferences-tabs" label="Preferences" mode=TabsMode::Local>
            <Tab id="general-tab" panel_id="general-panel" selected=true>"General"</Tab>
            <Tab id="security-tab" panel_id="security-panel">"Security"</Tab>
        </Tabs>
        <TabPanel id="general-panel" tab_id="general-tab" selected=true>"General preferences"</TabPanel>
        <TabPanel id="security-panel" tab_id="security-tab">"Security preferences"</TabPanel>
        <Dialog id="confirmation" labelled_by="confirmation-title" described_by="confirmation-hint" open=true class="caller-dialog">
            <h2 id="confirmation-title">"Confirm"</h2><p id="confirmation-hint">"Confirm this action."</p>
        </Dialog>
        <Sheet id="appearance" labelled_by="appearance-title" class="caller-sheet">
            <h2 id="appearance-title">"Appearance"</h2>
        </Sheet>
    }.to_html();
    assert!(html.contains("role=\"tablist\""));
    assert!(html.contains("aria-controls=\"general-panel\""));
    assert!(html.contains("aria-labelledby=\"general-tab\""));
    assert!(html.contains("role=\"tabpanel\""));
    assert!(html.contains("tabindex=\"-1\""));
    assert!(html.contains("aria-modal=\"true\""));
    assert!(html.contains("data-ui-modal"));
    assert!(html.contains("data-ui-kind=\"sheet\""));
    assert!(html.contains("class=\"caller-dialog\""));
    assert!(!html.contains("profile-"));
    assert!(!html.contains("theme-"));
}

#[test]
fn field_forwards_application_attributes_to_the_input() {
    let html = view! {
        <Field id="email" name="email" label="Email" input_type="email" attr:autocomplete="email"/>
    }
    .to_html();
    let input = html
        .split("<input")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    assert!(input.contains("autocomplete=\"email\""));
}
