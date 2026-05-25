## 2025-05-25 - ARIA Labels on Icon-Only RubyUI Components
**Learning:** When adding accessibility to icon-only buttons built with RubyUI's `m3_link` or `m3_button`, standard `<span class="sr-only">` elements inside the block are redundant if the wrapper supports `aria_label`.
**Action:** Move the accessible name to the `aria_label:` keyword argument on the component wrapper and explicitly pass `aria_hidden: 'true'` to the `Icons::` component to hide the SVG graphic from screen readers.
