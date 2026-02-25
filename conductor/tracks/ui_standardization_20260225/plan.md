# Implementation Plan: UI Standardization (Icons)

## Track ID: `ui_standardization_20260225`

### 1. Refactor `Components::Icons::Base`
- Inherit from `::RubyUI::Base`.
- Add `DEFAULT_SIZE = 16`.
- Define `default_attrs` with Lucide standard classes and SVG attributes.

### 2. Standardize existing icons
- Update all icons to inherit from `Base`.
- Remove manual SVG attributes and redundant logic.

### 3. Replace non-Lucide icons
- Update `Key` and `XCircle` to use Lucide SVG paths.

### 4. Cleanup usage
- Remove redundant `lucide` and `lucide-<name>` classes from views and components.
- Standardize sizing using the `size:` parameter or Tailwind classes.
- Update `XCircle` to use official Lucide `circle-x` path.
- Use `PlusCircle` for "Add Medicine" on Dashboard.

### 5. RuboCop Configuration
- Exclude `app/components/icons/**/*` from `Layout/LineLength` check to accommodate long SVG paths.

### 6. Verification
- Manual verification and automated tests.
