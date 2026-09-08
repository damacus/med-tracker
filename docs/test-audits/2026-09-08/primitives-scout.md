# Primitive component test-value audit

Read-only classification of low-value ongoing examples in the requested primitive and named component specs. No tests were run and no product behaviour was changed.

## Remove

- `spec/components/icons/inventory_spec.rb:35`, example `renders the Material Symbols inventory path`: exact SVG path data is decorative implementation detail.
- `spec/components/icons/hand_package_spec.rb:37`, example `renders the Material Symbols hand_package path`: same decorative path assertion.
- `spec/components/icons/medication_spec.rb:25`, example `renders the Material Symbols medication path`: same decorative path assertion.
- `spec/components/icons/passkey_spec.rb:30`, example `renders the Material Symbols passkey path`: same decorative path assertion.
- `spec/components/icons/chevron_right_spec.rb:6`, example `keeps the shared chevron defaults unchanged`: exact default path is decorative. Keep the custom path API example at line 14.
- `spec/components/ruby_ui/breadcrumb_separator_spec.rb:14`, `breadcrumb_ellipsis_spec.rb:15`, `select_item_spec.rb:6`, and `select_trigger_spec.rb:6`: repeated numeric SVG sizing and absence of `h-4`/`w-4` are implementation-level geometry details. Keep semantic wrappers and custom-block tests.
- `spec/components/ruby_ui/alert_dialog_content_spec.rb:6`, `dialog_content_spec.rb:6`, `dialog_header_spec.rb:6`, `dialog_middle_spec.rb:6`, and `sheet_content_spec.rb:17`: background, gradient, blur, and token class snapshots protect visual implementation rather than observable behaviour. Keep close-button, native cancellation, ARIA naming, focus hooks, duplicate-ID, and responsive semantic tests in these files.
- `spec/components/m3/input_spec.rb:6`, example `renders an M3 styled input`: only asserts radius, border, and height classes; it does not protect input attributes or behaviour.
- `spec/components/m3/card_spec.rb:6-27`: the elevated, outlined, and filled examples only assert colour/shadow/radius class mappings. Remove these style-only assertions unless a behavioural contract is added.
- `spec/components/m3/badge_spec.rb:15-38`: the filled, outlined, and tonal colour class mappings are style-only. Keep alias normalisation tests where they protect the public `outlined`/`tonal` API.
- `spec/components/person_medications/modal_spec.rb:18`, example `renders a token-driven modal shell for the medication workflow`: only checks visual token classes and absence of `bg-white`. Keep the dose payload and back-link tests.
- `spec/components/person_medications/modal_spec.rb:51-59`: retain the route, link text, and `data-turbo-frame` assertions, but remove the purely visual `not_to include('rounded-xl')` assertion.

## Consolidate

- The repeated currentColor SVG examples at `spec/components/icons/inventory_spec.rb:18`, `hand_package_spec.rb:19`, `medication_spec.rb:15`, and `passkey_spec.rb:20` can have one survivor, preferably the inventory example, covering the shared SVG contract. The other icon-specific path data remains decorative.
- `spec/components/ruby_ui/div_wrapper_spec.rb:24` generates one exact default-class assertion per wrapper, including flex and spacing maps. Consolidate to one representative wrapper merge test plus the existing fallback example at line 35. Preserve custom-class merging, wrapper content, and the div tag; do not remove all wrapper coverage because these are local wrappers.
- `spec/components/ruby_ui/action_style_helpers_spec.rb:33-54` should be treated as one helper contract rather than many exact class-string assertions. Defer removal until focus, disabled, and `aria-disabled` class hooks have a meaningful survivor elsewhere; those accessibility semantics must remain protected.
- `spec/components/m3/button_spec.rb:6-47` can consolidate repeated base-class assertions while retaining each public variant's observable behaviour. Remove only colour/shape class snapshots if a semantic variant contract remains.

## Defer

- `spec/components/ruby_ui/badge_spec.rb:8`, generated examples `includes min-h-[24px] and min-w-[24px] for :sm/:md/:lg size`: minimum target size is an accessibility contract. Defer removal until a survivor asserts meaningful geometry or an equivalent rendered target-size rule.
- `spec/components/ruby_ui/action_style_helpers_spec.rb:33-54`: defer until focus-visible, disabled, and `aria-disabled` semantics are covered without relying on the current full class strings.

## Keep

- `spec/components/barcode_scanner_spec.rb:10-64`: controller and target hooks, hidden initial state, live-region semantics, manual fallback, and custom/default barcode formats are runtime integration contracts.
- `spec/components/global_search/palette_spec.rb:10-49`: labelled icon hiding, polite live-region semantics, zero-query rendering, search URL, and `Current` isolation are accessibility, state, and performance contracts.
- `spec/components/modal_spec.rb:12-22`: native dialog/RubyUI controller contract and title/subtitle/body content.
- `spec/components/person_medications/modal_spec.rb:35-49`: dose-options payload consumed by Stimulus.
- M3 select and selectable-option examples covering native form elements, required/disabled attributes, caller data/action merging, checked state, radio/checkbox semantics, and hidden blank fields.
- RubyUI link, dropdown, breadcrumb ARIA/current-page, alert role, alias normalisation, custom attributes, disabled state, OTP labelled input, accordion trigger/content linkage, and dialog/sheet close, focus, and cancellation behaviour.
