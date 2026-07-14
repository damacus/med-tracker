# Manual accessibility smoke-test checklist

Use this checklist for a quick, manual screen-reader and keyboard pass over a
release candidate. It complements (and does not replace) the automated browser
and system specs.

## Automated blocking harness

Run the deterministic accessibility/responsive journey matrix locally with:

```fish
task test:preflight
task playwright TEST_FILE=spec/system/mobile_ui_audit_spec.rb
task playwright TEST_FILE=spec/system/mobile_overflow_spec.rb
```

The existing blocking Playwright system job discovers these examples because
it enumerates every `spec/**/*_spec.rb` file and runs the `browser` tag. No
workflow or task change is required when extending the harness.

## Supported test combinations

Run at least one desktop combination for each release candidate. Keep the
browser and assistive technology on their current stable release channels;
record the exact versions in the result below when reporting a finding.

| Browser channel | Operating system | Screen reader | Keyboard |
| --- | --- | --- | --- |
| Chrome stable | Windows 11 | NVDA stable | Standard PC keyboard |
| Edge stable | Windows 11 | Narrator | Standard PC keyboard |
| Safari stable | macOS current | VoiceOver | Mac keyboard |
| Firefox stable | Windows 11 | NVDA stable | Standard PC keyboard |

If a combination is unavailable, record the substitute and why. Test at the
responsive width used by the journey (desktop and, where relevant, mobile).

## Before starting

- [ ] Use only synthetic data. Do not enter real names, dates of birth,
      addresses, NHS numbers, medication identifiers, or health notes.
- [ ] Use the repository's fixture/test account and the password documented in
      `docs/testing.md`; never copy production data into a test environment.
- [ ] Turn off browser extensions that alter accessibility output unless the
      extension is the subject of the test.
- [ ] Reset zoom to 100%, then repeat one journey at 200% zoom if the change
      affects layout or responsive behavior.
- [ ] Start a fresh session and verify that the screen reader announces the
      page title and initial heading.

## Keyboard and screen-reader baseline

- [ ] Navigate with `Tab` and `Shift+Tab` only. Every interactive control has a
      visible focus indicator, and focus order follows the visual and reading
      order without jumping into hidden content.
- [ ] The first useful stop is a skip link or the main navigation; activating
      it moves focus to the main content landmark.
- [ ] Use screen-reader landmark navigation to find one `main`, the primary
      navigation, and any complementary or footer regions. Landmark names are
      unique and useful.
- [ ] Use heading navigation. There is one clear page heading and the heading
      levels describe the page structure without skipped levels.
- [ ] Every form control has an announced, associated label, instructions, and
      required state. Error text is announced and tied to the invalid control.
- [ ] Buttons, links, menu items, tabs, comboboxes, and dialogs announce their
      role and state. Do not rely on colour, position, or an icon alone.
- [ ] Press `Enter` or `Space` on the focused control as appropriate. Do not
      require a pointer gesture or a keyboard chord that the UI does not name.
- [ ] In dialogs and menus, `Tab` stays within the active surface when a trap is
      intended; `Escape` closes it. Focus returns to the button or link that
      opened it, and the page behind it is not reachable while it is modal.
- [ ] When an action updates content asynchronously (save, search, dose
      recording, inventory, or notifications), the status/result is announced
      once, focus remains sensible, and the update does not steal focus.

## Critical journeys

For each journey, complete the keyboard and screen-reader baseline checks above
and record any failure with the exact route and control name.

- [ ] **Sign in and account security:** sign in, handle a validation error,
      recover from an invalid submission, and exercise the available passkey or
      two-factor prompt. Verify focus moves to the first error and returns to
      the invoking control after a dialog or prompt closes. (See authentication,
      passkey, signup, and two-factor feature/system coverage.)
- [ ] **Navigation and dashboard:** move through desktop navigation and the
      mobile menu, open and close the menu with the keyboard, then reach the
      dashboard. Verify menu focus restoration and that current-page state is
      announced. (See navigation and dashboard system coverage.)
- [ ] **People and medicines:** add or edit a synthetic person, add a medicine
      using the finder, and submit both valid and invalid forms. Check labels,
      validation summaries, combobox/listbox announcements, and focus on the
      first invalid field. (See people, medication finder, and person-medicine
      workflow coverage.)
- [ ] **Schedules and dosage:** create or edit a schedule, use dosage options,
      and close the add-schedule modal. Verify the modal heading, focus trap,
      error announcement, and return focus to the launch control. (See schedule
      workflow and modal system coverage.)
- [ ] **Record a dose:** record a current and historical dose, including the
      confirmation or undo path. Verify the asynchronous status is announced and
      focus remains on a useful result or returns to the triggering control.
      (See historical-dose, offline-dose, and nurse-shift system coverage.)
- [ ] **Medication stock and refill:** adjust stock, refill or reorder a
      medication, and handle an error response. Verify status messages and that
      controls remain keyboard reachable after the update. (See stock, refill,
      and reorder system coverage.)
- [ ] **Search and reports:** run global search and open a report using only the
      keyboard. Verify result headings, empty/loading states, filters, and any
      live-region announcement. (See global-search and report feature coverage.)
- [ ] **Administrative flows (authorized tester only):** invite/manage a user
      and open audit logs. Verify table headers, pagination, dialogs, and error
      focus without exposing records outside the test household. (See admin
      user, invite, and audit-log system/feature coverage.)

## Result record

Complete this block for every pass. Attach evidence only after removing or
redacting any personal or health information.

- **Tester:**
- **Date (UTC):**
- **Environment/URL:** (local, CI preview, or hosted test tenant)
- **Browser and OS:**
- **Screen reader and version:**
- **Viewport/zoom:**
- **Journeys covered:**
- **Result:** pass / pass with findings / blocked
- **Evidence:** screenshot, screen-reader transcript, or short recording path
- **Findings:** route, control, expected result, actual result, and severity
