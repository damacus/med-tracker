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

## Completion execution packet (1 October 2026)

Continue with the [next-slice packet](../../../docs/plans/axum-leptos-port/next-slices-20261001.md). Parallel disjoint A localisation, B stale-form safety and C limited-member dashboard owners precede ordered D dosage options, E stock, F direct/seven-type scheduled assignments and G final parity.

One persistent Luna Medium owner exclusively executes all compiled/runtime tasks, shared builds, fixture/bootstrap and Docker operations. Sol 6.1 owners retain product diagnosis, separate test writing, independent review and coordinator integration. The visible queue records frozen manifests/digests, fixture identities, exact commands, RED/GREEN and resource waits. Results certify immutable inputs only.

Known validation text is translated through exact catalogue mappings; unknown messages receive a useful translated generic form error without exposing raw API text or inventing clinical advice. Browser dashboard profile preferences are optional only for 403/404 after mandatory authenticated identity; other read failures stay explicit failures and authorised clinical collections remain mandatory. Medication edit preconditions are rechecked after option discovery, but final mutation always receives the original submitted If-Match; conflicts retain scalar drafts and never silently adopt the current version.

The initial accepted slice remains 12/20 tasks. Each subsequent box requires real persistence/UI/locale evidence and independent requirements plus quality review. The two-hour assessment stops new dispatch then finishes in-flight verification/cleanup. Existing baseline audit failures remain RED under #2347. Rails, merge/deployment and authentication/scanner boundaries remain unchanged.

## Autonomous delivery continuation

After publishing the bounded continuation at 14/20 accepted tasks, Dan authorised
completion while AFK. Follow the [delivery plan](../../../docs/plans/axum-leptos-port/household-delivery-20261001/plan.md):
one persistent Sol 6.1 product/test writer, the existing exclusive Luna Medium
verifier and independent Sol review. The new two-hour retrospective feeds local
process improvements into the run immediately; it does not stop feature work.
Complete dosage management before stock, then all seven assignment/schedule
journeys before final parity acceptance. Preserve the existing cutover gates.
