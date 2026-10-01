# Household i18n run report

Status: bounded catalogue adapter implemented and focused tests green. Page integration remains with the household page owners.

## Outcome

`rust/web/src/household_i18n.rs` exposes Locale En/Cy/Ga/Es/Pt with explicit cookie, weighted Accept-Language and English-default resolution. Text takes Locale explicitly and reads the five embedded authoritative Rails YAML catalogues. Its get/plural methods return Result, require named interpolation values and preserve exact-zero/one/other selection. Interpolation scans the source template once; argument content cannot introduce another substitution. Normal Leptos text rendering escapes user values.

Text::source_locale exposes which catalogue supplied a key, including English fallback. Text::api_error translates existing blank/invalid/taken/not_a_number/inclusion messages and greater_than/greater_than_or_equal_to messages with the exact zero threshold. It returns None for unknown upstream details. Catalogue access is restricted to the namespaces used by new household forms/navigation and the existing interpolation/plural test witnesses. Parsed catalogues are immutable; there is no process-global selected locale. Missing keys and non-text values produce errors rather than invented copy.

## Verification

- RED: `rtk task -d rust/web test TEST_FILE=household_i18n` failed E0432 because the production module did not exist. No production helper had been added yet.
- GREEN: `rtk proxy env CARGO_NET_OFFLINE=true task -d rust/web test TEST_FILE=household_i18n` passed six tests.
- Second RED: adding api_error acceptance failed E0599 before adding that method.
- Second GREEN: the same offline Task command passed seven tests, including all five household titles, interpolation/HTML escaping, plural edge counts, language precedence, concurrent locale isolation, missing key/argument rejection and known/unknown API validation messages.
- The two owned Rust files were formatted through a temporary stdin Task definition running scoped rustfmt. The final post-format focused rerun passed 7/7 tests. `git diff --check` passed.

## Authorised follow-up

Root subsequently assigned this agent two canonical source-key additions and numeric/unit error mappings. `forms.medications.dosage_options_read_only` now exists in all five Rails catalogues and explains that editing details preserves dosage options. `forms.locations.new_title` now exists in all five catalogues, with English New Location matching the existing Rails heading. Non-English text was added explicitly; none of these keys relies on English fallback. The location owner was notified of the canonical heading key.

Numeric/unit mapping and dosage notice tests first failed (None instead of the canonical numeric message, and MissingKey respectively). The location heading test separately failed MissingKey before its source additions. Final post-format focused Task verification passed 10/10 tests, including all-locale source presence. `git diff --check` passed; the source locale diff is exactly two inserted keys per file.

API details `is not a valid decimal`, `is outside stock precision` and `must be a string` remain untranslated because this bounded mapping does not substitute a less precise generic error for them. Root was notified. The new translations have not received independent linguistic review.

## Inventory return translation preparation

`medications.index.stock_remaining` was added to every source locale after a focused MissingKey RED. It retains amount/unit interpolation: English `%{amount} %{unit} remaining`, Welsh `%{amount} %{unit} yn weddill`, Irish `%{amount} %{unit} fágtha`, Spanish/Portuguese `%{amount} %{unit} restantes`. The focused helper suite passed 11/11 after formatting; the bundled translate skill locale-tree checker passed across all five complete catalogue files. No root lib renderer edits were made by this agent.

The following maps every English static inspected in `render_medication_list_with_management` and `render_medication_detail_with_management`. Keys are exact existing source paths unless marked a gap or intentional copy replacement. For whole translated quantities, use stock_remaining with the entire amount/unit rather than concatenating an English suffix.

| Current visible text or attribute | Authoritative key / disposition |
| --- | --- |
| MedTracker | Product brand; `layouts.navigation.brand` if a key is desired |
| Household navigation aria-label | No exact label; `layouts.mobile_rail.primary_navigation` is an intentional replacement with Primary navigation |
| Dashboard | `layouts.sidebar.dashboard` |
| Inventory | `layouts.sidebar.inventory` |
| People / Locations | Already keyed as `layouts.sidebar.people` / `layouts.sidebar.locations` |
| HOUSEHOLD INVENTORY | No exact standalone key. `medications.index.your_inventory` is an intentional replacement with Your Inventory; stock_check.eyebrow includes a location interpolation and is not interchangeable |
| Medications and page title | `medications.index.title` |
| Add Medication | `medications.index.add_medication` already used |
| amount unit remaining | New `medications.index.stock_remaining`, arguments amount and unit |
| View medication | `medications.index.view` supplies shorter View; this is an intentional copy replacement, not an exact translated phrase |
| MEDICATION PROFILE | `medications.show.profile` |
| Edit Medication link | `medications.form.edit_title` already used |
| Overview | `medications.show.overview` |
| Inventory Status | `medications.show.inventory_status` |
| Stock source and Stock source: | `medications.take_action.stock_source`; colon is presentation punctuation |
| Log buttons/link | `medications.show.log_administration` |
| Log administration for medicine (dialog aria-label and heading) | Gap: requires a complete interpolated phrase, not concatenated fragments |
| Choose the person and source. | Gap |
| Close | `ruby_ui.common.close`; helper currently needs ruby_ui.common namespace allowlisting |
| MEDICATION SOURCE | Gap |
| Calculated for selected time / Dose calculated for selected time | Gap; two existing renderer phrasings should share one deliberately reviewed statement |
| View only | Gap |
| No eligible stock | Gap |
| Record dose (dialog aria-label and heading) | `medications.take_action.title` |
| Confirm the person, medication, time, and inventory source. | Exact `medications.take_action.description` |
| Please review this dose. | Gap |
| This source is unavailable. Choose another source before submitting. | Gap |
| Choose source, including initial person fallback | Gap. stock_removals.choose_source means Choose stock and cannot label a person/medication-source picker |
| PERSON | `medications.take_action.person` |
| MEDICATION | `medications.take_action.medication` |
| DOSE | `medications.take_action.dose` |
| Taken at | `medications.take_action.taken_at` |
| Stock option name — amount unit remaining | Name is user data, dash is punctuation, quantity uses stock_remaining |
| Validation/success notice from the API | Dynamic data: use known canonical validation keys; do not claim unknown upstream copy translated |

The administration/source modal gaps prevent a five-locale complete-journey claim even after list/detail headings are translated. New catalogue entries should preserve the existing renderer wording and clinical meaning. Existing generic stock-removal wording must not be repurposed for dose-source selection.

## Dependency and scope limits

leptos_i18n 0.6.2 is not adopted or compiled. It is absent from the local dependency cache; direct crates.io access failed DNS. No dependency installation was attempted. The bounded fallback uses root-authorised cached serde_yaml 0.9.34+deprecated. Its maintenance status is an explicit transitional gap, not a maintained-parser claim. Root owns its manifest/lockfile change. Offline resolution also downgraded android_system_properties 0.1.6 to cached 0.1.5; root was notified to inspect that incidental delta.

The helper permits the layouts namespace, including sidebar, mobile_rail.primary_navigation and navigation.skip_to_content keys requested by the root-owned shell. Each requested key still resolves against the authoritative catalogues and fails if English is absent.

The helper preserves existing Rails plural semantics. It does not provide reviewed missing Welsh/Irish CLDR forms, localised date/number formatting, a language-switch UI or dashboard/search hydration integration. It does not claim full household UI translation until each page owner wires locale/text and verifies its render/browser acceptance. Unknown API error details remain unverified translation scope and should be displayed with a localised summary rather than replaced with invented translations.
