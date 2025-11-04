# Authorization Completion Plan

**Created**: November 4, 2025
**Status**: In Progress
**Parent Document**: USER_MANAGEMENT_PLAN.md

## Overview

Complete the authorization implementation for MedTracker by addressing
remaining gaps in controller authorization and policy coverage.

## Current State Assessment

### ✅ Completed Authorization

- **Policies Exist**:
  - `ApplicationPolicy` - Base deny-by-default policy
  - `UserPolicy` - User management authorization
  - `PersonPolicy` - Person access control
  - `CarerRelationshipPolicy` - Carer relationship management
  - `PersonMedicinePolicy` - Non-prescription medicine authorization
  - `PrescriptionPolicy` - Prescription authorization (COMPLETE)
  - `MedicinePolicy` - Medicine database authorization (COMPLETE)

- **Controllers with Full Authorization**:
  - `PeopleController` - All actions protected with authorize and policy_scope
  - `PersonMedicinesController` - Full authorization including custom actions
  - `Admin::UsersController` - Admin-only access
  - `PrescriptionsController` - **FULLY AUTHORIZED** (all actions protected)
  - `MedicinesController` - **FULLY AUTHORIZED** (all actions protected)

### ❌ Missing Authorization

1. **DashboardController** - No authorization checks
   - Shows all people and prescriptions without scoping
   - No policy file exists

2. **MedicationTakesController** - No authorization checks
   - Anyone can record medication takes for any prescription
   - No policy file exists

3. **Other Controllers** - Need audit:
   - `TakeMedicinesController`
   - `PasswordsController`
   - `SessionsController` (public, but verify)
   - `UsersController` (signup, but verify)
   - `HomeController` (public, but verify)
   - `PwaController` (public, but verify)

## Detailed Implementation Plan

### Task 1: Create MedicationTakePolicy

**Objective**: Define authorization rules for recording medication takes

**Policy Rules**:

- `create?` - Who can record a medication take:
  - Administrator (always)
  - Doctor (always)
  - Nurse (always)
  - Carer (if assigned to patient)
  - Parent (if patient is their minor child)
  - Adult patient (for their own medications)

**Implementation Steps**:

1. Create `app/policies/medication_take_policy.rb`
2. Implement policy methods:
   - `create?` - Primary authorization method
   - `new?` - Alias to create?
   - Helper methods for role checks
3. Implement `Scope` class:
   - Admins/doctors/nurses see all
   - Carers see takes for assigned patients
   - Parents see takes for their children
   - Patients see their own takes
4. Write comprehensive RSpec tests in `spec/policies/medication_take_policy_spec.rb`

**Acceptance Criteria**:

- Policy file created with all required methods
- All role scenarios covered
- Tests verify each permission rule
- Scope properly filters records by role

### Task 2: Add Authorization to MedicationTakesController

**Objective**: Protect medication take recording with policy checks

**Implementation Steps**:

1. Add `authorize` call in `create` action
2. Use `policy_scope(Prescription)` in `set_prescription`
3. Add error handling for unauthorized access
4. Update system tests to verify authorization

**Code Changes**:

```ruby
# app/controllers/medication_takes_controller.rb
def create
  @medication_take = @prescription.medication_takes.build(medication_take_params)
  authorize @medication_take  # Add this line

  # ... rest of method
end

private

def set_prescription
  @prescription = policy_scope(Prescription).find(params[:prescription_id])
end
```

**Acceptance Criteria**:

- Unauthorized users cannot record medication takes
- Policy is enforced on all actions
- Error handling works correctly
- System tests verify authorization

### Task 3: Create DashboardPolicy

**Objective**: Define who can access the dashboard and what they see

**Policy Rules**:

- `index?` - Who can access dashboard:
  - All authenticated users (but scoped data)

**Scope Rules**:

- Admins/doctors/nurses: See all people and prescriptions
- Carers: See assigned patients and their prescriptions
- Parents: See their children and their prescriptions
- Patients: See only themselves

**Implementation Steps**:

1. Create `app/policies/dashboard_policy.rb`
2. Implement headless policy (no record, just user)
3. Create helper methods for scoping people and prescriptions
4. Write RSpec tests

**Acceptance Criteria**:

- Policy created with proper scoping logic
- Tests verify each role sees correct data
- Scope methods reusable in controller

### Task 4: Add Authorization to DashboardController

**Objective**: Scope dashboard data by user role

**Implementation Steps**:

1. Add `authorize :dashboard, :index?` to index action
2. Replace `Person.all` with scoped query based on role
3. Replace `Prescription.where(active: true)` with scoped query
4. Create private methods for data scoping
5. Update system tests

**Code Changes**:

```ruby
# app/controllers/dashboard_controller.rb
def index
  authorize :dashboard, :index?

  people = scoped_people
  active_prescriptions = scoped_prescriptions
  upcoming_prescriptions = active_prescriptions.group_by(&:person)

  render Components::Dashboard::IndexView.new(
    people: people,
    active_prescriptions: active_prescriptions,
    upcoming_prescriptions: upcoming_prescriptions,
    url_helpers: self
  )
end

private

def scoped_people
  if current_user.administrator? || current_user.doctor? || current_user.nurse?
    Person.includes(:user, prescriptions: :medicine).all
  elsif current_user.carer?
    current_user.person.patients.includes(:user, prescriptions: :medicine)
  elsif current_user.parent?
    current_user.person.patients.where(person_type: :child)
                .includes(:user, prescriptions: :medicine)
  else
    Person.where(id: current_user.person.id)
                .includes(:user, prescriptions: :medicine)
  end
end

def scoped_prescriptions
  person_ids = scoped_people.pluck(:id)
  Prescription.where(active: true, person_id: person_ids)
              .includes(person: :user, medicine: [])
end
```

**Acceptance Criteria**:

- Dashboard shows only authorized data
- Each role sees appropriate people/prescriptions
- System tests verify scoping for each role
- No N+1 queries

### Task 5: Audit Remaining Controllers

**Objective**: Verify all controllers have appropriate authorization

**Controllers to Audit**:

1. **TakeMedicinesController** - Check if authorization needed
2. **PasswordsController** - Verify public access is intentional
3. **SessionsController** - Verify public access is intentional
4. **UsersController** - Verify signup is appropriately protected
5. **HomeController** - Verify public access is intentional
6. **PwaController** - Verify public access is intentional

**For Each Controller**:

- Read the controller code
- Identify sensitive actions
- Determine if authorization is needed
- Add authorization if required
- Document decision in this plan

**Acceptance Criteria**:

- All controllers reviewed
- Authorization added where needed
- Decisions documented
- Tests updated

### Task 6: Write System Authorization Tests

**Objective**: End-to-end tests for authorization scenarios

**Test Scenarios**:

1. **MedicationTakes Authorization**:
   - Admin can record any medication take
   - Doctor can record any medication take
   - Nurse can record any medication take
   - Carer can record for assigned patients only
   - Parent can record for their children only
   - Adult patient can record their own only
   - Unauthorized users blocked

2. **Dashboard Authorization**:
   - Admin sees all people and prescriptions
   - Doctor sees all people and prescriptions
   - Nurse sees all people and prescriptions
   - Carer sees only assigned patients
   - Parent sees only their children
   - Patient sees only themselves

**Implementation**:

- Create `spec/features/medication_takes_authorization_spec.rb`
- Create `spec/features/dashboard_authorization_spec.rb`
- Use fixtures for different user roles
- Test both positive and negative cases

**Acceptance Criteria**:

- All role scenarios tested
- Both allowed and denied access verified
- Tests use realistic fixtures
- Tests are maintainable

## Implementation Order

### Phase 1: MedicationTakes (High Priority)

**Estimated Time**: 2-3 hours

1. Create MedicationTakePolicy (30 min)
2. Write MedicationTakePolicy tests (45 min)
3. Add authorization to MedicationTakesController (30 min)
4. Write system authorization tests (45 min)

### Phase 2: Dashboard (High Priority)

**Estimated Time**: 3-4 hours

1. Create DashboardPolicy (45 min)
2. Write DashboardPolicy tests (1 hour)
3. Add authorization and scoping to DashboardController (1.5 hours)
4. Write system authorization tests (1 hour)

### Phase 3: Controller Audit (Medium Priority)

**Estimated Time**: 2-3 hours

1. Audit TakeMedicinesController (30 min)
2. Audit public controllers (1 hour)
3. Add authorization where needed (1 hour)
4. Update tests (30 min)

## Testing Strategy

### Unit Tests (RSpec)

- Test each policy method independently
- Test scope filtering for each role
- Test edge cases (nil user, missing associations)
- Aim for 100% coverage

### System Tests (Capybara)

- Test complete user flows
- Verify UI reflects authorization
- Test error messages and redirects
- Use realistic fixtures

### Test Data Requirements

**Fixtures Needed**:

- Users with each role (administrator, doctor, nurse, carer, parent)
- People with each person_type
- CarerRelationships linking carers to patients
- Parent-child relationships
- Prescriptions for different people
- MedicationTakes for different prescriptions

## Success Criteria

### Functional Requirements

- All controller actions protected by policies
- Data scoped appropriately by role
- Unauthorized access blocked with clear messages
- No security bypasses possible

### Quality Requirements

- 100% test coverage for policies
- System tests for all authorization scenarios
- No N+1 queries in scoped data
- Clear, maintainable code

### Documentation Requirements

- Policy rules documented in code comments
- Authorization decisions documented
- USER_MANAGEMENT_PLAN.md updated
- This plan kept current

## Risks and Mitigation

### Risk 1: Breaking Existing Functionality

**Mitigation**:

- Run full test suite before and after changes
- Use TDD approach (write tests first)
- Make small, incremental changes
- Test in development environment

### Risk 2: Performance Impact from Scoping

**Mitigation**:

- Use eager loading (includes/joins)
- Add database indexes if needed
- Monitor query performance
- Use bullet gem to detect N+1 queries

### Risk 3: Complex Permission Logic

**Mitigation**:

- Keep policy methods simple and focused
- Extract complex logic to private methods
- Document edge cases
- Write comprehensive tests

## Progress Tracking

### Task Checklist

- [ ] Task 1: Create MedicationTakePolicy
  - [ ] Create policy file
  - [ ] Implement policy methods
  - [ ] Implement Scope class
  - [ ] Write RSpec tests
- [ ] Task 2: Add Authorization to MedicationTakesController
  - [ ] Add authorize calls
  - [ ] Use policy_scope
  - [ ] Add error handling
  - [ ] Update system tests
- [ ] Task 3: Create DashboardPolicy
  - [ ] Create policy file
  - [ ] Implement policy methods
  - [ ] Implement scoping helpers
  - [ ] Write RSpec tests
- [ ] Task 4: Add Authorization to DashboardController
  - [ ] Add authorize call
  - [ ] Implement scoped_people
  - [ ] Implement scoped_prescriptions
  - [ ] Update system tests
- [ ] Task 5: Audit Remaining Controllers
  - [ ] Audit TakeMedicinesController
  - [ ] Audit PasswordsController
  - [ ] Audit SessionsController
  - [ ] Audit UsersController
  - [ ] Audit HomeController
  - [ ] Audit PwaController
- [ ] Task 6: Write System Authorization Tests
  - [ ] MedicationTakes authorization spec
  - [ ] Dashboard authorization spec

### Completion Criteria

- All tasks completed
- All tests passing
- No authorization gaps
- USER_MANAGEMENT_PLAN.md updated
- Code reviewed and approved

## Next Steps After Completion

Once authorization is complete:

1. Update USER_MANAGEMENT_PLAN.md to mark Phase 1 as 100% complete
2. Begin Phase 2: Admin Interface implementation
3. Consider adding audit logging for authorization failures
4. Review and optimize query performance
