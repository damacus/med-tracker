# Inventory scout report

Baseline: `45db2112` on `codex/fix-web-ui-sweep`. This is a source and test inventory only; no browser run was performed here, so every item below is a candidate until reproduced in the local test/development UI. Canary is excluded per the sweep plan.

## Route and coverage inventory

The scoped household web routes are:

- Inventory: `GET /households/:household_slug/medications`, `GET /medications/:id`, `GET /medications/stock_check`, plus refill, stock adjustment, scan-restock and order-state actions (`config/routes.rb:216-234`).
- Locations: `GET /locations`, `GET /locations/:id`, location CRUD and membership add/remove (`config/routes.rb:216-218`).
- Finder: `GET /medication-finder` and JSON `GET /medication-finder/search` (`config/routes.rb:236-237`).
- Medicine reviews: `GET /medicine-reviews`, prompt updates, and `GET /medicine-reviews/report` (`config/routes.rb:207-210`).
- Reports: `GET /reports` and `GET /reports/health-history` (`config/routes.rb:207-209`).
- Profile/settings: `GET/PATCH /profile`, profile experiments, API tokens, data exports and avatar; household admin settings at `GET/PATCH /admin/settings` (`config/routes.rb:200-205`, `config/routes.rb:304`).

Existing relevant browser/system coverage includes `spec/system/medication_finder_spec.rb` (finder rendering, 390px search geometry, result details and restock flows), `spec/system/locations/delete_location_dialog_spec.rb` (390px location delete dialog), `spec/system/report_exports_spec.rb` (390px and desktop report/export overflow), `spec/system/profile_editing_spec.rb` (390px profile dialog overflow, tab switching and focus), and `spec/system/people_spec.rb` (people list and modal flows). Component/request coverage includes `spec/components/locations/index_view_spec.rb`, `spec/components/medications/finder_view_spec.rb`, `spec/components/views/profiles/show_spec.rb`, `spec/components/reports/filter_form_spec.rb`, `spec/requests/medication_review_prompts_spec.rb` and `spec/requests/medications_category_filter_spec.rb`.

Coverage gaps for this scope: no browser geometry test was found for the location show member controls or location medication cards; no long-name/long-badge finder result geometry test was found; no non-English browser copy test was found for location stock/count labels or finder package labels; and profile tests exercise 390px tabs but do not measure tab-label fit at 320px or enlarged text.

## Candidates for local reproduction

### INV-01 — Location medication count is built from a translated success sentence

- **Route/role/state:** `/locations`, signed-in household manager, one or more medications in a location; repeat under Welsh, Irish, Portuguese or Spanish locale.
- **Evidence:** `Components::Locations::IndexView#render_location_card` calls `pluralize(location.medications.size, t('medications.created').split.first.downcase)` (`app/components/locations/index_view.rb:64-67`). The source translation is a sentence (`Medication was successfully created.` in English), and other locales begin with inflected verbs such as Welsh `Crëwyd` and Irish `Cruthaíodh`, so the badge can become “1 crëwyd”/“2 crëwyd” rather than a medication noun.
- **Expected:** The badge should use a dedicated count translation with the correct noun/plural rules for the active locale.
- **Repro:** Seed a location with one and two medications; visit `/locations`; switch locale to `cy` or `ga`; inspect the badge text and singular/plural forms.
- **Severity:** P1 wording/understanding defect for supported locales; likely P2 if English-only reproduction is the acceptance boundary.
- **Missing coverage:** `spec/components/locations/index_view_spec.rb` checks actions/header but not count copy or locale structure; add focused component examples for one/many in every supported locale after confirmation.

### INV-02 — Location stock warning is hardcoded English

- **Route/role/state:** `/locations/:id`, any signed-in user allowed to view a location containing a low-stock medication; non-English locale.
- **Evidence:** `render_stock_badges` emits literal `'Low Stock'` (`app/components/locations/show_view.rb:139-145`) instead of an I18n key. The same card otherwise uses translated copy and locale files contain supported translations for the surrounding medication UI.
- **Expected:** The warning should be translated and retain the locale’s stock terminology.
- **Repro:** Create or use a medication where `low_stock?` is true, visit its location show page under `cy`, `ga`, `es` or `pt`, and inspect the badge.
- **Severity:** P1 wording defect in a clinically relevant inventory warning.
- **Missing coverage:** `spec/components/locations/show_view_spec.rb` has no locale assertion for low-stock copy; no browser locale check covers this route.

### INV-03 — Member removal control is hover-only discoverable

- **Route/role/state:** `/locations/:id`, manager with at least one member, keyboard navigation or touch device.
- **Evidence:** The remove-member trigger is rendered with `opacity-0 group-hover:opacity-100` and no focus-visible or always-visible state (`app/components/locations/show_view.rb:179-190`). The control has an accessible label, but a keyboard user tabbing through the member row can receive focus on an effectively invisible icon; touch users have no hover state.
- **Expected:** A focused trigger should be visible, and the action should remain discoverable on touch (for example, visible or exposed through a labelled action menu).
- **Repro:** Visit a location with members at 390px; tab through the member rows and inspect the focused remove control; repeat on a touch-emulation/device viewport and compare whether the action can be discovered without hover.
- **Severity:** P1 accessibility/task-completion candidate.
- **Missing coverage:** `spec/system/locations/delete_location_dialog_spec.rb` covers location deletion only; no location-show member keyboard/focus or touch test exists.

### INV-04 — Profile tab strip has four equal-width labels with no overflow handling

- **Route/role/state:** `/profile`, signed-in user, 320px viewport or enlarged text; inspect all four tabs.
- **Evidence:** The tab list is `flex w-full` without horizontal scrolling (`app/views/profiles/show.rb:54-62`), while each trigger is `min-w-0 flex-1 px-1 text-xs` (`app/views/profiles/show.rb:66-86`). Labels include “Notifications”; there is no truncation, wrap, or overflow strategy at the tab-list boundary.
- **Expected:** All tab labels should remain readable and operable at the plan’s 320px/enlarged-text stress points, with a deliberate scroll/wrap strategy if needed.
- **Repro:** Visit `/profile`, resize to 320px, then increase browser text size/zoom; inspect tab bounding boxes, visible text, horizontal scroll width and keyboard focus for each tab.
- **Severity:** P2 interaction/layout candidate; promote only if local geometry shows clipping or inaccessible focus.
- **Missing coverage:** `spec/system/profile_editing_spec.rb` checks 390px dialog overflow and tab switching, but no 320px/enlarged-text tab geometry; `spec/components/views/profiles/show_spec.rb` checks semantics only.

### INV-05 — Finder result header and badge column can force narrow-screen overflow

- **Route/role/state:** `/medication-finder`, signed-in user, populated results with a long source label, match reason or concept-class label; 320px/390px viewport.
- **Evidence:** The result header is `flex items-center justify-between` and its source paragraph has no `min-w-0`, wrapping or text alignment class (`app/javascript/controllers/medication_search_controller.js:115-127`). Each result card reserves a `shrink-0` right column for several badges (`app/javascript/controllers/medication_search_controller.js:143-192`), while the left title is only `truncate`. Long source/concept labels can therefore squeeze the result content or widen the document.
- **Expected:** Result metadata should wrap or stack within the card and keep the page within the viewport while preserving the full label.
- **Repro:** Stub finder JSON with long `source_label`, `match_reason_label` and `concept_class_label`; search at 320px and 390px; measure `document.documentElement.scrollWidth`, inspect card bounds and keyboard focus.
- **Severity:** P2 overflow/readability candidate.
- **Missing coverage:** `spec/system/medication_finder_spec.rb` covers 390px search controls and one related-medication overflow case, but no long-result metadata geometry.

### INV-06 — Finder package-size label bypasses translations

- **Route/role/state:** `/medication-finder`, populated result carrying `package_size`, non-English locale.
- **Evidence:** The controller interpolates literal `Pack size:` (`app/javascript/controllers/medication_search_controller.js:169-172`). Finder translations provide package detail labels (`details.package`) but no translation for this rendered label.
- **Expected:** Package size should use the locale’s package label consistently with the details panel.
- **Repro:** Stub a result with `package_size: '32 tablets'`; visit the finder under `cy`, `ga`, `es` or `pt`; search and inspect the result card.
- **Severity:** P2 wording/localisation defect.
- **Missing coverage:** Finder component tests validate the translation payload but do not assert rendered package-size copy in supported locales; browser specs use results without this field.

## Verification boundary

These are source-grounded candidates, not confirmed visual bugs. Local verification should use the project’s browser harness and fixture data at the requested 390px/1280px widths, plus 320px and enlarged text where called out. No files beyond this report were changed and no tests were run during the inventory pass.
