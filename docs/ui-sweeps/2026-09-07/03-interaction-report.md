# Interaction tranche report

Issue #2123. This tranche covers the four confirmed interaction and authorization defects from
the 03 brief. Canary was not used.

## Red evidence

Before the 03 production changes, the focused command ran in the cached Docker overlay:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/components/locations/show_view_spec.rb spec/components/views/profiles/show_spec.rb spec/services/admin/dashboard_metrics_query_spec.rb spec/system/admin_dashboard_spec.rb spec/system/user_sessions_spec.rb"
```

Result: **50 examples, 5 failures**. The failures were the hover-only location removal control,
missing profile tab wrapping classes, a query capability argument, household-manager visibility of
the dm+d import action, and the missing inline auth alert assertion. No 03 production change was
made before this red run.

## Implementation

- Location member removal keeps its accessible name, dialog and focus return while removing the
  hover-only opacity classes.
- Profile tabs wrap at narrow widths, break long labels at enlarged text sizes, and retain tab
  semantics. Button tabs now move focus and selection with ArrowLeft, ArrowRight, Home and End;
  server-navigation anchor tabs retain their href behaviour. Unavailable button tabs are skipped.
- The dashboard controller is the policy boundary. It passes the existing dm+d capability into the
  query and view; both default to fail closed, so omitted capability cannot advertise import.
- Auth inline alerts set a view marker only when an alert is rendered. The global notice stack is
  suppressed for that response, while the login-required fallback remains global. The Alert
  component's existing role is retained without creating a nested or duplicated live region.
- Medication finder result metadata and source labels now wrap inside their narrow card and header
  rails. The candidate was promoted only after the 320px geometry assertion reproduced 105px of
  page overflow.

## Green evidence

The complete bounded command passed:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/components/locations/show_view_spec.rb spec/components/views/profiles/show_spec.rb spec/services/admin/dashboard_metrics_query_spec.rb spec/system/admin_dashboard_spec.rb spec/system/user_sessions_spec.rb spec/system/locations/member_removal_spec.rb spec/system/profile_tabs_spec.rb spec/requests/login_layout_spec.rb spec/components/views/rodauth/login_spec.rb spec/components/admin/dashboard/index_view_spec.rb"
```

Result: **86 examples, 0 failures**. This includes the 390px location keyboard activation and
 cancel focus return, 320px profile tabs with 200% text and ArrowRight selection, household-manager
 and platform-admin dashboard visibility, invalid-login and logout exact alert counts, login-required
 global fallback, and reset-password POST inline feedback without a global duplicate.

The profile browser example was rerun using the repository's native Playwright `:right` key mapping
after the full bounded run: **1 example, 0 failures**.

The server-navigation review-filter regression was red with the generic keyboard action: after
ArrowRight, the current link became `data-state="inactive"` while the URL stayed unchanged. The
same browser example passed after anchor tabs stopped receiving the local `navigate` action. The
original 03 red command established profile wrapping but did not exercise keyboard behaviour; that
keyboard failure was source-confirmed from the prior click-only action and is now browser-green.

The finder candidate red run reported **105px** horizontal overflow at 320px for an unbroken source
label. After the contained metadata/header fix, the candidate passed. The affected browser and
geometry checks passed together as **10 examples, 0 failures**:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/system/medication_review_filters_spec.rb spec/system/profile_tabs_spec.rb spec/system/medication_finder_spec.rb:85 spec/system/mobile_overflow_spec.rb"
```

The existing direct-route authorization coverage also passed:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE="spec/requests/admin/nhs_dmd_imports_spec.rb"
```

Result: **11 examples, 0 failures**, including household-admin denial of the global dm+d import
form and non-admin denial.

## Evidence limits and screenshots

Browser tests ran through the isolated `task test TEST_FILE=...` path with the cached runtime and
Playwright browser overlay described in [local-browser-evidence.md](local-browser-evidence.md).
The red browser run produced failure artifacts under the container's `/app/tmp/capybara` path;
the fixed run passed without failure screenshots. Existing baseline and card screenshots remain
under `docs/screenshots/ui-sweep/`; the remaining route matrix will capture any additional 03
before/after route screenshots before publication. The finder change was backed by the failing
geometry assertion above; no mobile-rail or invitation production changes were made without a new
failing geometry assertion.

## Remaining scope

The broad route audit and light/dark screenshot matrix are the next bounded verification step after
this correction review. The separate dashboard headline issue is tracked as #2125 and is outside
this tranche.
