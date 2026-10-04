# Portable Rust web UI boundary

The canonical UI library is [Loom UI](https://github.com/damacus/loom-ui). MedTracker consumes its public API through a full Git revision in `rust/web/Cargo.toml`; both the web and API lockfiles record that same revision. The copied `rust/ui` package is removed.

## Ownership

| Loom UI | MedTracker |
| --- | --- |
| Reusable controls and browser lifecycle | Profile composition, labels and routes |
| Generic SSR semantics and hydration contracts | CSRF, permissions, persistence and API policy |
| Component keyboard, modal and package tests | Actual Profile forms and household browser journeys |
| Leptix primitives and narrow upstream adapters | Canonical Rails CSS, fonts and ten palettes |

Loom uses Leptix as its primitive foundation. Its initial release keeps native dialog/sheet compatibility controls while the upstream dialog SSR defect is resolved. MedTracker imports those tested public controls rather than a second local implementation. This dependency migration does not replace the application's existing navigation tabs with hydrated Leptix tabs.

`rust/ui-preview` remains an application-owned dashboard/search island. It is a separate Leptodon integration with its own build and application checks, so it stays in this repository.

## Application adapter

The API serves Loom's exported browser runtime before the MedTracker adapter through the existing Profile JavaScript URL. The adapter calls `window.LoomUI.init` with Profile selectors, focus storage and scroll-lock options. Appearance persistence, avatar requests, CSRF handling and business actions remain local.

Library controls receive application classes. The canonical styling locations remain documented in [design-system-reuse.md](design-system-reuse.md). No new fonts, colours, spacing or radii are introduced by this migration.

## Verification and upgrades

Loom's repository owns its standalone SSR, browser, hydration and package verification. MedTracker CI no longer installs or runs a copied component test suite. It still builds the consumer and dashboard island, runs web/API checks, and verifies the four Profile sections and their actual forms, permissions and overlays against the Rust listener.

CI and container builds resolve the same pinned Git revision from the committed lockfiles. There is no sibling-directory or symlink requirement, and the container/source-snapshot wiring no longer copies a local UI crate. Upgrades change the manifest revision and affected lockfiles together, then run MedTracker's application checks. Loom is public, so clean CI and container builds fetch it without repository credentials. A crates.io release is planned; the immutable Git pin remains the consumer contract until that release is published.

The UI package does not change the existing Axum API boundary or create a shared authentication/database framework. A second actual application remains a separate consumer integration.
