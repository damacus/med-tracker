# User Management Implementation Plan

## Current State Analysis

**Last Updated**: November 3, 2025

### ✅ Implemented Features

#### Core Architecture

- **Person/User Separation**: Clean separation between `Person` (demographic) and `User` (authentication) entities
- **Database Schema**: Well-designed schema with:
  - `people` table with person_type enum (patient, carer, nurse, doctor, administrator)
  - `users` table with role enum (administrator, doctor, nurse, carer, parent) linked to people
  - `carer_relationships` table for many-to-many patient-carer relationships
  - Capacity tracking via `has_capacity` boolean on people

#### Models

- **User Model**: Complete with:
  - Secure password authentication (`has_secure_password`)
  - Email validation and normalization
  - Role-based enum (5 roles)
  - Person association with nested attributes
  - Session management

- **Person Model**: Complete with:
  - Person type enum (5 types)
  - Capacity management
  - Carer/patient relationships (bidirectional)
  - Age calculation
  - Email validation

- **CarerRelationship Model**: Functional with:
  - Carer-patient linking
  - Relationship types
  - Active/inactive status
  - Uniqueness validation

#### Controllers
- **UsersController**: Basic signup functionality
  - Public new/create actions for registration
  - Nested person attributes
  - Session creation on signup

- **Admin::UsersController**: Basic admin interface
  - Index action to list users
  - Admin-only authorization check
  - Uses Phlex component for view

#### Authentication
- **Session Management**: Full authentication system
  - Cookie-based sessions
  - IP and user agent tracking
  - `Authentication` concern with `require_authentication` filter
  - Current user helpers

#### Views & Components
- **Phlex Components**: Modern component-based UI
  - `Admin::Users::IndexView` for user listing
  - Clean, accessible table with name, email, role display

- **Sign Up Form**: Complete registration form with:
  - Nested person fields (name, date_of_birth)
  - Email and password fields
  - Modern Tailwind styling

#### Testing
- **Model Tests**: Comprehensive coverage
  - User validations, associations, roles, normalization
  - Person validations, age calculation
  - CarerRelationship validations

- **System Tests**: Basic coverage
  - User signup flow
  - Admin user list access control
  - Authentication flows

### ✅ Recently Implemented (Phase 1 Progress)

#### Authorization Framework
- **Pundit Integration**: Pundit gem added and integrated into ApplicationController
- **Policy Files Created**:
  - `ApplicationPolicy` - Base policy with deny-by-default approach
  - `UserPolicy` - Administrator access for management, users can view/edit own profile
  - `PersonPolicy` - Role-based access (admin, clinician, carer, parent)
  - `CarerRelationshipPolicy` - Admin and clinician management
  - `PersonMedicinePolicy` - Complex authorization including `take_medicine?` action
- **Controllers with Authorization**:
  - `PeopleController` - All actions protected with `authorize` and `policy_scope`
  - `PersonMedicinesController` - Authorization including custom `take_medicine?` policy
  - `Admin::UsersController` - Index action protected
- **Error Handling**: `Pundit::NotAuthorizedError` rescue with user-friendly message
- **Policy Tests**: Comprehensive RSpec tests for UserPolicy, PersonPolicy, CarerRelationshipPolicy
- **System Authorization Tests**: `person_medicines_authorization_spec.rb` covering all roles

#### User Role Assignment (COMPLETED)
- **Default Role Fix (COMPLETED)**: Migration `ChangeUserDefaultRoleToParent` changes default from 0 (administrator) to 4 (parent)
- **Controller Fix**: UsersController sets person_type based on user role
- **Current Status**: New users default to parent role, controller sets appropriate person_type

#### Person Types Restructuring (COMPLETED)
- **New Person Types Added**: `child`, `parent`, `adult_patient`
- **Updated Enum**: 7 person types total (adult_patient: 0, child: 1, parent: 2, carer: 3, nurse: 4, doctor: 5, administrator: 6)
- **Migration Applied**: `AddMissingPersonTypes` safely restructures existing data
- **Proper Role-Person Mapping**: UsersController now correctly maps user roles to person types
- **Policy Updates**: PersonPolicy handles parent-child relationships and new person types
- **Authorization Logic**: Parents can access their children, carers can access assigned patients

### ❌ Missing Features

#### Critical Gaps

1. **Authorization Complete** ✅
   - ✅ Pundit framework implemented
   - ✅ Policies created for User, Person, CarerRelationship, PersonMedicine
   - ✅ **PrescriptionPolicy created and fully implemented**
   - ✅ **PrescriptionsController fully authorized** (all actions protected)
   - ✅ **MedicinePolicy created and fully implemented**
   - ✅ **MedicinesController fully authorized** (all actions protected)
   - ✅ **DashboardController fully authorized** (role-based scoping implemented)
   - ✅ **MedicationTakesController fully authorized** (all actions protected)
   - ✅ **MedicationTakePolicy created and fully implemented**
   - ✅ **DashboardPolicy created and fully implemented**
   - ✅ All controllers audited for authorization (TakeMedicines uses PrescriptionPolicy)

2. **Incomplete Admin Interface** (UNCHANGED)
   - ✅ Can view users with proper authorization
   - ❌ Cannot edit/update users
   - ❌ No user creation by admins
   - ❌ No role assignment interface
   - ❌ No user deactivation/activation
   - ❌ No password reset by admin
   - ❌ No carer relationship management UI

3. **Missing User Management Workflows**
   - No profile editing for users
   - No email change functionality
   - No user settings page
   - No account deletion
   - No user invitation system

4. **No People Management Integration**
   - People can be created separately from users
   - No UI to link existing people to user accounts
   - No validation that carers/nurses/doctors have user accounts
   - User signup doesn't set person_type

5. **Missing Carer Assignment Features**
   - No UI to assign carers to patients
   - No UI to view/manage carer relationships
   - No validation that patients without capacity have carers
   - No carer request/approval workflow

6. **No Audit Trail**
   - User changes not logged
   - Role changes not tracked
   - No admin action history

7. **Limited Role Management**
   - ⚠️ Roles assigned only during signup (defaults to carer in controller, but migration still has administrator default)
   - ❌ No role change functionality
   - ❌ No UI to manage permissions

8. **Missing Edge Cases**
   - ❌ No handling of duplicate emails between Person and User
   - ❌ Not possible to change a Person to a user
   - ❌ No soft delete for users
   - ❌ No account suspension
   - ❌ No failed login tracking
   - ❌ No account lockout

### ✅ Phase 1 Authorization Complete

All critical authorization issues have been resolved:

1. **DashboardController Fully Authorized** ✅
   - Role-based data scoping implemented
   - DashboardPolicy created with comprehensive tests
   - All 6 roles tested with system specs (9/9 passing)

2. **MedicationTakesController Fully Authorized** ✅
   - Authorization checks on all actions
   - MedicationTakePolicy created with comprehensive tests
   - Policy-level tests verify all role scenarios

3. **Complete Authorization Coverage** ✅
   - TakeMedicinesController - Uses PrescriptionPolicy#take_medicine?
   - PasswordsController - Intentionally public (password reset)
   - SessionsController - Intentionally public (login)
   - UsersController - Intentionally public (signup)
   - HomeController - Requires authentication
   - PwaController - Intentionally public (PWA assets)

## Improvement Plan

### Phase 1: Authorization & Security (High Priority)

#### 1.1 Implement Authorization Framework
**Objective**: Add comprehensive role-based access control

**Status**: ✅ **100% Complete**

**Tasks**:
- [x] Add Pundit gem to Gemfile
- [x] Generate base ApplicationPolicy
- [x] Create UserPolicy with role-based permissions
- [x] Create PersonPolicy with role-based permissions
- [x] Create CarerRelationshipPolicy
- [x] Create PersonMedicinePolicy with custom actions
- [x] Add policy checks to PeopleController
- [x] Add policy checks to PersonMedicinesController
- [x] Add policy checks to Admin::UsersController
- [x] **Create PrescriptionPolicy** (COMPLETED)
- [x] **Create MedicinePolicy** (COMPLETED)
- [x] **Add authorization to PrescriptionsController** (COMPLETED)
- [x] **Add authorization to MedicinesController** (COMPLETED)
- [x] **Create MedicationTakePolicy** (COMPLETED)
- [x] **Add authorization to MedicationTakesController** (COMPLETED)
- [x] **Create DashboardPolicy** (COMPLETED)
- [x] **Add authorization to DashboardController** (COMPLETED)
- [x] Audit remaining controllers (TakeMedicines, Passwords, Sessions, etc.) (COMPLETED)
- [x] Write policy tests (RSpec) - UserPolicy, PersonPolicy, CarerRelationshipPolicy
- [x] Write policy tests for PrescriptionPolicy and MedicinePolicy (COMPLETED)
- [x] Write policy tests for MedicationTakePolicy and DashboardPolicy (COMPLETED)
- [x] Write system authorization tests - person_medicines_authorization_spec.rb
- [x] Write system authorization tests - dashboard_authorization_spec.rb (9/9 passing)

**User Roles & Permissions Matrix**:

| Action                        | Administrator | Doctor | Nurse | Carer | Parent |
|-------------------------------|---------------|--------|-------|-------|--------|
| Manage all users              | ✅             | ❌      | ❌     | ❌     | ❌      |
| View all people               | ✅             | ✅      | ✅     | ❌     | ❌      |
| Create patients               | ✅             | ✅      | ✅     | ❌     | ❌      |
| Edit own profile              | ✅             | ✅      | ✅     | ✅     | ✅      |
| Manage prescriptions (any)    | ✅             | ✅      | ❌     | ❌     | ❌      |
| Record medications (assigned) | ✅             | ✅      | ✅     | ✅     | ✅      |
| Assign carers                 | ✅             | ✅      | ✅     | ❌     | ❌      |

**Acceptance Criteria**:
- All controller actions protected by policies
- Unauthorized access returns 403 or redirects appropriately
- Users can only see/edit resources they have permission for
- Tests verify all permission scenarios

#### 1.2 Fix Default Role Assignment
**Objective**: Prevent new users from defaulting to administrator

**Status**: ✅ **100% Complete**

**Tasks**:
- [x] Create migration to change default from 0 to 4 (parent)
- [x] Update UsersController to set person_type to `:carer` on signup
- [x] Migration applied: `ChangeUserDefaultRoleToParent`
- [x] New users default to parent role, controller sets person_type to carer
- [x] Tests updated and passing

**Acceptance Criteria**:
- New signups default to parent/carer role
- Only admins can create admin accounts
- Migration safely changes existing defaults

#### 1.3 Add Account Security Features
**Objective**: Improve account security using a centralized authentication framework.

**Status**: 🔴 **Planned** – detailed implementation is tracked in `USER_SIGNUP_PLAN.md` (Rodauth signup, verification, OAuth) and `USER_SIGNUP_AND_2FA_PLAN.md` (email delivery, 2FA).

**Tasks** (high level):
- [ ] Implement Rodauth-based signup, login, email verification, and Google OAuth as described in `USER_SIGNUP_PLAN.md`.
- [ ] Enforce environment-specific email verification rules (production requires verification; non-production has a 7-day grace period) via Rodauth configuration.
- [ ] Add session lifetime, failed login tracking, and account lockout through Rodauth features or supporting middleware.
- [ ] Introduce optional two-factor authentication following `USER_SIGNUP_AND_2FA_PLAN.md`.

**Acceptance Criteria** (summary):
- Email verification and account lifecycle behaviour match the Rodauth-based signup plan.
- Sessions expire within defined limits; repeated failures lead to lockout.
- 2FA is available for higher-risk roles.
- All authentication and account security flows are covered by tests.

### Phase 2: Admin User Management (High Priority)

#### 2.1 Complete Admin CRUD Operations
**Objective**: Full user lifecycle management for admins

**Status**: 🟡 **75% Complete** (Creation & editing done; deactivation/destroy outstanding)

**Tasks**:
- [x] `Admin::UsersController#index` with authorization and policy_scope
- [x] Add `Admin::UsersController#new` and `#create`
- [x] Add `Admin::UsersController#edit` and `#update`
- [ ] Add `Admin::UsersController#destroy`
- [x] Create Phlex form components for user creation/editing
- [x] Add role selection dropdown (admin-only)
- [ ] Add person selection/creation
- [ ] Add user activation/deactivation toggle
- [x] Add system tests for all admin user operations

**Notes**:
- Combined creation/editing form delivers role assignment and nested person details.
- Next step: support deactivate/reactivate flows and admin-driven destroy.

**Acceptance Criteria**:
- Admins can create users and assign roles
- Admins can edit user details and roles
- Admins can activate/deactivate accounts
- All operations are tested
- Proper flash messages and error handling

#### 2.2 Create Admin Dashboard
**Objective**: Central admin interface for system management

**Status**: 🟡 **60% Complete** (Core metrics live; additional insights pending)

**Tasks**:
- [x] Create `Admin::DashboardController`
- [x] Create dashboard Phlex component with metrics:
  - [x] Total users by role
  - [x] Total people by type
  - [ ] Recent signups
  - [ ] Active users
  - [x] Patients without capacity lacking carers
- [x] Add navigation to admin area (quick links to manage users/people)
- [x] Add authorization (admin-only)
- [x] Write system tests

**Notes**:
- Warning card highlights patients without carers; add additional metrics in follow-up.

**Acceptance Criteria**:
- Dashboard shows key metrics *(partially met; add remaining metrics)*
- Links to user, people, and relationship management ✅
- Only accessible by administrators ✅
- Tested with system specs ✅

#### 2.3 Add User Search & Filtering
**Objective**: Help admins find users quickly

**Status**: 🟡 **55% Complete** (Search + role filter shipped; pagination/status filters pending)

**Tasks**:
- [x] Add search form to admin users index
- [x] Implement search by name, email
- [x] Add filters by role, active status *(role complete; active status pending)*
- [ ] Add pagination (Pagy or Kaminari)
- [ ] Add sorting by name, email, created_at
- [x] Write tests for search/filter functionality

**Notes**:
- Search persists form state and includes clear action; extend to pagination/sorting next.

**Acceptance Criteria**:
- Admins can search users by name/email
- Filtering by role and status works
- Results paginated (25 per page)
- Tests verify search accuracy

### Phase 3: Carer Relationship Management (High Priority)

#### 3.1 Build Carer Assignment Interface
**Objective**: Allow management of carer-patient relationships

**Status**: 🔴 **0% Complete** (Policy exists, no controller/UI)

**Tasks**:
- [x] CarerRelationshipPolicy created with full CRUD permissions
- [ ] Create `CarerRelationshipsController`
- [ ] Add `#index` action scoped to current user/admin
- [ ] Add `#new` and `#create` actions
- [ ] Add `#destroy` action (deactivate, not delete)
- [ ] Create Phlex components for relationship listing
- [ ] Create form to assign carer to patient
- [ ] Add relationship_type dropdown
- [ ] Add authorization (policies)
- [ ] Write system tests

**Acceptance Criteria**:
- Carers can view their assigned patients
- Admins/doctors/nurses can assign carers to patients
- Relationship types are selectable
- Relationships can be deactivated
- All actions authorized and tested

#### 3.2 Add Patient Dashboard for Carers
**Objective**: Carers see their assigned patients

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Create `Dashboard::PatientsController` for carers
- [ ] Show list of assigned patients
- [ ] Show patient medication schedules
- [ ] Add quick actions (record medication)
- [ ] Scope to only assigned patients
- [ ] Add authorization
- [ ] Write tests

**Acceptance Criteria**:
- Carers only see assigned patients
- Dashboard shows medication schedules
- Quick medication recording
- Unauthorized access blocked

#### 3.3 Add Capacity Validation
**Objective**: Ensure patients without capacity have carers

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Add model validation: `Person.without_capacity.must_have_carer`
- [ ] Create background job to check compliance
- [ ] Add admin warning for non-compliant records
- [ ] Create UI to assign carer when creating patient without capacity
- [ ] Write model and integration tests

**Acceptance Criteria**:
- Cannot save patient without capacity unless carer assigned
- Admin sees warnings for non-compliant records
- Tests verify validation logic

### Phase 4: User Self-Service (Medium Priority)

#### 4.1 User Profile Management
**Objective**: Users can manage their own profiles

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Create `ProfilesController` or `Users::ProfilesController`
- [ ] Add `#show` action for current user
- [ ] Add `#edit` and `#update` actions
- [ ] Allow editing name, date_of_birth (from person)
- [ ] Allow changing email (with confirmation)
- [ ] Allow changing password
- [ ] Create Phlex profile components
- [ ] Add authorization (user can edit own profile)
- [ ] Write system tests

**Acceptance Criteria**:
- Users can view their profile
- Users can update name, DOB, email, password
- Email changes require confirmation
- Unauthorized users cannot edit others' profiles

#### 4.2 User Settings Page
**Objective**: Centralized settings for user preferences

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Create `Users::SettingsController`
- [ ] Add notification preferences
- [ ] Add timezone selection
- [ ] Add language selection (future)
- [ ] Add privacy settings
- [ ] Create settings Phlex component
- [ ] Write tests

**Acceptance Criteria**:
- Settings page accessible to logged-in users
- Preferences persist correctly
- Tests verify all settings

#### 4.3 Account Deletion
**Objective**: Allow users to delete their accounts

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Add soft delete to User model (add `deleted_at` column)
- [ ] Create `Users::AccountsController#destroy`
- [ ] Add confirmation dialog
- [ ] Handle cascade: what happens to person, prescriptions, etc.
- [ ] Add re-activation option (admin-only)
- [ ] Write tests

**Acceptance Criteria**:
- Users can delete their accounts
- Deletion is soft (recoverable)
- Admins can reactivate accounts
- Associated data handled appropriately

### Phase 5: Advanced Features (Low Priority)

#### 5.1 User Invitation System
**Objective**: Invite users to join the system

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Create `InvitationsController`
- [ ] Generate invitation tokens
- [ ] Send invitation emails
- [ ] Create invitation acceptance flow
- [ ] Pre-fill role and person info
- [ ] Add expiration to invitations
- [ ] Write tests

**Acceptance Criteria**:
- Admins can send invitations
- Invitees receive email with link
- Acceptance creates user account
- Expired invitations don't work

#### 5.2 Audit Log
**Objective**: Track all user and admin actions

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Add PaperTrail or Audited gem
- [ ] Track User changes (create, update, delete)
- [ ] Track role changes
- [ ] Track carer relationship changes
- [ ] Create `Admin::AuditLogsController`
- [ ] Create audit log viewer (Phlex component)
- [ ] Add filtering by user, action, date
- [ ] Write tests

**Acceptance Criteria**:
- All changes to users logged
- Admins can view audit logs
- Logs filterable and searchable
- Tests verify logging

#### 5.3 Impersonation
**Objective**: Allow admins to impersonate users for support

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Add impersonation functionality
- [ ] Create `Admin::ImpersonationsController`
- [ ] Add "Impersonate" button in admin user list
- [ ] Add banner when impersonating
- [ ] Add "Stop Impersonating" button
- [ ] Log impersonation actions in audit log
- [ ] Write tests

**Acceptance Criteria**:
- Admins can impersonate any user
- Clear visual indicator when impersonating
- All impersonation actions logged
- Tests verify security

#### 5.4 Role-Based Navigation
**Objective**: Show different navigation based on role

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Update `Layouts::Navigation` component
- [ ] Show "Admin" link only for administrators
- [ ] Show "My Patients" for carers
- [ ] Show "Prescriptions" for doctors
- [ ] Hide features based on permissions
- [ ] Write tests

**Acceptance Criteria**:
- Navigation items match user role
- Unauthorized items hidden
- Tests verify correct links shown

### Phase 6: Integration & Polish (Low Priority)

#### 6.1 Person-User Linking Improvements
**Objective**: Better integration between Person and User creation

**Status**: 🟡 **75% Complete** (Major restructuring completed)

**Tasks**:
- [x] Restructure person types to include child, parent, adult_patient
- [x] Add migration `AddMissingPersonTypes` with proper data mapping
- [x] Update UsersController to map user roles to person types correctly
- [x] Update PersonPolicy to handle parent-child relationships
- [x] Add authorization logic for parents accessing children
- [ ] Add person_type selection to user signup form
- [ ] Add UI to convert existing person to user account
- [ ] Validate role and person_type alignment
- [ ] Write tests for new person types and relationships

**Acceptance Criteria**:
- New users set appropriate person_type
- Existing people can be given user accounts
- Validation prevents mismatches

#### 6.2 Improved Error Handling
**Objective**: Better user experience on errors

**Status**: 🟡 **20% Complete** (Pundit error rescue implemented)

**Tasks**:
- [x] Pundit::NotAuthorizedError rescue with flash message
- [ ] Custom 403 Forbidden page
- [ ] Custom 404 Not Found page
- [ ] Better validation error messages
- [ ] Inline form validation
- [ ] Flash message improvements
- [ ] Write tests

**Acceptance Criteria**:
- All error pages have consistent styling
- Error messages are clear and helpful
- Form errors highlight specific fields

#### 6.3 Performance Optimization
**Objective**: Ensure user management scales

**Status**: 🔴 **0% Complete** (Not Started)

**Tasks**:
- [ ] Add database indexes for common queries
- [ ] Implement eager loading in controllers
- [ ] Add caching for user/role lookups
- [ ] Optimize admin user list queries
- [ ] Add performance tests

**Acceptance Criteria**:
- Queries optimized (no N+1)
- User list loads in <200ms with 1000+ users
- Tests verify performance

## Testing Strategy

### Test Coverage Requirements
- **Unit Tests (RSpec)**: 100% coverage for models, policies
- **Controller Tests**: All actions tested for authorization
- **System Tests (Capybara)**: Critical user flows end-to-end
- **Fixtures**: Maintain realistic test data for all person types, roles, and relationships

### Test Scenarios to Cover

1. **Authentication**
   - Sign up, sign in, sign out
   - Password reset
   - Session expiration
   - Remember me

2. **Authorization**
   - Each role's permissions
   - Unauthorized access attempts
   - Cross-user data access prevention

3. **User Management**
   - Admin creates user
   - Admin updates user role
   - Admin deactivates user
   - User updates own profile

4. **Carer Relationships**
   - Assign carer to patient
   - Deactivate relationship
   - View assigned patients
   - Capacity validation

5. **Edge Cases**
   - Duplicate emails
   - Invalid person types
   - Missing required fields
   - Concurrent updates

## Migration Strategy

### Data Migrations Required

1. **Update Default User Role**
   ```ruby
   # Change default role from 0 (administrator) to 4 (parent)
   change_column_default :users, :role, from: 0, to: 4
   ```

2. **Add Soft Delete to Users**
   ```ruby
   add_column :users, :deleted_at, :datetime
   add_index :users, :deleted_at
   ```

3. **Add Account Security Columns**
   ```ruby
   add_column :users, :confirmed_at, :datetime
   add_column :users, :confirmation_token, :string
   add_column :users, :failed_attempts, :integer, default: 0
   add_column :users, :locked_at, :datetime
   ```

4. **Add Audit Columns**
   ```ruby
   # If using PaperTrail
   create_table :versions do |t|
     t.string :item_type, null: false
     t.bigint :item_id, null: false
     t.string :event, null: false
     t.string :whodunnit
     t.text :object
     t.datetime :created_at
   end
   ```

### Backwards Compatibility
- All new columns should have sensible defaults
- Existing records should continue working
- Policies should default to denying access (fail-safe)
- Tests should verify migration safety

## Success Metrics

### Functional Metrics
- ✅ All CRUD operations available for users (admin)
- ✅ All roles have appropriate permissions
- ✅ Users can manage their profiles
- ✅ Carers can view assigned patients
- ✅ Capacity validation enforced

### Quality Metrics
- 100% test coverage for critical paths
- No authorization bypasses
- All user actions logged
- <200ms page load for user management pages

### Security Metrics
- No hardcoded credentials
- All endpoints protected
- CSRF protection enabled
- SQL injection prevented (parameterized queries)
- XSS prevented (escaped output)

## Implementation Order (Recommended)

### Sprint 1 (Week 1-2): Security Foundation ✅ 100% Complete
1. ✅ Add Pundit authorization (DONE)
2. ✅ Fix default role assignment (COMPLETED - migration applied)
3. ✅ Write comprehensive policy tests (DONE for all policies)
4. ✅ Create MedicationTakePolicy and add authorization (COMPLETED)
5. ✅ Create DashboardPolicy and add authorization/scoping (COMPLETED)
6. ✅ Audit all remaining controllers (COMPLETED)
7. ⏸️ Add password policies (DEFERRED)

**Phase 1 Complete!** All authorization gaps addressed. Ready to begin Phase 2.

**Next Phase Actions**:

1. 🟢 **BEGIN PHASE 2**: Complete Admin CRUD operations for users
2. 🟢 Build admin dashboard with metrics
3. 🟢 Add user search and filtering
4. 📋 **REFERENCE**: See below for Phase 2 detailed tasks

### Sprint 2 (Week 3-4): Admin Interface
1. Complete admin CRUD for users
2. Build admin dashboard
3. Add user search/filtering
4. System tests for admin flows

### Sprint 3 (Week 5-6): Carer Management
1. Build carer relationship interface
2. Add patient dashboard for carers
3. Implement capacity validation
4. Integration tests

### Sprint 4 (Week 7-8): User Self-Service
1. User profile management
2. User settings page
3. Account deletion (soft delete)
4. End-to-end user journey tests

### Sprint 5+ (Future): Advanced Features
1. Invitation system
2. Audit logging
3. Impersonation
4. Performance optimization

## Risks & Mitigation

### Risk 1: Breaking Existing Functionality
- **Mitigation**: Comprehensive test suite before starting
- **Mitigation**: Feature flags for new authorization
- **Mitigation**: Incremental rollout

### Risk 2: Poor Performance with Many Users
- **Mitigation**: Load testing before launch
- **Mitigation**: Database indexing strategy
- **Mitigation**: Pagination and caching

### Risk 3: Complexity of Permissions
- **Mitigation**: Clear permission matrix documentation
- **Mitigation**: Policy tests covering all scenarios
- **Mitigation**: Admin tools to debug permissions

### Risk 4: Data Migration Issues
- **Mitigation**: Test migrations on copy of production data
- **Mitigation**: Rollback plan for each migration
- **Mitigation**: Gradual migration with validation

## Documentation Requirements

- [ ] Update README with user management features
- [ ] Create admin user guide
- [ ] Create carer user guide
- [ ] Update API documentation (if applicable)
- [ ] Maintain this plan as living document
- [ ] Create runbook for common admin tasks

## Progress Summary

### ✅ Completed (Sprint 1 - 100%)

- Pundit authorization framework fully integrated
- Four comprehensive policy classes created (User, Person, CarerRelationship, PersonMedicine)
- Authorization added to 3 controllers (People, PersonMedicines, Admin::Users)
- Policy and system authorization tests written
- Error handling for unauthorized access
- Default role fix COMPLETED - migration changes default from administrator to parent
- UsersController sets default person_type to carer

### ✅ Phase 1 Complete - No Critical Blockers

All authorization implemented and tested:
- DashboardController fully protected with role-based scoping
- MedicationTakesController fully authorized
- All controllers audited and documented
- Comprehensive policy and system tests passing

### 🟢 Phase 2 Ready to Begin

**Next Sprint Priorities**:

1. Complete Admin CRUD operations (Task 2.1)
   - Add `Admin::UsersController#new` and `#create`
   - Add `Admin::UsersController#edit` and `#update`
   - Add `Admin::UsersController#destroy`
2. Create Admin Dashboard (Task 2.2)
3. Add User Search & Filtering (Task 2.3)

**Completed Plan**: See AUTHORIZATION_COMPLETION_PLAN.md for Phase 1 details

## Conclusion

**Phase 1 (Authorization & Security) is now 100% complete!** ✅

The Pundit framework is fully integrated with comprehensive policies for all entities:
- User, Person, CarerRelationship, PersonMedicine
- Prescription, Medicine, MedicationTake, Dashboard

All controllers are properly authorized:
- Role-based access control implemented throughout
- Data scoping ensures users only see authorized records
- Comprehensive policy and system tests (all passing)
- Default role security issue resolved

**Ready for Phase 2: Admin Interface Implementation**

The existing foundation is solid with excellent separation of concerns, clean models, comprehensive test coverage, and best-practice authorization with deny-by-default policies.
