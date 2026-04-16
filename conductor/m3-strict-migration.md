# Strict Material 3 Migration & Upgradability Plan

## 1. Background & Motivation
The current UI architecture utilizes a hybrid approach, mixing modern Material 3 tokens (e.g., `--surface-container`, `--elevation-3`) with standard Shadcn/RubyUI-style names (e.g., `--muted`, `--accent`) and traditional Tailwind classes. The goal is to fully embrace Google's Material Design 3 (Material You) by implementing a strict token hierarchy, eliminating design drift, and standardizing component interactions (State Layers) and typography scales. We also need a robust strategy to ensure these customizations survive future upgrades to the base `ruby_ui` components.

## 2. Scope & Impact
*   **CSS Theme:** `app/assets/tailwind/application.css` will be refactored to exclusively use M3 token naming (`--secondary-container`, `--tertiary-container` instead of `--muted`, `--accent`).
*   **Component Architecture:** `RubyUI::Button` and related interactive components in `app/components/ruby_ui/` will be updated to reflect M3 variants (`Filled`, `Tonal`, `Elevated`, `Outlined`, `Text`).
*   **State Layers:** Implement a global state layer CSS utility (CSS-only implementation) for hover, focus, and active states to replace manual `hover:bg-opacity` or `hover:bg-primary-dark` hacks.
*   **Elevation System:** Transition surfaces and cards to utilize the M3 elevation system tied to surface-container colors.
*   **Typography:** The current font stack (`Plus Jakarta Sans` and `Inter`) will be maintained as modern alternatives to the default M3 Roboto font, while adhering to M3 typography scale principles where possible.

## 3. Proposed Solution
1.  **Token Renaming:** 
    *   Map `--muted` to `--secondary-container` and `--muted-foreground` to `--on-secondary-container`.
    *   Map `--accent` to `--tertiary-container` and `--accent-foreground` to `--on-tertiary-container`.
    *   Ensure all color palettes in the CSS file adhere to this M3 structure.
2.  **State Layers:** Create an `@utility state-layer` in Tailwind v4 that automatically applies the correct opacity overlays for interactions (0.08 for hover, 0.12 for focus/active).
3.  **RubyUI Updates:** Modify `app/components/ruby_ui/button.rb` to accept M3 variants (`:filled`, `:tonal`, `:elevated`, `:outlined`, `:text`) and apply the corresponding M3 Tailwind classes. Ensure the shapes default to `--shape-full` (fully rounded) per M3 guidelines.

## 4. RubyUI Upgrade Strategy (Maintaining M3 Styling)
Because `ruby_ui` components are generated/copied into `app/components/ruby_ui/`, running an upgrade generator will overwrite our Material 3 customizations. To prevent losing our M3 styling while keeping up with upstream fixes:

1.  **CSS-First Customization:** Push as much styling as possible into `application.css` via custom `@layer components` or Tailwind `@utility` directives. Upstream `ruby_ui` updates will overwrite Ruby files, but our CSS will remain untouched.
2.  **Base Class Overrides:** Instead of extensively modifying the generated `app/components/ruby_ui/` files directly, we will create `App::UI::` wrappers or monkey-patch the default variants in an initializer. 
    *   *Implementation:* If `ruby_ui` defines `RubyUI::Button`, we will create `app/components/m3/button.rb` that inherits from or wraps `RubyUI::Button`, forcing M3 defaults (like `rounded-full` and `state-layer` classes) and mapping M3 variant names (`:tonal`) back to upstream classes or injecting our own custom classes.
    *   *Result:* When `ruby_ui` is updated, `app/components/ruby_ui/` files are overwritten safely, while `app/components/m3/` retains our design logic.
3.  **Automated Patching (Alternative):** Maintain a `.patch` file of our M3 changes to the `ruby_ui` folder. After upgrading `ruby_ui`, we apply the patch (`git apply m3-rubyui.patch`). If there are conflicts, we manually resolve them.

*Decision:* We will proceed with the **Base Class Overrides / Wrapper** approach (`App::UI::` or `M3::` namespace) for components that require significant Ruby API changes (like variants), and CSS-first for everything else.

## 5. Implementation Plan
### Phase 1: CSS Architecture Refactor
*   Update `app/assets/tailwind/application.css` to rename all Shadcn tokens to M3 semantic roles across all defined themes.
*   Implement the `.state-layer` utility class for standardized interaction feedback.

### Phase 2: Component Architecture Updates
*   Create an `M3::` or `App::UI::` namespace for heavily modified components (like Button).
*   Implement `M3::Button` wrapping `RubyUI::Button` to support `:filled`, `:tonal`, etc., and injecting `.state-layer` and `.rounded-full`.
*   Update `docs/theming.md` to reflect the new M3 token architecture and component wrapper strategy.

### Phase 3: Application-Wide Rollout (Big Bang)
*   Search and replace outdated token usage and button variant calls across the entirety of `app/views/` and `app/components/` in a single pass to ensure UI consistency.
*   Example: Replace `<%= render RubyUI::Button.new(variant: :secondary) %>` with `<%= render M3::Button.new(variant: :tonal) %>`.

## 6. Verification
*   Visual inspection of the UI to confirm buttons exhibit correct M3 state layers and elevation.
*   Run the RSpec and Capybara test suite (`task test`) to ensure component refactoring didn't break any view expectations.
*   Check for Tailwind compilation errors via `task dev:up`.

## 7. Migration & Rollback
*   **Migration:** Executed systematically via a single PR encompassing the CSS changes, wrapper components, and the codebase-wide search-and-replace.
*   **Rollback:** Standard git revert. Since the changes are atomic to the UI layer, a revert will instantly restore the hybrid architecture without affecting business logic.