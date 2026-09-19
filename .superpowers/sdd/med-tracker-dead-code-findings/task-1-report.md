# Task 1 report: confirmed orphaned UI components

## Status

DONE

Implementation commit: `0e0dce7 refactor(ui): remove confirmed dead components`

## Implementation

Removed the confirmed unreachable UI code, its test-only specifications, and the six stale non-browser timing entries. The change contains 2,675 deletions across 56 files. It does not alter locale files, live data, routes, models, controllers, or current UI behaviour.

The shared wizard helper module lost only these nine unreachable private methods:

- `render_dose_amount_field`
- `render_dose_unit_field`
- `render_primary_dosage_option_fields`
- `render_primary_dose_amount_field`
- `render_primary_dose_unit_field`
- `render_primary_frequency_field`
- `render_primary_dosage_default_number_field`
- `render_primary_dosage_default_cycle_field`
- `render_primary_dosage_default_checkbox`

The retained `Metrics/ModuleLength` comment was left unchanged, as required. Live helpers including `render_current_supply_field`, `render_reorder_threshold_field`, `dose_units`, `primary_dosage_record_for_wizard`, `hidden_primary_dosage_field`, and `render_frequency_template_buttons` remain.

## Per-finding dispositions and reference evidence

All findings were present at base `aef2eac764ff7b9fb5e60904899a605e7086eb9a` and were removed. None was already absent, retained as live, or deferred.

1. **Finding 1 — removed.** Deleted the nine-file legacy dashboard schedule/stats subtree and its twelve component/i18n specs. Repo-wide qualified and bare-name searches found production references only within the dead subtree; the remaining references were its own specs. Current rendering remains rooted in `DashboardPresenter`, `IndexView`, `TimeFirstView`, `FamilyLanesView`, and `CalmFocusView`. `Components::Shared::MetricCard`, `PersonTaskCard`, and the Rails `Schedule` model remain live.
2. **Finding 2 — removed.** Deleted `Components::Dosages::Modal` and `Components::Dosages::Form`. Searches found only the internal modal-to-form edge and the definitions. There is no dosage controller/resource route; the `Dosage` model and medication dosage APIs remain.
3. **Finding 8 — removed.** Deleted all six breadcrumb primitives, their six specs, and their six timing entries. Searches for qualified constants, bare Phlex-kit calls, and lowercase breadcrumb wiring found no production consumer. `MoreHorizontal` was not removed.
4. **Finding 14 — removed.** Deleted `RubyUI::AlertDialogAction`. Word-boundary and qualified searches found only its definition. The live alert-dialog siblings remain.
5. **Finding 15 — removed.** Deleted the three RubyUI avatar primitives. Qualified, bare, and dynamic-construction searches found no consumer. The separate live profile-avatar service, controller, model behaviour, and `Components::Shared::PersonAvatar` remain and passed focused specs.
6. **Finding 16 — removed.** Deleted `ComboboxCheckbox`, `ComboboxToggleAllCheckbox`, and `ComboboxListGroup`. Searches covered qualified constants, bare kit calls, and Stimulus target emission. The single-select combobox components and controller remain live. The unused multi-select controller handlers were left for a separate decision, as required.
7. **Finding 18 — removed.** Deleted `Components::Icons::Globe`; no qualified or bare component reference remained outside its definition.
8. **Finding 19 — removed.** Deleted `Components::Icons::Fingerprint`; no qualified or bare component reference remained outside its definition.
9. **Finding 20 — removed.** Deleted `Components::Icons::Compliance`; no qualified component reference remained, and unrelated compliance copy was ignored.
10. **Finding 21 — removed.** Deleted `Components::Icons::Menu`; no Ruby component reference remained. The unrelated Android Compose `Icons.Default.Menu` import is live and untouched.
11. **Finding 22 — removed.** Deleted `Components::NotificationSettings::FormView`. Searches for qualified, partially-qualified, path-string, controller, and route references found none. The live profile notifications card and `notification_settings.categories.*` usage remain and passed its component spec.
12. **Finding 23 — removed.** Deleted `StepDosageSupply` and the nine private methods listed above. The step name occurred only in its definition, and each removed helper was either called only by that step/cluster or had no caller. Current `StepDoseSchedule` and `StepSupply` flows and their shared helpers remain; the wizard request/component specs passed.
13. **Finding 27 — removed.** Deleted the three RubyUI popover primitives and `ruby-ui--popover` Stimulus controller. Searches for qualified/bare kit calls, `ruby-ui--popover`, and `ruby_ui__popover` found only the deleted family. `ComboboxPopover` and `DropdownMenu` are separate live paths and remain.
14. **Finding 28 — removed.** Deleted `InputOtpSeparator`. The live Rodauth view uses `InputOtp`, `InputOtpGroup`, and `InputOtpSlot`; focused OTP and two-factor view specs passed.
15. **Finding 29 — removed.** Deleted `people/new.turbo_stream.erb`. No explicit template reference exists; `PeopleController#new` renders via the current modal/page path. The people request spec, including Turbo Stream create/update behaviour, passed.

After deletion, a combined repo-wide search returned no matches for the removed constants, the nine helper names, `StepDosageSupply`, `notification_settings/form_view`, `ruby-ui--popover`, or the removed icon constants. `git diff --check` and `jq empty scripts/ci/non_browser_timings.json` both exited successfully.

## Candidate orphan translations for Task 4

No locale file was edited. These keys or subtrees lost their final known caller and should be checked by Task 4 across all locales:

- `dashboard.delete_confirmation.delete`, `delete_schedule`, and `are_you_sure`; retain `dashboard.delete_confirmation.cancel`, which is still called by `app/components/person_medications/card/actions_component.rb`.
- all `dashboard.person_schedule.*`
- `dashboard.quick_actions.title`; retain `dashboard.quick_actions.add_person` and `add_medication`, which current dashboard code still calls.
- all `dashboard.quick_stats.*`
- all `dashboard.schedule_card.*`
- `dosages.new.title` and `dosages.edit.title`
- `forms.medications.dosage_and_supply`
- `notification_settings.title`, `description`, `browser.*`, `reminders.*`, `period_labels.*`, and `actions.*`; retain `notification_settings.categories.*`, which the live profile notifications card calls.

## Verification

Preflight was completed by the controller before this implementation: 15 examples, 0 failures.

Focused command:

```text
rtk proxy task test TEST_FILE='spec/components/dashboard/index_view_spec.rb spec/presenters/dashboard_presenter_spec.rb spec/requests/dashboard_home_spec.rb spec/requests/dosages_wizard_spec.rb spec/components/medications/wizard/modal_wrapper_spec.rb spec/components/views/profiles/notifications_card_spec.rb spec/requests/people_spec.rb spec/components/ruby_ui/input_otp_spec.rb spec/components/views/rodauth/two_factor_auth_spec.rb spec/components/shared/person_avatar_spec.rb spec/requests/person_avatars_spec.rb'
```

Result:

```text
Finished in 9.43 seconds (files took 3.96 seconds to load)
132 examples, 0 failures
```

The output contained only the two controller-confirmed baseline startup warnings: Rodauth `already_logged_in` is unused, and `json_parser` lacks `content_type_regexp`. No new warning or failure appeared.

The controller owns the one-time full Rails suite and full RuboCop gate after all task batches, so neither was run here.

## Self-review

- Re-read the final diff and confirmed it contains only requested component/spec/controller/template deletions, the nine dead helper removals, and stale breadcrumb timing removal.
- Confirmed all fifteen findings are accounted for and all current/live exceptions named in the brief remain.
- Confirmed no locale edits, data/schema changes, route changes, API changes, dependency pruning, multi-select Stimulus cleanup, or retained-comment edits slipped into the batch.
- Confirmed the worktree began at the required base and branch, and the implementation commit used `Dan Webb <dan.webb@damacus.io>` with configured SSH signing enabled.
- Expected integration caveat: deleting vendored RubyUI primitives intentionally makes `task ruby-ui:compare` report those primitives as missing locally. This is the confirmed project-owned dead-code scope, not an unverified failure.

## Concerns

None. Full-suite and full-lint results remain the controller's final branch-level gate.
