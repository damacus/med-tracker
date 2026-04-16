# Specification: Medication UI Modernization (M3 Implementation)

## Track ID: `m3_medication_ui_20260415`

### 1. Objective
Apply the new Material Design 3 (M3) foundation and component system to the Medication Profile and Administration pages. This validates the M3 components in a high-complexity area and addresses long-standing UI inconsistencies in the project's core feature set.

### 2. Core Requirements
- **Standardize Layout**: Use `m3_input`, `M3::Button`, and `M3::Card` across all medication-related views.
- **Surface Contrast**: Implement the `surface-container-low` background for pages and white `M3::Card` surfaces for content areas.
- **Navigation Consistency**: Standardize action buttons in `Medications::ShowView` and `Medications::AdministrationModal` with M3 variants (`filled`, `tonal`, `outlined`).
- **Typography Fixes**: Replace all invalid `weight: 'black'` calls with M3 typography scales or Tailwind `font-black` utility.
- **Icon Standardization**: Ensure icons within buttons use the correct M3 semantic color tokens.

### 3. Constraints
- **Zero Breakage**: All existing functional tests (RSpec/Capybara) for medications must pass.
- **M3 Compliance**: All changes must strictly follow the defined M3 Foundation and Components tracks.

### 4. Verification
- **System Tests**: `task test` must remain green for all medication-related scenarios.
- **Component Specs**: Update `spec/components/medications/show_view_spec.rb` and `spec/components/medications/administration_modal_spec.rb` to assert M3-specific classes and components.
- **Manual QA**: Visual inspection of the medication flow to ensure consistency.
