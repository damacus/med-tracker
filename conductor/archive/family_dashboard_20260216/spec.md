# Specification: Unified Family Dashboard

## Objective
Create a central, unified dashboard that provides parents with a clear overview of medication schedules for themselves and their dependents.

## User Stories
- **As a Parent**, I want to see a single timeline of all medications due today for my whole family, so I don't miss any doses.
- **As a Parent**, I want to see which doses were already taken and by whom, so I can ensure everyone is safe.
- **As a Parent**, I want to be alerted to "Missed" doses that were scheduled but not recorded as taken.

## Functional Requirements
- **Family View:** Aggregate schedules from the current user (`Account`) and all their associated `Person` records (dependents).
- **Today's Timeline:** Display a chronological list of doses for the current 24-hour period.
- **Dose Status:**
    - **Taken:** Recorded via a `MedicationTake` entry.
    - **Upcoming:** Scheduled in the future based on `Prescription` or `PersonMedicine` rules.
    - **Missed:** Scheduled in the past but no `MedicationTake` recorded.
- **Quick Action:** Ability to record a dose directly from the dashboard.
- **Safety Warnings:** Visual indicators for daily limit nearing or reached.

## Technical Details
- **Query Logic:** A service object `FamilyDashboard::ScheduleQuery` will be responsible for merging `Prescription` and `PersonMedicine` schedules and correlating them with `MedicationTake` records.
- **Frontend:** Implement using Phlex components in `app/components/dashboard/`.
- **Mobile UX:** Prioritize a vertical, one-handed scrolling timeline.
- **Interactivity:** Use Stimulus for quick-logging actions without full page reloads.

## Success Criteria
- [ ] Dashboard displays doses for both the user and their dependents.
- [ ] Doses are correctly categorized as Taken, Upcoming, or Missed.
- [ ] The interface is responsive and works well on mobile devices.
- [ ] Test coverage for the query logic and UI components is >80%.
