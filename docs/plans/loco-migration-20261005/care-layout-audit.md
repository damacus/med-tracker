# Care-page layout audit — 7 October 2026

The care forms now separate dose, timing, stock and secondary information.
Save, Cancel and recording actions keep their natural width. Medication,
person and location pages put current care before management actions. Barcode
remains stored metadata and survives edits to other medication details.

## Evidence

The rendered checks used the root Loco application, synthetic signed-in care
fixtures and disposable PostgreSQL 18 databases. They covered medication,
dose-option, treatment, person, location, order and stock journeys at 1440×1000
and 390×844. Additional reflow checks used a 320px viewport.

The final hierarchy suite passed 18 cases on each viewport. Existing medication
CRUD and treatment journeys also passed on desktop and mobile. Checks include
keyboard access to Cancel, compact action widths, related field rows, ordering,
field-linked validation errors, stale drafts, barcode retention and legacy
fractional-hour preservation. Representative labels, inputs, selects, primary
buttons and secondary actions meet the asserted 4.5:1 text contrast threshold
in the Default light and dark themes.

Lighthouse 13.5.0 audited the signed-in dose-option form at
`/households/persistence-fixture/medications/80001/dosage_options/new`.
The requested and final URLs matched. This was a single local navigation at
19:09 UTC, using a 375×812 mobile viewport, simulated 4× CPU slowdown, 150ms RTT
and 1,638Kbps throughput. Login and audit used the same browser, with storage
retained and user-agent emulation disabled to preserve the bound session.

| Signal | Result |
| --- | --- |
| Performance | 100/100 |
| Accessibility | 100/100 |
| Best practices | 100/100 |
| Agentic browsing | 100/100 |
| SEO | 90/100 |
| First contentful paint | 1.05s |
| Largest contentful paint | 1.05s |
| Total blocking time | 0ms |
| Cumulative layout shift | 0.000063 |

## Findings and disposition

- **Corrected:** oversized actions, flat form hierarchy, exposed barcode input,
  decimal formatting for whole hours and the expanded Pause action's width.
- **Corrected after independent review:** stale fractional-hour drafts now show
  conflicts before validation; hours errors identify their field; a legacy
  fractional taper interval cannot be transferred to a different step by
  changing hidden form metadata.
- **Intentional:** short fields remain paired on mobile where they fit. The
  narrow-screen checks verify reflow; long timing labels stack on small screens.
- **No public SEO change:** the missing meta description explains the SEO score.
  This authenticated care form is not a public search landing page.
- **Keep private caching behaviour:** Lighthouse noted that `Cache-Control:
  no-store` prevents back/forward cache restoration. Do not relax private-page
  caching to improve this diagnostic.
- **Optional performance follow-up:** the lab run estimated 420ms of render
  blocking from the existing asset chain and 6KiB of document transfer savings.
  No performance regression or before/after speed-up is established by this
  single measurement.
- **Existing shell issue:** an earlier login navigation requested a missing
  favicon, already tracked in [issue #2438](https://github.com/damacus/med-tracker/issues/2438).

Two preliminary CLI audits were redirected to login and are excluded from the
care-page results. The final audit used Lighthouse's same-browser authenticated
recipe. The temporary audit harness was removed after measurement.

## Limits and screenshots

The scores describe this representative form and lab conditions. They do not
establish field Core Web Vitals, INP, every theme's accessibility or a complete
screen-reader assessment. Keyboard and visual checks supplement automated
results; no production measurement or deployment is claimed.

- [Dose option: desktop](../../screenshots/care-layout/dose-option-desktop.png),
  [mobile](../../screenshots/care-layout/dose-option-mobile.png),
  [dark desktop](../../screenshots/care-layout/dose-option-dark-desktop.png),
  [dark mobile](../../screenshots/care-layout/dose-option-dark-mobile.png).
- [Medication editing: desktop](../../screenshots/care-layout/medication-edit-desktop.png)
  and [mobile](../../screenshots/care-layout/medication-edit-mobile.png).
- [Treatment editing: desktop](../../screenshots/care-layout/treatment-edit-desktop.png)
  and [mobile](../../screenshots/care-layout/treatment-edit-mobile.png).
