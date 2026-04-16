# Implementation Plan: M3 Core Components (Wrappers & State Layers)

## Track ID: `m3_components_20260415`

### 1. Phase 1: M3 Component Infrastructure
- **Directory**: `app/components/m3/`
- **Steps**:
    - Create a base `M3::Base` component inheriting from `RubyUI::Base`.
    - Implement a centralized method for mapping M3 variants to Tailwind classes.

### 2. Phase 2: Building Core M3 Wrappers
- **M3::Button**: 
    - Wrap `RubyUI::Button`.
    - Map M3 variants (`:filled`, `:tonal`, `:elevated`, `:outlined`, `:text`) to internal classes.
    - Inject `.state-layer` and `.rounded-full`.
- **M3::Card**: 
    - Wrap `RubyUI::Card`.
    - Standardize M3 elevation shadows (`shadow-elevation-1` through `shadow-elevation-5`).
- **M3::Input**: 
    - Wrap `RubyUI::Input`.
    - Standardize M3 focus rings and label placement.

### 3. Phase 3: Component Verification & Docs
- **File**: `docs/theming.md`
- **Steps**:
    - Add examples of new `M3::` component usage.
    - Document the mapping from `RubyUI` variants to `M3` variants.
- **Verification**:
    - Create and run `spec/components/m3/button_spec.rb`, `spec/components/m3/card_spec.rb`, and `spec/components/m3/input_spec.rb`.
    - Ensure all tests pass.

### 4. Definition of Done
- `M3::Button`, `M3::Card`, and `M3::Input` are implemented and tested.
- All new components strictly follow M3 design and interaction guidelines.
- Upgradability strategy is preserved (no modification to core `RubyUI` components).
