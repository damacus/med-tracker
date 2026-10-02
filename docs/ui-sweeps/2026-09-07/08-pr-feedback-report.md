# PR 2126 component and stacking feedback

The closed card Actions trigger was in the shared dropdown root's `z-50`
stacking context, above the mobile rail's `z-40`. The dropdown content retains
its own `z-50` overlay layer; only the root stacking class was removed.

Red evidence came from the focused mobile overflow browser spec after
populating long schedule and person-medication names and scrolling each trigger
under the fixed rail. The run had 10 examples and 1 failure: the schedule
trigger overlapped the rail at 390px (`top: 739.5`, `bottom: 783.5`, rail top
764), but the overlapping rail link's hit test returned `navigationHit: false`
because the dropdown root was above it. The same regression iterates both the
schedule and person-medication triggers.

Green focused verification:

- `spec/system/mobile_overflow_spec.rb`: 10 examples, 0 failures.
- `spec/components/schedules/card/actions_component_spec.rb`: 5 examples, 0 failures.
- `spec/components/person_medications/card_spec.rb`: 17 examples, 0 failures.
- RuboCop: 1,824 files inspected, 0 offenses.

The first parent full-suite attempt reported 5,515 examples, 3 failures, and 1
pending because the test image reused stale compiled Tailwind output from an
earlier branch. Rebuilding with `task test:exec CMD='rails tailwindcss:build'`
and rerunning the affected enlarged-text and mobile-navigation specs produced
13 examples and 0 failures. The mobile overflow spec and all four screenshots
were then rerun with that rebuilt CSS.

The enlarged-text failure also exposed a deterministic tab-panel readiness
gap: the profile personal-information and summary geometry could be measured
while the profile panel retained RubyUI's default CSS `hidden` class before
Stimulus connected, despite its HTML `hidden` attribute being false. The spec
now waits for the visible, unhidden profile tab panel
before running its unchanged geometry assertions. The focused enlarged-text
run passed with 2 examples and 0 failures; no production code changed.

The focused browser run retained the existing open-menu placement, menu
geometry, focus return, dismissal, overflow, and privacy assertions. Refreshed
local evidence was inspected and copied to:

- [390px light](../../screenshots/ui-sweep/card-after-390-light-closed.png)
- [390px dark](../../screenshots/ui-sweep/card-after-390-dark-closed.png)
- [1280px light](../../screenshots/ui-sweep/card-after-1280-light-closed.png)
- [1280px dark](../../screenshots/ui-sweep/card-after-1280-dark-closed.png)

The full run with rebuilt assets passed the layout checks but encountered one
intermittent schedule selection timeout (5,515 examples, one failure, one
pending). The dosage-selection file immediately passed all three examples on
retry without changes. Follow-up: [#2140](https://github.com/damacus/med-tracker/issues/2140).
Final full-suite run: 5,515 examples, zero failures, one existing OIDC-dependent
pending example. The profile readiness correction and stacking regression both
passed within that full run.

No modal, hover-shadow, production workflow, or unrelated component changes
were made. Root owns the final full suite, commit, push, and remote CI checks.
