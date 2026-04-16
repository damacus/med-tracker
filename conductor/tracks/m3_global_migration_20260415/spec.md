# Specification: Global M3 Application-Wide Migration

## Track ID: `m3_global_migration_20260415`

### 1. Objective
Complete the project-wide transition to Material Design 3 (M3) by migrating all remaining `RubyUI::` and Shadcn-style components, tokens, and layouts to the new `M3::` system. This track replaces the "Big Bang" approach with a final, systematic cleanup and ensures overall UI consistency across the entire application.

### 2. Core Requirements
- **Global Search & Replace**: Systematically replace remaining `RubyUI::Button`, `RubyUI::Card`, `RubyUI::Input`, etc., with their `M3::` equivalents.
- **Legacy Cleanup**: 
    - Remove backward compatibility aliases for `--muted` and `--accent` from `application.css`.
    - Delete or deprecate unused RubyUI classes that do not align with M3.
- **Layout Consistency**: Ensure all application surfaces (Dashboard, People, Profile, Admin) consistently utilize the defined M3 surface hierarchy and state layers.

### 3. Constraints
- **Low Risk**: This migration must be broken into smaller commits to prevent regression and facilitate testing.
- **Strict Adherence**: Every UI change must conform to the M3 Foundation and Components tracks already established.

### 4. Verification
- **Full Test Suite**: `task test` must pass for all application areas.
- **Quality Gates**: `task rubocop` must return no offenses for the modified files.
- **System-Wide UI Audit**: Manual verification across all major user flows.
