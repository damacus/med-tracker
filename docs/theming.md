# Theming and Styling

MedTracker uses **RubyUI** (Phlex components) and **Tailwind CSS v4** for styling. This document outlines how to manage themes and ensure consistent styling across the application.

## Core Principles

- **Utility First**: Leverage Tailwind CSS utility classes whenever possible.
- **Component-Based**: Use `RubyUI` components for standard UI elements (buttons, inputs, cards, etc.).
- **Variables for Tokens**: Use CSS variables for colors and spacing to support theming and dark mode.
- **Accessibility First**: All styles must meet the criteria in [Accessibility Guidelines](accessibility.md).

## CSS Variables

The application's theme is defined in `app/assets/tailwind/application.css` using CSS variables in the `:root` and `.dark` blocks.

### Primary Colors

- `--primary`: The main brand color (used for primary buttons, links, etc.).
- `--primary-foreground`: Contrast color for text on primary backgrounds.
- `--secondary`: Background for secondary elements.
- `--secondary-foreground`: Text color for secondary elements.

### Semantic Colors

- `--destructive`: Used for dangerous actions (delete, remove).
- `--warning`: Used for cautionary alerts.
- `--success`: Used for positive feedback.

## Button Styling

All buttons in the application should use the `RubyUI::Button` component to ensure consistency and accessibility.

### Usage

```ruby
# In a Phlex view or component
render RubyUI::Button.new(variant: :primary, size: :md) { "Save Changes" }
```

### Variants

- `:primary` (default): Main actions.
- `:secondary`: Alternative actions.
- `:outline`: Subtle actions with a border.
- `:ghost`: Low-emphasis actions.
- `:destructive`: Dangerous actions.
- `:link`: Actions styled as text links.

### Accessibility Requirements

Buttons must follow the [WCAG 2.2 Target Size](accessibility.md#target-size-requirements-sc-258) standards:
- **Minimum size**: 24×24px (standard for `sm`, `md`, `lg`, `xl` sizes in `RubyUI::Button`).
- **Recommended touch target**: 44×44px (use `size: :xl` or `min-h-[44px]`).

## Changing the Theme

To update the application's look and feel:

1.  Open `app/assets/tailwind/application.css`.
2.  Modify the `oklch` or `hsl` values in the `:root` block for light mode.
3.  Modify the corresponding values in the `.dark` block for dark mode.
4.  Ensure the `@theme inline` block correctly maps these variables to Tailwind theme colors.

Example:
```css
:root {
  --primary: oklch(0.5 0.2 240); /* New primary color */
}
```

## CSS Architecture

- **Base Styles**: Reset and standard element styles live in the `@layer base` block.
- **Components**: Shared class patterns (like legacy `.btn` if still needed) live in the `@layer components` block.
- **Utilities**: Custom utility classes live in the `@layer utilities` block.

**Note**: Prefer Phlex components over creating new CSS classes in `@layer components`.
