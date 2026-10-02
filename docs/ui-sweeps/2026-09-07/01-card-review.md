# Independent card-containment review

Reviewed shared working-tree diff and `01-card-report.md` on 7 September. This reviewer did not
run a test, server, or browser command. The report records one genuine 320px light baseline failure
for the person-medication card, but the current focused example remains red while opening the
schedule Actions menu. It is therefore not ready for acceptance.

## Requirements verdict: changes requested

The intended implementation is narrow: a larger footer inset plus stacked, full-width narrow
actions on the schedule and person-medication cards, returning to the existing horizontal layout at
`sm`. It leaves dose actions, authorisation, and menu items alone. The recorded baseline is useful:
the person-medication action row and controls reached the rounded card perimeter at 320px.

Two required acceptance conditions are still unmet:

1. The browser regression is red. The menu DOM node exists but is not visible after the schedule
   trigger is clicked. Resolve whether that is a product regression or an invalid test interaction,
   then rerun the complete 320/390 light/dark matrix. Existing schedule and person-medication menu
   tests establish the expected interaction and can supply the known-good open/escape pattern.
2. The present baseline only proves that rectangular action/control boxes touch the mathematical
   rounded boundary. It does not prove that painted text, the pill surface, or a focus outline was
   clipped. Capture and retain the required local before/after screenshots at 320px and 390px in
   both themes, including a focused action and an open Actions menu. Record the original failing
   predicate and the matching green output.

The final check should cover rendered action-label ranges contained by their controls and the card,
focus remaining visible for each interactive action, no page overflow, and a visible on-screen menu
with usable items. It should not claim that an unpainted rectangular corner is content clipping.

## Code-quality verdict: changes requested

The product code is proportionate. The new responsive classes are confined to the two owned card
surfaces and use the existing RubyUI dropdowns. The bare `CardFooter(...)` call on the
person-medication card is a necessary delivery correction for the supplied inset attributes.

The 216-line system-spec addition is not proportionate yet. `controlsWithinRoundedCard` and
`actionWithinRoundedCard` apply the rounded-card calculation to whole rectangular controls. A pill
button's bounding-box corners are transparent, so those predicates can fail even when neither its
painted surface nor its label is clipped; they can force needless footer inset. Retain only a small
geometry helper for the evidence that actually failed: text range containment, control internal
overflow, card/page bounds, and opened-menu viewport containment. Remove whole-control and
whole-action rounded-corner predicates unless a screenshot shows a painted edge or focus ring at
that exact point. The title predicates should also leave this action-focused regression unless a
recorded baseline failure requires them.

Do not merge the stack/full-width layout solely to satisfy the current box-corner assertions. Once
the focused browser example is green with the reduced, content-based predicate and the screenshots
show focused controls clear of the curved card edge, the implementation meets the brief's narrow
scope.

## Corrected-diff re-review

**Requirements verdict: changes requested.** The corrected system regression is now evidence-based:
it uses natural pointer activation, checks rendered text ranges and control content rather than
transparent pill corners, verifies visible keyboard focus and focus restoration, and records a
green seven-example matrix. The menu fix is appropriately card-local: removing the transformed
hover state restores a fixed-position menu that the card no longer clips. The coordinator also
visually confirmed the 320px light person-medication result.

**Code-quality verdict: changes requested.** The browser test and product diff are now proportionate
to the confirmed behaviour. One relevant component regression remains stale:
`spec/components/schedules/card/actions_component_spec.rb` still requires `px-6` and a one-line
action row, although the component now renders `px-8 flex-col sm:flex-row`. That file was absent
from the reported green command. Update its assertions to the responsive contract and run it with
`spec/components/person_medications/card_spec.rb` and the focused overflow spec. Acceptance follows
when those three relevant specs are recorded green; no further implementation change is requested.

## Final verification re-review

**Requirements verdict: accepted.** The writer recorded the isolated focused command over both
card component specs and `mobile_overflow_spec.rb`: 27 examples, 0 failures. It exercises the
320, 390, 768, and 1280 viewport matrix in light and dark appearances. The regression preserves
natural pointer menu activation, keyboard-visible focus, Escape focus restoration, rendered-label
containment, page overflow, and on-screen menu items. The selected screenshots truthfully label
the pre-fix 320px artifact as a failure and show the fixed mobile menus plus closed-card mobile and
desktop captures. The coordinator independently inspected the fixed 320px person-medication menu.

**Code-quality verdict: accepted.** Both relevant component specs now assert the delivered
responsive footer/action contract, and the system test keeps the content-based checks that detected
the real interaction failure. The card-local removal of the hover transform is proportional to the
fixed-position-menu clipping mechanism. No further change is requested.

## Screenshot provenance correction

The prior final review overstated the selected screenshot provenance. Hash and image-content review
found that the file first selected as a `before` capture was an already-stacked intermediate state,
not the original one-line layout. It has been renamed `card-intermediate-320-light-failure.png`; the
earlier capture is retained as `card-initial-320-light-failure.png`. Neither is a close visual
before/after counterpart of the accepted fixed-menu captures. The acceptance rests on the recorded
browser regressions, focused component coverage, and the coordinator's live inspection; the images
remain truthful supporting artifacts, not paired proof of the original visual state.
