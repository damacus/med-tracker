# Shared navigation components and search shortcuts

The final correction uses DaisyUI's documented Drawer structure and popover
Dropdown. The account dropdown anchors to the pressed button: below the mobile
header button, or above the desktop sidebar button. It has no full-page backdrop.
The drawer uses drawer-toggle, drawer-content, drawer-side and drawer-overlay,
with DaisyUI's own slide transition. Theme surfaces and panel elevation remain.

The initial patch animated custom dialogs without correcting the account menu's
fixed top-right placement. The user rejected that behaviour after watching the
recording. The corrected captures supersede the earlier after captures.

Captured on 26 October 2026 in installed Chrome against the existing synthetic
fixture preview at localhost:18152. Browser request overrides served the current
CSS/JavaScript and the exact static household drawer fragments from base.html.
The running Compose app, its mounted views and care data remained unchanged.
This verifies frontend behaviour against the seeded app; it does not represent
a rebuilt Rust executable or the full browser suite.

- `before-*.png` and `before-geometry.json` retain the original modal baseline.
- `corrected-desktop-account.png`: dropdown immediately above the sidebar button.
- `corrected-mobile-account.png` and `corrected-small-mobile-account.png`:
  dropdown immediately below the header button at 390px and 320px.
- `corrected-mobile-drawer.png` and `corrected-small-mobile-drawer.png`:
  standard Drawer, with its background inert and page geometry unchanged.

The focused Playwright placement check failed against the original desktop menu.
It passes at 1440×1000, 390×844 and 320×740, both with native CSS anchoring and
with CSS anchor support disabled to exercise the measured-position fallback.
Checks cover trigger-to-menu distance (0–12px), viewport bounds, native light
dismissal, Escape/focus return, drawer structure, inert background, Tab containment
and opening global search from either menu. Delaying the navigation response while
typing into a main-page input preserves focus and selection after initialisation.
The wrapper is rendered with the initial household layout, avoiding reparenting
the live page. Login has no drawer wrapper.

The original width jump was not reproduced with macOS overlay scrollbars. The
stable root gutter is retained, and drawer opening preserves the checked main
rectangle. No claim of reproducing that original symptom is made.

The approved search shell now uses the standard DaisyUI Modal with its backdrop,
native focus containment and transitions. The shared immediate-hide override was
removed. `approved-search-open.png` and `approved-search-mobile.png` show the
implemented search at desktop and 320px. Playwright checks covered real fixture
results, Escape/focus return, account-to-search handoff, Tab containment, mobile
bounds and backdrop dismissal. Search data and permission handling are retained.

Frontend build, all 24 remaining lightweight frontend checks, formatting and
whitespace checks pass. The owner explicitly requested removal of the search
shortcut test and its task entry after the UI correction; both were removed.
Permanent Rust browser assertions cover dropdown anchoring and
Drawer structure; their local compilation/execution was not run. The owner waived
the full local suite for this UI slice. Independent review findings about focus
preservation and measuring after animation completion were addressed.
