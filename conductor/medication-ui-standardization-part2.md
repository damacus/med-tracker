# Medication UI Standardization Part 2 Plan

## Objective
Fix the styling inconsistencies in the Medication Action Buttons grid, and update the Inventory Medicine view page (Person Medication view) so its header seamlessly matches the main Medication Profile.

## Implementation Steps

### 1. Fix Action Button Consistency
- **File**: `app/components/medications/show_view.rb`
- **Change**: 
  - Use a `base_action_classes` variable to store the shared styles: `'w-full py-6 rounded-shape-xl flex items-center justify-center font-bold transition-all shadow-elevation-1 hover:shadow-elevation-2 active:scale-[0.98]'`.
  - Pass the explicit `size: :lg` parameter to `Button.new` for the "Log Administration" button to fix the smaller text.
  - Set the "Log Administration" icon color to `text-success-foreground` (or just remove `text-white`) to ensure it's visible on the green background.
  - Remove the `@utility btn-action` from `app/assets/tailwind/application.css` as it's causing compilation/rendering issues in this environment.
  - Ensure all four action buttons are rendered within a single `div(class: 'grid grid-cols-2 gap-3')` block.

### 2. Fix Icon Compatibility
- **Files**: `app/components/icons/clock.rb`, `app/components/icons/activity.rb`, `app/components/icons/log_out.rb`, `app/components/icons/home.rb`
- **Change**: Refactor any `<polyline>` tags to equivalent `<path>` tags. This ensures maximum compatibility across all Phlex/Tailwind rendering environments where `<polyline>` might be stripped.

### 3. Match Inventory and Profile Headers
- **Files**: `app/components/medications/index_view.rb`, `app/components/medications/show_view.rb`
- **Change**: Ensure both files use the exact same markup for their headers:
  - Container: `div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 pb-8 border-b border-border')`
  - Pill Icon: `w-20 h-20 rounded-shape-xl bg-primary/10 flex items-center justify-center text-primary shadow-inner`
  - Eyebrow: `Text(size: '2', weight: 'bold', class: 'uppercase tracking-[0.2em] opacity-40 block mb-1 font-black')`
  - Title: `Heading(level: 1, size: '8', class: 'font-black tracking-tight')`

## Verification
- Run `task rubocop` to ensure no formatting issues.
- Verify `task test` passes.
- Manually run `task dev:up` to visually confirm that the action buttons render uniformly (with correct text size, icons, and background colors) and that the inventory page header aligns with the new design tokens.