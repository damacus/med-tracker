# Dashboard Experiments Design QA

- Time-first source visual truth: `/Users/damacus/.codex/generated_images/019f7ab0-4797-7f12-9c91-867ff46aa861/exec-b1bc0277-1382-4fd2-8db5-19227074f52a.png`
- Family-lanes source visual truth: `/Users/damacus/.codex/generated_images/019f7ab0-4797-7f12-9c91-867ff46aa861/exec-344a2c3f-3e5b-46de-b8a2-3f7f3d2a1f4e.png`
- Calm-focus source visual truth: `/Users/damacus/.codex/generated_images/019f7ab0-4797-7f12-9c91-867ff46aa861/exec-267f6ed6-9d09-44af-a48f-1acad20fdc2d.png`
- Time-first implementation: `docs/screenshots/dashboard-time-first-desktop.jpg`, `docs/screenshots/dashboard-time-first-mobile.jpg`
- Family-lanes implementation: `docs/screenshots/dashboard-family-lanes-desktop.jpg`, `docs/screenshots/dashboard-family-lanes-mobile.jpg`
- Calm-focus implementation: `docs/screenshots/dashboard-calm-focus-desktop.jpg`, `docs/screenshots/dashboard-calm-focus-mobile.jpg`
- Desktop viewport: 1488 x 1058
- Mobile viewport: 390 x 844
- State: dark theme, seeded household, real medication schedules, real dose history and stock data

## Full-View Comparison

Each source mockup was inspected beside its corresponding desktop implementation. The three implementations preserve the visual concepts as separate, contained layouts while using MedTracker's current navigation shell, person selector, RubyUI components, design tokens, seeded data, authorization, and dose-recording flow.

- Time-first retains a dominant Next up action, chronological day periods, a compact Later today rail, and a separate stock-review card.
- Family lanes retains side-by-side person cards, person/time grouping controls, task actions inside each lane, and a full-width stock summary.
- Calm focus retains one dominant safe action, a secondary After this rail, collapsible completed history, and a compact stock notice.

The source concepts use illustrative names, schedules, and stock units. The implementation deliberately renders the current household's real records and uses the product's existing blue palette instead of hard-coding each mockup's generated accent variation.

## Responsive Comparison

All three variants were reloaded and captured at 390 x 844. Each uses a single-column mobile hierarchy, retains the primary action above the fold, and reports a 390px document width with no horizontal overflow. The existing mobile top bar and bottom navigation remain unchanged.

## Fidelity Surfaces

- Fonts and typography: existing Inter family, product heading hierarchy, strong task labels, tabular times, and current body styles retained.
- Spacing and layout rhythm: source card hierarchy and column ratios reproduced with existing spacing, shape, border, and elevation tokens.
- Colors and visual tokens: current dark-theme surface, primary, warning, success, border, and on-surface tokens used throughout.
- Images and assets: no raster content is required inside the dashboard; existing icon and avatar components are used. No custom SVG or CSS artwork was introduced.
- Copy and content: concise source-inspired labels were translated across every supported locale; medication, person, dose, time, status, and stock values come from live presenter data.

## Interactions Tested

- Each dashboard choice in Profile > Experiments auto-submits and persists independently of the medication-wizard experiment.
- Time-first renders the existing safe Take action and stock-review navigation.
- Family lanes switches between By person and By time without leaving the dashboard.
- Calm focus opens the existing Record dose confirmation dialog with person, medication, dose, time, and stock source; the test did not submit a dose.
- Selecting Current restores the unchanged dashboard component with no experimental variant marker.
- Desktop and mobile person selectors retain their current behavior.
- Browser console: no warnings or errors on the final dashboard state.

## Comparison History

1. Initial captures revealed that new Tailwind utilities had not yet been compiled, collapsing Calm focus into one column and leaving new responsive rules inactive. Rebuilt development assets and recaptured every design.
2. Mobile comparison showed status badges stretching across Family lanes rows. Constrained badges to their intrinsic width.
3. Time-first comparison showed a truncated stock heading and a crowded Later today row. Stacked the narrow stock card and gave the schedule row an explicit time/content grid.
4. Final source-to-implementation comparisons found no actionable P0, P1, or P2 differences.

## Findings

No actionable P0, P1, or P2 findings remain.

## Follow-up Polish

- P3: revisit relative experiment performance after enough accounts have tried each layout; no design should replace Current without outcome data.

final result: passed
