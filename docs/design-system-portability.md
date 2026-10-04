# Portable Rust UI layer

The bake-off was a fit test for the existing MedTracker renderer and styling. It did not prove that local ownership is universally better, nor evaluate maintaining upstream patches, adapting copied components or future applications with different rendering requirements. Those approaches remain possible behind a stable API owned by us.

The first extraction is one independently packageable Leptos crate at rust/ui. It owns semantic controls and browser interaction, with a small explicit public API. It depends on Leptos, not MedTracker, Rails, Axum, authentication, database models or application routes. Source CSS, fonts, themes, labels, persistence and API actions belong to each consuming application.

## Initial structure

```text
rust/                              existing grouping directory
├── ui/                            real reusable library package
│   ├── Cargo.toml                 feature/runtime and dependency boundary
│   ├── src/lib.rs                 Button, Field, Tabs, Dialog and Sheet API
│   ├── src/runtime.js             explicit browser initialisation and teardown
│   ├── tests/semantics.rs         SSR semantics and escaped caller data
│   └── README.md                 public contract and independent consumption
├── web/                           MedTracker consumer and styling adapter
│   └── src/assets/profile.js      application behaviour and UI initialisation
└── api/                           existing Axum executable and API routes
```

UI controls may import Leptos and package-owned interaction. They must not import rust/web, rust/api, Rails assets or product models. MedTracker may import the public package API; its product composites and business data stay local. Library controls receive caller classes without stock palettes or guessed values. No server render may access browser APIs.

Begin with the five measured primitives. Use native dialog modality behind the public abstraction, rather than promoting the prototype's custom focus code. Browser enhancement is explicitly initialised after native SSR or after Leptos hydration, with an idempotent lifecycle and teardown. The application retains navigation, theme persistence and API policy. The implementation behind this API may later change to an upstream library without changing every application.

Prove the boundary through semantic SSR tests, real browser contracts, an unbranded consumer, the MedTracker adapter, package verification and the separate hydrated build/runtime path. Do not infer portability from a MedTracker screenshot or hydration support from successful compilation. First migrate a bounded consumer, then move other application controls with their tests. Publishing a shared repository and broad Profile migration are separate delivery steps.

Rejected for this first extraction: a generic backend platform, a second colour system, one crate per primitive, copying whole Profile screens, and coupling the UI package to API/authentication models. These add ownership without a second concrete consumer or a stable shared contract. A dedicated repository can follow the standalone-package proof; current location is reversible.

## Implemented boundary

The package exports the five primitive families and its browser runtime. An unbranded consumer verifies both server rendering and actual Leptos hydration with the same view. Hydration keeps the server-rendered elements and wires an application event. The package archive includes only public library sources, semantic tests, its manifest/lock, licence and README; application assets, Node dependencies and browser build output are excluded.

MedTracker's first adapter passes its existing Profile selectors to the runtime and uses the shared Button for the timezone form footer. The API serves the runtime before the application adapter through the existing profile.js URL. Appearance persistence, avatar requests, CSRF, routes, translated labels and canonical Rails CSS/fonts remain application-owned. The contract Dockerfile and isolated source snapshot include the new path dependency. CI exercises the standalone crate, package verification and both browser rendering modes.

The next useful extraction is a bounded set of Profile controls, preserving the existing semantic and browser contracts. Moving the package to a dedicated repository and consuming it from a second real application are separate follow-ups. Broader shared backend code should wait for a repeated concrete need; this UI package does not change the existing Axum API boundary.
