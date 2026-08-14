# ADR 0003: UI Component Library

- Status
  Accepted
- Date
  2025-11-27

## Context

MedTracker needs consistent, accessible views that work well on desktop and
mobile. The project also needs server-rendered HTML, reusable components, and a
small client-side JavaScript surface.

## Decision

Use this view stack:

- **Phlex Rails** for object-oriented views written in Ruby;
- **Tailwind CSS** for utility classes and shared design tokens;
- **RubyUI** for generated Phlex components that become part of this
  repository; and
- **Hotwire** for navigation, partial updates, and small Stimulus controllers.

Application views live under `app/components/`. Generated and adapted RubyUI
components live under `app/components/ruby_ui/`. Their Stimulus controllers
live under `app/javascript/controllers/ruby_ui/`.

RubyUI uses a copy-and-change model. The generated component code is local
application code, so MedTracker owns its accessibility, maintenance, and
upgrade checks. Use `docs/ruby-ui-comparison.md` to compare selected local
component families with the locked gem.

Views must meet the current accessibility rules in
[`docs/accessibility.md`](../accessibility.md). Prefer an existing RubyUI
component before adding another UI primitive.

## Component rules

- Keep components focused on rendering and interaction.
- Pass prepared data into components. Do not query the database from a view.
- Compose larger views from smaller components.
- Use semantic HTML and accessible names, states, errors, and focus behaviour.
- Use Turbo for server updates and Stimulus only for browser behaviour that
  HTML cannot provide alone.

## Consequences

- The UI has one server-rendered component model.
- Shared components provide consistent structure and styling.
- Local RubyUI copies can drift from the locked gem and need explicit review.
- Complex browser interactions may need a small MedTracker Stimulus controller.

## Related documents

- [Accessibility guidelines](../accessibility.md)
- [RubyUI comparison workflow](../ruby-ui-comparison.md)
- [`app/components/base.rb`](../../app/components/base.rb)
- [`app/components/ruby_ui/`](../../app/components/ruby_ui/)
