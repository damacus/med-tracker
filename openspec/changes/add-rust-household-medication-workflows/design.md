# Design

## Context

Current Axum browser handlers dispatch internally through the shared API and forward cookies, CSRF, actual Origin/Referer and renewal cookies. Leptos SSR and a dashboard search hydration island coexist. Existing Leptodon inputs may initialise edit values only through effects; mutation forms must work with native named inputs.

## Goals / Non-Goals

Goals: completed household journeys, observable Rails behaviour parity, existing account/tenant boundaries, truthful mobile/accessibility/translation acceptance.
Non-goals: new identity provider, domain-rule duplication, production activation, arbitrary new component framework, new offline medical storage.

## Decisions

- Keep internal API dispatch and existing cookie-only browser boundary. Split page handlers into child modules under web_pages so module authors do not edit shared routers.
- Use application-owned Field/FormShell components and native text/date/time/decimal/select controls. Reuse existing styling; do not adopt ActionForm/server functions as a second write contract.
- Keep decimal drafts as strings. Preserve validation drafts, use post/redirect/get and verified success notices. Forward API preconditions/idempotency where supported; do not claim duplicate protection where absent.
- Expose action affordances from API policy, never a single inferred manager flag: people creation also accepts active manage grants.
- Use typed schedule draft serialization. Do not copy Rails wizard JSON verbatim; current Rust config rejects its metadata keys. Domain behaviours and discrepancies are verified against Rails.
- Translation candidate is pinned leptos_i18n/leptos_i18n_build 0.6.2, gated by compile proof. Generate a deterministic bounded catalogue from Rails YAML; explicit request locale flows to SSR and hydration. Preserve Rails exact-zero/one/other plural selection with a compatibility helper. Fallback is a typed catalogue adapter if crate adoption fails.
- Language cookie, then supported weighted Accept-Language, then English. Keep account timezone separate; canonical submitted dates/decimal values remain locale-independent. Initial language changes reload. Existing five catalogues remain authoritative.
- No medical HTML, drafts or API responses in offline caches.

## Risks / Trade-offs

SSR values, API affordances, schedule round-trips and locale/hydration parity need focused red tests. The two-hour run does not promise all journeys; accepted and unaccepted work are recorded separately. Authentication compatibility may exceed this run and must not block independent household UI work.
