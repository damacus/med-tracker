# Loco profile captures

These captures show the four-tab profile candidate on 10 October 2026.
They are review evidence, not a declaration of accepted Rails parity.

## Capture conditions

- Synthetic Jane Doe account (`jane.doe@example.com`), ordinary Member.
- Adult with capacity, date of birth 1980-01-02, saved time zone London.
- English (UK), browser time zone Europe/London.
- Command Centre palette, explicit Light and Dark modes.
- Desktop 1440 × 1000; mobile 390 × 844.
- Tabs and expanded sections use full-page screenshots; dialogs use the viewport
  with animations disabled. The mobile navigation remains fixed at the viewport
  edge and therefore appears partway down tall full-page images.
- No real health records, credentials or user accounts are included.

The reproducible capture journey is `tests/browser/profile-visual.spec.mjs`.
File names identify the tab or dialog, appearance and viewport. The journey
checks visible panels, horizontal overflow, dialog cancellation and focus return.
Additional interaction tests cover 320px doubled text, keyboard navigation,
permissions, persistence and failure states; these ordinary captures do not
replace those tests.

## Comparison limits

The retained Rails images are in `../profile-reference/`. The comparison covers
the four-tab hierarchy, personal-information/settings layout, grouped security
methods, summaries, disclosures and connected dialogs. It is not pixel-identical:
the root Loco navigation lacks the broader Rails sidebar search, notification
switches use accessible native checkbox controls, and sensitive actions retain
the agreed Loco authentication and shared-history retention semantics.

This ordinary fixture has no managed-person grant, so managed-person controls
are covered by their separate browser tests rather than these captures.
Gravatar remains disabled. External Web Push delivery is not demonstrated by
local screenshots. Independent source review and the Gravatar decision remain
tracked in issue #2502; publication does not authorize merge or deployment.
