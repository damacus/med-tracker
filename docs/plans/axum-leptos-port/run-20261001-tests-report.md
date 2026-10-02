# Household test lane report

Status at 2026-10-01 09:21 UTC: route/API baseline RED recorded; all 9 rendering
cases and 5 entry-navigation/inventory locale rendering cases GREEN; 3 added
management-detail locale cases RED before coordinator implementation. Initial browser workflow
and 10-case black-box acceptance are running against an isolated fixed product
snapshot. Separate navigation and medication lifecycle runtime RED is pending.

Changed paths:

- `rust/web/tests/household-routes.test.mjs`
- `rust/web/tests/household-workflows.test.mjs`
- `rust/web/tests/household_rendering.rs`
- `rust/web/tests/household_navigation.rs` (coordinator added this ownership)
- `rust/contract-tests/tests/household_web.rs`
- `rust/contract-tests/tests/household_navigation.rs` (coordinator added this ownership)
- `rust/contract-tests/tests/household_lifecycle.rs` (coordinator-requested lifecycle guards)
- `docs/plans/axum-leptos-port/run-20261001-tests-brief.md`
- This report.

Recorded RED: `rtk task -d rust/web test TEST_FILE=household_rendering`
failed with Cargo exit 101 (task exit 201). `household_rendering.rs:1` imports
the absent `medtracker_web::household` module, producing E0432 before the
coordinator implements the shell. Cases cover escaping untrusted labels,
preserving native caller forms, current-household navigation, mobile viewport,
five authoritative document locales and rejecting an injected language.

Per-module compile RED was then recorded with the same task under
`CARGO_NET_OFFLINE=true`: E0432 for `people`, `locations` and
`medication_management`. Tests use agreed public renderer types and exercise
observable native form contents rather than internal component structure.

Runtime RED from the isolated runner's unchanged product snapshot
`d78ff0355198b24dce59b8792116af8d7df001ae8688b6b40ad54b4267979cbd`:
authenticated `/people`, `/locations`, `/medications/new` and
`/api/v1/households/{id}/ui_capabilities` returned 404; medication read lacked
`friendly_name`. The five subtests all ran despite earlier failures. Runner
task exited 201; raw evidence is `/tmp/medtracker-runner-route-red-browser-raw.log`.
Latest route test SHA256 at baseline:
`91069131822cb14b08efba6f1e3649f4ec5cda64262e1b36e6558b0a17dedd6b`.

Rendering GREEN: `CARGO_NET_OFFLINE=true rtk task -d rust/web test
TEST_FILE=household_rendering` passed all 9 cases. Initial retained-textarea
checks exposed unescaped location description and medication warning values.
Owners fixed escaping. Tests now use adversarial closing-textarea/script text,
require encoded HTML and reject raw injection. Browser acceptance additionally
requires the exact decoded input value and no injected script element.
The suite covers native person type selection/capacity state and fixed Rails
person/medication edit headings in English, Welsh, Irish, Spanish and Portuguese.
An exact per-locale blank-field error assertion exposed untranslated People
errors; the owner fixed them through the existing catalogue adapter.

A ninth rendering case first failed to compile because the dosage-option guard
renderer was absent, then failed with `MissingKey` for
`forms.medications.dosage_options_read_only`. The coordinator added its
catalogue text; the case now passes. The guard must retain identity
controls and exclude scalar dose/unit/supply controls for medication whose
dosage options already track balances.

Static checks: `rtk task api:household-browser-syntax` passed.
`CARGO_NET_OFFLINE=true rtk task api:contract-selected-compile
TEST_TARGET=household_web` passed. Owned Rust files formatted via
`rtk task api:fmt-file FILE=...`.

Browser route tests require authenticated private SSR responses for people,
locations and manual medication creation. Browser workflow cases use native
forms without JavaScript at 1400x900 and 390x844. They exercise person/location/
medication creation and edit persistence, 422 invalid drafts, associated field
errors, immediate location selection and medication display/warning retention.
Medication validation retains the exact decimal draft `2.50`.

Black-box tests separately cover private HTML/form/session boundaries, foreign
household denial, invalid CSRF with API read-back, policy-derived owner/viewer
UI capabilities, and editable medication identity/warning API representation.
Boundary expectations use existing `web_pages.rs`: anonymous 302 to login,
foreign slug 404, invalid CSRF/origin 403. Form success permits either standard
POST redirect code 302/303; validation requires 422.

The black-box module now has 10 cases, including manage-granted member
capabilities, dependent capacity normalisation, view-only session denial,
dosage-option identity-edit preservation and missing/blank/stale edit
preconditions. Missing/blank browser ETag requires 428; a stale ETag requires
409 with retained values and no record change. Existing core API stale handling
returns 409.

Harness dependency requested from coordinator: native form mutation helper
must send the current target's Origin. Existing `Target::post_html_form`
omits Origin/Referer; using it for these protected browser writes would only
test origin denial. Existing helpers must remain unchanged for their callers.
The coordinator added `Target::post_browser_form`; all household mutation tests
now use it. Selected compilation passed and the 10-case source was frozen for
the first acceptance snapshot. No later checks were added to that live-mounted
file while acceptance runs.

The runner fixed the first acceptance product at 08:59 UTC, digest
`ee1f625dd4357de7dfef110076e6d47f92ec0f4797b42407dc0443fbc9831f74`.
Its log is `/tmp/medtracker-runner-green.log`; project is
`mtcontract-dfaa00f2263c488a`. Runtime results have not yet been recorded.

Next entry-navigation RED is separate from that frozen file. Unit command
`CARGO_NET_OFFLINE=true rtk task -d rust/web test
TEST_FILE=household_navigation` failed with E0432 for the absent management
wrappers at lines 3-4 before their implementation. It requires People/Locations
navigation and creation/edit links only when the respective permission is true.
The separate black-box navigation target compiles and requires labelled owner
links from inventory/detail while viewer mutation links remain absent.
After implementation, the same focused rendering task passed both cases.
Runtime navigation RED is pending.

At 09:18 UTC the focused `household_navigation` task recorded 2 passes and
3 failures (task 201/Cargo 101) before inventory localisation. The added checks
cover English, Welsh, Irish, Spanish and Portuguese title/H1/document language,
sidebar labels, the catalogue View link and exact `1.25 ml` stock text.
Failures show Welsh document language still English, Welsh stock text absent,
and the English card link using `View medication` rather than catalogue `View`.
Stock text comparison ignores Leptos text-boundary serialization markers;
the decimal amount, unit and translated text remain exact requirements.

After coordinator implementation, all five navigation/inventory cases pass.
At 09:21 UTC three added management-detail locale cases failed in the same
focused task (5 pass/3 fail, task 201/Cargo 101). They require all five document
languages, unchanged medication-name title/H1, translated profile/overview/
inventory-status headings and dashboard/inventory/edit link labels. Failures
show an uppercase English profile literal instead of the catalogue text,
Welsh language declared English, and Welsh Dashboard still labelled English.
Existing dose-dialog controls and English Log selectors are outside these
added assertions.

Separate `household_lifecycle` selected compilation passed. Its two cases
require a medication with neither scalar dose nor dosage options to retain
editable scalar dose/unit/supply controls, then persist its first scalar dose.
They also require an old scalar form submitted after real dosage options were
created to return 409, retain the exact `2.50` draft/display/warning values and
leave medication and dosage-option API state unchanged. These tests await
runtime RED on the original fixed product before the medication owner fixes
mode detection and conflict precedence.

Rails observable references: `app/components/people/form_view.rb`,
`app/components/locations/form_view.rb`, medication form components and
`config/locales/en.yml`. Native controls use their exact English labels.

Unverified: contract runtime acceptance, all workflow runtime GREEN, screenshots, locale
workflow rendering, stock actions, treatment workflows and offline cache
privacy. No production source, fixture or shared runner edits by this lane.
