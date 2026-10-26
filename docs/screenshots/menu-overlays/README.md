# Shared navigation menus and search shortcuts

Captured on 26 October 2026 in installed desktop Chrome, using the existing
synthetic fixture preview at localhost:18152. Browser request overrides served
this checkout's CSS and JavaScript; the shared Compose app and its mounts were
unchanged. The dashboard main region was extended to 1800px to check a nonzero
180px scroll offset. No care data was changed.

The Rails references are the shared SheetContent and DropdownMenuContent:
the drawer slides from the left; account dropdowns retain panel elevation without
a page-wide dim layer. The Rust native dialog keyboard/focus behaviour remains.

- `before-*.png`: original application CSS, built from the branch baseline,
  with current JavaScript to isolate the CSS behaviour.
- `after-*.png`: updated shared CSS and JavaScript at 1440×1000 and 390×844.
- `after-touch-320.png`: touch-only 320×740; desktop shortcut hint hidden and
  no horizontal overflow.
- `before-geometry.json` and `after-geometry.json`: root width, document height,
  header/main/body rectangles and scroll offsets before opening, while open and
  after closing the desktop account, mobile account and mobile drawer.
- `interaction-checks.json`: actual opening animation frames, Escape/focus,
  click-away, reduced-motion and simulated MacIntel/Win32 hint/shortcut checks.

The fixed geometry is identical within each open/close sequence. Account backdrop
colour changes from black at 45% opacity to transparent. Menu panels retain their
theme elevation. Both menus have intermediate opacity/translation frames during
their 300ms entrance. The existing global reduced-motion rule shortens transitions.

The original body-level overflow lock and shared dark backdrop were confirmed in
the owning CSS. The default macOS browser uses overlay scrollbars and did not
reproduce the reported width jump; these captures do not prove that symptom's
cause on the user's browser. The change reserves a stable root scrollbar gutter
and moves drawer scroll locking from the body to the root. This reserves space
before opening rather than changing available space during the interaction.

The shortcut regression checks fail against the original search script and pass
against the updated script. They cover macOS, Windows, Linux and userAgentData
platform precedence, visible hints, aria-keyshortcuts and both existing modifiers.
The existing frontend task runs these checks. The full local Rust suite was
explicitly waived for this bounded UI slice.
