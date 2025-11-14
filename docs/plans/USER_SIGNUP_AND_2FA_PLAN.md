# User Signup and 2FA Implementation Plan

## Current State

### What's Already Implemented

**From PR #119 (Rodauth Foundation):**
- Rodauth-rails with email verification feature
- Google OAuth integration via rodauth-omniauth
- `accounts` table (separate from `people`)
- Account/Person association (Account has_one Person)
- Email verification flow with views
- Password reset flow with mailer views
- Remember me feature
- Auto-account creation for OAuth users
- Account linking (OAuth to existing email accounts)
- All Rodauth views styled with Tailwind CSS

**Existing Infrastructure:**
- Action Mailer configured
- Audit logging with paper_trail
- Session tracking with IP addresses

### What's Missing

- Email verification on signup
- Email confirmation before account activation
- 2FA (Two-Factor Authentication)
- Email notification improvements
- Production email delivery setup

## Goals

1. **Email Verification**: Require users to verify their email before
   accessing the system
2. **Password Reset**: Enhance existing password reset flow with proper
   email delivery
3. **2FA**: Add optional/mandatory two-factor authentication using TOTP
   (Time-based One-Time Password)
4. **Email Delivery**: Set up reliable email sending for development and production

---

## ✅ DECISION MADE: Rodauth (PR #119)

**PR #119 already implements Rodauth with:**
- Email verification on signup
- Password reset flow
- Google OAuth integration
- Account/Person separation
- All necessary views and mailers

**What we're adding:**
- Postmark for production email delivery
- 2FA with TOTP (rodauth-otp feature)
- Email template branding

---

## Decision Point #2: Email Service Provider (Production)

### Requirements

- Reliable delivery
- UK/EU data residency (for GDPR compliance)
- API for transactional emails
- Reasonable pricing for healthcare app scale
- Good deliverability rates

### Options

#### Option A: Postmark

**Pros:**

- Excellent deliverability
- UK/EU servers available
- Simple pricing (£10/month for 10k emails)
- Great for transactional emails
- Good documentation

**Pricing:** £10/mo (10k emails), £50/mo (100k emails)

#### Option B: SendGrid

**Pros:**

- Popular choice
- Free tier (100 emails/day)
- More features (marketing emails, etc.)

**Cons:**

- More complex than needed
- Deliverability issues in past
- US-based (data residency concerns)

**Pricing:** Free (100/day), £15/mo (40k emails)

#### Option C: AWS SES

**Pros:**

- Extremely cheap (£0.10 per 1k emails)
- Scales well
- Good if already using AWS

**Cons:**

- Requires AWS setup/knowledge
- More configuration needed
- Deliverability requires reputation building

**Pricing:** £0.10 per 1k emails

#### Option D: Mailgun

**Pros:**

- EU region available
- Good documentation
- Popular choice

**Cons:**

- Recent ownership changes
- More expensive than alternatives

**Pricing:** £35/mo (50k emails)

### ✅ DECISION MADE: Postmark

**Chosen for:**
- UK/EU data residency
- Excellent deliverability
- Simple transactional email focus
- Healthcare compliance friendly
- £10/mo for 10k emails

---

## Implementation Plan

### Phase 0: Merge PR #119 (1 day)

#### 0.1 Review and Test PR #119

- Review Rodauth configuration in `lib/rodauth_config.rb`
- Test email verification flow locally
- Test Google OAuth flow (with test credentials)
- Test password reset flow
- Verify Account/Person association working correctly

#### 0.2 Resolve Merge Conflicts

- PR #119 shows `mergeable_state: 'dirty'`
- Resolve conflicts with main branch
- Run full test suite
- Fix any breaking changes

**Acceptance Criteria:**

- [ ] PR #119 tests pass
- [ ] No merge conflicts
- [ ] Email verification works locally
- [ ] Password reset works locally
- [ ] Account/Person association tested

---

### Phase 1: Postmark Email Setup (1 day)

#### 1.1 Postmark Account Setup

- Create Postmark account
- Verify sender email domain
- Generate API token
- Add to Rails encrypted credentials

#### 1.2 Development Email Preview

- Add `letter_opener` gem to development group
- Configure for Rodauth mailer
- Test with verification & reset emails

#### 1.3 Production Email Configuration

```ruby
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

#### 1.4 Email Template Branding

- Update `app/views/rodauth_mailer/*.text.erb` with MedTracker branding
- Add HTML email templates (optional)
- Test deliverability with Postmark's spam checker

**Acceptance Criteria:**

- [ ] Postmark account configured
- [ ] Development emails preview in browser
- [ ] Production emails send via Postmark
- [ ] Email templates branded
- [ ] Deliverability tested

---

### Phase 2: Email Verification on Signup (2-3 days)

#### 2.1 Database Changes

Add email verification fields to users table:

```ruby
add_column :users, :email_verified, :boolean, default: false, null: false
add_column :users, :email_verification_token, :string
add_column :users, :email_verification_sent_at, :datetime
add_index :users, :email_verification_token, unique: true
```

#### 2.2 Model Updates

- Generate secure verification token (using `SecureRandom.urlsafe_base64`)
- Add token expiration logic (24 hours default)
- Add `verified?` method
- Prevent login if not verified (grace period option?)

#### 2.3 Mailer Implementation

Create `UserMailer` with:

- `verification_email` - Send verification link on signup
- `verification_reminder` - Resend verification (if needed)

#### 2.4 Controller Updates

- `UsersController#create`: Send verification email instead of auto-login
- Add `EmailVerificationsController`:
  - `#show` - Verify token and activate account
  - `#create` - Resend verification email

#### 2.5 View Updates

- Signup success page explaining verification needed
- Verification success page with login link
- "Resend verification" option

#### 2.6 Routes

```ruby
resources :email_verifications, only: [:show, :create]
get 'verify/:token', to: 'email_verifications#show', as: :verify_email
```

**Acceptance Criteria:**

- [ ] New users receive verification email immediately
- [ ] Users cannot login until email verified
- [ ] Verification links expire after 24 hours
- [ ] Users can resend verification email
- [ ] System tests cover happy path and error cases
- [ ] Verified status tracked in audit logs

---

### Phase 3: Password Reset Enhancements (1 day)

#### 3.1 Current Implementation Review

- Verify `PasswordsMailer.reset` works correctly
- Check token security (using `signed_id`?)
- Add token expiration if missing

#### 3.2 Improvements Needed

- Add rate limiting (already exists on sessions, extend to passwords)
- Token expiration (4 hours recommended)
- Email notification on successful password change
- Track password reset in audit logs

#### 3.3 Database Changes (if needed)

```ruby
add_column :users, :password_reset_sent_at, :datetime
```

#### 3.4 Additional Mailer

- `PasswordsMailer.changed` - Notify user of password change

**Acceptance Criteria:**

- [ ] Password reset tokens expire after 4 hours
- [ ] Rate limiting prevents abuse
- [ ] Users notified when password changes
- [ ] All password actions logged in audit trail
- [ ] Tests cover token expiration and security

---

### Phase 4: Two-Factor Authentication (3-4 days)

#### 4.1 Gem Selection

**Option A: `rotp` + Custom Implementation** ⭐ RECOMMENDED

- Lightweight TOTP library
- ~200KB, no dependencies
- Full control over UI/UX
- Integrates with existing auth

```ruby
gem 'rotp' # For TOTP generation/validation
gem 'rqrcode' # For QR code generation
```

**Option B: `devise-two-factor`**

- Only if migrating to Devise
- More features but requires Devise

**⚠️ DECISION NEEDED:** Gem choice depends on Decision Point #1

#### 4.2 Database Schema

```ruby
create_table :user_two_factor_settings do |t|
  t.references :user, null: false, foreign_key: true
  t.string :otp_secret, null: false # Encrypted by Rails
  t.boolean :enabled, default: false
  t.datetime :enabled_at
  t.text :backup_codes # Encrypted, array serialized
  t.datetime :backup_codes_generated_at
  t.timestamps
end
```

Use Rails 7+ encrypted attributes:

```ruby
class UserTwoFactorSetting < ApplicationRecord
  encrypts :otp_secret
  encrypts :backup_codes
end
```

#### 4.3 Features to Implement

**Setup Flow:**

1. User enables 2FA from settings
2. Generate TOTP secret
3. Display QR code for authenticator app (Google Authenticator, Authy, etc.)
4. Require user to enter code to confirm setup
5. Generate 10 backup codes
6. Force user to save backup codes before enabling

**Login Flow:**

1. Regular email/password authentication
2. If 2FA enabled, redirect to OTP entry page
3. Validate TOTP code (30-second window)
4. Option to use backup code instead
5. Track used backup codes

**Management:**

- View 2FA status in settings
- Disable 2FA (with password confirmation)
- Regenerate backup codes (invalidates old ones)
- View backup codes (with password re-authentication)

#### 4.4 Implementation Components

**Models:**

```ruby
# app/models/user_two_factor_setting.rb
class UserTwoFactorSetting < ApplicationRecord
  belongs_to :user

  encrypts :otp_secret
  encrypts :backup_codes

  serialize :backup_codes, Array

  validates :otp_secret, presence: true

  def verify_code(code)
    totp = ROTP::TOTP.new(otp_secret)
    totp.verify(code, drift_behind: 30, drift_ahead: 30)
  end

  def generate_backup_codes
    Array.new(10) { SecureRandom.hex(4).upcase }
  end

  def verify_backup_code(code)
    return false unless backup_codes.include?(code)
    backup_codes.delete(code)
    save!
    true
  end
end

# app/models/user.rb
class User < ApplicationRecord
  has_one :two_factor_setting, class_name: 'UserTwoFactorSetting', dependent: :destroy

  def two_factor_enabled?
    two_factor_setting&.enabled?
  end
end
```

**Controllers:**

```ruby
# app/controllers/two_factor_settings_controller.rb
class TwoFactorSettingsController < ApplicationController
  def new
    # Show QR code setup page
  end

  def create
    # Enable 2FA after confirming TOTP code
  end

  def destroy
    # Disable 2FA
  end
end

# app/controllers/two_factor_authentications_controller.rb
class TwoFactorAuthenticationsController < ApplicationController
  allow_unauthenticated_access

  def new
    # Show OTP entry page
  end

  def create
    # Verify OTP and complete login
  end
end
```

**Views:**

- Setup page with QR code
- Backup codes display (one-time view)
- OTP entry form during login
- Settings page for managing 2FA

#### 4.5 Security Considerations

- Encrypt `otp_secret` using Rails encrypted attributes
- Use time-based codes (30-second window)
- Allow drift (±30 seconds) for clock differences
- Backup codes must be one-time use
- Force backup code save before enabling 2FA
- Log all 2FA events in audit trail
- Require password re-authentication for 2FA changes

#### 4.6 User Experience

- Clear instructions during setup
- Support for popular authenticator apps
- Backup codes as PDF download
- "Remember this device" option (optional feature)
- Recovery flow if 2FA device lost

**Acceptance Criteria:**

- [ ] Users can enable 2FA from settings
- [ ] QR code generated for authenticator apps
- [ ] Login requires TOTP code when 2FA enabled
- [ ] Backup codes work as alternative
- [ ] Backup codes are one-time use
- [ ] 2FA can be disabled with password confirmation
- [ ] All 2FA events logged in audit trail
- [ ] System tests cover setup, login, and backup code usage
- [ ] Documentation for users on how to use 2FA

---

### Phase 5: Testing & Documentation (1-2 days)

#### 5.1 Test Coverage

- Unit tests for all new models
- Controller tests for new endpoints
- System tests for complete flows
- Email delivery tests
- Security tests (token expiration, brute force, etc.)

#### 5.2 Documentation

- Update README with email setup instructions
- Document 2FA setup for administrators
- Create user guide for 2FA
- Update deployment documentation
- Add ADR for authentication decisions

#### 5.3 Security Audit

- Review token generation methods
- Check rate limiting on all auth endpoints
- Verify encrypted attributes are working
- Test account recovery flows
- Penetration testing (optional)

**Acceptance Criteria:**

- [ ] Test coverage >90% for new code
- [ ] All documentation updated
- [ ] Security checklist completed
- [ ] Team trained on new features

---

## Migration Path (If Choosing Rodauth)

If deciding on Option B from Decision Point #1, follow this migration path:

### Step 1: Add Rodauth

```ruby
gem 'rodauth-rails'
```

### Step 2: Generate Rodauth Configuration

```bash
rails generate rodauth:install
```

### Step 3: Enable Required Features

```ruby
# config/initializers/rodauth.rb
class RodauthApp < Rodauth::Rails::App
  route do |r|
    rodauth.load_memory
    r.rodauth
  end
end

# lib/rodauth_config.rb
class RodauthConfig < Rodauth::Rails::Auth
  configure do
    enable :login, :logout, :create_account,
           :verify_account, :reset_password,
           :otp, :recovery_codes

    # Use existing User model
    accounts_table :users

    # Configure email
    send_email(&:deliver_later)

    # Configure OTP
    otp_digits 6
    otp_period 30
  end
end
```

### Step 4: Migrate Existing Users

- Create migration script for existing users
- Add Rodauth columns to users table
- Test migration with fixtures

### Step 5: Update Tests

- Replace custom auth test helpers
- Update system tests for new flows

**Migration Effort:** 3-5 days

---

## Timeline Estimates

### Option A: Custom Implementation (Recommended)

- **Phase 1 (Email):** 1-2 days
- **Phase 2 (Verification):** 2-3 days
- **Phase 3 (Password):** 1 day
- **Phase 4 (2FA):** 3-4 days
- **Phase 5 (Testing):** 1-2 days
- **Total:** 8-12 days

### Option B: Rodauth Migration

- **Migration:** 3-5 days
- **Testing & Documentation:** 2-3 days
- **Total:** 5-8 days (but all features included)

---

## Dependencies & Gems

### Required Gems

```ruby
# Gemfile

# Email preview in development
group :development do
  gem 'letter_opener' # Preview emails in browser
end

# For custom implementation (Option A)
gem 'rotp'    # TOTP generation and validation
gem 'rqrcode' # QR code generation for 2FA setup

# For Rodauth implementation (Option B)
# gem 'rodauth-rails' # Comprehensive auth solution
```

### No New Gems Needed For

- Email sending (Action Mailer built-in)
- Token generation (SecureRandom built-in)
- Encryption (Rails encrypted attributes built-in)

---

## Configuration Files to Update

### Development Environment

```ruby
# config/environments/development.rb
config.action_mailer.delivery_method = :letter_opener
config.action_mailer.perform_deliveries = true
config.action_mailer.default_url_options = { host: 'localhost', port: 3000 }
```

### Test Environment

```ruby
# config/environments/test.rb
config.action_mailer.delivery_method = :test
config.action_mailer.default_url_options = { host: 'example.com' }
```

### Production Environment

```ruby
# config/environments/production.rb
config.action_mailer.delivery_method = :smtp
config.action_mailer.smtp_settings = {
  address: Rails.application.credentials.dig(:smtp, :address),
  port: Rails.application.credentials.dig(:smtp, :port),
  user_name: Rails.application.credentials.dig(:smtp, :username),
  password: Rails.application.credentials.dig(:smtp, :password),
  authentication: 'plain',
  enable_starttls_auto: true
}
config.action_mailer.default_url_options = {
  host: ENV['APP_HOSTNAME'] || 'medtracker.example.com',
  protocol: 'https'
}
```

---

## Security Checklist

### Email Verification Security

- [ ] Tokens are cryptographically random
- [ ] Tokens expire after reasonable time (24 hours)
- [ ] Tokens are single-use
- [ ] Email addresses validated and normalized
- [ ] Rate limiting on verification email sending

### Password Reset

- [ ] Reset tokens expire (4 hours)
- [ ] Reset tokens are single-use
- [ ] Old password invalidated on reset
- [ ] User notified of password changes
- [ ] Rate limiting on reset requests

### Two-Factor Authentication

- [ ] OTP secrets encrypted at rest
- [ ] Backup codes encrypted at rest
- [ ] Backup codes are one-time use
- [ ] Time-based codes prevent replay attacks
- [ ] Recovery flow documented
- [ ] 2FA disable requires password confirmation

### General

- [ ] All auth events logged in audit trail
- [ ] HTTPS enforced in production
- [ ] CSRF protection enabled
- [ ] Session hijacking prevention (IP tracking exists)
- [ ] Brute force protection (rate limiting)

---

## Compliance Considerations

### GDPR (UK/EU)

- [ ] Email verification is legitimate interest
- [ ] Users can export their data
- [ ] Users can request deletion
- [ ] Email provider has DPA (Data Processing Agreement)
- [ ] Data stored in UK/EU (if using EU servers)

### Healthcare Context

- [ ] Strong authentication required (2FA helps)
- [ ] Audit trail for all auth events (already exists)
- [ ] Password policy enforced (min 8 chars exists)
- [ ] Account recovery process documented
- [ ] Consider mandatory 2FA for healthcare staff

---

## Rollout Strategy

### Development Phase

1. Implement in feature branch
2. Test with development fixtures
3. Code review with security focus
4. Update documentation

### Staging Phase

1. Deploy to staging environment
2. Configure real email provider (test mode)
3. Test complete user journeys
4. Load testing for email sending

### Production Rollout

1. **Soft Launch**: Make features optional initially
2. **Email Verification**: Required for new signups only
3. **2FA**: Optional for all users
4. **Monitor**: Watch for issues, email deliverability
5. **Communicate**: Email existing users about new features
6. **Enforce**: Consider mandatory 2FA for admin/medical roles

### Monitoring

- Email delivery success rates
- Verification completion rates
- 2FA adoption rates
- Failed authentication attempts
- User support requests

---

## Open Questions & Risks

### Questions for Discussion

1. **Grace period for email verification?** Should users have X days to verify
   before account locked?
2. **Mandatory 2FA?** Should administrators/doctors be required to use 2FA?
3. **Remember device?** Should we implement "trust this device for 30 days" for 2FA?
4. **Email branding:** Do we have email templates designed?
5. **Regulatory review:** Does this need NHS Digital/CQC review?

### Known Risks

- **Email deliverability**: Transactional emails may go to spam initially
- **User friction**: Email verification adds signup friction
- **Support burden**: Users may need help with 2FA setup
- **Device loss**: Need clear recovery process for lost 2FA devices
- **Migration complexity**: If choosing Rodauth, existing users need migration

### Mitigation Strategies

- Start with optional features
- Provide excellent documentation
- Build recovery flows upfront
- Monitor adoption and issues closely
- Have support plan ready

---

## Success Metrics

### Email Verification Metrics

- Verification completion rate >85%
- Average time to verify <2 hours
- Email delivery success rate >98%

### 2FA Adoption

- Initial target: 30% of users within 3 months
- Admin/medical staff: 100% within 6 months
- Failed 2FA login attempts <5%

### Security

- Zero successful account takeovers
- Zero token-related vulnerabilities
- Password reset abuse attempts blocked

### User Experience

- Support tickets <5% of new signups
- Setup completion time <3 minutes for 2FA
- User satisfaction score >4/5

---

## Related Documentation

- [Authentication Architecture Decision Record](
  ../adrs/0XXX-authentication-strategy.md) _(to be created)_
- [User Management Plan](./USER_MANAGEMENT_PLAN.md)
- [UK Regulatory Compliance Plan](../uk-regulatory-compliance-plan.md)
- [Audit Trail Documentation](../audit-trail.md)

---

## Next Steps

1. **Review this plan** with team
2. **Make decisions** on Decision Points #1 and #2
3. **Set up email provider** account (test mode)
4. **Create feature branch** for implementation
5. **Start with Phase 1** (email infrastructure)

---

**Document Status:** Draft for Review
**Last Updated:** {{ current_date }}
**Owner:** Development Team
**Reviewers Needed:** Product, Security, Compliance
