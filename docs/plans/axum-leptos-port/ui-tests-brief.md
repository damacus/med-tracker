# Web and PWA test writing brief

Start after the API test suite and matrix meet their Rails-baseline gate. Use
one writer per bounded journey group and an independent review after each.

## Result

Produce target-independent Playwright browser tests under
`rust/tests/browser/` that exercise the current Rails app and later the Rust
app through the same base URL setting. Port observable journeys from
`spec/system/`, `spec/features/`, web request specs, and PWA tests. Record
each case and its source in `rust/parity-matrix.md`.

## Journey groups

1. Authentication, MFA/passkeys, household navigation, profile, sessions,
   responsive navigation, and denied access.
2. People, relationships, permissions, locations, medication catalogue,
   stock, schedules, and direct assignments.
3. Scheduled/direct dose recording, corrections, pause periods, dose
   outcomes, reports, history, and administrative workflows.
4. PWA manifest/installability, offline shell, queued doses, local snapshot,
   stock selection, reconnection, retries, revocation, service-worker update,
   and push notification behaviour where supported by the test browser.
5. Accessibility: keyboard navigation, labelled controls and errors, focus
   management, headings, table semantics, and desktop/mobile screenshots.

## Completion gate

Run each journey against Rails with repeatable fixtures in desktop and mobile
viewports. Investigate baseline failures before using the test as a Rust
oracle. Review screenshots for layout and interaction, not just DOM presence.
The matrix must account for every system/feature spec and browser-observable
behaviour in other specs. Rust implementation begins only after this suite and
the API suite have passed their Rails-baseline gates.
