# ADR 0003: UI Component Library

- Status: Accepted
- Date: 2025-11-27

## Context

MedTracker needs a consistent, accessible, and maintainable UI. The application serves healthcare users who may have varying technical abilities and accessibility needs. UK healthcare compliance (WCAG 2.1 AA) requires accessible interfaces.

Key requirements:

- Consistent design language across all views
- Accessibility compliance (WCAG 2.1 AA)
- Server-side rendering for performance and SEO
- Type-safe, testable components
- Modern, responsive design
- Minimal JavaScript dependencies

## Decision

### View Layer: Phlex

We adopt **Phlex** (`phlex-rails`) as the primary view layer, replacing ERB templates.

**Rationale:**

1. **Type-safe**: Ruby classes with compile-time checking, reducing runtime errors
2. **Composable**: Components are plain Ruby objects, easily composed and tested
3. **Performance**: Faster than ERB due to optimized rendering
4. **Testable**: Components can be unit-tested in isolation
5. **No context switching**: Write views in Ruby, not a template language

**Implementation:**

- `Components::Base` class inheriting from `Phlex::HTML`
- All views as Phlex components under `app/components/`
- Organized by domain: `admin/`, `dashboard/`, `people/`, `prescriptions/`, etc.

### Styling: Tailwind CSS

We adopt **Tailwind CSS** (`tailwindcss-rails`) for styling.

**Rationale:**

1. **Utility-first**: Rapid development without context switching to CSS files
2. **Consistent**: Design tokens enforce consistency
3. **Responsive**: Built-in responsive utilities
4. **Accessible**: Focus states, screen reader utilities included
5. **Rails integration**: First-class Rails support via `tailwindcss-rails`

**Supporting tools:**

- `tailwind_merge` - Intelligent class merging for component variants

### Component Library: RubyUI

We adopt **RubyUI** as our component library, providing pre-built accessible components.

**Rationale:**

1. **Phlex-native**: Built specifically for Phlex, not a wrapper
2. **Accessible**: WCAG 2.1 AA compliant out of the box
3. **Tailwind-based**: Uses Tailwind for styling, matches our stack
4. **Comprehensive**: Buttons, forms, dialogs, tables, navigation, etc.
5. **Customizable**: Easy to extend and theme

**Components used:**

- `Button` - Primary actions with variants (primary, secondary, destructive)
- `Card` - Content containers
- `Table` - Data display with sorting
- `Dialog` - Modal dialogs for confirmations
- `Form` - Form inputs with validation states
- `Badge` - Status indicators
- `Alert` - Notifications and messages

### Frontend Interactivity: Hotwire

We use **Hotwire** (Turbo + Stimulus) for interactivity.

**Rationale:**

1. **Server-first**: Minimal JavaScript, server renders HTML
2. **Progressive enhancement**: Works without JavaScript
3. **Rails default**: First-class Rails support
4. **Simple**: No build step, no complex state management

**Implementation:**

- Turbo Drive for navigation
- Turbo Frames for partial page updates
- Turbo Streams for real-time updates
- Stimulus for JavaScript sprinkles (dialogs, form validation)

## Component Architecture

```text
app/components/
├── base.rb                    # Base class with RubyUI, helpers
├── admin/                     # Admin-specific components
│   ├── dashboard/
│   └── users/
├── dashboard/                 # Main dashboard components
├── layouts/                   # Layout components (navigation, etc.)
├── people/                    # Person management components
├── prescriptions/             # Prescription components
├── person_medicines/          # OTC medicine components
└── ruby_ui/                   # RubyUI component overrides
```

### Component Guidelines

1. **Single responsibility**: One component, one purpose
2. **Props over slots**: Prefer explicit props for data
3. **Composition**: Build complex UIs from simple components
4. **Testing**: Every component has a corresponding spec
5. **Accessibility**: All interactive elements have proper ARIA attributes

## Consequences

### Positive

- Type-safe views catch errors at development time
- Consistent UI through shared components
- Accessible by default with RubyUI
- Fast server-side rendering
- Easy to test components in isolation
- Minimal JavaScript reduces complexity

### Negative

- Learning curve for Phlex syntax
- Fewer community resources compared to ERB
- RubyUI is newer, smaller ecosystem than React/Vue component libraries
- Some complex interactions may require custom Stimulus controllers

### Trade-offs Accepted

- **No SPA**: We accept page reloads for simplicity over SPA complexity
- **Limited animations**: Prefer functional over flashy
- **Server rendering**: Accept slightly higher server load for simpler architecture

## Related Documents

- `app/components/base.rb` - Base component class
- `app/components/ruby_ui/` - RubyUI component customizations
- `.windsurf/rules/code-style-and-architecture.md` - Code style guidelines
