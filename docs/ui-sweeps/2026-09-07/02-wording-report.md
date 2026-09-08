# Wording tranche report

Issue #2123. This tranche updates the accepted copy defects in the wording brief while preserving submitted enum values and existing permissions. The five locale trees remain structurally synchronised.

## Red evidence

Before production changes, the focused command below ran in the cached Docker overlay (`COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml`):

```text
task test TEST_FILE="spec/components/person_medications/card_spec.rb spec/components/locations/index_view_spec.rb spec/components/locations/show_view_spec.rb spec/components/medications/finder_view_spec.rb spec/requests/schedules_workflow_spec.rb spec/requests/admin/invitations_spec.rb spec/system/medications/stock_check_spec.rb spec/system/navigation_spec.rb"
```

Result: **47 examples, 8 failures**. The failures were the expected old timing, location count and stock-warning, finder package label, schedule workflow, invitation labels, stock-check plural, and auth-footer assertions. No wording-tranche production files were changed until this red run completed.

## Green evidence

After the bounded implementation, this focused command passed:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/components/person_medications/card_spec.rb spec/components/locations/index_view_spec.rb spec/components/locations/show_view_spec.rb spec/components/medications/finder_view_spec.rb spec/components/admin/users/search_form_spec.rb spec/components/admin/users/users_table_spec.rb spec/requests/admin/invitations_spec.rb spec/requests/admin_users_index_spec.rb spec/requests/schedules_workflow_spec.rb spec/system/medications/stock_check_spec.rb spec/system/navigation_spec.rb spec/system/mobile_overflow_spec.rb"
```

Result: **87 examples, 0 failures**. The stock-check coverage verifies the real disabled state for an incomplete one-item selection, then enabled one-item and two-item labels after valid quantities. Location count coverage uses isolated locations so fixture medications cannot mask the one/other branches.

The separately requested card action component check also passed:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/components/schedules/card/actions_component_spec.rb"
# 5 examples, 0 failures
```

## Review follow-up

The review follow-up added all five locale timing singular/plural assertions, invalid and missing cycle display normalisation, exact invitation option value/label mappings, and the plural stock-check label on a server validation-error rerender with two adjustments. The location count now delegates plural selection to the translation layer with `count:`. The affected regression command passed **36 examples, 0 failures**:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/requests/admin/invitations_spec.rb spec/requests/medications_stock_check_spec.rb spec/components/person_medications/card_spec.rb spec/components/locations/index_view_spec.rb"
```

The locale sync checker still passes across five files and `git diff --check` is clean.

Additional checks passed:

```text
/Users/damacus/.agents/skills/translate/scripts/check_locale_sync.sh config/locales/en.yml
# Locale trees are in sync across 5 files.
git diff --check
```

Browser examples ran through the isolated `task test TEST_FILE=...` browser specs in the local Docker environment. Canary was not used, and no non-English browser locale coverage is claimed; translated node structure and copy branches are covered by the locale sync gate and rendered component/request assertions.
