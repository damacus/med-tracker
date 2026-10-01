# Medication lane brief

Owned production paths: rust/api/src/web_pages/medications.rs and
rust/web/src/medication_management.rs. Coordinator registers modules, shared
helpers, styles, translations and capabilities. Test owner supplies RED.

Deliver manual medication creation and editing with native SSR controls.
Keep drafts as strings, map API errors to fields, preserve submitted values,
load authorised location options and return to verified inventory after save.
Preserve the current detail and dose flows. No new dependency or Rails change.

Proposed routes: GET medications/new, POST medications, GET medications/{id}/edit,
POST medications/{id}, all under /households/{slug}.

Shared helper requests: policy-derived create/update capabilities, ETag access,
If-Match forwarding, private redirects carrying renewed cookies, shared
FormShell/Field and locale label interfaces.

Agreed coordinator interfaces: WebApi locale: Locale; capabilities(household_id)
returns data.medications.create/update/manage_stock; get_reply returns ApiReply
including optional etag; call_with_headers forwards If-Match/idempotency-key;
redirect(location,cookie). Renderer uses household::household_document and
household_i18n::{Locale,Text,TranslationError}; translation errors fail closed.

Security expectations follow existing native dose handlers: origin and CSRF denial
403, foreign household 404, anonymous login redirect. Draft errors preserve exact
decimal strings such as 2.50, rather than normalising the user's rejected input.
Do not submit metadata unavailable on the edit response. Existing dosage-option
mode must survive identity edits; option management is a separate API journey.

Production implementation waits for a recorded missing-route/browser RED.
Deadline 10:27 UTC; no new journeys after 10:12 UTC.

Next option-editor tasks, conditional on current journey acceptance and new RED:

1. Test native option list/new/edit routes, stock/default preservation and conflicts.
2. Add owned option drafts/renderers and nested child handlers using shared helpers.
3. Populate all required API defaults and precise scalar types; preserve custom units.
4. Restrict initial creation to option-mode parents; explain scalar-mode transitions.
5. Forward required edit ETags and preserve generic API validation summaries.
6. Verify independent option changes, parent aggregate stock and existing dose flow.
7. Capture actual desktop/mobile browser evidence and obtain independent review.

No batch medication-plus-option save: separate native journeys avoid partial-save
claims. No deletion or create replay guarantees beyond existing API support.

Review follow-up accepted by coordinator: preserve optional-dose API semantics.
The edit form must detect existing dosage options from the authorised collection,
not infer their existence from a null scalar dose. A blank-dose medication with
no options must retain scalar dose, unit and supply controls and allow a later
scalar dose save. Tests precede the handler change. A stale form whose medication
has gained an option must return 409 with its submitted scalar draft retained,
before the options-mode guard can return 400. API If-Match remains authoritative
for any race between the browser adapter read and mutation.
