# 4. Use Phlex and RubyUI for Views instead of ERB

Date: 2026-04-13

## Status

Accepted

## Context

MedTracker needs consistent, accessible UI components (WCAG 2.1 AA) and type-safe views without context switching.

## Decision

- **View Layer:** Use `Phlex` (`phlex-rails`) for Ruby-based, composable view components.
- **Component Library:** Use `RubyUI` for pre-built, accessible components.
- **Styling:** Use `Tailwind CSS`.
- **Interactivity:** Use `Hotwire` (Turbo + Stimulus).

## Consequences

- Views are plain Ruby classes, easily testable and composable.
- No ERB templates; avoid context switching between Ruby and HTML templates.
- Learning curve for Phlex's Ruby-based UI structure.
- Pre-built accessible components with RubyUI.
