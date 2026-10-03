# Design

## Context

Issue: https://github.com/damacus/med-tracker/issues/2386. Baseline: a5e22b34506e23b2e62ff863dce519414a692760, PR #2383. The API already owns preferences, household locking, audit records and person permissions. Its singleton endpoint is /api/v1/households/{id}/notification_preference. Browser routes and navigation do not expose it.

Read rust/api/src/notification_preferences.rs, web_pages/api_client.rs, profile.rs, ui_capabilities.rs, rust/web/src/household.rs and the existing form modules before implementing. Existing Rails notification copy is the behaviour reference.

## Goals / Non-Goals

Goals: reach My notifications from the household shell; inspect and edit five boolean fields; distinguish saved, unsaved and unavailable states; work with keyboard, mobile and all five locales.

Non-goals: editing another person's preferences, reminder times, push subscriptions, browser permissions, delivery infrastructure, migrations, Rails changes and profile timezone work.

## Decisions

### Routes and form

Use GET and POST /households/{slug}/settings/notifications, following existing browser route/form conventions. Link directly as My notifications from the household shell and the post-login dashboard. The dashboard link appears only when the preference GET confirms readable saved preferences or an authorised unconfigured row; it uses an encoded household slug and hides on masked denial or read error. Avoid taking ownership of the concurrently developed timezone page or creating a second Settings hub. Render a heading, descriptive help, labelled switches, Save preferences and an accessible success/error region.

Map labels to enabled (Notifications enabled), dose_due_enabled (Dose due reminders), missed_dose_enabled (Missed dose reminders), low_stock_enabled (Low stock warnings), private_text_enabled (Hide sensitive notification content). Private content true hides medication names. Explain that these preferences do not enrol this browser for push.

Turning the master switch off must preserve category selections. Keep category controls editable, and explain that they apply when notifications are enabled. Unchecked HTML checkboxes must become explicit false values; reject ambiguous/malformed submissions through normal form parsing. On success use the existing redirect-after-save pattern and reload persisted values. On failure retain attempted values with associated errors and never show success.

### API and missing state

Use authenticated WebApi for GET and PATCH, passing exactly the singular notification_preference wrapper and five booleans. Never send morning_time, afternoon_time, evening_time or night_time. Never expose bearer tokens or call the database from rendering.

GET 404 is ambiguous: absent row and masked denial share a status. Keep both statuses, but return a distinct `not_configured` error code only after the API has authorised the person's read. The browser offers the database defaults for that case and creates the row only on Save. A masked `not_found` still produces a neutral unavailable state with no form. Add a self-management value to the existing UI capabilities response, derived from the same active person grant used by the preference API. Existing preference values remain visible but disabled when the caller can view and cannot manage them; no Save action appears.

### Security, audit and concurrency

Reuse current authenticated browser context, household resolution and CSRF verification before PATCH. The API rechecks current membership and manage permission on every save. Test cross-household slugs, revoked access and invalid CSRF. Mask inaccessible identity, omit PHI from errors/logs, and retain normal no-store page behaviour.

The API owns its transaction, audit/version/sync changes and no-op updates. No duplicate UI audit writes. Do not add ETag preconditions absent from this endpoint: current updates retain its existing last-write behaviour. A failed API transaction must not show success or change saved values. Reloading the form proves the stored result.

### Ownership and localisation

Add a dedicated notification web-page module and renderer, keeping changes to web_pages.rs, WebApi and household navigation minimal. Reuse maintained Rust form components and en/cy/ga/es/pt catalogue patterns. Use a distinct notification namespace; preserve unrelated locale entries and code comments. No new package dependencies.

## Risks / Trade-offs

- The API's absent-row error code and UI capability must stay aligned with its read and manage checks. The browser never infers permission from a bare 404.
- Saving preferences does not guarantee push delivery. Copy must make that boundary clear.
- The timezone and reports sessions touch shell/navigation/catalogue files in separate worktrees. Keep each diff narrow; integration/rebase happens serially.
- No unresolved product decisions block this plan. If inspection disproves an assumed route/rendering convention, choose the existing equivalent and record it without broadening scope.

## Verification and delivery

Write failing observable form/browser tests first, then the minimum implementation. Cover all five flags including explicit false, times preservation, read/reload, denied saves, invalid CSRF, expired session, masked 404 and API failure. Check audit/no-op outcomes using existing API acceptance tests. Use task api:openapi-notifications-acceptance plus existing relevant Rust format, lint and browser tasks discovered via task -l. Add a focused task selector only if necessary; do not invent a task that is not defined. No Rails test preflight for Rust-only work.

Verify actual desktop and mobile UI, keyboard focus and all five locale labels; capture docs/screenshots/notifications-desktop.png and notifications-mobile.png. Record commands, results and any limitations in the delivery summary. Commit this plan with implementation, push a focused branch, open a follow-up PR and close #2386 only after the work is merged. No merge or deployment in this session.
