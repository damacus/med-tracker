# Native profile component evidence

Uses stock DaisyUI Tabs, single-open named Accordion groups with arrow icons,
and the mobile Dock. The existing profile JavaScript, form actions and controls
remain in place.

Chrome checks covered 320, 390, 1023, 1024 and 1200 px: tab deep links, reload,
URL query preservation, keyboard navigation, pointer and keyboard disclosure
operation, and single-open groups in Profile, Notifications and Advanced.
They also covered the existing authorised shortcut destinations, active route,
horizontal overflow, responsive Dock clearance and drawer layering.

The existing behaviour first failed the single-open expectation with two open
settings. The changed markup passed. Frontend build, all 24 frontend tests and
diff checks passed. The full local Rust suite was explicitly waived.

These captures use synthetic content from the existing backend on
`localhost:18152`. Browser response overrides supply this checkout's built CSS
and JavaScript, and adapt the existing HTML classes and attributes to match the
candidate templates. This verifies browser behaviour with the candidate markup
and styles; it does not prove freshly rendered Tera templates or hosted acceptance.

No form submissions, alternate permission roles, translated labels or physical
devices with non-zero safe-area insets were exercised. The GIFs play at normal
speed. No container was restarted and no fixture data was changed.
