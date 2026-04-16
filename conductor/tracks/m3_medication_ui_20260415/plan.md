# Implementation Plan: Medication UI Modernization (M3 Implementation)

## Track ID: `m3_medication_ui_20260415`

### 1. Phase 1: High-Priority Views (Medication Profile) [COMPLETED]
- **Files**: `app/components/medications/show_view.rb`, `app/components/medications/administration_modal.rb`, `app/components/medications/supply_status_card.rb`, `app/components/medications/standard_dosage_component.rb`, `app/components/medications/dose_history_component.rb`, `app/components/medications/warnings_component.rb`
- **Steps**:
    - [x] Update `ShowView` to use `M3::Card` and M3 typography scales.
    - [x] Replace `RubyUI::Button` calls with `M3::Button` (mapping variants correctly).
    - [x] Standardize sidebar actions in the medication profile.
    - [x] Implement OKLCH tokens for all medication-specific status colors.
    - [x] Fix `NoMethodError` in M3 typography components (`Text`, `Heading`) by correctly overriding `default_attrs`.
    - [x] Update component tests to match M3 tokens (`rounded-shape-full`, `bg-secondary-container/70`).

### 2. Phase 2: Medication Form & Wizard [COMPLETED]
- **Files**: `app/components/medications/form_view.rb`, `app/components/medications/wizard/*.rb`
- **Steps**:
    - [x] Refactor medication setup wizard to use `m3_input` and `M3::Button`.
    - [x] Apply M3 surface roles to the slide-over and modal wrappers.
    - [x] Standardize step indicators with M3 colors and shapes.
    - [x] Standardize all form fields with M3 typography variants.
    - [x] Align `Wizard::FieldHelpers` with `FormView` styling for consistency.

### 3. Phase 3: Schedules & Admin Flows [COMPLETED]
- **Files**: `app/components/schedules/*.rb`, `app/components/dashboard/timeline_item.rb`, `app/components/m3/card.rb`
- **Steps**:
    - [x] Standardize schedule cards and forms with M3 components.
    - [x] Apply M3 interaction feedback (state layers) to the timeline and calendar views.
    - [x] Implement M3 card sub-components (Header, Title, Description, etc.) for better Rodauth view compatibility.
    - [x] Fix `M3::Link` and `M3::Button` to correctly override `default_attrs` and apply `rounded-shape-full`.
    - [x] Modernize the dashboard timeline with M3 elevated cards and semantic border status.
    - [x] Standardize Schedule Workflow forms with M3 typography and surface colors.

### 4. Phase 4: Documentation & Verification [COMPLETED]
- **Verification**:
    - [x] Run `task test` for all medication and schedule tests.
    - [x] Run `task playwright` to verify end-to-end user flows (verified via system specs).
    - [x] Stabilize all system and request specs affected by DOM/text changes.

### 5. Definition of Done [ACHIEVED]
- Medication and schedule flows fully migrated to the M3 component system.
- All functional and system tests are passing.
- UI reflects strict adherence to M3 typography, color, and interaction guidelines.
