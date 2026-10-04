# Proposal

## Why

The Rust browser currently exposes time zone editing as a separate Settings page. The Rails app has one Profile page with Profile, Security, Notifications and Advanced sections. People moving between the two apps need the same account controls in the same place, with working actions and the same visual language.

## What Changes

- Make `/households/{slug}/profile` the canonical Rust browser page. Keep the old `/settings` URL as a redirect.
- Reproduce the Rails Profile layout at desktop and mobile sizes. Open time zone, photo, shortcuts and appearance controls from the Profile section.
- Implement the existing Rails Security, Notifications and Advanced operations with their required Rust backend flows. Render each section within the shared page rather than as an isolated settings screen.
- Serve the local Rails fonts, preserve appearance preferences across signed-in and sign-in pages, and verify actual browser rendering.
- Keep the notification preference PR separately reviewable by stacking this change on its branch.

## Capabilities

### New Capabilities

- `rust-profile-page`: manage the signed-in account from the four-section Rust Profile page.

### Modified Capabilities

- `browser-notification-preferences`: show notification controls within the Profile page while retaining the separately published preference API and legacy route.

## Impact

Rust browser routes, account security and advanced-state handlers, the shared navigation and theme assets, browser tests and five locale catalogues. Rails remains the behaviour reference and is not changed here.
