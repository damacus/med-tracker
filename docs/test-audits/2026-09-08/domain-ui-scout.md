# Domain UI component test-value audit

Scope: `spec/components/{medications,person_medications,schedules,dashboard,locations,people,shared,reports}/` and the directly rendered component behaviour reviewed alongside them. This is a read-only classification against the current worktree, which already contains staged UI-sweep fixes and regression tests. No specs or implementation files were changed and no tests were run.

The rule used here is deliberately narrow: a short test is not low value by itself. A test is a removal candidate only when it proves an incidental utility/class/icon/layout detail with no observable contract, and a named survivor covers the behaviour that matters. Touch targets, actual overflow prevention, state transitions, role/access checks, timing, stock/units, translations, and accessibility assertions stay.

## Safe removal or consolidation candidates

| Exact example | Classification | Why | Retained coverage / survivor |
| --- | --- | --- | --- |
| `spec/components/medications/form_view_spec.rb:67-75`, `renders without the decorative pill icon and uses compact spacing` | **safeRemove** | It asserts one absent icon class string and one exact spacing selector. Neither affects form submission, field semantics, dose configuration, or copy. | Keep the surrounding i18n translations, category combobox, supply-field, and dosage-option examples in the same file. |
| `spec/components/medications/show_view_spec.rb:50-57`, `offsets the content to align with the header title column` | **safeRemove** | An exact `md:pl-[6.5rem]` class is a spacing implementation detail; no geometry is measured here. | Keep the existing action href/modal/policy examples and the real browser overflow checks in the sweep/system specs. |
| `spec/components/medications/show_view_spec.rb:59-108`, five `uses action button utility tokens...` examples plus `col-span-2` class assertion | **safeRemove** for the rounded-token examples; **defer** the grid assertion | `rounded-shape-full` on each action is decorative and repeated across several component specs. The grid placement may affect narrow-screen reachability, so do not remove that assertion without checking the current mobile evidence. | Named survivor for action behaviour: the same file’s `renders the log administration href`, Turbo-frame, order workflow, and policy-visibility examples; browser geometry remains authoritative. |
| `spec/components/medications/wizard/step_dose_schedule_spec.rb:13-19`, `renders rounded RubyUI-style form controls for dose setup` | **safeRemove** | It proves three identical rounded utility classes and no form value, label, validation, or submission contract. | Keep the wizard/system form tests and the dosage/timing field specs. |
| `spec/components/shared/metric_card_spec.rb:109-115`, `renders the active schedules icon path` | **safeRemove** | It snapshots the exact SVG viewbox/path for a decorative icon. A path change does not change the metric contract. | Keep the metric title/value/link/variant tests in this file. Unit-to-icon mapping remains covered by `spec/components/shared/medication_icon_spec.rb` where the icon is meaningful to medication units. |
| `spec/components/dashboard/stat_card_spec.rb:19-29`, users/inventory icon examples | **safeRemove** | Both only assert that an SVG exists; they do not protect a labelled control or an information-bearing value. | Keep title/value and link-wrapper behaviour in `stat_card_spec.rb:7-16,32-56`; retain any explicit icon mapping test only if the product treats the icon as semantic. |
| `spec/components/dashboard/schedule_card_spec.rb:36-41`, hand-package icon example | **safeRemove** | Decorative icon presence is separate from the actionable take-dose behaviour. | `schedule_card_spec.rb:30-34` and the schedule action/system specs verify that the take action and stock quantity render. Timing/stock state tests remain. |
| `spec/components/dashboard/timeline_item_spec.rb:127-132`, hand-package icon example; `:134-140`, hover-treatment classes | **safeRemove** | These are decorative SVG and hover utility assertions. They do not protect upcoming/cooldown/out-of-stock state or whether the action exists. | Retain `:75-125` state/action-presence examples and the timing/status copy. |
| `spec/components/medications/administration_modal_spec.rb:12-25`, hand-package icon example | **safeRemove** | It asserts only a decorative icon on a rendered action. | The modal/action route and administration behaviour are covered by the medication administration system/request specs; if a component survivor is wanted, retain an action-presence assertion rather than an icon selector. |
| `spec/components/medications/list_item_component_spec.rb:57-73`, the two medication-icon examples | **safeRemove** as duplicate decorative coverage | The first checks the icon class; the second repeats icon presence while adding a brittle wrapper selector. Neither protects the medication name or inventory meter. | `list_item_component_spec.rb:44-55` retains friendly display-name behaviour; `:83-96` retains the stock meter contract. `spec/components/shared/medication_icon_spec.rb` is the named survivor for unit/icon mapping. |
| `spec/components/schedules/index_view_spec.rb:19-24`, table `caption-bottom`/border utility assertions | **safeRemove** | These exact table utility classes are decorative and do not prove table semantics or responsive content. | Keep the mobile/desktop list and dosage-unit text assertions in `schedules/index_view_spec.rb:8-34`; those protect the rendered schedule data. |
| `spec/components/people/add_medication_landing_spec.rb:6-17` and `:19-29`, rounded/state-layer class assertions | **partial safeRemove** | The route, two workflow choices, labels, and minimum touch target are useful; `rounded-2xl`, `rounded-xl`, and `state-layer` are implementation styling. | Retain option count/text, modal target, back href/text, and `min-h-11`/`min-h-[44px]` touch-target checks. |

## Partial trims where the behaviour should stay

| Exact example | Keep | Remove or consolidate |
| --- | --- | --- |
| `spec/components/locations/index_view_spec.rb:22-35`, `renders location card actions with shared M3 sizing and shape` | The presence of actionable links and the minimum 44px touch target, because this is an accessibility/interaction contract. | The `rounded-shape-full` and banned `rounded-xl`/`w-10`/`h-10` assertions are decorative token checks. The current browser delete-dialog spec is the survivor for actual mobile interaction. |
| `spec/components/locations/index_view_spec.rb:43-50`, `uses the shared responsive page header` | Keep the add-location link and heading text. | Drop exact `flex-col`, `md:flex-row`, `md:items-center`, `w-full`, and `md:w-auto` assertions unless a browser geometry test demonstrates a regression; they duplicate CSS implementation details. |
| `spec/components/people/person_card_spec.rb:40-58`, `renders card actions with shared M3 sizing and shape` | Keep action presence and the minimum touch-target check. | Drop `rounded-shape-full` and banned `rounded-xl`; no user-facing behaviour depends on those tokens. |
| `spec/components/medications/index_view_spec.rb:28-35`, `offsets the content...` | None at component level unless a measured alignment contract is introduced. | Remove exact `md:pl-[6.5rem]`; rely on the mobile/desktop browser geometry audit. |
| `spec/components/medications/index_view_spec.rb:37-47`, `renders medication actions with m3 link variants` | Keep action presence and labels. | Remove exact background/hover/text class assertions; those are visual tokens without a component behaviour contract. |
| `spec/components/medications/index_view_spec.rb:77-115` | Keep `flex-wrap`, `max-w-full`, and minimum touch-target assertions because they protect the known narrow-header overflow/action reachability. | Remove rounded-shape and banned height class checks. |
| `spec/components/reports/export_panel_spec.rb:16-22`, `renders a responsive export control with the active scope` | Keep export scope/title text. | Remove exact `flex-col`/`sm:flex-row`; `spec/system/report_exports_spec.rb:11-37` already measures 390px/desktop overflow and visible controls. |
| `spec/components/reports/filter_form_spec.rb:6-19`, labelled apply button with hidden icon | **Keep**. The `aria-hidden` assertion is accessibility coverage, not decoration-only styling. |
| `spec/components/locations/index_view_spec.rb:53-61`, and matching icon assertions in `locations/show_view_spec.rb:61-68` and `list_item_component_spec.rb:98-105` | **Keep**. These verify decorative SVGs are hidden from assistive technology inside labelled controls. |
| `spec/components/schedules/card/actions_component_spec.rb:52-65,93-107` and `spec/components/person_medications/card_spec.rb:95-108` | Keep the minimum touch target, overflow-visible/fixed-menu strategy, and narrow-screen stacking assertions. | Consider dropping only exact padding/background/state-layer token checks (`px-4`, `bg-background`, `not px-1.5`, etc.) after the browser menu/overflow checks remain green. |

## Retain without change

- Medication timing, dose-cycle, max-dose, minimum-hours, and singular/plural assertions in `person_medications`, `schedules`, and medication wizard specs.
- Stock quantity/status, low/out-of-stock, supply meter, reorder and refill assertions.
- Policy and role/access visibility examples, including manager versus non-manager actions.
- Translation and locale contract specs, including dashboard/schedule/report i18n and the new location medication-count/stock-warning checks.
- Real overflow/viewport tests and focus/dialog tests in system specs; these are stronger evidence than static class checks.
- Functional links, Turbo frame targets, form field names/values, alert-dialog controls, and state-specific action presence.
- `spec/components/schedules/form_component_size_spec.rb`: this is a structural extraction/maintenance guard rather than a decorative UI assertion; defer removal until the component-boundary policy is intentionally changed.

## Deferred / needs a product or evidence decision

- `spec/components/shared/metric_card_spec.rb:69-107` compact/dashboard-summary layout assertions: exact utility classes are implementation details, but `break-words`, responsive density, and centring may protect the dashboard’s known small-screen content contract. Keep until the corresponding browser geometry evidence is reviewed.
- `spec/components/dashboard/index_view_spec.rb:123-239` responsive option-grid and action-row/touch-target checks: some class assertions are incidental, but the dashboard has active mobile/enlarged-text coverage. Consolidate only after comparing each assertion with that system evidence.
- `spec/components/medications/show_view_spec.rb:99-107` adjust-inventory grid placement: retain pending narrow-width verification.
- `spec/components/medications/list_item_component_spec.rb:83-96` supply meter classes/style: retain because this is a stock/units visual contract, not decorative styling.
- Any icon assertion that is the only evidence an action is present or that distinguishes a medication unit should be retained; the safe removals above are limited to redundant decorative checks.

## Result

The safest first cleanup is to remove exact decorative icon/path/rounded-token/spacing examples, consolidate repeated medication-icon coverage behind `shared/medication_icon_spec.rb`, and trim class-only assertions from mixed behaviour tests. Keep all state, role, accessibility, translation, stock/units, timing, and measured overflow coverage. No tests were run during this audit.
