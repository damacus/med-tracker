# PR 2126 component and stacking feedback

The user identified closed Actions buttons painting over the mobile navigation
bar in the accepted screenshot. The shared dropdown root carries `z-50`, above
the rail's `z-40`. Verify this with browser hit testing before changing code.

Nightingale is the sole Luna writer. Fix the shared dropdown trigger stacking
while preserving open-menu placement, clipping avoidance, focus, and dismissal.
Add meaningful browser coverage for navigation hit targets under overlapping
schedule and person-medication Actions buttons; avoid class-token assertions.
Capture refreshed mobile light/dark and desktop evidence. Use the isolated test
environment, never Canary or shared `task playwright`.

Root and the independent reviewer also assess the show-view layout wrapper and
the removed card hover scale. Keep hover shadow feedback. Do not restore scaling
that creates a containing block for fixed menus. No modal rewrite or renaming
unless current component evidence shows it is needed.

Run preflight before production edits, prove the regression red, then focused
browser tests, relevant component coverage, full Ruby lint and required full
suite. Sol independently reviews all changes. Root owns final acceptance,
commit and push to the existing PR. Preserve comments and unrelated changes.
