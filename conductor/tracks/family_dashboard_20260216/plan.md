# Implementation Plan: Unified Family Dashboard

## Phase 1: Data Architecture & Querying
Implement the logic to aggregate medication schedules across the care team.

- [x] Task: Conductor - Define `FamilyDashboard::ScheduleQuery` requirements
    - [x] Define the interface for fetching a 24-hour window of doses.
- [x] Task: TDD - `FamilyDashboard::ScheduleQuery`
    - [x] Write tests in `spec/services/family_dashboard/schedule_query_spec.rb`.
    - [x] Verify failure (Red).
- [x] Task: Implement `FamilyDashboard::ScheduleQuery`
    - [x] Aggregate `Prescription` and `PersonMedicine` schedules.
    - [x] Join with `MedicationTake` to determine status.
    - [x] Verify tests pass (Green).
- [ ] Task: Conductor - User Manual Verification 'Data Architecture' (Protocol in workflow.md)

## Phase 2: View Components
Build the Phlex components for the dashboard UI.

- [ ] Task: Create `Dashboard::TimelineItem` Phlex component
    - [ ] Display medicine name, person name, and scheduled time.
    - [ ] Style based on status (Upcoming, Taken, Missed).
- [ ] Task: Create `Dashboard::FamilySummary` Phlex component
    - [ ] Main dashboard layout aggregating timeline items.
- [ ] Task: TDD - Dashboard Components
    - [ ] Write component specs in `spec/components/dashboard/`.
- [ ] Task: Conductor - User Manual Verification 'View Components' (Protocol in workflow.md)

## Phase 3: Integration & Dashboard View
Connect the components to a controller and the main application navigation.

- [ ] Task: Implement `DashboardController#show`
    - [ ] Fetch data using `ScheduleQuery`.
    - [ ] Render `Dashboard::FamilySummary`.
- [ ] Task: Update Root Route
    - [ ] Redirect authenticated users to the dashboard.
- [ ] Task: Add "Record Dose" Quick Action
    - [ ] Implement a Stimulus controller for recording a `MedicationTake` from the timeline.
- [ ] Task: Conductor - User Manual Verification 'Integration' (Protocol in workflow.md)

## Phase 4: Safety & Polishing
Add safety warnings and refine the mobile experience.

- [ ] Task: Add Safety Warnings
    - [ ] Visual indicators for daily limit thresholds.
- [ ] Task: Mobile UI Audit
    - [ ] Ensure all touch targets are >44px and one-handed use is easy.
- [ ] Task: Final Quality Gate Check
    - [ ] Verify >80% coverage and no linting errors.
- [ ] Task: Conductor - User Manual Verification 'Polishing' (Protocol in workflow.md)
