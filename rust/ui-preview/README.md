# Dashboard UI bundle

This crate shares the authenticated dashboard's Leptodon controls between server rendering and browser hydration. The Rust API reads the generated CSS, JavaScript and WASM at compile time. Search data is rendered into the private dashboard HTML for the signed-in user; the public bundle contains only code and styles.

Run `task -d rust/ui-preview build` to regenerate the bundle. The task installs the locked npm dependencies, the wasm32 Rust target and wasm-bindgen CLI 0.2.127 if needed. `task api:build` and the contract image build run this task automatically.

Run `task -d rust/ui-preview test` for the shared search filtering test. For a live browser check, provision a disposable contract fixture, rebuild the Rust preview service and run `LEPTODON_DASHBOARD_EMAIL=<fixture-email> task -d rust/web test:browser-dashboard`.
