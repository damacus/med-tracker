# Design system and theming

MedTracker combines local RubyUI components with a small Material 3 layer.
Phlex views render both component families. Tailwind CSS maps the design tokens
to utility classes.

## Source of truth

Use these files when changing the interface:

- `app/assets/tailwind/application.css` defines colour, shape, elevation,
  motion, and typography tokens.
- `app/components/ruby_ui/` contains the locally installed RubyUI components.
- `app/components/m3/` contains MedTracker's Material 3 specialisations.
- `app/components/m3_helpers.rb` exposes the `m3_*` rendering helpers.

The local RubyUI files are application code. They can differ from the installed
gem, so compare them with the locked RubyUI version before replacing or
regenerating a component.

## Choose a component

Use an existing RubyUI component when it already provides the required
structure and behaviour. Use the Material 3 wrapper when the interface needs
MedTracker's standard visual treatment.

The Material 3 layer currently provides helpers for:

- buttons, links, badges, and cards;
- headings and body text;
- inputs, selects, and selectable options.

Do not recreate an existing component with raw HTML and utility classes. A
specialised view can add layout classes around a component when no shared
wrapper fits.

## Semantic colours

Use semantic token utilities instead of literal colours. Common roles include:

- `primary`, `on-primary`, `primary-container`, and `on-primary-container`;
- `secondary-container` and `on-secondary-container`;
- `error`, `error-container`, and their matching `on-*` tokens;
- `warning-container` and `success-container`;
- `surface`, `on-surface`, `outline`, and `outline-variant`.

The surface container scale runs from `surface-container-lowest` to
`surface-container-highest`. Use it to show hierarchy without introducing a
new colour.

Do not add literal hex, HSL, or generic Tailwind palette colours to a view. Add
or adjust a semantic token when the design needs a new shared meaning.

## Shape and elevation

Use the shared shape utilities:

- `rounded-shape-xs`
- `rounded-shape-sm`
- `rounded-shape-md`
- `rounded-shape-lg`
- `rounded-shape-xl`
- `rounded-shape-full`

Use `shadow-elevation-0` through `shadow-elevation-5` for elevation. Choose the
lowest level that communicates the required hierarchy.

## Interaction states

The `state-layer` utility supplies the shared hover, active, and keyboard-focus
overlay. The Material 3 button and link components include it. Do not add a
second state layer to those components.

Keep the visible focus ring supplied by the component. Preserve disabled and
`aria-disabled` behaviour when changing variants or classes.

## Material component examples

```ruby
m3_button(variant: :filled) { "Save changes" }
m3_link(href: person_path(person), variant: :outlined) { "View person" }
```

Button variants include `filled`, `tonal`, `elevated`, `outlined`, `text`, and
destructive treatments. Link variants include `filled`, `tonal`, `outlined`,
and `text`.

```ruby
m3_card(variant: :elevated) do
  m3_card_header do
    m3_card_title { "Inventory status" }
    m3_card_description { "Current stock at this location" }
  end
  m3_card_content { "..." }
end
```

Card variants are `elevated`, `outlined`, and `filled`.

```ruby
m3_heading(variant: :headline_small, level: 2) { "Medication" }
m3_text(variant: :body_medium) { "Take with food." }
```

Choose the HTML heading level from the page structure. The visual variant does
not determine the semantic level.

## Change the theme

Update the OKLCH token values in `app/assets/tailwind/application.css`. Check
both light and dark themes after a colour change. Confirm text and control
contrast, visible focus states, and destructive action styling.

For visible interface changes, use the real browser flow at desktop width and
at 390 by 844 pixels. Save the required review screenshots under
`docs/screenshots/`.
