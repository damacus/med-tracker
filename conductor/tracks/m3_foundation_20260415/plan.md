# Implementation Plan: M3 Foundation (OKLCH & Semantic Tokens)

## Track ID: `m3_foundation_20260415`

### 1. Phase 1: CSS Theme Refactor (OKLCH)
- **File**: `app/assets/tailwind/application.css`
- **Steps**:
    - Update all theme variables (`:root`, `.dark`) to use `oklch()`.
    - Define core M3 semantic roles: `primary`, `on-primary`, `primary-container`, `on-primary-container`, etc.
    - Define surface roles: `surface`, `on-surface`, `surface-container`, `surface-variant`.
    - **Maintain Compatibility**: Add alias variables (e.g., `--muted: var(--secondary-container)`) to prevent breaking existing Shadcn/RubyUI components.

### 2. Phase 2: State Layers & Utilities
- **File**: `app/assets/tailwind/application.css`
- **Steps**:
    - Implement `@utility state-layer` in Tailwind v4.
    - Add hover/focus/active overlays for interaction feedback.
    - Define elevation tokens (`--elevation-1` through `--elevation-5`).

### 3. Phase 3: Documentation & Verification
- **File**: `docs/theming.md`
- **Steps**:
    - Update documentation with new semantic token naming and OKLCH structure.
    - Explain the surface hierarchy and state layer usage.
- **Verification**:
    - `task dev:up` to check for compilation issues.
    - `task test` to ensure no visual regressions in existing system tests.
    - Add new RSpec test `spec/lib/theme_token_contract_spec.rb` to enforce M3 token presence.

### 4. Definition of Done
- All CSS theme variables in OKLCH.
- M3 semantic tokens defined and documented.
- Full backward compatibility for `--muted` and `--accent` aliases.
- No breakage in the current UI after implementation.
