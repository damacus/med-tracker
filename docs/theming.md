# Design System & Theming

MedTracker uses a custom design system based on **Material Design 3 (M3)**, implemented with **Phlex components** and **Tailwind CSS v4**. All colors are defined using the **OKLCH** color space for perceptual uniformity and accessibility.

## Core Principles

- **Material 3 Foundation**: We follow M3 guidelines for surface hierarchy, elevation, and interaction states.
- **OKLCH Semantic Tokens**: Every color is a semantic token (e.g., `primary`, `on-surface-variant`) instead of literal hex codes or generic Tailwind colors.
- **Component-First**: Use `Components::M3` wrappers instead of legacy `RubyUI` components or raw Tailwind classes for consistent UI patterns.
- **State Layers**: Interactive elements use the `.state-layer` utility to handle hover, focus, and pressed states with standard opacity overlays.

## CSS Tokens (OKLCH)

Tokens are defined in `app/assets/tailwind/application.css`. They are automatically mapped to Tailwind colors (e.g., `bg-primary`, `text-on-surface`).

### Semantic Roles
- `primary` / `on-primary`: Main brand color and its contrasting text.
- `secondary-container` / `on-secondary-container`: Subtle backgrounds for secondary UI elements.
- `error` / `error-container` / `on-error-container`: Destructive actions and critical alerts.
- `warning-container` / `on-warning-container`: Cautionary alerts.
- `success-container` / `on-success-container`: Positive feedback.

### Surface Hierarchy
M3 uses a tiered surface system instead of a single "card" background. Use these to create depth:
- `surface-container-lowest`: Main page background.
- `surface-container-low`: Default card background.
- `surface-container`: Default secondary containers.
- `surface-container-high`: Modals and floating elements.
- `surface-container-highest`: Accent containers.

## Components (M3)

Always prefer the `m3_` helpers defined in `Components::M3Helpers`.

### M3::Button
Wraps `RubyUI::Button` with M3 styling and state layers.
- **Variants**: `:filled`, `:tonal`, `:outlined`, `:elevated`, `:text`, `:destructive`.
- **Usage**: `m3_button(variant: :filled) { "Save Changes" }`

### M3::Card
Provides elevated or outlined containers. Inherits from `RubyUI::Card` but enforces M3 tokens.
- **Variants**: `:elevated` (default), `:outlined`, `:filled`.
- **Sub-components**: `m3_card_header`, `m3_card_title`, `m3_card_description`, `m3_card_content`, `m3_card_footer`.
- **Usage**:
  ```ruby
  m3_card(variant: :elevated) do
    m3_card_header do
      m3_card_title { "Inventory Status" }
      m3_card_description { "Current stock levels for this location" }
    end
    m3_card_content { "..." }
  end
  ```

### M3::Typography
Standardizes text sizes and weights according to the M3 scale. This replaces legacy `RubyUI` `size` and `weight` attributes.
- **Heading Variants**: `:display_large/medium/small`, `:headline_large/medium/small`, `:title_large/medium/small`.
- **Text Variants**: `:body_large/medium/small`, `:label_large/medium/small`.
- **Usage**: `m3_heading(variant: :display_small, level: 1) { "Medication Name" }`

### M3::Link
Accessible links that support button-like variants for primary actions.
- **Variants**: `:filled`, `:tonal`, `:outlined`, `:text`.
- **Usage**: `m3_link(href: path, variant: :outlined) { "Edit Details" }`

## Utilities & Styles

### State Layers
The `.state-layer` class adds a `::after` pseudo-element that provides a standard 8% opacity overlay on hover and 12% on press. It should be applied to all custom interactive elements.

### Shape Tokens
Use `rounded-shape-*` classes for consistent corner radiuses:
- `rounded-shape-xs`: 4px
- `rounded-shape-sm`: 8px
- `rounded-shape-md`: 12px
- `rounded-shape-lg`: 16px (Buttons)
- `rounded-shape-xl`: 28px (Main Cards)
- `rounded-shape-full`: 9999px (Pills)

### Elevation
Use `shadow-elevation-*` (0 to 5) for M3-style elevation shadows.

## Changing the Theme
Theme updates must be done by modifying the OKLCH values in `application.css`. 

**Note**: Avoid hardcoding hex colors, `hsl()`, or literal Tailwind colors (e.g., `bg-blue-500`) in views. Always use semantic tokens.

```css
:root {
  --primary: oklch(0.57 0.21 260); /* Update primary using OKLCH */
}
```
