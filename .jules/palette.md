## 2024-05-18 - Component icon-only `aria-label` conventions
**Learning:** Found multiple icon-only components across the Ruby UI layout using `span.sr-only`, but icon-only buttons implemented using `Button` or `Link` components should simply accept the `aria_label:` prop. Ensure it exists.
**Action:** When working with `<Button>` or `<Link>`, default to setting `aria_label` directly on the component rather than nesting an `sr-only` span.
