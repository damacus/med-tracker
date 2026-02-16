# Implementation Plan: Unified Family Dashboard

## Phase 1: Data Architecture & Querying
Implement the logic to aggregate medication schedules across the care team.

- [x] Task: Conductor - Define `FamilyDashboard::ScheduleQuery` requirements
    - [x] Define the interface for fetching a 24-hour window of doses.
- [x] Task: TDD - `FamilyDashboard::ScheduleQuery` [93a7129]
    - [x] Write tests in `spec/services/family_dashboard/schedule_query_spec.rb`.
    - [x] Verify failure (Red).
- [x] Task: Implement `FamilyDashboard::ScheduleQuery` [93a7129]
    - [x] Aggregate `Prescription` and `PersonMedicine` schedules.
    - [x] Join with `MedicationTake` to determine status.
    - [x] Verify tests pass (Green).
- [ ] Task: Conductor - User Manual Verification 'Data Architecture' (Protocol in workflow.md)

## Phase 2: View Components
Build the Phlex components for the dashboard UI.

- [x] Task: Create `Dashboard::TimelineItem` Phlex component [1b998a3]
    - [x] Display medicine name, person name, and scheduled time.
    - [x] Style based on status (Upcoming, Taken, Missed).
- [x] Task: Create `Dashboard::FamilySummary` Phlex component [1b998a3]
    - [x] Main dashboard layout aggregating timeline items.
- [x] Task: TDD - Dashboard Components [1b998a3]
    - [x] Write component specs in `spec/components/dashboard/`.
- [ ] Task: Conductor - User Manual Verification 'View Components' (Protocol in workflow.md)

## Phase 3: Integration & Dashboard View
Connect the components to a controller and the main application navigation.

- [x] Task: Implement `DashboardController#show` [8628a37]
    - [x] Fetch data using `ScheduleQuery`.
    - [x] Render `Dashboard::FamilySummary`.
- [x] Task: Update Root Route [8628a37]
    - [x] Redirect authenticated users to the dashboard.
- [x] Task: Add "Record Dose" Quick Action [8628a37]
    - [x] Implement a Stimulus controller for recording a `MedicationTake` from the timeline.
- [ ] Task: Conductor - User Manual Verification 'Integration' (Protocol in workflow.md)

## Phase 4: Safety & Polishing
Add safety warnings and refine the mobile experience.

- [x] Task: Add Safety Warnings [19d5ae5]
    - [x] Visual indicators for daily limit thresholds.
- [x] Task: Mobile UI Audit [19d5ae5]
    - [x] Ensure all touch targets are >44px and one-handed use is easy.
- [x] Task: Final Quality Gate Check [19d5ae5]
    - [x] Verify >80% coverage and no linting errors.
- [ ] Task: Conductor - User Manual Verification 'Polishing' (Protocol in workflow.md)
