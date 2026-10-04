# Damacus Web UI

An unstyled Leptos library for Rust web applications. It owns semantic controls and browser interaction. Applications own CSS, fonts, themes, translated labels, navigation destinations, persistence and API policy.

The first public controls are Button, Field, Tabs, Tab, TabPanel, Dialog and Sheet. This is a small initial contract, not a complete component catalogue. Sheet uses the same native modal behaviour as Dialog; the application supplies its positioning class.

## Consume the package

For server rendering:

```toml
[dependencies]
damacus-web-ui = { path = "../ui", version = "0.1.0" }
```

For a separate hydration build, disable the default server feature:

```toml
damacus-web-ui = { path = "../ui", default-features = false, features = ["hydrate"] }
```

Leptos is pinned to 0.8.21. Use the same version in the application. The MIT-licensed package verifies independently with cargo package. It has no Rails files or MedTracker dependencies. It is not published to crates.io; a versioned Git dependency or an extracted repository can replace the path dependency later.

```rust
use damacus_web_ui::{Button, ButtonType, Field};
use leptos::prelude::*;

view! {
    <Field id="email" name="email" label="Email" input_type="email"
        class="application-input" attr:autocomplete="email"/>
    <Button kind=ButtonType::Submit class="application-primary">"Save"</Button>
}
```

Button defaults to type=button. Field associates its label, optional hint and error with the input. Extra Field attributes go to the input; wrapper_class, label_class and message_class are explicit styling hooks. Children and application attributes/events use Leptos conventions. IDs must be unique in the document. Supply a valid initial selection for each tab list; disabled choices cannot be selected. Navigation tabs need caller-supplied href values.

## Browser lifecycle

Serve BROWSER_RUNTIME as JavaScript before the application's entry script. It defines window.DamacusUI and performs no automatic DOM mutation. For server-rendered pages, call init after the DOM exists. For hydrated pages, call it after Leptos hydration has completed:

```javascript
const controls = window.DamacusUI.init(document);
controls.dispose();
```

Repeated initialisation for the same root returns the same controller. Dispose removes its listeners, closes overlays opened by the controller and releases its scroll lock. A root may be Document or a container element; do not initialise overlapping roots. Shadow roots and iframe integration are not covered by this first contract.

Use data-ui-open="dialog-id" on an opener and data-ui-close="dialog-id" on a close control. Dialog and Sheet render native dialog elements. The runtime calls showModal, delegates modality and Escape handling to the browser, closes on backdrop clicks, preserves scroll styles and restores connected openers. Native keyboard traversal can enter browser chrome; background page controls remain unavailable. An SSR open attribute is a visible progressive enhancement state, not native modality until init promotes it with showModal. Use a browser with native dialog support.

Tabs default to local panel activation, with arrow keys, Home and End, disabled-choice skipping and one keyboard tab stop. TabsMode::Navigation preserves normal anchor destinations and navigates on arrow keys. A ui:tab-change event is emitted for local selection. Applications remain responsible for saving their own data.

Legacy application markup can opt in through modalSelector, triggerAttribute, closeAttribute, tabsSelector, navigation, focusStorageKey and scrollLockClass. These options configure existing selectors and storage names; they do not inject application styles. Initialise once with the final options. Use dispose before changing those options.

## Verification

Run tasks from this package directory, or use task -t rust/ui/Taskfile.yml from the checkout root:

```text
task deps
task test
task fmt
task lint
task package
task browser:install
task test:browser
task hydrate:tool
task hydrate:build
task test:hydrated
```

The unbranded consumer renders the same controls for SSR and hydration. Browser contracts exercise native form controls, local tabs, real modality, background focus and accessibility exclusion, opener restoration, initially open overlays, lifecycle, nested overlays, backdrop dismissal and retained application navigation. The hydrated run also preserves SSR elements and exercises a Leptos application event. No CSS values are copied into the library.
