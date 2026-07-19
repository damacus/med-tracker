# Stock amendment mode design QA

## Target

- Selected direction: option 3, the two-pane inventory administration workspace.
- Reference: `/Users/damacus/.codex/generated_images/019f7b0c-a1f9-7b51-a683-786e055c7109/exec-529d79d2-8d11-4ab6-86da-917cf2ab0910.png`
- Prototype: `http://localhost:51204/households/fixture-household/medications/stock_check?location_id=836111569`

## Matched desktop state

- Viewport: 1440 x 1024.
- Selected medicines: Paracetamol, Aspirin, Calpol, and Vitamin D.
- Remaining supply: 74, 0, 32, and 42 units.
- Total net change: -32 units.
- Paired comparison: `docs/screenshots/stock-check-design-qa-comparison.png`.
- Prototype capture: `docs/screenshots/stock-check-desktop.png`.

The paired comparison confirms the reference hierarchy, two-pane proportions, spacing, batch-row density, amendment controls, reason field, and footer actions. Fixture medicine ordering and the existing MedTracker navigation labels remain product data rather than visual defects.

## Responsive and interaction checks

- Mobile viewport: 390 x 844.
- Final document width: 390 pixels; no horizontal overflow.
- Mobile captures: `docs/screenshots/stock-check-mobile.png` and `docs/screenshots/stock-check-mobile-batch.png`.
- Search, location selection, medicine selection, clear/remove, set-to-zero, quantity editing, net-change calculation, and batch submission are functional.
- A browser regression covers the long-medicine-name overflow found during the mobile pass.

## Findings

| Severity | Finding | Status |
| --- | --- | --- |
| P1 | A selected medicine with a long name expanded the mobile grid from 390 to 685 pixels. | Fixed and covered by a browser regression. |
| P2 | An initial zero net change rendered as `-0 units`. | Fixed and covered by a request regression. |
| P0 | No blocking usability or data-integrity findings remain. | Passed. |

## Result

Passed. No open P0, P1, or P2 design-QA findings remain.

---

# Medicine Reviews Design QA

- Source visual truth: `/var/folders/3f/7gk0v01945nby3cfstkkj3d00000gn/T/codex-clipboard-4c8055fd-6afb-42c0-8d2f-605fd6e9725a.png`
- Desktop implementation: `docs/screenshots/medicine-reviews-desktop.png`
- Mobile implementation: `docs/screenshots/medicine-reviews-mobile.png`
- Full-view comparison: `docs/screenshots/medicine-reviews-comparison.png`
- Desktop viewport: 1536 x 1024
- Mobile viewport: 390 x 844
- State: dark theme, Needs review tab, All priorities, filtered items excluded

## Full-View Comparison

The side-by-side comparison confirms the same information hierarchy: concise page context, review-state tabs, a single priority control, a visible noise-filter control, and large progressive-disclosure cards. The implementation intentionally retains MedTracker's current dark theme, navigation shell, typography, tokens, household grouping, and seeded records.

The reference's oversized alert circles were intentionally omitted at the user's direction. The unsupported per-card "Add to appointment" action was not invented; the existing appointment-ready PDF export remains available in the page header.

## Focused Comparison

A separate crop was not needed because the 1536 x 1024 comparison keeps the tabs, toggle group, switch, card typography, badges, and actions legible. The mobile capture separately verifies responsive control labels and card wrapping.

## Fidelity Surfaces

- Fonts and typography: existing MedTracker type family, weights, line heights, and heading hierarchy retained; card titles remain prominent and wrap cleanly.
- Spacing and layout rhythm: filters align with the reference hierarchy; cards use the existing container, spacing scale, shape tokens, and elevation.
- Colors and visual tokens: existing dark-theme surface, border, primary, warning, and error tokens are used throughout.
- Image and asset fidelity: no raster assets are required; existing application icon components are used. No custom SVG or CSS artwork was introduced.
- Copy and content: patient-facing summaries remain short; evidence provenance and practitioner-review fields are available only after disclosure.

## Interactions Tested

- Review-state links navigate while retaining active priority and noise filters.
- Priority Toggle Group submits and reduces the visible cards.
- Include filtered items Switch submits without duplicate query parameters.
- View evidence expands the public-label excerpt, match rationale, source link, and practitioner-review form.
- Desktop browser console: no warnings or errors.

## Comparison History

1. Initial capture: generated RubyUI classes were absent from the running development CSS, making the Switch visually unclear. Rebuilt development assets and recaptured.
2. Mobile capture: long priority labels overflowed the initial viewport and the noise helper compressed its label. Added concise mobile labels using existing responsive utilities and stacked the helper below the Switch.
3. Final capture: no actionable P0, P1, or P2 differences remain. Theme, data, omitted alert artwork, and the retained PDF workflow are intentional product constraints.

## Findings

No actionable P0, P1, or P2 findings remain.

## Follow-up Polish

- P3: consider a future user preference for light theme screenshots when comparing directly with light visual concepts.

final result: passed
