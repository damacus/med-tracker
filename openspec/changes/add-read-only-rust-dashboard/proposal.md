# Proposal

## Why

The completed Rust API needs a small usable first-party interface. A faithful, read-only dashboard lets users sign in and inspect their medication information before write workflows are ported. Originating issue: https://github.com/damacus/med-tracker/issues/2299.

## What Changes

- Provide working password login and sign-out using existing Rust authentication.
- Match the supplied dashboard screenshot with responsive Material You styling and real authorised dashboard data.
- Enable person selection, All Family, keyboard navigation and disclosures; retain unavailable navigation and write controls with accessible explanations.
- Show an honest “Insights coming soon” card.
- Provide an installable online-only PWA with a private, data-free offline fallback.
- Port relevant Rails read-only assertions into the existing browser harness and Rust calculation tests.

Explicit non-goals: clinical writes, other application pages, OIDC, alternate dashboard layouts, insight analysis, offline medical records, public API changes, migrations, performance benchmarking, deployment and merging.

## Capabilities

### New Capabilities

- `rust-read-only-dashboard`: authenticated, authorised, visually faithful dashboard and read-only interactions.
- `rust-online-pwa`: installable application shell with network-only private data and safe offline behaviour.

### Modified Capabilities

None. Existing authentication, medication safety and public API requirements remain unchanged.

## Impact

Extends Rust Leptos SSR views, the Axum browser-session/read boundary, static assets and isolated browser tests. Reuses Rails visual assets and authorised API reads. No new public API operations, clinical writes or database migrations. Implementation starts from API commit `d4624e8267be82c382dc0b3108a838371a3331a6`; paused UI work is preserved.
