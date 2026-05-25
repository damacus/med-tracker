## 2024-05-18 - Component icon-only `aria-label` conventions
**Learning:** Found multiple icon-only components across the Ruby UI layout using `span.sr-only`, but icon-only buttons implemented using `Button` or `Link` components should simply accept the `aria_label:` prop. Ensure it exists.
**Action:** When working with `<Button>` or `<Link>`, default to setting `aria_label` directly on the component rather than nesting an `sr-only` span.

## 2026-03-05 - Swatch pickers need announced selection state
**Learning:** Visual-only selection rings on theme swatches are not enough; button groups that behave like toggles need `aria-pressed` and a keyboard-visible focus treatment so the current choice is discoverable without sight or a mouse.
**Action:** Treat palette, theme, and icon swatches as toggle buttons by default: set `type="button"`, expose pressed state, and keep a visible `focus-visible` ring on the interactive element itself.
## 2024-03-05 - Add Descriptive Placeholders for Better Guidance
**Learning:** Form fields lacking `placeholder` attributes (such as the Medication form inputs) can cause user friction since it’s not immediately clear what expected format or type of data to provide, especially on larger, empty textareas like `warnings` or `description`. Adding placeholder text does not break existing test locators compared to changing ARIA labels.
**Action:** When working with forms or input fields via `RubyUI::Input` or `RubyUI::Textarea`, add a corresponding translated placeholder where appropriate to guide users effectively without cluttering the UI with additional helper text.

## 2025-05-25 - ARIA Labels on Icon-Only RubyUI Components
**Learning:** When adding accessibility to icon-only buttons built with RubyUI's `m3_link` or `m3_button`, standard `<span class="sr-only">` elements inside the block are redundant if the wrapper supports `aria_label`.
**Action:** Move the accessible name to the `aria_label:` keyword argument on the component wrapper and explicitly pass `aria_hidden: 'true'` to the `Icons::` component to hide the SVG graphic from screen readers.
