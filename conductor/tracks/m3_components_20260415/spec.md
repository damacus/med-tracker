# Specification: M3 Core Components (Wrappers & State Layers)

## Track ID: `m3_components_20260415`

### 1. Objective
Establish a new `M3::` component library by creating wrappers around the existing `RubyUI::` components. This allows us to strictly enforce Material Design 3 (M3) interaction patterns (like State Layers and standard rounded shapes) and variant naming conventions (`filled`, `tonal`, `elevated`, `outlined`, `text`) without losing the ability to upgrade base components.

### 2. Core Requirements
- **M3 Namespace**: All new components must live in `app/components/m3/`.
- **Component Wrappers**:
    - `M3::Button`: Wrap `RubyUI::Button` to support M3 variants and inject the `.state-layer` and `.rounded-full` classes.
    - `M3::Card`: Wrap `RubyUI::Card` to use M3 elevation and surface roles.
    - `M3::Input`: Wrap `RubyUI::Input` to standardized M3 text input styles.
- **State Layers**: Components must utilize the `.state-layer` utility defined in the Foundation phase for all interactive states.
- **Upgradability**: Keep modifications to `app/components/ruby_ui/` to an absolute minimum, pushing design logic into the `M3::` wrappers instead.

### 3. Constraints
- **Zero Breakage**: Existing `RubyUI::` components must continue to function.
- **Naming**: M3 variant names must be used (e.g., `variant: :tonal` instead of `variant: :secondary`).

### 4. Verification
- **Component Specs**: Each new M3 component must have a corresponding RSpec component test in `spec/components/m3/`.
- **UI Audit**: Manually verify component states (hover, focus, active) in the browser.
