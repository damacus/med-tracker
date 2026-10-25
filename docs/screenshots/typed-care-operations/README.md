# Typed care operations browser evidence

The `reference/` images were captured from unchanged source
`784f6b391ee2ef45cfdb12b27d3e31604ab961bf` before the layer 4 Rust
implementation. The images in this directory use the final typed candidate.
Both runs use the synthetic persistence fixture, browser locale `en-GB`, full
page captures at 1440 × 1000 and 390 × 844, disabled animations, and a visible
`body[data-navigation-ready]` guard immediately before every capture.

The three short journeys save a pause then resume it, record an occurrence as
not taken then reopen it, and create then edit a linked health event. Each
capture follows an assertion of the saved state. The browser journeys use
synthetic records only. The occurrence's “Recorded at” text depends on the
capture clock. The final run passed all three journeys. Eight paired PNGs are
byte-identical. Each paused/resumed desktop pair differs by exactly 28 pixels
within the sidebar search control at `(14, 100)`–`(209, 137)`, with a maximum
colour-channel change of 1/255; the saved treatment content and layout match.
The not-taken pair differs only in
the expected recorded time (11:01:39 versus 11:25:30 UTC). The saved state,
navigation and layout match at both viewports.

| Saved state | Desktop reference | Mobile reference | Typed desktop | Typed mobile |
| --- | --- | --- | --- | --- |
| Paused | [reference](reference/paused-desktop.png) | [reference](reference/paused-mobile.png) | [candidate](paused-desktop.png) | [candidate](paused-mobile.png) |
| Resumed | [reference](reference/resumed-desktop.png) | [reference](reference/resumed-mobile.png) | [candidate](resumed-desktop.png) | [candidate](resumed-mobile.png) |
| Not taken | [reference](reference/not-taken-desktop.png) | [reference](reference/not-taken-mobile.png) | [candidate](not-taken-desktop.png) | [candidate](not-taken-mobile.png) |
| Reopened | [reference](reference/reopened-desktop.png) | [reference](reference/reopened-mobile.png) | [candidate](reopened-desktop.png) | [candidate](reopened-mobile.png) |
| Health created | [reference](reference/health-created-desktop.png) | [reference](reference/health-created-mobile.png) | [candidate](health-created-desktop.png) | [candidate](health-created-mobile.png) |
| Health edited | [reference](reference/health-edited-desktop.png) | [reference](reference/health-edited-mobile.png) | [candidate](health-edited-desktop.png) | [candidate](health-edited-mobile.png) |
