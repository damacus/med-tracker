# Reports Page UI Consistency Fix

## Changes
* **Refactor Submit Button**: In `app/views/reports/index.rb`, replace the raw HTML `button` tag (used for "Apply Filters") with the standardized `Button` component (`render Button.new(...)`). This ensures visual consistency across the app while retaining all accessibility attributes (like `aria-label`).

## Verification
* **Linting**: Ensure `task rubocop` passes with no errors.
* **Testing**: Ensure `task test` completes successfully.
* **Visual Validation**: Launch the dev server via `task dev:up` and visually inspect the reports page to confirm the new button renders correctly.