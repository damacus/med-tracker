# Medication form research

Recorded 2026-10-01 08:35 UTC. Read-only inspection precedes implementation.

Use native named HTML controls for medication writes. Pinned Leptodon GenericInput
initialises values with DOM effects and omits an SSR input value attribute.
Leptodon FormInput does not associate feedback with aria-describedby/aria-invalid.
Existing native dose form and WebApi internal dispatch remain the compatibility reference.

Sources: rust/web/src/lib.rs:370, rust/api/src/web_pages.rs:363,
rust/api/src/medication_management.rs:434, and the pinned Leptodon revision
195c8fd78b13718004b1e26b2aad8aee5b451585 input/form_input source.
Context7 resolved /openanalytics/leptodon and /leptos-rs/leptos and returned
current input, date picker, form and SSR/hydration documentation.

The medication API accepts scalar attributes only; decimal quantities are strings.
Creation requires name, positive household location ID and reorder threshold.
Dose amount may be null; units are constrained by the API. Blank barcode is valid.
Update accepts If-Match and rejects stale values with conflict.
Creation policy differs from manager-only update and must be supplied by API policy.

No Rails wizard JSON is copied. Its schedule_config metadata is rejected by Rust.
This lane does not own treatment, stock or existing dose handlers.

Adoption decision: reuse existing native forms, app-owned accessible field wrappers,
existing Leptodon layout and styling. No new dependencies. Form-tool adoption would
not replace API error mapping or medication rules.

Primary references:
- https://github.com/openanalytics/leptodon
- https://book.leptos.dev/progressive_enhancement/action_form.html
- https://docs.rs/leptos_form_tool/latest/leptos_form_tool/

Remaining proof: SSR edit values, rejected draft retention, persisted create/edit,
fresh location options, policy affordances, foreign record and CSRF denial,
private response headers, safe inventory return and existing dose regression.

Edit-read gap: serialize_many in rust/api/src/lib.rs omits friendly_name, barcode
and warnings. Coordinator must expose these existing attributes before the form
can restore them faithfully; display_name cannot recover friendly_name reliably.
Medication creation also accepts an active manage grant via may_create, whereas
update is owner/administrator only. The earlier manager-only creation assumption
is superseded by this direct source check.

Exact Rails labels agreed with test owner: Add a New Medication; Name;
Display name; Description; Dose; Unit; Starting Supply/Remaining Supply;
Reorder Threshold; Warnings; Location; Save Medication.

## Next bounded dosage-option editor

Design only while the initial medication journey awaits acceptance. No option
production code is authorised before its RED evidence and coordinator assignment.

Use nested browser routes under medications/{id}/dosage_options for list/new/edit
and native POST saves. Reuse WebApi, policy capabilities, native draft controls,
ETags and the private household shell. Verify each selected option belongs to the
selected medication, not merely the household. Load the authorised paginated option
collection once and filter at the handler boundary; no component queries.

Existing API supports option collection/show/create/PATCH/PUT. It has no delete
route, so do not show a delete action. Writes require owner/administrator, unlike
medication creation with a manage grant. Parent medication visibility remains
authoritative. Existing sources: rust/api/src/dosage_options.rs and
rust/contract-tests/tests/dosage_health.rs.

Create requires medication_id as a string, amount as an exact decimal string,
nonempty unit/frequency, integer default_max_daily_doses, decimal-string
default_min_hours_between_doses and daily/weekly/monthly default_dose_cycle.
Optional description and adult/child defaults are strings/booleans respectively.
Nullable current_supply/reorder_threshold use decimal strings with stock precision
eight integer digits and two decimal places. Minimum hours has three integer digits
and one decimal place. Keep draft text unchanged on rejected submissions.

Unlike scalar medication units, the API accepts any nonempty dosage-option unit.
A shared common-unit selector must preserve existing custom units or offer a text
input; a scalar-unit whitelist must not reject them silently.

Creating an option switches the parent dose_amount to null. Tracked option supply
is aggregated into parent stock and threshold inside the API transaction. Therefore
the first slice should add/edit options for existing option-mode medications only;
switching an existing scalar medicine requires an explicit reviewed transition.
Stock arithmetic, default uniqueness, audit and sync stay in the API.

Option update accepts If-Match. Native browser updates should require it. Option
create has no idempotency implementation and no general duplicate option index;
do not claim replay protection or silently aggregate duplicate creates. Existing
API validation returns a whole-option error, rather than per-field errors. Expose
that as an accessible form summary and use field associations only when a precise
transport parsing error is known.

Essential next RED cases: list existing options and retain custom units; create a
second independently stocked option; edit exact decimals/defaults with ETag;
retain invalid draft; reject duplicate adult/child default; stale and missing ETag;
foreign medication/option mismatch; CSRF denial; parent stock equals committed
tracked-option total while other option remains unchanged; treatment choices and
existing dose journey remain usable; desktop/mobile keyboard and screenshots.
Use synthetic new medications to avoid changing shared dose fixture balances.
