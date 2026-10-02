# Route verification report

Issue #2123. This bounded route step used the isolated Docker test overlay and the installed
Playwright browser. Canary was not used.

The matrix covers authenticated household-owner pages at 390px and 1280px in light and dark
themes, including dashboard, profile, reports, offline, locations, medicines, stock check, medicine
reviews, schedules, people, household settings, and admin management pages. The new schedule page
records its intentional redirect to the schedule workflow. The platform-only dm+d import and
`platform_settings_path` journeys are checked separately with an explicit `PlatformAdmin`
capability.

Signed-out coverage uses a valid invitation token for create-account and invitation acceptance, plus
login, password-reset request, verification resend, and unlock-account request. The health-history
report is download-only: the browser check verifies its rendered form action, while the PDF response
remains covered by the existing request specs. Passkey and other stateful authentication screens are
not faked.

## Red evidence

The initial full matrix produced **9 examples, 1 failure**. The medication show route failed at
1280px in both light and dark themes with 40px document overflow. The focused reproduction against
the original layout produced **1 example, 1 failure** with aggregate failures for the two themes:

```text
route=/households/fixture-household/medications/576052154
offending text=Adjust Inventory
element=button
rect=left 1138, right 1320, width 182
button class includes=w-full justify-center col-span-2
ancestor class=inline-block (RubyUI dialog trigger wrapper)
```

The original-layout screenshot is retained at
[`medication-show-1280-light-before.png`](../../screenshots/ui-sweep/medication-show-1280-light-before.png).

## Correction and green evidence

The accepted narrow correction wraps the adjust-inventory modal in a `col-span-2 min-w-0` grid item
and removes the ineffective `col-span-2` token from the button itself. The shared dialog component
was not changed.

Component check:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/components/medications/show_view_spec.rb
21 examples, 0 failures
```

Final route matrix:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/mobile_ui_audit_spec.rb
9 examples, 0 failures
```

The fixed 1280px light screenshot is retained at
[`medication-show-1280-light-fixed.png`](../../screenshots/ui-sweep/medication-show-1280-light-fixed.png).

No full-suite or lint run was performed in this tranche; the parent agent owns those final gates.
