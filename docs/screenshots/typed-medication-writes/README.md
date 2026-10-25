# Typed medication write browser evidence

These captures use synthetic persistence fixtures, the browser harness's `en-GB`
locale, and a visible `body[data-navigation-ready]` guard immediately before each
screenshot. The desktop viewport is 1440 × 1000; the mobile viewport is 390 × 844.

The baseline ran from the unchanged parent source at
`79e4fa6a006e9dad4d40f84276131a42aaf47c5b`, with only the same capture
guard added to its browser test. The final run used the typed medication write
candidate restored from an exact 28-file SHA-256 manifest. Both browser journeys
passed on each source state. The desktop journey creates a medication and dose
option through the wizard, then saves a dose-option edit after the screenshot.
The mobile journey resolves a barcode to an existing medicine and opens refill.

| View | Baseline | Final | SHA-256 for both |
| --- | --- | --- | --- |
| Desktop | [baseline](baseline-desktop.png) | [final](final-desktop.png) | `9ae7b5c8d0fc2740f24c68f1d2fbea1c7b5a5e6a554ca25e173906e3be7e22ed` |
| Mobile | [baseline](baseline-mobile.png) | [final](final-mobile.png) | `619f0b717a5bf944045d69e18196f280c90ac7e978c39d231459bd6bea0a00db` |

Each baseline/final pair is byte-identical. Earlier captures taken before the
navigation-ready guard showed incomplete navigation and are not the comparison
reference.
