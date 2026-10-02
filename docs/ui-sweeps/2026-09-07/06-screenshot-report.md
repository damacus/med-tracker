# Final screenshot evidence

This bounded capture used synthetic fixtures through the isolated Docker test overlay. Canary was
not used, and no real recovery email was requested.

## Matching card pair

The temporary capture spec rendered John’s person-medication page at 390px by 844px in light mode
with the same long fixture names in both runs:

- `Paracetamol extended release oral suspension with an intentionally long label`
- `Vitamin D high-strength daily supplement with an intentionally long label`

The original run temporarily replaced only these four layout files with their `45db2112` versions:

- `app/components/person_medications/card.rb`
- `app/components/person_medications/card/actions_component.rb`
- `app/components/schedules/card.rb`
- `app/components/schedules/card/actions_component.rb`

The temporary spec captured the Vitamin D person-medication card element. The original and fixed
images are:

- [`card-layout-before-390-light.png`](../../screenshots/ui-sweep/card-layout-before-390-light.png)
- [`card-layout-after-390-light.png`](../../screenshots/ui-sweep/card-layout-after-390-light.png)

The original image shows the single-row actions reaching the rounded card edge. The fixed image shows
the actions stacked within the card inset.

## Representative fixed states

- [`enlarged-text-320-before.png`](../../screenshots/ui-sweep/enlarged-text-320-before.png): initial 320px enlarged-text state retained as before evidence after the first capture showed the notice and shell overlap.
- [`profile-tabs-320-enlarged-intermediate.png`](../../screenshots/ui-sweep/profile-tabs-320-enlarged-intermediate.png): rejected intermediate capture. It shows fragmented labels in four narrow columns and a fading notice over the tablist. Step 07 owns the layout correction and final recapture; this is not fixed-state evidence.
- [`enlarged-text-320-fixed.png`](../../screenshots/ui-sweep/enlarged-text-320-fixed.png): final 320px shell and warning at root font size 32px, with the warning text clear of its dismiss control.
- [`profile-tabs-320-enlarged-fixed.png`](../../screenshots/ui-sweep/profile-tabs-320-enlarged-fixed.png): final 320px profile tabs at root font size 32px after the notice was dismissed and hidden; the four labels occupy two rows and remain contained.
- [`location-member-controls-390-fixed.png`](../../screenshots/ui-sweep/location-member-controls-390-fixed.png): Grandmas location at 390px with John’s member-removal dialog open.
- [`household-owner-administration-1280-fixed.png`](../../screenshots/ui-sweep/household-owner-administration-1280-fixed.png): signed-in household owner administration dashboard at 1280px.
- [`auth-invalid-login-390-fixed.png`](../../screenshots/ui-sweep/auth-invalid-login-390-fixed.png): invalid login feedback at 390px, with one inline alert and no recovery submission.
- [`long-invitation-email-390-fixed.png`](../../screenshots/ui-sweep/long-invitation-email-390-fixed.png): pending invitation row at 390px using a valid 64-character local-part address; the invitation was created directly and not submitted through email delivery.

## Capture and restoration checks

The temporary card baseline capture completed with **1 example, 0 failures**:

```text
env COMPOSE_FILE=compose.yaml:/private/tmp/medtracker-ui-sweep-compose.yaml task test TEST_FILE=spec/system/ui_sweep_screenshot_capture_spec.rb:13
```

The fixed card capture completed with **2 examples, 0 failures** using the same isolated task runner
and temporary spec. The temporary spec was removed after capture. All four card/action files were
restored from their backups, and `git diff --check` passed. No screenshot-only example remains in the
permanent suite.

The targeted enlarged-text recapture used the existing admin profile fixture and completed with
**1 example, 0 failures** using the same isolated task runner. It dismissed the warning and waited
for the alert to be hidden before scrolling the tablist into view. Its temporary spec was removed
after capture.

The card pair is an element capture rather than a full-page capture so the layout change is directly
visible. Representative states use the 390px, 320px, or 1280px viewport requested for each state;
the invitation screenshot is scrolled to the pending row so its long text and actions are visible.
