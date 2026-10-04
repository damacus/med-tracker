use damacus_web_ui::{Button, ButtonType, Dialog, Field, Sheet, Tab, TabPanel, Tabs};
use leptos::prelude::*;

pub fn app(open: &str) -> impl IntoView + use<> {
    let count = RwSignal::new(0);
    let dialog_open = open == "dialog";
    let sheet_open = open == "sheet";
    view! {
        <main id="app">
            <h1>"Account preferences"</h1>
            <Button attr:id="reactive-button" on:click=move |_| count.update(|value| *value += 1)>"Count "{move || count.get()}</Button>
            <form id="preferences-form">
                <Field id="account-name" name="name" label="Display name" value="Example"
                    hint="Shown to colleagues" error="Enter a longer name"/>
                <Field id="disabled-name" name="disabled_name" label="Locked value" disabled=true/>
                <Button kind=ButtonType::Submit attr:id="save-button">"Save"</Button>
                <Button disabled=true attr:id="disabled-button">"Unavailable"</Button>
                <output id="save-count">"0"</output>
            </form>
            <Tabs id="preferences-tabs" label="Preferences">
                <Tab id="general-tab" panel_id="general-panel" selected=true>"General"</Tab>
                <Tab id="disabled-tab" panel_id="disabled-panel" disabled=true>"Unavailable section"</Tab>
                <Tab id="security-tab" panel_id="security-panel">"Security"</Tab>
            </Tabs>
            <TabPanel id="general-panel" tab_id="general-tab" selected=true>"General preferences"</TabPanel>
            <TabPanel id="disabled-panel" tab_id="disabled-tab">"Unavailable preferences"</TabPanel>
            <TabPanel id="security-panel" tab_id="security-tab">"Security preferences"</TabPanel>
            <Button attr:id="open-dialog" attr:data-ui-open="confirmation">"Open confirmation"</Button>
            <Button attr:id="open-sheet" attr:data-ui-open="appearance">"Open appearance"</Button>
            <Dialog id="confirmation" labelled_by="confirmation-title" described_by="confirmation-hint" open=dialog_open>
                <h2 id="confirmation-title">"Confirm change"</h2>
                <p id="confirmation-hint">"Review before saving."</p>
                <Field id="confirmation-name" name="confirmation" label="Confirmation value"/>
                <Button attr:data-ui-close="confirmation">"Close confirmation"</Button>
            </Dialog>
            <Sheet id="appearance" labelled_by="appearance-title" open=sheet_open>
                <h2 id="appearance-title">"Appearance"</h2>
                <Button attr:data-ui-close="appearance">"Close appearance"</Button>
            </Sheet>
            {(0..60).map(|_| view! { <p>"Standalone consumer content"</p> }).collect_view()}
        </main>
    }
}
