# Admin/layout/view component test-value classification

Scope scanned: all 42 files under `spec/components/admin/`, `spec/components/layouts/`, and `spec/components/views/` (3,017 lines), plus matching admin, navigation, profile, authentication, accessibility, and responsive request/system/feature specs. This is evidence-only classification; no tests were run and no spec or product files were edited.

## Remove

These examples assert implementation-specific decoration or markup with no user-visible behaviour that is not covered elsewhere.

- `spec/components/layouts/navigation_spec.rb:22-30`, `brand readability / uses foreground text color utility for the brand link`: remove. It only requires the literal `text-foreground` class on a link. The link text/route and signed-out navigation are covered by the same file’s i18n example and by `spec/system/navigation_spec.rb`; the exact colour token is decorative.
- `spec/components/admin/dashboard/index_view_spec.rb:8-25`, `renders the active schedules metric with the active schedules icon`: remove the SVG `path[d=...]` assertion. The metric rendering/state is covered by the dashboard status, attention, metrics, and request/system dashboard specs; asserting the complete path data couples the test to an icon drawing.
- `spec/components/admin/dashboard/index_view_spec.rb:102-112`, `renders recent activity rows with stable card hover styling`: remove. The test checks `rounded-xl`, `border`, `shadow-sm`, `hover:shadow-md`, and absence of another hover token. Recent activity presence/content and audit-log navigation are covered at `:91-100` and by `spec/features/admin/audit_logs_spec.rb`; hover classes are decorative.
- `spec/components/admin/users/users_table_spec.rb:73-80`, `renders edit links with outline button styling`: remove the `class` assertion. Keep the href assertion if this example is retained, or rely on the canonical row-selector/edit href assertion at `:47-55`; `border` does not prove the edit action works.
- The token-ban-only examples in `spec/components/views/profiles/danger_zone_card_spec.rb:6-20`, `account_security_card_spec.rb:10-17`, `version_info_spec.rb:38-45`, `experiments_card_spec.rb:10-18`, and `show_spec.rb:32-40`: remove the `banned_classes` checks. They only forbid particular gradients, radii, shadows, or literal fills. Functional/profile coverage remains in `spec/components/views/profiles/show_spec.rb:42-145`, `spec/requests/profiles_show_spec.rb:22-42,345-406`, `spec/system/profile_editing_spec.rb:88-227`, and `spec/system/appearance_mode_spec.rb:33-57`. For the cards whose dedicated specs contain behaviour, retain those behaviour examples.

## Consolidate

- `spec/components/layouts/mobile_menu_spec.rb:15-21` (`renders the mobile wrapper and trigger`) already asserts `button[aria-label="Open menu"]`; `:54-60` repeats the same labelled-trigger assertion under `accessibility`. Merge the accessibility assertion into the rendering example or keep the accessibility example and narrow the rendering example to wrapper presence. Retain the distinct sheet name, close-button, focus-ring, and touch-target checks at `:80-110`.
- `spec/components/layouts/flash_spec.rb:6-10` (`renders messages for the layout notification region`) overlaps the notice message example at `:12-18`. Keep one example that asserts the notification region and exact message/no redundant title; retain the warning/alert variant, dismissal action, and icon behaviour examples.
- `spec/components/admin/users/pagination_spec.rb:36-41` (`renders page number links`) only checks that the pagination nav exists. This is subsumed by the accessibility example at `:136-150` and the previous/next/page-state examples at `:43-97`; fold any needed nav-label assertion into the surviving accessibility example and remove the tautological example.
- The six profile token-ban checks listed in Remove should be consolidated only if the team still wants a design-system migration guard: one explicit profile-surface policy example with a named contract is enough. Do not replace them with more exact class-token assertions; the profile interaction/state tests listed above are the surviving coverage.

## Keep

These assertions protect behaviour, accessibility, permissions, localisation, data boundaries, or a responsive contract rather than arbitrary presentation.

- `spec/components/layouts/mobile_menu_spec.rb:62-78`: keep dark-mode-safe contrast/focus classes and `currentColor` hamburger lines. These protect readable controls in both themes and avoid hard-coded black; the test is accessibility/contrast intent, not a cosmetic colour preference.
- `spec/components/layouts/mobile_menu_spec.rb:90-110`: keep dialog naming, close-button accessible name, hidden icon, focus ring, and 44px touch target.
- `spec/components/layouts/sidebar_spec.rb:54-61,72-91`: keep active-state semantics, avatar presence, admin-only navigation, and unauthenticated omission. The class checks here are the state’s selected-surface contract and sit beside route/role behaviour.
- `spec/components/layouts/mobile_rail_spec.rb:31-92`: keep quick-link order/routes, shortcut filtering, unauthenticated omission, icon hiding, and `aria-current`. These protect navigation behaviour and accessibility.
- `spec/components/layouts/navigation_spec.rb:10-20,33-61`: keep translated signed-out labels, skip link, labelled search icon, preloaded shell context, route generation, zero household queries, and `Current` immutability.
- `spec/components/layouts/navigation_surface_spec.rb:6-14`: keep for now as a deliberate dark/light shell-surface contract; it is more than an arbitrary colour check because it rejects transparent/blurred mobile chrome and requires a token-driven solid surface. Revisit only with visual evidence and an agreed design contract.
- `spec/components/layouts/auth_layout_spec.rb:6-31`: keep font preload attributes and the no-global-flash boundary; these are loading and composition contracts.
- `spec/components/layouts/flash_spec.rb:27-63`: keep warning variant, dismiss action, alert/notice content, and icon presence; these affect message meaning and interaction.
- `spec/components/admin/users/search_form_spec.rb`, `users_table_spec.rb` query-count examples, `pagination_spec.rb` state/link examples, and all admin audit-log/dashboard/relationship component specs: keep role/status selection, filtering, empty/populated states, URLs, pagination state, eager-loading/query boundaries, audit data, and admin workflow semantics.
- `spec/components/admin/carer_relationships/form_view_spec.rb:12-25`: keep. The explicit surface text tokens are documented as dark-mode legibility/contrast intent; do not classify them as arbitrary colour-token tests.
- Profile responsive/accessibility assertions such as `spec/components/views/profiles/show_spec.rb:51-78` and the matching `spec/system/profile_tabs_spec.rb` / `spec/system/enlarged_text_spec.rb`: keep. They protect tab semantics, wrapping, keyboard use, and enlarged-text containment.
- Rodauth component specs under `spec/components/views/rodauth/`: keep form actions, WebAuthn/OTP/recovery semantics, labels, hidden icons, and authentication state. These are signed-out/authentication contracts.

## Defer

- `spec/components/views/profiles/theme_picker_card_spec.rb:26-33` (`renders palette options in a responsive grid`) asserts a full utility-class selector. The count and `data-theme`/toggle semantics are valuable, but whether the exact grid breakpoints are a product requirement needs visual/design evidence. If retained, prefer a behavioural geometry assertion in the browser suite; if not required, remove only the exact class selector while keeping theme option semantics.
- `spec/components/admin/users/pagination_spec.rb:43-50,84-90` checks `cursor-not-allowed`. It may be a meaningful disabled affordance, but the component should ideally expose disabled semantics/absence of links as the contract. Defer until the browser/accessibility expectation is confirmed.
- `spec/components/views/profiles/notifications_card_spec.rb:61-72` mixes meaningful touch-target/contrast protection with exact radius and banned white/rounded classes. Keep the touch-target and contrast intent; defer splitting/removing the radius assertions until the shared button contract is identified.

## Coverage boundary

The matching behaviour suite already protects the high-value surfaces: admin workflow requests and system journeys (`spec/requests/admin_*`, `spec/system/admin_*`), navigation (`spec/system/navigation_spec.rb`, `navigation_flow_spec.rb`, `mobile_navigation_spec.rb`), profile interactions (`spec/system/profile_editing_spec.rb`, `profile_tabs_spec.rb`, `appearance_mode_spec.rb`, `enlarged_text_spec.rb`), authentication (`spec/features/authentication*`, Rodauth system/features), and mobile overflow/contrast (`spec/system/mobile_overflow_spec.rb`, `mobile_ui_audit_spec.rb`). The proposed removals therefore target duplicate or decorative assertions only; no permissions, i18n, route, state, accessibility, or data-query coverage should be removed.
