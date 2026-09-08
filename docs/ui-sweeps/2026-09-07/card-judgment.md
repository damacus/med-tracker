# Card menu judgment

Route: `team-tranche-development`

Approval: `bucky_sol_xhigh`

Escalation: `same_nightingale_no_scope_broadening`

## Decision

Treat the current schedule-menu failure as a real pointer-interaction defect exposed by the browser
test, rather than weakening the menu assertion. The focused Docker example reproduces the failure at
320 pixels: the schedule menu node remains in the DOM after the click, but it is not visible.

The active schedule and person-medication card shells combine `hover:scale-[1.02]` with
`overflow-hidden`. Their dropdowns use `position: fixed` descendants and Floating UI viewport
coordinates. A transformed ancestor establishes the containing block for a fixed descendant, while
the card's overflow clips that descendant. This explains why the Playwright pointer click can leave
the menu invisible. A local accessibility activation at the same viewport opens the menu normally;
that path leaves the card's computed transform as `none`, so it does not disprove the pointer-only
failure.

The smallest safe correction is for the same Nightingale to remove only `hover:scale-[1.02]` from
the active schedule and person-medication card classes. Keep their hover shadow and keep the shared
RubyUI dropdown unchanged. This stays within the card brief's owned product paths and avoids a shared
component scope expansion.

## Test judgment

Keep the visible-menu and menu-item viewport checks. Keep checks for action-label ranges within each
control and card, control internal overflow, card/page bounds, keyboard focus visibility, and page
overflow. Do not replace the failing click with JavaScript activation or a forced click.

Remove `actionWithinRoundedCard` and `controlsWithinRoundedCard`. They compare rectangular element
corners with the card's curved boundary, although pill-button bounding-box corners are not painted.
Those predicates can fail without clipped text, surface, or focus outline. Remove the title geometry
from this action regression unless retained red evidence ties it to the supplied defect. The required
screenshots at 320 and 390 pixels in both themes must show the focused controls and open menus clear
of the card edge.

After the card-local hover correction and test reduction, rerun the complete 320, 390, and 1280
matrix in both themes. The 1280 run is required because `sm:flex-row` can still produce narrow grid
cards at a desktop viewport. Nightingale retains sole production and test write ownership.

## Evidence

The focused command reproduced one failure at the schedule menu visibility assertion:

```text
rtk proxy env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml \
  task test TEST_FILE=spec/system/mobile_overflow_spec.rb:104
```

Result: 1 example, 1 failure at `spec/system/mobile_overflow_spec.rb:157`. The matched menu node was
not visible. The existing card report records that the preceding 320-pixel geometry assertions had
passed. No product or test files were changed during this judgment.
