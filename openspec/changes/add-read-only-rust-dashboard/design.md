# Design

## Context

The motivation and scope are defined in proposal.md and issue #2299. The baseline is completed API commit `d4624e8267be82c382dc0b3108a838371a3331a6`. Its Leptos SSR views and Axum browser-session boundary already support login and a basic dashboard. Dashboard reads use the authenticated API router in process. The paused UI checkout must remain untouched.

## Goals / Non-Goals

Deliver the screenshot's current Rails dashboard layout with authorised live data, password login, useful read-only interaction and an installable online-only shell. Preserve the fixed 118-operation public API.

Non-goals are clinical writes, other pages, OIDC, completing MFA enrolment/challenges, real insights, experimental dashboard layouts, offline medical records, migrations, deployment, merging and memory benchmarks.

## Decisions

### Keep the existing browser boundary

Extend `rust/api/src/web_pages.rs`, existing authentication and `rust/web` SSR components. Continue authenticated in-process API calls; do not introduce a parallel database query path or public dashboard endpoint. Preserve session renewal, CSRF protection, MFA restrictions and existing access auditing. Credentials and tokens never enter browser storage. Authentication/session changes are permitted; clinical mutations are not.

### Project authorised data before rendering

Build a dashboard view model from existing authorised reads. Apply household and person visibility before computing any metric or inventory. Validate selected person IDs; malformed or inaccessible selections produce a safe error without unrelated records or counts. Honour pagination when aggregating data. Use request-scoped reads outside components, avoiding per-card queries.

Port pure calculations from the Rails dashboard presenter and schedule projection with a controllable clock and the existing display-timezone rules. Reuse current medication eligibility rules. Do not infer due status independently from labels or introduce a second safety policy. Read-only rendering requires no new transaction or clinical concurrency protocol; preserve current API semantics if records change during reads.

### Preserve the reference appearance

Use Rails Material You tokens, bundled fonts, icons, spacing, rounded surfaces and responsive rules. The reference is the current dashboard variant, not experimental layouts. Synthetic fixture names and medications may differ from the personal screenshot. Render the Smart Insights surface with “Insights coming soon”. Disabled actions retain recognisable styling, accessible unavailable semantics and a concise explanation.

Use GET forms, normal links for supported navigation and native disclosures. Keep person selection in the URL, including All Family, so refresh and history work without a new client runtime. Reuse existing JavaScript only where required for keyboard behaviour or PWA registration.

### Restrict PWA caching to public assets

Provide a manifest, appropriately sized icons, service worker and patient-free reconnect page. Use an explicit public-static allowlist. Never cache authenticated HTML, API responses, redirects containing private state, or medical images. Private responses remain network-only with private/no-store headers. Failed navigation displays the reconnect page without retaining a previous patient's screen. Verify logout, expiry and offline transitions in the browser; do not promise offline authentication.

### Reuse the browser harness

Use existing Playwright acceptance and isolated Compose runner rather than adding a Rust browser driver. Native Rust tests cover pure calculations and rendered component structure. Capture RED before production edits. Browser tests cover behaviour; desktop/mobile screenshots and independent review establish visual fidelity.

Reference mapping:

| Rails source | Reuse |
| --- | --- |
| `spec/system/dashboard_spec.rb` | Greeting, schedule and visible records; omit clinical write steps |
| `spec/system/dashboard_person_selector_spec.rb` | All Family, URL, keyboard, 390/1280 px layouts |
| `spec/components/dashboard/index_view_spec.rb` | Dashboard content, inventory and conditional history |
| `spec/presenters/dashboard_presenter_spec.rb` | Metrics, selection and day boundaries; omit insight engine |
| `spec/services/family_dashboard/schedule_query_spec.rb` | Routine/as-needed projection and completed/paused/not-taken states |
| `spec/components/dashboard/person_task_card_spec.rb` | Read-only task states |
| `spec/requests/dashboard_home_spec.rb` | Access and selection boundaries |

Implementation must record exact reused examples and deliberate exclusions after inspecting current sources. Known Rails bugs are corrected against intended behaviour, not copied into parity tests.

## Risks / Trade-offs

- Metrics can drift from Rails if day boundaries, pagination or eligibility differ: freeze time, exercise boundary fixtures and reuse existing rules.
- Aggregate counts can leak hidden people: apply permissions before projection and test mixed-visibility households.
- Styling can appear close while breaking mobile or keyboard use: compare desktop/mobile screenshots and exercise native controls.
- Service worker mistakes can retain medical data: use a narrow allowlist and inspect browser caches after authenticated and offline flows.
- Some data may not be obtainable through authorised existing reads: report a concrete blocker rather than inventing values or silently expanding the API.

## Delivery and Migration

This thread produces planning artifacts only. A fresh implementation thread owns test-first delivery, independent review, evidence and publication. Sol owns production and shared runner changes; Luna owns bounded test files. Keep ownership disjoint and use another checkout only for a concrete conflict. No database migration or data conversion is required. A future deployment can roll back the UI binary/static assets without modifying clinical records; deployment itself is outside this change.
