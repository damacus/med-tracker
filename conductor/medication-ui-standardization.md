# Medication Page UI Standardization Plan

## Objective
Standardize the visual design of the Medication Profile page to address inconsistencies in backgrounds, border radiuses, typography, and icon readability.

## Implementation Steps

### 1. Global Background Contrast
- **File**: `app/views/layouts/application.html.erb`
- **Change**: Update the `<body class="bg-background">` to `<body class="bg-surface-container-low text-foreground">`. This subtle off-white background will allow white cards to stand out and eliminates the "shit grey" appearance when cards blend into a white page.

### 2. Standardize Card Component
- **File**: `app/components/ruby_ui/card/card.rb`
- **Change**: Update `default_attrs` to use `bg-card` (pure white) instead of `bg-surface-container-low`. This ensures all cards across the application have a clean, white background that contrasts with the new page background.

### 3. Standardize Link Corners
- **File**: `app/components/ruby_ui/link/link.rb`
- **Change**: Replace `rounded-2xl` with `rounded-shape-xl` (28px) in `BASE_CLASSES` to match the standard border-radius used by the `Button` and `Card` components.

### 4. Cleanup Medication Show View
- **File**: `app/components/medications/show_view.rb`
- **Typography Fixes**:
  - Remove the invalid `weight: 'black'` property from `Text` component calls (which only supports up to `bold`) and replace it with the `font-black` utility class.
  - Standardize eyebrow text (e.g., "MEDICATION PROFILE") to use a consistent opacity and weight.
- **Button Standardization**:
  - Remove hardcoded `rounded-2xl` overrides from action buttons (like "Log Administration") to inherit the standard `rounded-shape-xl`.
- **Icon Readability**:
  - Ensure all icons within outline buttons explicitly use `text-primary` for visibility.

### 5. Cleanup Refill Modal Action
- **File**: `app/components/medications/refill_modal.rb`
- **Change**: Ensure the refill button trigger inherits the correct background (`bg-surface-container-low` to `bg-card` or defaults) and rounding classes to match the other sidebar actions.

## Verification
- Run `task rubocop` to ensure no linting offenses.
- Run `task test` to verify no components or system tests break due to class changes.
- Launch `task dev:up` and visually inspect the medication profile page to ensure the background contrast, rounded corners, and icons meet the requested standard.