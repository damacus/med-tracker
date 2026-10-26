# Native Menu evidence

These screenshots use synthetic seeded content from the existing backend on
`localhost:18152`. They are browser evidence for the native Menu row change.
They do not prove a newly built Rust application or hosted acceptance.

The browser supplied this checkout's compiled CSS and navigation/search/shortcut
JavaScript through read-only response overrides. The backend still serves the
older sidebar template. A browser-only DOMContentLoaded adapter mirrors the
candidate sidebar template's list wrappers before navigation initialises; it
retains the backend-rendered controls, labels and forms. The candidate template
itself has not been rendered by a newly built Rust process.

Files ending `-before.png` use main's unchanged navigation JavaScript and original
custom row rules. The other PNGs use the candidate. `desktop` is 1440×1000,
`mobile` is 390×844, and `narrow` is 320×844. Narrow candidate captures use long
synthetic navigation and household labels and reduced motion.

The focused browser check observed the native-list expectation fail before the
change and pass afterwards. It checked selected destinations, real POST forms,
CSRF and household fields, submit-button associations, Tab, Escape with focus
return, Ctrl+K search, shortcut hint retention, long-label row bounds, and a
reduced-motion viewport. It sent no POST requests.

The normal-speed mobile recording and executable browser check remain outside
the repository at `/private/tmp/medtracker-css-comparisons/menus/`.
`native-menu-mobile.gif` is the normal-speed recording, and `acceptance.json`
records the focused checks without CSRF values.

`task frontend:build`, all 24 `task frontend:test` checks, and `git diff --check`
passed. The full Rust suite was explicitly waived for this bounded UI change.
Compatibility with the separate Drawer/popover work needs the delivery review;
this change preserves main's existing dialog containers.
