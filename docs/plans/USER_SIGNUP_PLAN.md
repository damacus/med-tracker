# User Signup (Rodauth) Implementation Plan

**Created**: 2025-11-15
**Updated**: 2025-11-27
**Status**: ⚠️ PARTIALLY IMPLEMENTED (~20% complete)
**Parent Document**: USER_MANAGEMENT_PLAN.md
**Related**: Issue #118, PR #119 (closed), USER_SIGNUP_AND_2FA_PLAN.md, RODAUTH_SIGNUP_IMPLEMENTATION.md

> **Note**: This plan has been superseded by `RODAUTH_SIGNUP_IMPLEMENTATION.md` which provides a more focused, actionable implementation based on learnings from PR #119.

## Current Status (2025-11-18)

**What's Actually Working:**

- ✅ Rodauth gems installed (`rodauth-rails`, `rodauth-omniauth`, `omniauth-google-oauth2`)
- ✅ `Account` model exists with Rodauth integration
- ✅ Rodauth configuration files present (`app/misc/rodauth_app.rb`, `app/misc/rodauth_main.rb`)
- ✅ Login via Rodauth works (`/login` endpoint)
- ✅ Logout via Rodauth works
- ✅ `current_account` helper available
- ✅ Account-Person relationship exists (`Account has_one :person`, `Person belongs_to :account, optional: true`)

**What's NOT Working:**

- ❌ **No Rodauth signup form** - Legacy `UsersController` signup still in use
- ❌ **No email verification flow** - Tests marked as pending
- ❌ **No Google OAuth integration** - Not implemented
- ❌ **No account linking** - Not implemented
- ❌ **No 7-day grace period** - Not implemented
- ❌ **Legacy auth code still active** - `UsersController`, `SessionsController` not removed
- ❌ **Person creation on signup** - Not integrated with Rodauth
- ❌ **Migration scripts** - Existing users not migrated to Account model

**Technical Debt:**

- Legacy `has_secure_password` still in use alongside Rodauth
- Dual authentication systems running in parallel
- Test suite has 13 pending tests related to authentication/profile features
- Admin "create people" functionality removed for security but signup flow incomplete

## Objectives

- Replace the current custom `User` + `has_secure_password` signup with Rodauth-based authentication.
- Keep `Person` as the source of truth for demographics and care relationships.
- Support email+password signup plus Google OAuth, with safe account linking.
- Enforce email verification in production, with a 7-day grace period in non-production environments.
- Drive implementation via RSpec/Capybara tests (TDD) and keep existing authorization (Pundit) intact.

## Requirements (from Issue #118)

- Users can sign up with email and password.
- New email/password accounts are verified via an email link.
- Users can log in using Google (OmniAuth).
- Logged-in email/password users can link a Google account to their existing account.
- Google sign-in with no existing account creates a new verified account.
- Google sign-in with an existing email links the Google identity to that account.

## Architecture Decisions

### Authentication framework

- Use `rodauth-rails` as the primary authentication system.
- Use `rodauth-omniauth` for Google OAuth.
- Consolidate all authentication (login, logout, password reset, remember me, verification) behind Rodauth routes.

### Data model

- Introduce an `Account` model backed by an `accounts` table managed by Rodauth.
- Relationship:
  - `Account` has_one `Person`.
  - `Person` belongs_to `Account` (optional).
- Only people who access the app directly (administrators, clinicians, carers, parents, self-managing adults) have an account.
- Minors and dependent adults may have `Person` records without accounts.

### Email verification policy

- **Production**:
  - Unverified accounts cannot sign in until they confirm their email.
  - Verification tokens expire according to Rodauth defaults or explicit config.
- **Non-production (development, test, staging)**:
  - Allow a 7-day grace period where newly created, unverified accounts can sign in.
  - After 7 days from account creation, unverified accounts are treated as blocked until verified.
- Implement behaviour via Rodauth configuration rather than custom flags on `User`.

### Compatibility with existing code

- Keep existing authorization (Pundit policies) operating on `Person` and roles.
- Provide compatibility helpers so existing code can move gradually:
  - `current_account` (Rodauth).
  - `current_person` derived from `current_account`.
  - Temporary `current_user` shim if needed, delegating to `current_person` + role info.
- Deprecate `SessionsController`/`UsersController` login/signup paths once Rodauth flows are in place.

## Phase 0: Discovery and Test Harness

**Status**: ❌ **NOT COMPLETE**

**Goal**: Capture desired behaviour in tests before changing authentication.

- **Tasks**
  - [x] Review existing authentication specs (sessions, signup) and identify what to keep or delete.
  - [ ] Add feature specs for signup and login under `spec/features/authentication/`:
    - [ ] Email/password signup and first login.
    - [ ] Email verification required in production.
    - [ ] 7-day grace behaviour in non-production.
    - [ ] Google login for new user (account + person created).
    - [ ] Google login linking to existing account.
  - [x] Add model/policy specs where needed for `Account`–`Person` associations.

- **Acceptance criteria**
  - ❌ Failing specs describe all flows from the issue requirements and grace-period decision.
  - ✅ No production code added or changed beyond what is needed to wire tests.

**Reality Check**: Only basic model specs exist. No comprehensive feature specs for signup flows, email verification, or OAuth. Legacy signup test marked as pending instead of being replaced with Rodauth specs.

## Phase 1: Rodauth Foundation and Account Model

**Status**: ⚠️ **PARTIALLY COMPLETE** (60%)

**Goal**: Install Rodauth and introduce the `Account` model without breaking existing flows.

- **Tasks**
  - [x] Add `rodauth-rails` and `rodauth-omniauth` gems (if not already present).
  - [x] Generate Rodauth app and base configuration.
  - [x] Create `accounts` table and `Account` model aligned with Rodauth (email, password hash, verification fields, OAuth identity table).
  - [x] Wire Rodauth into the Rails router and replace the low-level authentication logic in the `Authentication` concern with Rodauth helpers.
  - [x] Introduce `current_account` and `current_person` helpers and update controllers/views to use them where straightforward.

- **Acceptance criteria**
  - ✅ Existing users can still sign in via the old path or a temporary compatibility layer.
  - ✅ New Rodauth routes are reachable in development and covered by smoke tests.
  - ⚠️ No major changes yet to signup behaviour (that comes in Phase 2).

**Reality Check**: Rodauth is installed and login works, but dual authentication systems exist. `SessionsController` and legacy auth code not removed. OAuth identity table may not be properly configured. `current_person` helper exists but not consistently used throughout codebase.

## Phase 2: Email/Password Signup with Person Creation

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Implement Rodauth-based email/password signup that creates a `Person` and enforces email verification.

- **Tasks**
  - [ ] Design a single signup form that collects:
    - [ ] Email, password, password confirmation.
    - [ ] Person attributes: name, date of birth, and any required person_type defaults.
  - [ ] Configure Rodauth `create_account` and `verify_account` features to:
    - [ ] Create an `Account`.
    - [ ] Immediately create a linked `Person` with appropriate default `person_type` (adult patient / parent / staff as per user role mapping).
  - [ ] Implement the email verification flow:
    - [ ] Send verification email on signup.
    - [ ] Show a "check your email" page after signup.
    - [ ] Confirm account when user follows the link.
  - [ ] Implement environment-specific behaviour:
    - [ ] Production: block login for unverified accounts (no grace).
    - [ ] Non-production: allow login for unverified accounts for 7 days based on `created_at`, then block.
  - [ ] Remove or deprecate the old `UsersController#create` signup path once Rodauth signup is green and covered by tests.

- **Acceptance criteria**
  - ❌ All Phase 2 feature specs pass for both production-like and non-production-like configurations.
  - ❌ Every account has a corresponding `Person` record where required.
  - ❌ Unverified accounts behave correctly with and without the grace period.
  - ❌ Old signup routes are no longer used in UI flows.

**Reality Check**: Nothing from this phase is implemented. Legacy `UsersController` signup still active. No Rodauth signup form exists. No email verification. No Person creation on signup. This is the critical missing piece blocking proper user onboarding.

## Phase 3: Google OAuth and Account Linking

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Allow users to sign in with Google and safely link identities.

- **Tasks**
  - [ ] Configure `rodauth-omniauth` for Google using `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`.
  - [ ] Implement flows:
    - [ ] New Google user: create `Account` + `Person`, mark account verified.
    - [ ] Google login where an account with that email exists: link the identity to the existing account.
    - [ ] Logged-in email/password user: add Google as a linked identity from a settings/profile page.
  - [ ] Ensure linking cannot hijack another user's account (require matching email or explicit confirmation).
  - [ ] Add feature specs for each flow, including error handling when linking is not allowed.

- **Acceptance criteria**
  - ❌ OAuth-based tests pass and are deterministic (e.g. using OmniAuth test mode).
  - ❌ Email/password and Google flows share the same `Account` and `Person` records.
  - ❌ No duplicate accounts created for the same email.

**Reality Check**: Zero OAuth implementation. Gems installed but not configured. No Google OAuth routes, no identity linking, no tests. Cannot start until Phase 2 is complete.

## Phase 4: Cleanup, Migration, and Rollout

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Remove legacy auth paths and document the new signup system.

- **Tasks**
  - [ ] Migrate existing staff users onto `Account` records (backfill and data migration scripts).
  - [ ] Remove or shrink legacy authentication code:
    - [ ] `SessionsController` actions that duplicate Rodauth.
    - [ ] `UsersController` signup actions.
    - [ ] Password reset and remember-me logic that is now handled by Rodauth.
  - [ ] Update Pundit policies and tests to rely on `current_person` / `current_account`.
  - [ ] Update documentation:
    - [ ] `USER_MANAGEMENT_PLAN.md` Phase 1.3 account security section.
    - [ ] `USER_SIGNUP_AND_2FA_PLAN.md` to focus on 2FA only.
    - [ ] Readme/setup notes for environment variables and Postmark configuration.
  - [ ] Run full test suite and perform manual smoke testing of signup and login in development.

- **Acceptance criteria**
  - ❌ No references remain to the old `has_secure_password`-based login.
  - ❌ All authentication-related specs are green and clearly organized.
  - ❌ Documentation describes Rodauth-based signup, verification, OAuth, and the 7-day non-production grace rule.

**Reality Check**: Cannot start until Phases 2 and 3 are complete. Legacy code still fully operational and being used. No migration scripts exist. Documentation not updated to reflect current hybrid state.

---

## Summary: What Needs to Happen Next

### Critical Path (Must Do)

1. **Phase 2 is the blocker** - Without Rodauth signup, users cannot properly onboard
2. **Write comprehensive tests first** - Phase 0 incomplete, need feature specs for all flows
3. **Implement Rodauth signup form** - Replace `UsersController` with Rodauth create-account
4. **Person creation on signup** - Hook into Rodauth to create Person records automatically
5. **Email verification** - Configure and test verification flow

### Current State Assessment

**Overall Progress**: ~20% complete

- **Phase 0**: 30% (basic specs only, no feature tests)
- **Phase 1**: 60% (foundation exists but incomplete)
- **Phase 2**: 0% (critical blocker)
- **Phase 3**: 0% (blocked by Phase 2)
- **Phase 4**: 0% (blocked by Phases 2 & 3)

### Risks

- **Dual authentication systems** create confusion and security risks
- **No proper user onboarding** - can't safely add new users
- **Technical debt accumulating** - longer we wait, harder the migration
- **Admin create people removed** but no replacement signup flow
- **Test coverage gaps** - 13 pending tests, missing feature specs

### Recommendation

**Stop adding features. Focus on completing Phase 2.**

The application cannot safely onboard new users without proper Rodauth signup. This is a critical gap that should be addressed before any other feature work.
