# Rodauth Signup Implementation Plan

**Created**: 2025-11-15
**Updated**: 2025-11-27
**Status**: ⚠️ Partially Implemented (~20% complete)
**Related**: Issue #118, PR #119 (closed), USER_SIGNUP_PLAN.md (current status)

## Overview

Implement user signup using Rodauth with email verification, Google OAuth, and a 7-day grace period for email verification in non-production environments.

> **Current Status**: Rodauth foundation is installed but signup flow is NOT complete.
> See `USER_SIGNUP_PLAN.md` for detailed current state assessment.
>
> **What Works**: Login, logout, `current_account` helper
> **What's Missing**: Rodauth signup form, email verification, Google OAuth, Person creation on signup

## Key Decisions

- **Framework**: Rodauth (rodauth-rails gem)
- **OAuth**: Google via rodauth-omniauth
- **Email Verification**: Required in production; 7-day grace period in development/test/staging
- **Data Model**: Separate `accounts` table from `people` table
- **Migration**: Replace existing `has_secure_password` authentication

## Architecture

### Data Model

```text
accounts (Rodauth)
├── id
├── email (unique, not null)
├── password_hash
├── status (unverified, verified, closed)
└── timestamps

people
├── id
├── account_id (optional, foreign key)
├── name
├── date_of_birth
├── person_type
└── other fields...
```

**Relationships**:

- `Account` has_one `Person`
- `Person` belongs_to `Account` (optional - minors/dependents have no account)

### Email Verification Policy

**Production** (`RAILS_ENV=production`):

- Unverified accounts CANNOT sign in
- Must verify email before first login
- No grace period

**Non-Production** (development, test, staging):

- Unverified accounts CAN sign in for 7 days after creation
- After 7 days, verification required
- Allows testing without email setup

## Implementation Phases

### Phase 1: Install Rodauth and Create Account Model

**Status**: ⚠️ **60% Complete**

**Goal**: Set up Rodauth foundation without breaking existing auth

**Tasks**:

1. Add gems to Gemfile
2. Run Rodauth generator
3. Create accounts migration
4. Create Account model
5. Add account_id to people table
6. Configure Rodauth base features

**Acceptance Criteria**:

- Rodauth routes accessible
- Account model created with proper associations
- Existing authentication still works
- All existing tests pass

---

### Phase 2: Configure Rodauth Features

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Configure email verification with environment-specific grace period

**Features to Enable**:

- `create_account` - User signup
- `verify_account` - Email verification
- `login` - Sign in
- `logout` - Sign out
- `reset_password` - Password reset
- `remember` - Remember me functionality

**Grace Period Implementation**:

```ruby
# lib/rodauth_main.rb
class RodauthMain < Rodauth::Rails::Auth
  configure do
    # ... other config ...

    # Environment-specific verification requirement
    if Rails.env.production?
      # Production: strict verification required
      require_login_confirmation? false
      verify_account_grace_period 0
    else
      # Non-production: 7-day grace period
      require_login_confirmation? false
      verify_account_grace_period 7 * 24 * 60 * 60 # 7 days in seconds
    end

    # Custom verification check
    before_login do
      if !account_from_login
        # Account doesn't exist
        next
      end

      if Rails.env.production?
        # Production: must be verified
        if account[:status] == 'unverified'
          set_redirect_error_flash verify_account_email_recently_sent_error_flash
          redirect verify_account_resend_path
        end
      else
        # Non-production: check grace period
        if account[:status] == 'unverified'
          grace_period_end = account[:created_at] + 7.days
          if Time.current > grace_period_end
            set_redirect_error_flash "Your account verification grace period has expired. Please verify your email."
            redirect verify_account_resend_path
          end
        end
      end
    end
  end
end
```

**Acceptance Criteria**:

- Production blocks unverified logins immediately
- Non-production allows 7-day grace period
- After grace period, verification required
- Tests cover both scenarios

---

### Phase 3: Implement Signup with Person Creation

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Create Account + Person in single signup flow

**Signup Form Fields**:

- Email (for Account)
- Password (for Account)
- Password Confirmation (for Account)
- Name (for Person)
- Date of Birth (for Person)

**Implementation**:

```ruby
# lib/rodauth_main.rb
after_create_account do
  # Create associated Person record
  person = Person.create!(
    account_id: account_id,
    name: param('name'),
    date_of_birth: param('date_of_birth'),
    email: account[:email],
    person_type: :adult # Default to adult, can be changed by admin
  )

  # Store person_id in session for later use
  session[:person_id] = person.id
end
```

**Views**:

- Customize Rodauth signup view to include Person fields
- Use Phlex components for consistent styling
- Add proper validation and error handling

**Acceptance Criteria**:

- Signup creates both Account and Person
- Person fields validated before account creation
- Errors displayed clearly
- System tests cover happy path and errors

---

### Phase 4: Google OAuth Integration

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Allow Google sign-in with account linking

**Setup**:

1. Add `rodauth-omniauth` gem
2. Configure Google OAuth credentials
3. Implement OAuth callbacks

**OAuth Flows**:

**New User (no existing account)**:

- Sign in with Google
- Account created automatically
- Account marked as verified (OAuth = verified email)
- Person created with name from Google
- Redirect to dashboard

**Existing Account (matching email)**:

- Sign in with Google
- Link Google identity to existing account
- If account unverified, mark as verified
- Redirect to dashboard

**Logged-in User Linking**:

- User already logged in with email/password
- Click "Link Google Account"
- OAuth flow links Google identity
- Can now sign in with either method

**Implementation**:

```ruby
# lib/rodauth_main.rb
enable :omniauth

omniauth_provider :google_oauth2,
  ENV['GOOGLE_CLIENT_ID'],
  ENV['GOOGLE_CLIENT_SECRET'],
  scope: 'email,profile'

# Handle OAuth account creation
after_omniauth_create_account do
  # Mark OAuth accounts as verified
  account_update(status: 'verified')

  # Create Person from OAuth data
  Person.create!(
    account_id: account_id,
    name: omniauth_name,
    email: omniauth_email,
    person_type: :adult
  )
end

# Handle OAuth account linking
after_omniauth_link_account do
  # If account was unverified, mark as verified
  if account[:status] == 'unverified'
    account_update(status: 'verified')
  end
end
```

**Environment Variables**:

- `GOOGLE_CLIENT_ID` - Google OAuth client ID
- `GOOGLE_CLIENT_SECRET` - Google OAuth client secret

**Acceptance Criteria**:

- New users can sign up with Google
- Existing users can link Google account
- OAuth accounts marked as verified
- No duplicate accounts created
- Tests use OmniAuth test mode

---

### Phase 5: Migration and Cleanup

**Status**: ❌ **NOT STARTED** (0%)

**Goal**: Migrate existing users and remove old auth code

**Data Migration**:

```ruby
# db/migrate/YYYYMMDDHHMMSS_migrate_users_to_accounts.rb
class MigrateUsersToAccounts < ActiveRecord::Migration[7.2]
  def up
    # Create accounts for existing users
    User.find_each do |user|
      account = Account.create!(
        email: user.email,
        password_hash: user.password_digest, # Rodauth compatible
        status: 'verified', # Existing users are verified
        created_at: user.created_at,
        updated_at: user.updated_at
      )

      # Link person to account
      if user.person
        user.person.update!(account_id: account.id)
      end
    end
  end

  def down
    # Rollback if needed
    Account.destroy_all
    Person.update_all(account_id: nil)
  end
end
```

**Cleanup Tasks**:

1. Remove `has_secure_password` from User model
2. Deprecate `SessionsController` (use Rodauth routes)
3. Deprecate `UsersController#create` (use Rodauth signup)
4. Update `Authentication` concern to use Rodauth
5. Update tests to use Rodauth helpers
6. Remove `password_digest` column from users table (optional)

**Helper Updates**:

```ruby
# app/controllers/concerns/authentication.rb
module Authentication
  extend ActiveSupport::Concern

  included do
    before_action :require_authentication
  end

  private

  def current_account
    @current_account ||= Account.find_by(id: rodauth.session_value)
  end

  def current_person
    @current_person ||= current_account&.person
  end

  # Compatibility shim (temporary)
  def current_user
    current_person
  end

  def require_authentication
    rodauth.require_authentication
  end
end
```

**Acceptance Criteria**:

- All existing users migrated to accounts
- Old authentication code removed
- All tests updated and passing
- No breaking changes for existing users
- Documentation updated

---

## Testing Strategy

### Test Coverage

**Unit Tests** (RSpec):

- Account model validations
- Account-Person associations
- Grace period logic

**Integration Tests** (RSpec):

- Rodauth configuration
- Email verification flow
- OAuth flows

**System Tests** (Capybara):

- Email/password signup
- Email verification (production mode)
- Grace period behavior (non-production mode)
- Google OAuth signup
- Google OAuth linking
- Login/logout flows

### Test Helpers

```ruby
# spec/support/rodauth_helpers.rb
module RodauthHelpers
  def sign_up_with_email(email:, password:, name:, date_of_birth:)
    visit '/create-account'
    fill_in 'Email', with: email
    fill_in 'Password', with: password
    fill_in 'Confirm Password', with: password
    fill_in 'Name', with: name
    fill_in 'Date of Birth', with: date_of_birth
    click_button 'Create Account'
  end

  def sign_in_with_email(email:, password:)
    visit '/login'
    fill_in 'Email', with: email
    fill_in 'Password', with: password
    click_button 'Login'
  end

  def verify_account(account)
    # Simulate clicking verification link
    token = account.verification_key
    visit "/verify-account?key=#{token}"
  end
end
```

---

## Security Considerations

1. **Password Security**:
   - Rodauth uses bcrypt by default
   - Minimum 8 characters enforced
   - Password confirmation required

2. **Email Verification**:
   - Tokens are cryptographically secure
   - Tokens expire after 24 hours
   - Single-use tokens

3. **OAuth Security**:
   - CSRF protection enabled
   - State parameter validated
   - Only verified OAuth providers

4. **Grace Period Security**:
   - Only in non-production environments
   - Time-based, not request-based
   - Clear expiration messaging

5. **Session Security**:
   - Secure cookies in production
   - Session fixation protection
   - Remember me tokens rotated

---

## Configuration Files

### Environment Variables

```bash
# .env.development
GOOGLE_CLIENT_ID=your_dev_client_id
GOOGLE_CLIENT_SECRET=your_dev_client_secret

# .env.production
GOOGLE_CLIENT_ID=your_prod_client_id
GOOGLE_CLIENT_SECRET=your_prod_client_secret
```

### Email Configuration

```ruby
# config/environments/development.rb
config.action_mailer.delivery_method = :letter_opener
config.action_mailer.default_url_options = { host: 'localhost', port: 3000 }

# config/environments/production.rb
config.action_mailer.delivery_method = :smtp
config.action_mailer.smtp_settings = {
  address: 'smtp.postmarkapp.com',
  port: 587,
  user_name: Rails.application.credentials.dig(:postmark, :api_token),
  password: Rails.application.credentials.dig(:postmark, :api_token),
  authentication: 'plain',
  enable_starttls_auto: true
}
config.action_mailer.default_url_options = {
  host: ENV['APP_HOSTNAME'],
  protocol: 'https'
}
```

---

## Rollout Plan

### Development Phase

1. Implement on feature branch
2. Test with fixtures
3. Code review
4. Merge to main

### Staging Phase

1. Deploy to staging
2. Test all flows manually
3. Verify grace period behavior
4. Test OAuth with real Google credentials

### Production Phase

1. Run data migration (off-peak hours)
2. Monitor error rates
3. Verify email delivery
4. Monitor OAuth success rates
5. Be ready to rollback if issues

---

## Success Metrics

- [x] All existing tests pass
- [ ] New Rodauth tests pass
- [ ] Email verification works in production
- [ ] Grace period works in non-production
- [ ] Google OAuth works
- [ ] No duplicate accounts created
- [x] Existing users can still log in (via legacy auth)
- [ ] Documentation complete

---

## References

- [Rodauth Documentation](https://rodauth.jeremyevans.net/)
- [rodauth-rails Gem](https://github.com/janko/rodauth-rails)
- [rodauth-omniauth Gem](https://github.com/janko/rodauth-omniauth)
- Issue #118: Add sign up
- PR #119: Previous implementation (closed)
- USER_MANAGEMENT_PLAN.md: Overall user management strategy
