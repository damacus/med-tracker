# Track: Standardize icon usage with Lucide icons

## Summary
Standardize icon usage across the application by using Lucide icons exclusively, ensuring a consistent structure, and leveraging RubyUI's attribute merging for easier styling.

## Context
The application currently has multiple icon components in `app/components/icons/` with inconsistent inheritance and attribute handling. Some icons are not from the Lucide library (e.g., Heroicons are used for `Key` and `XCircle`).

## Objectives
- Standardize all icon components to inherit from `Components::Icons::Base`.
- Use Lucide icons exclusively.
- Implement automatic class application for Lucide icons.
- Cleanup redundant icon classes in the codebase.
