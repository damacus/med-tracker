# Proposal

## Why

Rust browser users cannot configure the notification preferences already supported by the household API. Give the signed-in person a small, accessible settings page.

Originating issue: https://github.com/damacus/med-tracker/issues/2386

## What Changes

- Add a My notifications destination with master, dose due, missed dose, low stock and private content switches.
- Load and save through the existing notification preference API, with visible success and recoverable errors.
- Preserve schedule times and other account settings. Localise the page and verify desktop and mobile use.
- Non-goals: push enrolment, browser permission prompts, schedule editing, delivery transport, other people's preferences, Rails changes, migrations and authentication changes.

## Capabilities

### New Capabilities

- `browser-notification-preferences`: configure the signed-in person's existing notification preferences safely through the Rust browser.

### Modified Capabilities

None.

## Impact

Rust browser routes, authenticated internal API adapter, household navigation, rendering, locale catalogues and browser tests. Existing API persistence, permissions and audit rules remain authoritative. No new dependencies are expected.
