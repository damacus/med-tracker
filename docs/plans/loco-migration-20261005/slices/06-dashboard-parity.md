# Dashboard recovery and migration

Owner: this dashboard workstream. Branch: codex/dashboard-parity.

## Scope

Recover the dashboard candidate into an isolated worktree, then preserve the Rails
dashboard layouts and complete dashboard dose recording using established Loco
authentication, person permissions, stock resolution and dose operations.

Layouts: current, time first, family lanes and calm focus. Profile settings,
medication launcher, wizard and changes to their downstream behaviour are excluded.

## Recovery

Base: ce02755847cac913a794f3fc0bfc89e5d4201f7b.
Source: profile-scope-checkpoint/20261010T000955Z/changed-files.tar.gz.
Archive SHA256: 24c7d107d6f198d5057edead65ebe54b5bba399774618cc0c23de51e7d04c656.
Only dashboard files and necessary shared dose integration were recovered.
The original checkout, profile worktree and complete recovery archive are preserved.

## Reference and decisions

Rails remains the reference under rails/. Dashboard views, presenter,
FamilyDashboard::ScheduleQuery and RoutineDoseProgress define the journey.
Retained desktop references are under docs/screenshots/dashboard-reference/.
The running reference used London time, English, dark Command Centre and Jane Doe
with Child Patient. Loco browser checks use isolated synthetic care fixtures;
their content differs, so these images alone cannot establish pixel parity.

Preserve the collapsed person selector, sidebar, spacing, cards, layout hierarchy,
routine versus as-needed treatment, today-only dose history and stock summary.
PRN treatment must not inflate routine metrics. Each routine source shows its next
open dose. Show actual recorded amounts in history. Use the existing source-stock
resolver and dose command; no second recording operation.

The Rails Add medication launcher is a separate dependency: it is not present on
the Loco base. A focused owner question is pending; no launcher work is authorised
by silence. Profile, finder, medicine-review navigation and protected avatar delivery
also depend on their own migration streams. Initials provide the existing fallback.
Do not claim whole-application or complete navigation parity from this slice.

## Acceptance

- All four layouts render on desktop and mobile without horizontal overflow.
- Person selection respects grants; hidden people return 404 without health data.
- Take or Give opens confirmation on the dashboard; cancel keeps the selection.
- Successful recording returns to that dashboard selection and persists one dose.
- Invalid input retains the dialog; CSRF and unauthorised source submissions fail.
- Matching authorised stock can be selected and is deducted from the chosen pack.
- Legacy PRN, cooldown, cycle history and routine counts match the Rails rules.
- An available dose precedes a future dose in focused layouts.
- Compare screenshots with Rails; document remaining differences accurately.
- Independent correctness/security review, relevant browser checks and task ci pass.

## Routing and review

Use the existing primary model for this bounded migration and established code
paths. One read-only independent reviewer checks correctness and security.
Do not create a standing team. Reassess against evidence after failed acceptance.
No Devin job has been launched for this dashboard slice.

## Current evidence

Recovered candidate and focused fixes are complete in the isolated worktree.
Independent review found and verified fixes for alternative stock, focused priority,
numeric weekday strings and stock quantity labels. No additional in-scope blocker remains.
Desktop and mobile screenshots were inspected against retained Rails views and the live
mobile reference. Card hierarchy, spacing, period grouping, selector, action and navigation
rail were compared; missing rail and compact-card spacing were corrected. Both themes
were exercised. Fixture/role/content differences prevent a pixel-equality claim.
Global search is also a shared-shell dependency; it is not implemented by this slice.

All fourteen focused browser checks passed across desktop and mobile: twelve journey
checks plus the two stock checks after correcting the fixture's decimal scale.
The six domain checks and documentation build passed. Shared navigation dependencies
are tracked in https://github.com/damacus/med-tracker/issues/2503.
Full CI caught a handler argument-count lint; the existing extractors were grouped
without changing the request contract. Rust and tooling gates then passed. The first
full browser run passed 475 of 476 checks; one unrelated signup fixture failed with
PoolTimedOut before its server started. That unchanged test passed in isolation.
The unchanged full browser retry passed all 476 checks, with no skips or retries.
The startup failure is recorded on issue #2498; its cause remains unverified.
Publication follows these gates. This work does not merge or deploy the application.
