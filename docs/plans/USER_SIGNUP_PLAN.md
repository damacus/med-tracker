# User Signup (Rodauth) Implementation Plan

**Created**: 2025-11-15
**Status**: Draft
**Parent Document**: USER_MANAGEMENT_PLAN.md
**Related**: Issue #118, PR #119, USER_SIGNUP_AND_2FA_PLAN.md

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

**Goal**: Capture desired behaviour in tests before changing authentication.

- **Tasks**
  - [ ] Review existing authentication specs (sessions, signup) and identify what to keep or delete.
  - [ ] Add feature specs for signup and login under `spec/features/authentication/`:
    - Email/password signup and first login.
    - Email verification required in production.
    - 7-day grace behaviour in non-production.
    - Google login for new user (account + person created).
    - Google login linking to existing account.
  - [ ] Add model/policy specs where needed for `Account`–`Person` associations.

- **Acceptance criteria**
  - Failing specs describe all flows from the issue requirements and grace-period decision.
  - No production code added or changed beyond what is needed to wire tests.

## Phase 1: Rodauth Foundation and Account Model

**Goal**: Install Rodauth and introduce the `Account` model without breaking existing flows.

- **Tasks**
  - [ ] Add `rodauth-rails` and `rodauth-omniauth` gems (if not already present).
  - [ ] Generate Rodauth app and base configuration.
  - [ ] Create `accounts` table and `Account` model aligned with Rodauth (email, password hash, verification fields, OAuth identity table).
  - [ ] Wire Rodauth into the Rails router and replace the low-level authentication logic in the `Authentication` concern with Rodauth helpers.
  - [ ] Introduce `current_account` and `current_person` helpers and update controllers/views to use them where straightforward.

- **Acceptance criteria**
  - Existing users can still sign in via the old path or a temporary compatibility layer.
  - New Rodauth routes are reachable in development and covered by smoke tests.
  - No major changes yet to signup behaviour (that comes in Phase 2).

## Phase 2: Email/Password Signup with Person Creation

**Goal**: Implement Rodauth-based email/password signup that creates a `Person` and enforces email verification.

- **Tasks**
  - [ ] Design a single signup form that collects:
    - Email, password, password confirmation.
    - Person attributes: name, date of birth, and any required person_type defaults.
  - [ ] Configure Rodauth `create_account` and `verify_account` features to:
    - Create an `Account`.
    - Immediately create a linked `Person` with appropriate default `person_type` (adult patient / parent / staff as per user role mapping).
  - [ ] Implement the email verification flow:
    - Send verification email on signup.
    - Show a "check your email" page after signup.
    - Confirm account when user follows the link.
  - [ ] Implement environment-specific behaviour:
    - Production: block login for unverified accounts (no grace).
    - Non-production: allow login for unverified accounts for 7 days based on `created_at`, then block.
  - [ ] Remove or deprecate the old `UsersController#create` signup path once Rodauth signup is green and covered by tests.

- **Acceptance criteria**
  - All Phase 2 feature specs pass for both production-like and non-production-like configurations.
  - Every account has a corresponding `Person` record where required.
  - Unverified accounts behave correctly with and without the grace period.
  - Old signup routes are no longer used in UI flows.

## Phase 3: Google OAuth and Account Linking

**Goal**: Allow users to sign in with Google and safely link identities.

- **Tasks**
  - [ ] Configure `rodauth-omniauth` for Google using `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`.
  - [ ] Implement flows:
    - New Google user: create `Account` + `Person`, mark account verified.
    - Google login where an account with that email exists: link the identity to the existing account.
    - Logged-in email/password user: add Google as a linked identity from a settings/profile page.
  - [ ] Ensure linking cannot hijack another user's account (require matching email or explicit confirmation).
  - [ ] Add feature specs for each flow, including error handling when linking is not allowed.

- **Acceptance criteria**
  - OAuth-based tests pass and are deterministic (e.g. using OmniAuth test mode).
  - Email/password and Google flows share the same `Account` and `Person` records.
  - No duplicate accounts created for the same email.

## Phase 4: Cleanup, Migration, and Rollout

**Goal**: Remove legacy auth paths and document the new signup system.

- **Tasks**
  - [ ] Migrate existing staff users onto `Account` records (backfill and data migration scripts).
  - [ ] Remove or shrink legacy authentication code:
    - `SessionsController` actions that duplicate Rodauth.
    - Password reset and remember-me logic that is now handled by Rodauth.
  - [ ] Update Pundit policies and tests to rely on `current_person` / `current_account`.
  - [ ] Update documentation:
    - `USER_MANAGEMENT_PLAN.md` Phase 1.3 account security section.
    - `USER_SIGNUP_AND_2FA_PLAN.md` to focus on 2FA only.
    - Readme/setup notes for environment variables and Postmark configuration.
  - [ ] Run full test suite and perform manual smoke testing of signup and login in development.

- **Acceptance criteria**
  - No references remain to the old `has_secure_password`-based login.
  - All authentication-related specs are green and clearly organized.
  - Documentation describes Rodauth-based signup, verification, OAuth, and the 7-day non-production grace rule.
