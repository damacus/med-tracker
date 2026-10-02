# Card containment brief

Outcome: schedule and person-medication card actions stay readable, visible and usable on narrow
cards, with intact focus/outline and on-screen menus. Reproduce the supplied screenshot locally first.

Owned product paths: `app/components/schedules/card*`, `app/components/person_medications/card*`.
Owned tests: related component specs, `spec/system/mobile_overflow_spec.rb`,
`spec/system/mobile_ui_audit_spec.rb`, and directly necessary `spec/support` geometry helpers.
Shared M3/RubyUI code is read-only unless the coordinator explicitly expands ownership after evidence.

Keep medication actions, timing semantics and authorisation unchanged. No comment additions/removals.
Prefer responsive layout and sufficient footer inset over clipping or hiding labels. Do not globally
remove overflow guards. Replace tests requiring no flex-wrap with behavioural browser assertions.

First add failing browser tests with authorised schedule and person-medication fixtures at 320/390px,
including long names and both themes. Measure labels/controls against card bounds, check clipping,
open Actions and assert viewport containment. Capture baseline and fixed screenshots locally.
Run focused component and browser specs. Report exact red/green commands and findings in
`01-card-report.md`; independent reviewer writes `01-card-review.md`. Do not commit/push.
