# PR 2351 bounded follow-up review

1 October 2026. Six live review threads were read through the GitHub connector;
all were unresolved and current when inspected. Findings below were checked
against current source, not accepted from bot text. No source edits, runtime,
Git mutations or external replies were performed. D's accepted report remains
immutable. The coordinator owns issue creation and compatible E/G placement.

## Stock correctness for E

**4158481668 — confirmed P2, `rust,bug`.**
`rust/api/src/dosage_options/create.rs:100` replaces tracked parent quantity via
aggregation without changing the old scalar dose unit. The inventory card at
`rust/api/src/web_pages/inventory.rs:23` takes parent current_supply and
`dose_unit`; `rust/web/src/medication.rs:273` presents those together. A scalar
tablet medication acquiring an ml option can therefore display ml stock as
tablets. Mixed option units make changing the parent unit to the first option
incorrect too. E should present option stock as individual quantities with each
option's unit; any retained numeric aggregate must not claim the scalar unit
or a physical total across unlike units. Preserve API aggregation semantics.
Require real first-option and mixed-unit list/detail browser read-back evidence.

**4158481830 — confirmed P2 browser affordance defect, `rust,bug`.**
`rust/web/src/medication_management.rs:213` hides only dose_amount/current_supply
in option mode; reorder_threshold stays editable. The adapter payload at
`rust/api/src/web_pages/medications.rs:127` always submits it, while API
`medication_management/validation.rs:328` assigns it. Option aggregation at
`dosage_options/inventory.rs:43` later restores the sum, so an ordinary name edit
can replace the derived threshold and give inconsistent low-stock behaviour.
Compatible E remedy: hide/read-only the parent threshold in option mode, omit it
from that browser payload, and reject a forged scalar threshold field before
writing, like existing scalar stock/dose checks. Preserve the submitted token,
drafts and API permission boundary. Prove medication edits leave option and
parent thresholds intact, and actual option edits recalculate the aggregate.
Rails permits the field (`app/controllers/api/v1/medications_controller.rb:92`)
and OpenAPI MedicationUpdateAttributes exposes it, so this is a truthful browser
restriction, not authority to silently change the public API.

**4158481435 — confirmed P2 UI/recording mismatch; proposed API fix not accepted.**
`dosage_options/create.rs:109` clears only dose_amount for an untracked new
option. `dose.rs:1297` falls back to parent current_supply when no tracked options
exist; finite scalar stock can still be consumed and block further doses.
`config/locales/en.yml:1337` describes blank stock as untracked without explaining
that parent fallback. This needs a truthful E presentation and actual recording
tests, but unconditional parent reset would change the existing API contract.
OpenAPI `docs/api/openapi.v1.yaml:1803` explicitly recalculates create inventory
only if the option has current supply. Rails
`app/models/medication_dosage_option.rb:98` gates aggregation via `:107` on current
supply or its saved change; source alone does not prove first-null creation
resets old scalar stock. No Rails runtime proof was obtained here.

The coordinator ruled that E preserves this API behaviour. Explain that null
means the option has no separate tracking, and that existing parent stock is
used when all options are untracked. Distinguish this from parent null, where
there is no finite fallback. Verify finite/empty fallback using actual dose
recording, including out-of-stock no-write, and verify parent-null behaviour.
Recommended label `rust,bug` for the misleading browser presentation, with no
claim that the current API is an unapproved parity defect.

## Scalability for G

**4158481976 — confirmed P2, `rust,bug`.**
`rust/api/src/web_pages/api_client.rs:177` fetches at most five pages of 100 and
returns 503 if total_count exceeds that bound. The dosage adapter at
`web_pages/dosage_options.rs:71` filters by parent only after this complete fetch.
Thus 501 visible household options block a single medication's list/new/create
path, and medication editing also calls the same household collection for mode
detection (`web_pages/medications.rs:73`). Existing option member edit reads its
member directly (`web_pages/dosage_options.rs:331`), so not every dosage route is
blocked. Reserve G for a bounded, authorised pagination solution using the
documented list API. Its OpenAPI parameters at `:1775` have page/per_page and
updated_since, not medication_id; do not invent a filter. Never treat a partial
collection as proof of zero options. Require actual 501-option regression and
unrelated-parent privacy/mode checks before accepting the remedy.

## Copy and styling

**4158482147 — no material defect demonstrated; no required change.**
The internal translation key names stock removals, but its visible English copy
at `config/locales/en.yml:33` says “Reload this form and try again.” All five
catalogues explicitly say reload. `web_pages/dosage_options.rs:390` retains the
original missing token, and `medication_management.rs:38` maps the error to this
reload instruction. It neither mentions removals nor promises retrying the same
missing-token draft succeeds. Dedicated edit guidance could improve copy, but
the comment is not an authorisation/concurrency defect or acceptance blocker.

**4158482316 — confirmed already-recorded P3, E integration.**
`rust/web/src/dosage_options.rs:234` omits med-primary on the submit action. D's
report already records the desktop styling limitation and mobile minimum target
size; the coordinator queued shared-form polish for E. No duplicate issue or
new D acceptance loop is warranted.

These are source findings and compatibility recommendations. Requirements and
quality acceptance for E/G remain pending actual RED/GREEN, matching-source
runtime and screenshots. They do not alter D's recorded verification scope.

The coordinator accepted these remedies and filed the three E stock-truth gaps
as [2352](https://github.com/damacus/med-tracker/issues/2352), and the G collection
limit as [2353](https://github.com/damacus/med-tracker/issues/2353). Both carry
`rust,bug`; the coordinator owns publication and comment responses.
