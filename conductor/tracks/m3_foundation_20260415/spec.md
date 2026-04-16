# Specification: M3 Foundation (OKLCH & Semantic Tokens)

## Track ID: `m3_foundation_20260415`

### 1. Objective
Refactor the global CSS theme to utilize Material Design 3 (M3) semantic token naming and OKLCH color spaces. This provides a robust, accessible foundation for all future UI components while eliminating Shadcn/RubyUI-style "muted" and "accent" naming conventions.

### 2. Core Requirements
- **OKLCH Migration**: All CSS color variables in `application.css` must use `oklch()` for better perceptual uniformity and accessibility.
- **Semantic Renaming**: 
    - Map all instances of `--muted` to `--secondary-container`.
    - Map all instances of `--accent` to `--tertiary-container`.
    - Map their respective `-foreground` tokens to `--on-*` tokens (e.g., `--on-secondary-container`).
- **Surface Hierarchy**: Define M3 surface levels (`--surface`, `--surface-container-low`, `--surface-container`, `--surface-container-high`).
- **State Layers**: Implement a `.state-layer` utility using Tailwind v4 that handles hover/focus/active overlays (0.08, 0.12 opacity).

### 3. Constraints
- **Zero UI Breakage**: Existing components must still render correctly, even if they are using the old class names (until Phase 4). We will achieve this by aliasing the old token names to the new M3 ones.
- **Accessibility**: All M3 color pairs must meet WCAG AA contrast standards.

### 4. Verification
- **CSS Compilation**: `task dev:up` must build without errors.
- **Theme Tests**: Update or create tests in `spec/lib/theme_token_contract_spec.rb` to verify the presence and format of new tokens.
