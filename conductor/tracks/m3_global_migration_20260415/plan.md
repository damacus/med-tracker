# Implementation Plan: Global M3 Application-Wide Migration

## Track ID: `m3_global_migration_20260415`

### 1. Phase 1: Dashboard & High-Traffic Areas
- **Files**: `app/components/dashboard/*.rb`, `app/components/people/*.rb`, `app/components/layouts/*.rb`
- **Steps**:
    - Migrate Dashboard cards, metrics, and actions to `M3::`.
    - Apply M3 surface tokens to the global navigation and sidebar components.
    - Update people-related cards and profile views.

### 2. Phase 2: Administrative & Profile Flows
- **Files**: `app/components/admin/*.rb`, `app/views/profiles/*.rb`, `app/views/rodauth/*.rb`
- **Steps**:
    - Update admin dashboards and tables to use `M3::Card` and M3 typography.
    - Refactor profile settings and rodauth (auth) views to follow M3 forms and components.

### 3. Phase 3: Global Cleanup & Final Refactor
- **File**: `app/assets/tailwind/application.css`
- **Steps**:
    - Remove `--muted` and `--accent` aliases.
    - Systematic search for remaining Shadcn-style classes (e.g., `bg-muted`) and replace with M3 tokens (`bg-secondary-container`).
    - Standardize all rounding classes (replace `rounded-xl`, `rounded-2xl` with `rounded-shape-xl` or M3 equivalent).

### 4. Phase 4: Verification & Hand-off
- **Verification**:
    - Final run of `task test` and `task playwright`.
    - Run `task rubocop` for the entire `app/` directory.
    - Manual walkthrough of the entire application to ensure 100% M3 consistency.

### 5. Definition of Done
- 100% of the application migrated to the M3 component and token system.
- No remaining legacy `RubyUI` calls or Shadcn tokens.
- All tests passing and code standards met.
