# Test pruning record

This pruning pass is test-only. It removes assertions classified as decorative or
duplicate in the 2026-09-08 audit, while retaining observable content, routes,
roles, state, policy, accessibility, touch-target, translation, stock, timing,
and overflow contracts.

## Counts

- 12 complete obsolete spec files removed.
- 78 deleted `it` declarations are paired with 7 renamed surviving examples;
  the net reduction is 71 RSpec examples.
- 156 deleted `expect` lines and 12 deleted `let` declarations are part of
  those removed examples or their now-unused setup.
- 39 spec files were edited in place to drop only the approved decorative
  assertions; 51 Ruby spec paths are changed in total.
- `git diff --check` passes. The Rails suite and RuboCop are owned by the
  parent task and remain pending at this checkpoint.

## Complete file removals

These files contained only the approved low-value contracts:

- `spec/helpers/application_helper_spec.rb`
- `spec/services/ai_medication/source_page_spec.rb`
- `spec/components/m3/card_spec.rb`
- `spec/components/m3/badge_spec.rb`
- `spec/components/medications/administration_modal_spec.rb`
- `spec/components/medications/wizard/step_dose_schedule_spec.rb`
- `spec/components/ruby_ui/dialog_header_spec.rb`
- `spec/components/ruby_ui/dialog_middle_spec.rb`
- `spec/components/ruby_ui/select_item_spec.rb`
- `spec/components/ruby_ui/select_trigger_spec.rb`
- `spec/components/views/profiles/account_security_card_spec.rb`
- `spec/components/views/profiles/danger_zone_card_spec.rb`

## Trimmed specs and survivor evidence

- Admin/layout cleanup is in `spec/components/admin/dashboard/index_view_spec.rb`,
  `spec/components/admin/users/pagination_spec.rb`,
  `spec/components/admin/users/users_table_spec.rb`,
  `spec/components/layouts/flash_spec.rb`,
  `spec/components/layouts/mobile_menu_spec.rb`, and
  `spec/components/layouts/navigation_spec.rb`. Dashboard content/order,
  pagination state and accessible navigation, edit URLs, flash roles/messages/
  dismissal, menu naming/focus/touch targets, translated navigation, labelled
  controls, route generation, query boundaries, and `Current` immutability
  remain covered.
- Decorative icon paths and repeated numeric SVG sizing were removed from
  `spec/components/icons/{chevron_right,hand_package,inventory,medication,passkey}_spec.rb`
  and `spec/components/ruby_ui/{breadcrumb_ellipsis,breadcrumb_separator}_spec.rb`.
  Custom icon blocks and semantic/custom-attribute coverage remain.
- Primitive style-only assertions were trimmed from
  `spec/components/m3/button_spec.rb`,
  `spec/components/ruby_ui/{alert,alert_dialog_content,badge,dialog_content,sheet_content}_spec.rb`,
  and `spec/components/person_medications/modal_spec.rb`. Remaining checks
  cover labelled rendering and minimum touch target, alert role/content, size
  and alias contracts, modal naming/ARIA/native cancellation/focus hooks,
  duplicate IDs, dose payloads, routes, and Turbo-frame state.
- Domain decoration was trimmed from
  `spec/components/dashboard/schedule_card_spec.rb`,
  `spec/components/dashboard/stat_card_spec.rb`,
  `spec/components/dashboard/timeline_item_spec.rb`,
  `spec/components/locations/index_view_spec.rb`,
  `spec/components/medications/form_view_spec.rb`,
  `spec/components/medications/index_view_spec.rb`,
  `spec/components/medications/list_item_component_spec.rb`,
  `spec/components/medications/show_view_spec.rb`,
  `spec/components/people/add_medication_landing_spec.rb`,
  `spec/components/people/person_card_spec.rb`,
  `spec/components/reports/export_panel_spec.rb`,
  `spec/components/schedules/index_view_spec.rb`, and
  `spec/components/shared/metric_card_spec.rb`.
  The survivors retain medication and schedule content, action presence and
  URLs, policy visibility, modal/frame contracts, responsive wrapping where it
  protects known narrow-screen reachability, minimum touch targets, stock
  meters, table content, export scope, and labelled icon hiding.
- `spec/controllers/locations_controller_spec.rb` drops only the inherited
  `ApplicationController` example. `spec/services/medication_finder_search_responder_spec.rb`
  drops only the `Data` body/status default example while retaining responder
  behaviour. `spec/presenters/medications/supply_status_presenter_spec.rb`
  drops only the six stock/list class examples; all six list supply-bar
  threshold examples, status, units, forecast, and reorder contracts remain.
- `spec/components/views/profiles/{experiments_card,show,version_info}_spec.rb`
  retain dashboard/launcher semantics, profile content/state, and development
  metadata after token-ban-only assertions were removed.

The pass deliberately leaves the deferred wrapper/helper, dashboard responsive
grid, stock-meter, action-grid, currentColor icon, and M3 input coverage in
place. No production files, fixtures, configuration, or system specs were
changed.
