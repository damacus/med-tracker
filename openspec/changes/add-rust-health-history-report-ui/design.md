# Design

## Context

Issue: https://github.com/damacus/med-tracker/issues/2387. Baseline: a5e22b34506e23b2e62ff863dce519414a692760, PR #2383. reports.rs already generates and audits health-history PDF responses. Browser WebApi currently decodes JSON; feeding a PDF through it would fail.

Read reports.rs, ui_capabilities.rs, people/access.rs, review_prompts.rs, web_pages/api_client.rs and the current household form/navigation patterns. The existing report endpoint, not a new renderer, is authoritative.

## Goals / Non-Goals

Goals: an authorised user reaches Reports, selects a person and date range, optionally includes medication takes, and downloads the existing PDF; failures are understandable and preserve filters.

Non-goals: charts, preview JSON requests, medication-review reports, new PDF generation, exports, report storage, emails, background jobs, changed permissions, Rails changes and profile timezone behaviour.

## Decisions

### Page and request mapping

Add GET /households/{slug}/reports and GET /households/{slug}/reports/health-history.pdf using existing route conventions. A labelled GET form has person, start date, end date, Include medication takes (unchecked by default) and Download PDF. Link Reports from the household shell.

Send person_id, start_date and end_date to /api/v1/households/{id}/reports/health_history.pdf. Map the checkbox explicitly to include_medication_takes=1 or 0; true/false strings are invalid for this API. Reject unknown inputs and encode query values safely. No status field or JSON format parameter.

Initial date inputs must match existing API defaults: today in configured TZ and twelve calendar months earlier, including leap-day handling. Do not silently use the user's new timezone setting: that is a separate session and the report API currently uses configured TZ. Validate date shape, start <= end and maximum elapsed span 366 days for useful feedback; the API revalidates everything. Reuse any existing date helper instead of duplicating calendar arithmetic.

### Person choice and permission

Populate choices from existing household people data intersected with ui_capabilities.people.manage_ids, verifying that this matches report selected-person permissions. Do not display readable-only people as report choices. A health report also requires owner/administrator membership OR an adult actor; selected-person manage permission still applies to managers.

Do not invent an age/role policy in rendering or add database queries to the UI. Existing API download remains the final authority. Where existing payloads cannot express actor eligibility, show the eligible manageable choices and handle the API's 403 with a neutral explanatory state; never broaden backend permissions merely to hide a UI error. An empty choice list disables Download and explains that no available person can be selected.

Changed/revoked grants between page render and download must fail through the API. Cross-household/tampered person IDs produce the existing masked failure, without disclosing person details.

### Binary response boundary

Add the smallest binary response method beside WebApi's JSON methods, reusing authenticated internal router invocation and its existing cookie/session handling. Do not mint bearer tokens, make a second network client or change JSON response behaviour. Preserve refreshed Set-Cookie headers even when the result is binary.

Return successful PDF bytes with application/pdf, the existing safe attachment filename and Cache-Control: no-store. Use an explicit header allowlist rather than forwarding arbitrary internal headers. Bound buffered response bytes consistently with existing adapter limits, or use an equally bounded streaming response if that is the established pattern. Test oversize/failure behaviour; do not silently truncate documents.

For non-success responses decode the normal API error envelope and render the form with retained filters and accessible messages. Expired sessions follow the existing login flow. Mask inaccessible records; use generic errors for generation/internal failures. Do not show JSON or corrupt PDF downloads as success. Avoid raw upstream diagnostics and PHI in logs.

### Audit, privacy and repeat downloads

Call the PDF API once per user download. Do not first call JSON for preview/validation: both are audited operations. Let existing reporting transactions own audit records and outcome semantics. Each intentional repeated download may have its own existing audit entry; do not invent mutation idempotency for a GET.

No server-side saved copies, browser storage, service-worker caching or client-side report assembly. Keep PHI out of URLs beyond the existing required person/date filter identifiers; do not include names in filenames/queries. On generation failure use existing audit/rollback semantics and return no partial attachment.

### Ownership and localisation

Add a dedicated reports page module/renderer and bounded WebApi binary helper, with narrow route/nav additions. Share no worktree with the other sessions. Localise labels, help, empty states and errors in en/cy/ga/es/pt; keep distinct report keys and preserve unrelated entries/comments. No PDF, RSA or crypto dependencies.

## Risks / Trade-offs

- Binary support is the main adapter change; regression tests must prove JSON calls and cookie refresh still work.
- manage_ids alone does not represent the adult actor gate; current API must remain authoritative until a trusted existing capability expresses it.
- PDF response limits must accommodate the existing permitted maximum report. If an oversized fixture exceeds the current adapter limit, document it and use a minimal bounded adjustment rather than an unlimited buffer.
- Parallel sessions may edit navigation and locale catalogues in different branches. Resolve integration serially.
- No unresolved product decisions block this plan; record verified implementation details without expanding scope.

## Verification and delivery

Begin with failing observable tests for a working download and JSON/binary adapter separation. Then cover dates reversed, malformed and >366 days, leap dates/defaults, include takes on/off, no entries, permission denial, cross-household selection, revoked grant, expired session, generation failure and response size limits. Check actual PDF content against fixture person and dates using existing PDF parsing helpers, not just a PDF magic prefix.

Run task api:openapi-reports-acceptance and changed-area Rust format/lint/tests and focused browser tasks discovered through task -l. Verify cookie renewal and no-store/attachment headers. Use existing failure fixtures rather than introducing new report logic. No Rails preflight/full suite for Rust-only work.

Verify actual desktop/mobile filters, errors and successful browser download, keyboard use and five locales; save docs/screenshots/health-history-reports-desktop.png and health-history-reports-mobile.png. Record exact verification and any limitations. Commit plans with implementation, push a focused follow-up PR; close #2387 only after merge. No merge or deployment.
