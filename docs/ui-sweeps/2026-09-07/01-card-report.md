# Card containment tranche report

Issue: #2123

## Scope

The regression covers the schedule and person-medication cards rendered on the
person page. The browser example uses the authorised admin fixture, replaces the
two medication names with long labels, and checks both cards at 320, 390, 768,
and 1280 pixels in light and dark appearance modes.

For each card it measures the action row, controls, and their rendered text
ranges against the card bounds, checks action controls for internal horizontal
clipping, verifies keyboard-visible focus, and opens the Actions menu to check
the menu and each menu item against the viewport. This replaces a source-level
flex-wrap assumption with observable browser geometry.

## Implementation

Added the browser regression and its DOM geometry helpers to
`spec/system/mobile_overflow_spec.rb`. The current product diff is limited to
the two owned card surfaces:

- both card footers use `px-8 pb-8 pt-2` so controls clear the rounded shell;
- both action rows stack controls below `lg`, with full-width narrow controls,
  and return to a horizontal row at `lg` and above;
- the person-medication card uses the bare `CardFooter(...)` invocation so the
  supplied inset attributes reach the rendered footer.
- both active card shells retain their hover shadow but no longer scale under
  the pointer, so fixed-position dropdowns are not trapped by a transformed,
  clipped ancestor.

Existing comments were left unchanged.

## Verification

Parent environment preflight: passed, 31 examples.

Focused command (using the isolated test wrapper and temporary runtime
overlay):

```text
set -lx COMPOSE_FILE compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml
task test TEST_FILE="spec/components/person_medications/card_spec.rb spec/components/schedules/card/actions_component_spec.rb spec/system/mobile_overflow_spec.rb"
```

The first run reached RSpec but every browser example stopped during fixture
login because the test container did not contain
`/ms-playwright/chromium_headless_shell-1234/chrome-linux/headless_shell`.
Chromium was then restored by the coordinator in the temporary test runtime.

The genuine baseline geometry failure at 320px light mode was the
person-medication card: `actionWithinRoundedCard=false` and
`controlsWithinRoundedCard=false`; its action bounds were `[16, 2335.25, 304,
2431.25]` inside card bounds `[16, 1770.5, 304, 2431.25]`. The schedule card
passed the rounded checks at the same viewport.

After the footer inset, the schedule card exposed internal clipping:
`controlContentContained=false` and `controlLabelWithinControl=false` at 320px
light mode. The stacked narrow layout made all card geometry predicates pass at
320px light mode, but the next assertion failed when opening the schedule
Actions menu: `schedule-actions-menu-995299929` existed but was not visible
after clicking its trigger. The Sol judgment treated this as a pointer
interaction defect and approved removing the hover scale from the two card
shells.

The regression was then reduced to card/page bounds, action-control overflow,
actual text-range containment, keyboard-visible focus, natural pointer menu
activation, menu-item viewport containment, and focus restoration after Escape.
The rounded rectangle-corner predicates and unrelated title checks were removed
because they did not prove painted content clipping.

The green focused run used the same `task test TEST_FILE=...` wrapper with the
temporary `COMPOSE_FILE` overlay (the repository's isolated test boundary; the
separate `task playwright` route uses the shared local database). Result: 27
examples, 0 failures, including both component specs and the system example
across 320, 390, 768, and 1280 pixels in both themes. The 768px pass is the
tablet stress case that keeps the action controls readable inside narrower grid
cards.

`git diff --check` passes on the current diff. Context7 was checked for the
current Playwright Ruby `evaluate` and screenshot APIs. The browser failures
generated local artifacts under `/app/tmp/capybara/`, including the genuine
320px geometry failure and the menu-visibility failure. The coordinator checked image content
and hashes: the initially selected `_459.png` was an intermediate failure with stacked actions,
so it is retained as `card-intermediate-320-light-failure.png`, not as an original baseline.
The earlier `_320.png` is retained as `card-initial-320-light-failure.png`; it shows the initial
single-row schedule actions. Neither is a matching close-up of the reported person-medication card.
The green run generated fixed screenshots for both cards at 320/390 in both
themes and closed-card page captures at 390/1280:

```text
tmp/capybara/ui-sweep-card-fixed-schedule-320-light-menu.png
tmp/capybara/ui-sweep-card-fixed-schedule-320-dark-menu.png
tmp/capybara/ui-sweep-card-fixed-schedule-390-light-menu.png
tmp/capybara/ui-sweep-card-fixed-schedule-390-dark-menu.png
tmp/capybara/ui-sweep-card-fixed-person-medication-320-light-menu.png
tmp/capybara/ui-sweep-card-fixed-person-medication-320-dark-menu.png
tmp/capybara/ui-sweep-card-fixed-person-medication-390-light-menu.png
tmp/capybara/ui-sweep-card-fixed-person-medication-390-dark-menu.png
tmp/capybara/ui-sweep-card-fixed-page-390-light-closed.png
tmp/capybara/ui-sweep-card-fixed-page-390-dark-closed.png
tmp/capybara/ui-sweep-card-fixed-page-1280-light-closed.png
tmp/capybara/ui-sweep-card-fixed-page-1280-dark-closed.png
```

The initial and intermediate screenshots remain labelled as failure artifacts and are not
presented as matching before/after visual comparisons. The fixed screenshots show a focused Actions
control with its open menu and visible menu items; the closed-card captures
cover both mobile and desktop widths in light and dark modes. The selected
artifacts copied for review are:

```text
docs/screenshots/ui-sweep/card-initial-320-light-failure.png
docs/screenshots/ui-sweep/card-intermediate-320-light-failure.png
docs/screenshots/ui-sweep/card-after-schedule-320-light-menu.png
docs/screenshots/ui-sweep/card-after-person-medication-320-light-menu.png
docs/screenshots/ui-sweep/card-after-390-light-closed.png
docs/screenshots/ui-sweep/card-after-390-dark-closed.png
docs/screenshots/ui-sweep/card-after-1280-light-closed.png
docs/screenshots/ui-sweep/card-after-1280-dark-closed.png
```

No Canary evidence is used.

## Next safe action

Obtain independent Hubble re-review of the corrective diff and evidence, then
hand off to the coordinator for acceptance. Canary remains excluded.
