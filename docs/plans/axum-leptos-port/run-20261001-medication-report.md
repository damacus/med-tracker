# Medication lane report

Status: medication SSR renderer and native HTTP handlers integrated after recorded
RED. Compile and focused renderer checks pass; actual browser acceptance pending.

Owned documents: medication-research.md, medication-brief.md and this report under
docs/plans/axum-leptos-port/run-20261001-.

Changed production paths: rust/web/src/medication_management.rs and
rust/api/src/web_pages/medications.rs. Root owns router/export/shared integration.

Read-only verification: API attribute whitelist, scalar decimal rules, create and
update policies, current medication serializer, ETag handling, Rails labels,
existing native dose form, current hydration entry and pinned Leptodon controls.

RED: test owner recorded missing medtracker_web::medication_management import
at 08:46 UTC using CARGO_NET_OFFLINE=true rtk task -d rust/web test
TEST_FILE=household_rendering (cargo 101/task 201).

GREEN: rtk task -d rust/web test TEST_FILE=household_rendering, exit 0, six passed.
The initial run compiled but failed textarea escaping assertions; explicit text
encoding fixed the retained warning and prevents raw closing-textarea injection.
SSR checks cover native POST/CSRF/ETag, exact decimal draft, escaped display/warning
and associated Name error. No browser journey acceptance is claimed yet.

Runtime RED supplied by test owner: immutable baseline source digest
d78ff0355198b24dce59b8792116af8d7df001ae8688b6b40ad54b4267979cbd,
authenticated medications/new returned 404 instead of 200; editable medication
friendly_name absent. Evidence: /tmp/medtracker-runner-route-red-browser-raw.log.

Existing dosage options have an explicit read-only boundary. The scalar form omits
dose/unit/supply in that mode; identity edits preserve all options and
the adapter rejects forged scalar-mode changes. This avoids API mode-switch logic
that would delete options. Full dosage-option management is not yet implemented.
The options renderer followed another recorded compile RED. Its first integrated
run had eight passed and one failed for the coordinator-owned missing
forms.medications.dosage_options_read_only key. The coordinator added that key in
all five source catalogues. Latest renderer run at 08:58 UTC: nine passed, exit 0.
Tests include adversarial closing-textarea content, translated forms and options
read-only controls. No fallback wording hides missing catalogue keys.

rtk task api:check: exit 0 after coordinator helper/route registration. Initial
integration found a redirect argument type mismatch, fixed in the owned module.
Browser edit requires a current ETag: missing or blank submitted ETag returns 428;
stale If-Match returns 409 with the rejected draft. A successful mutation is read
back and the saved ID verified before a constant, encoded inventory redirect.

rtk task -d rust/web fmt: task 201 due current formatting differences across
shared and owner files. No global formatter was run by this owner.

Outstanding acceptance: relevant Rust format/lint checks; actual authenticated
native browser create/edit/error journeys;
desktop/mobile screenshots; non-English errors and policy affordances; independent
review and any fixes. No pass claims are made for unexecuted checks.

Review follow-up queued behind separate lifecycle runtime RED: detect actual
scoped dosage options rather than using null dose as their proxy. This preserves
optional-dose creation and editing, including legacy null-dose/no-options records.
Check stale browser ETags before the options-field guard and retain the submitted
form mode and exact draft on 409. The coordinator owns full timestamp precision
in medication representations; API locking and unchanged forwarded If-Match are
required for safe concurrent first-option creation. The adapter check alone is
not the atomic write boundary. No stock or option editor production work started.

Owned-file git diff --check passed at 09:16 UTC.
