# Native profile component evidence

Uses stock DaisyUI Tabs, single-open named Accordion groups with arrow icons,
and the mobile Dock. The existing profile JavaScript, form actions and controls
remain in place.

The profile slice starts at `61b8f36cca2bc0818e2f2296208da8fbe26a754f`.
Ordinary merge `a72c5424f0e1f71c5f5ced8632712c1b445142af` integrates the
PR #2937 navigation parent `10823ee7314a3b61b93e421820d60e4a4aa166ae`.
The refreshed captures below use that integrated candidate. Dock stays inside
the parent Drawer content so opening the Drawer makes Dock inert. Parent
popover account menus and native modal search remain in place.

Chrome checks covered 320, 390, 1023, 1024 and 1200 px: tab deep links, reload,
URL query preservation, keyboard navigation, pointer and keyboard disclosure
operation, and single-open groups in Profile, Notifications and Advanced.
They also covered the existing authorised shortcut destinations, active route,
horizontal overflow, responsive Dock clearance and drawer layering.
Integrated checks also covered account popover opening, Ctrl+K handoff to
modal search, results, Escape and focus restoration to the account opener.
In Chrome, Escape first clears a non-empty native search input; a second
Escape dismisses the modal and restores focus. This preserves the parent
browser behaviour and is not a profile component change.

The existing behaviour first failed the single-open expectation with two open
settings. The changed markup passed. Frontend build, all 24 frontend tests and
diff checks passed. The full local Rust suite was explicitly waived.

These captures use synthetic content from the existing backend on
`localhost:18152`. Browser response overrides supply this checkout's built CSS
and JavaScript, and adapt the existing HTML classes and attributes to match the
candidate templates. This verifies browser behaviour with the candidate markup
and styles; it does not prove freshly rendered Tera templates or hosted acceptance.
The integrated capture additionally adapts the shell wrapper and search dialog
to the parent template markup. No running app or server mount was changed.

No form submissions, alternate permission roles, translated labels or physical
devices with non-zero safe-area insets were exercised. The GIFs play at normal
speed. No container was restarted and no fixture data was changed.
